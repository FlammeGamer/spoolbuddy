//! Backend Client with C-callable interface
//!
//! Provides HTTP polling to the SpoolBuddy backend server for printer status.
//! Uses mDNS to discover the server automatically.

use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use log::{info, warn};
use serde::Deserialize;
use std::ffi::{c_char, c_int};
use std::sync::Mutex;
use embedded_svc::http::client::Client as HttpClient;

/// Maximum number of printers to cache (reduced for memory)
const MAX_PRINTERS: usize = 4;

/// Maximum number of AMS units per printer
const MAX_AMS_UNITS: usize = 4;

/// HTTP timeout in milliseconds
const HTTP_TIMEOUT_MS: u64 = 5000;

/// Backend connection state
#[derive(Debug, Clone, PartialEq)]
pub enum BackendState {
    Disconnected,
    Discovering,
    Connected { ip: [u8; 4], port: u16 },
    #[allow(dead_code)]
    Error(String),
}

/// AMS tray from backend API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiAmsTray {
    #[serde(rename = "ams_id")]
    _ams_id: i32,
    #[serde(rename = "tray_id")]
    _tray_id: i32,
    tray_type: Option<String>,
    tray_color: Option<String>,  // RGBA hex (e.g., "FF0000FF")
    remain: Option<i32>,         // 0-100 percentage, or negative if unknown
}

/// AMS unit from backend API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiAmsUnit {
    id: i32,
    humidity: Option<i32>,
    temperature: Option<f32>,
    extruder: Option<i32>,  // 0=right, 1=left
    trays: Vec<ApiAmsTray>,
}

/// Printer status from backend API
#[derive(Debug, Clone, Deserialize)]
struct ApiPrinter {
    serial: String,
    name: Option<String>,
    ip_address: Option<String>,
    access_code: Option<String>,
    connected: bool,
    gcode_state: Option<String>,
    print_progress: Option<u8>,
    subtask_name: Option<String>,
    mc_remaining_time: Option<u16>,
    cover_url: Option<String>,
    stg_cur: Option<i8>,           // Current stage number (-1 = idle)
    stg_cur_name: Option<String>,  // Human-readable stage name
    #[serde(default)]
    ams_units: Vec<ApiAmsUnit>,
    tray_now: Option<i32>,
    tray_now_left: Option<i32>,
    tray_now_right: Option<i32>,
    active_extruder: Option<i32>,  // 0=right, 1=left, None=unknown
}

/// Time response from backend API
#[derive(Debug, Clone, Deserialize)]
struct ApiTime {
    hour: u8,
    minute: u8,
}

/// Cached AMS tray info
#[derive(Debug, Clone, Copy, Default)]
struct CachedAmsTray {
    tray_type: [u8; 16],    // Material type
    tray_color: u32,        // RGBA packed (0xRRGGBBAA)
    remain: u8,             // 0-100 percentage
}

/// Cached AMS unit info
#[derive(Debug, Clone, Copy)]
struct CachedAmsUnit {
    id: i32,
    humidity: i32,          // -1 if not available
    temperature: i16,       // Celsius * 10, -1 if not available
    extruder: i8,           // -1 if not available, 0=right, 1=left
    tray_count: u8,
    trays: [CachedAmsTray; 4],
}

impl Default for CachedAmsUnit {
    fn default() -> Self {
        Self {
            id: 0,
            humidity: -1,
            temperature: -1,
            extruder: -1,
            tray_count: 0,
            trays: [CachedAmsTray::default(); 4],
        }
    }
}

/// Cached printer info (internal)
#[derive(Debug, Clone)]
struct CachedPrinter {
    name: [u8; 32],
    serial: [u8; 20],
    ip_address: [u8; 20],
    access_code: [u8; 16],
    connected: bool,
    gcode_state: [u8; 16],
    print_progress: u8,
    subtask_name: [u8; 64],
    remaining_time_min: u16,
    stg_cur: i8,            // Current stage number (-1 = idle)
    stg_cur_name: [u8; 48], // Human-readable stage name
    // AMS data
    ams_unit_count: u8,
    ams_units: [CachedAmsUnit; MAX_AMS_UNITS],
    tray_now: i32,          // -1 if not available
    tray_now_left: i32,     // -1 if not available
    tray_now_right: i32,    // -1 if not available
    active_extruder: i32,   // -1 if not available, 0=right, 1=left
}

impl Default for CachedPrinter {
    fn default() -> Self {
        Self {
            name: [0; 32],
            serial: [0; 20],
            ip_address: [0; 20],
            access_code: [0; 16],
            connected: false,
            gcode_state: [0; 16],
            print_progress: 0,
            subtask_name: [0; 64],
            remaining_time_min: 0,
            stg_cur: -1,
            stg_cur_name: [0; 48],
            ams_unit_count: 0,
            ams_units: [CachedAmsUnit::default(); MAX_AMS_UNITS],
            tray_now: -1,
            tray_now_left: -1,
            tray_now_right: -1,
            active_extruder: -1,
        }
    }
}

/// Backend manager state
struct BackendManager {
    state: BackendState,
    server_url: String,
    printers: [CachedPrinter; MAX_PRINTERS],
    printer_count: usize,
}

const EMPTY_AMS_TRAY: CachedAmsTray = CachedAmsTray {
    tray_type: [0; 16],
    tray_color: 0,
    remain: 0,
};

const EMPTY_AMS_UNIT: CachedAmsUnit = CachedAmsUnit {
    id: 0,
    humidity: -1,
    temperature: -1,
    extruder: -1,
    tray_count: 0,
    trays: [EMPTY_AMS_TRAY; 4],
};

const EMPTY_PRINTER: CachedPrinter = CachedPrinter {
    name: [0; 32],
    serial: [0; 20],
    ip_address: [0; 20],
    access_code: [0; 16],
    connected: false,
    gcode_state: [0; 16],
    print_progress: 0,
    subtask_name: [0; 64],
    stg_cur_name: [0; 48],
    remaining_time_min: 0,
    stg_cur: -1,
    ams_unit_count: 0,
    ams_units: [EMPTY_AMS_UNIT; MAX_AMS_UNITS],
    tray_now: -1,
    tray_now_left: -1,
    tray_now_right: -1,
    active_extruder: -1,
};

impl BackendManager {
    const fn new() -> Self {
        Self {
            state: BackendState::Disconnected,
            server_url: String::new(),
            printers: [EMPTY_PRINTER; MAX_PRINTERS],
            printer_count: 0,
        }
    }
}

// Global backend manager
static BACKEND_MANAGER: Mutex<BackendManager> = Mutex::new(BackendManager::new());

// Cover image storage (max 64KB for thumbnail)
const MAX_COVER_SIZE: usize = 65536;
static COVER_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static COVER_VALID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAST_COVER_URL: Mutex<String> = Mutex::new(String::new());

/// Initialize the backend client
pub fn init() {
    info!("Backend client initialized");
}

/// Set the backend server URL manually
pub fn set_server_url(url: &str) {
    let mut manager = BACKEND_MANAGER.lock().unwrap();
    manager.server_url = url.to_string();

    // Parse IP from URL for status
    if let Some(ip_str) = url.strip_prefix("http://") {
        if let Some(ip_port) = ip_str.split('/').next() {
            if let Some(ip_only) = ip_port.split(':').next() {
                if let Ok(ip) = ip_only.parse::<std::net::Ipv4Addr>() {
                    let octets = ip.octets();
                    let port = ip_port.split(':').nth(1)
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(3000);
                    manager.state = BackendState::Connected { ip: octets, port };
                    info!("Backend server set to: {}", url);
                    return;
                }
            }
        }
    }

    manager.state = BackendState::Disconnected;
    warn!("Failed to parse server URL: {}", url);
}

/// Poll the backend server for printer status and time
/// Called from main loop every ~2 seconds
pub fn poll_backend() {
    let manager = BACKEND_MANAGER.lock().unwrap();

    // Check if we have a server URL
    if manager.server_url.is_empty() {
        return;
    }

    let base_url = manager.server_url.clone();
    drop(manager); // Release lock before HTTP calls

    // Send heartbeat to indicate display is connected
    send_heartbeat(&base_url);

    // Send current scale weight to backend (so other clients can see it)
    let weight = crate::scale_manager::scale_get_weight();
    let stable = crate::scale_manager::scale_is_stable();
    send_device_state(None, weight, stable);

    // Fetch printers
    let printers_url = format!("{}/api/printers", base_url);
    let mut cover_url_to_fetch: Option<String> = None;

    match fetch_printers(&printers_url) {
        Ok(printers) => {
            // Check if cover URL changed before updating cache
            cover_url_to_fetch = check_cover_url_changed(&printers, &base_url);

            let mut manager = BACKEND_MANAGER.lock().unwrap();
            update_printer_cache(&mut manager, &printers);
        }
        Err(e) => {
            warn!("Failed to fetch printers: {}", e);
        }
    }

    // Fetch cover image if URL changed (outside of lock)
    if let Some(url) = cover_url_to_fetch {
        fetch_cover_image(&url);
    }

    // Fetch time from backend
    fetch_and_set_time(&base_url);
}

/// Get WiFi status parameters for backend state updates
/// Returns URL query string fragment like "&wifi_state=3&wifi_ssid=MyNetwork&wifi_ip=192.168.1.50&wifi_rssi=-45"
fn get_wifi_params() -> String {
    let mut status = crate::wifi_manager::WifiStatus {
        state: 0,
        ip: [0, 0, 0, 0],
        rssi: 0,
    };

    crate::wifi_manager::wifi_get_status(&mut status as *mut _);

    if status.state == 0 {
        // Uninitialized - don't send WiFi params
        return String::new();
    }

    let mut params = format!("&wifi_state={}", status.state);

    // Get SSID if connected
    if status.state == 3 {
        let mut ssid_buf = [0u8; 33];
        let ssid_len = crate::wifi_manager::wifi_get_ssid(ssid_buf.as_mut_ptr() as *mut c_char, 33);
        if ssid_len > 0 {
            // Convert buffer to string (find null terminator)
            let ssid = ssid_buf.iter()
                .position(|&b| b == 0)
                .map(|end| String::from_utf8_lossy(&ssid_buf[..end]).to_string())
                .unwrap_or_default();
            if !ssid.is_empty() {
                // URL encode the SSID
                let encoded_ssid = ssid.replace(' ', "%20").replace('#', "%23");
                params.push_str(&format!("&wifi_ssid={}", encoded_ssid));
            }
        }

        // Add IP address
        params.push_str(&format!("&wifi_ip={}.{}.{}.{}",
            status.ip[0], status.ip[1], status.ip[2], status.ip[3]));

        // Add RSSI
        params.push_str(&format!("&wifi_rssi={}", status.rssi));
    }

    params
}

// External C function to shutdown display before reboot
extern "C" {
    fn display_shutdown();
}

/// Send heartbeat to backend to indicate display is connected
/// Also checks for pending commands (e.g., reboot)
/// Includes WiFi status so backend always has current network info
fn send_heartbeat(base_url: &str) {
    use esp_idf_sys::esp_restart;

    let version = env!("CARGO_PKG_VERSION");
    let update_available = crate::ota_manager::is_update_available();
    let wifi_params = get_wifi_params();
    let url = format!(
        "{}/api/display/heartbeat?version={}&update_available={}{}",
        base_url, version, update_available, wifi_params
    );

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(2000)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut client = HttpClient::wrap(connection);

    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return,
    };

    // Read response to check for commands
    let mut buf = [0u8; 256];
    if let Ok(n) = response.read(&mut buf) {
        if n > 0 {
            let body = String::from_utf8_lossy(&buf[..n]);
            // Check for update command (triggers OTA)
            if body.contains("\"command\":\"update\"") || body.contains("\"command\": \"update\"") {
                log::info!("Received update command from backend - starting OTA");
                if let Err(e) = crate::ota_manager::perform_update(base_url) {
                    log::error!("OTA update failed: {}", e);
                }
                // perform_update reboots on success, so we only get here on failure
            }
            // Check for reboot command
            else if body.contains("\"command\":\"reboot\"") || body.contains("\"command\": \"reboot\"") {
                log::info!("Received reboot command from backend");
                // Properly shutdown display before reboot to prevent display shift
                unsafe { display_shutdown(); }
                std::thread::sleep(std::time::Duration::from_millis(100));
                unsafe { esp_restart(); }
            }
            // Check for scale tare command
            else if body.contains("\"command\":\"scale_tare\"") || body.contains("\"command\": \"scale_tare\"") {
                log::info!("Received scale_tare command from backend");
                let result = crate::scale_manager::scale_tare();
                log::info!("Scale tare result: {}", result);
            }
            // Check for scale calibrate command (e.g., "scale_calibrate:100.0")
            else if body.contains("\"command\":\"scale_calibrate:") || body.contains("\"command\": \"scale_calibrate:") {
                log::info!("Detected scale_calibrate command in response body");
                // Extract the weight value from command
                if let Some(start) = body.find("scale_calibrate:") {
                    let after_cmd = &body[start + 16..];
                    // Find end of weight value (quote or whitespace)
                    let end = after_cmd.find(|c: char| c == '"' || c.is_whitespace()).unwrap_or(after_cmd.len());
                    let weight_str = &after_cmd[..end];
                    log::info!("Parsing weight value: '{}'", weight_str);
                    if let Ok(known_weight) = weight_str.parse::<f32>() {
                        log::info!("Received scale_calibrate command from backend: {}g", known_weight);
                        let result = crate::scale_manager::scale_calibrate(known_weight);
                        log::info!("Scale calibrate result: {} (0=success, -1=error)", result);
                    } else {
                        log::warn!("Failed to parse weight value: '{}'", weight_str);
                    }
                } else {
                    log::warn!("Could not find scale_calibrate: in body");
                }
            }
            // Check for scale reset command
            else if body.contains("\"command\":\"scale_reset\"") || body.contains("\"command\": \"scale_reset\"") {
                log::info!("Received scale_reset command from backend");
                let result = crate::scale_manager::scale_reset_calibration();
                log::info!("Scale reset result: {}", result);
            }
        }
    }
}

/// Send device state to backend (weight, tag, WiFi) and receive decoded tag data
/// Returns true if tag data was received and set
pub fn send_device_state(tag_uid_hex: Option<&str>, weight: f32, stable: bool) -> bool {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if manager.server_url.is_empty() {
        return false;
    }
    let base_url = manager.server_url.clone();
    drop(manager);

    // Get WiFi status to include in state update
    let wifi_params = get_wifi_params();

    // Build URL with query params, including decoded tag data if available
    let url = if let Some(tag_id) = tag_uid_hex {
        // Get decoded tag data from NFC manager
        let vendor = crate::nfc_bridge_manager::get_tag_vendor();
        let material = crate::nfc_bridge_manager::get_tag_material();
        let subtype = crate::nfc_bridge_manager::get_tag_subtype();
        let color = crate::nfc_bridge_manager::get_tag_color_name();
        let color_rgba = crate::nfc_bridge_manager::get_tag_color_rgba();
        let spool_weight = crate::nfc_bridge_manager::get_tag_spool_weight();
        let tag_type = crate::nfc_bridge_manager::get_tag_type();

        if !vendor.is_empty() {
            // Include decoded tag data (simple URL encoding - replace spaces with %20)
            let encode = |s: &str| s.replace(' ', "%20").replace('#', "%23");
            format!(
                "{}/api/display/state?weight={:.1}&stable={}&tag_id={}&tag_vendor={}&tag_material={}&tag_subtype={}&tag_color={}&tag_color_rgba={}&tag_weight={}&tag_type={}{}",
                base_url, weight, stable, tag_id,
                encode(&vendor),
                encode(&material),
                encode(&subtype),
                encode(&color),
                color_rgba,
                spool_weight,
                encode(&tag_type),
                wifi_params
            )
        } else {
            // Just send tag_id without decoded data
            format!(
                "{}/api/display/state?weight={:.1}&stable={}&tag_id={}{}",
                base_url, weight, stable, tag_id, wifi_params
            )
        }
    } else {
        format!(
            "{}/api/display/state?weight={:.1}&stable={}{}",
            base_url, weight, stable, wifi_params
        )
    };

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(3000)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut client = HttpClient::wrap(connection);

    // POST request
    let request = match client.post(&url, &[]) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let response = match request.submit() {
        Ok(r) => r,
        Err(_) => return false,
    };

    if response.status() != 200 {
        return false;
    }

    // If we have a tag, fetch decoded data from display/status
    if tag_uid_hex.is_some() {
        fetch_decoded_tag_data(&base_url);
        return true;
    }

    false
}

/// Fetch decoded tag data from backend
fn fetch_decoded_tag_data(base_url: &str) {
    let url = format!("{}/api/display/status", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(3000)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut client = HttpClient::wrap(connection);

    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return,
    };

    if response.status() != 200 {
        return;
    }

    // Read response
    let mut body = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(_) => return,
        }
    }

    // Parse JSON and extract tag_data
    #[derive(Deserialize)]
    struct DisplayStatus {
        tag_data: Option<TagData>,
    }

    #[derive(Deserialize)]
    struct TagData {
        vendor: Option<String>,
        material: Option<String>,
        subtype: Option<String>,
        color_name: Option<String>,
        color_rgba: Option<u32>,
        spool_weight: Option<i32>,
        tag_type: Option<String>,
    }

    if let Ok(status) = serde_json::from_slice::<DisplayStatus>(&body) {
        if let Some(tag_data) = status.tag_data {
            crate::nfc_bridge_manager::set_decoded_tag_data(
                tag_data.vendor.as_deref().unwrap_or(""),
                tag_data.material.as_deref().unwrap_or(""),
                tag_data.subtype.as_deref().unwrap_or(""),
                tag_data.color_name.as_deref().unwrap_or(""),
                tag_data.color_rgba.unwrap_or(0),
                tag_data.spool_weight.unwrap_or(0),
                tag_data.tag_type.as_deref().unwrap_or(""),
            );
            info!("Received decoded tag data from backend");
        }
    }
}

/// Fetch time from backend and update time manager
/// Can be called independently for quick time sync
pub fn fetch_and_set_time(base_url: &str) {
    let time_url = format!("{}/api/time", base_url);
    match fetch_time(&time_url) {
        Ok(time) => {
            crate::time_manager::set_backend_time(time.hour, time.minute);
        }
        Err(_) => {
            // Silently ignore time fetch errors
        }
    }
}

/// Quick time sync - call after setting server URL
pub fn sync_time() {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if manager.server_url.is_empty() {
        return;
    }
    let base_url = manager.server_url.clone();
    drop(manager);
    fetch_and_set_time(&base_url);
}

/// Fetch printers from backend API
fn fetch_printers(url: &str) -> Result<Vec<ApiPrinter>, String> {
    // Create HTTP client
    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = EspHttpConnection::new(&config)
        .map_err(|e| format!("HTTP connection failed: {:?}", e))?;

    let mut client = HttpClient::wrap(connection);

    // Make GET request
    let request = client.get(url)
        .map_err(|e| format!("GET request failed: {:?}", e))?;

    let mut response = request.submit()
        .map_err(|e| format!("Request submit failed: {:?}", e))?;

    // Check status
    let status = response.status();
    if status != 200 {
        return Err(format!("HTTP error: {}", status));
    }

    // Read response body
    let mut body = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(e) => return Err(format!("Read error: {:?}", e)),
        }
    }

    // Parse JSON
    let printers: Vec<ApiPrinter> = serde_json::from_slice(&body)
        .map_err(|e| format!("JSON parse error: {:?}", e))?;

    Ok(printers)
}

/// Fetch time from backend API
fn fetch_time(url: &str) -> Result<ApiTime, String> {
    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(2000)), // Short timeout for time
        ..Default::default()
    };

    let connection = EspHttpConnection::new(&config)
        .map_err(|e| format!("HTTP connection failed: {:?}", e))?;

    let mut client = HttpClient::wrap(connection);

    let request = client.get(url)
        .map_err(|e| format!("GET request failed: {:?}", e))?;

    let mut response = request.submit()
        .map_err(|e| format!("Request submit failed: {:?}", e))?;

    let status = response.status();
    if status != 200 {
        return Err(format!("HTTP error: {}", status));
    }

    let mut body = Vec::new();
    let mut buf = [0u8; 128];
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(e) => return Err(format!("Read error: {:?}", e)),
        }
    }

    let time: ApiTime = serde_json::from_slice(&body)
        .map_err(|e| format!("JSON parse error: {:?}", e))?;

    Ok(time)
}

/// Update the cached printer data
/// Parse RGBA hex string to u32 (e.g., "FF0000FF" -> 0xFF0000FF)
fn parse_rgba_color(color: &str) -> u32 {
    u32::from_str_radix(color, 16).unwrap_or(0)
}

fn update_printer_cache(manager: &mut BackendManager, printers: &[ApiPrinter]) {
    manager.printer_count = printers.len().min(MAX_PRINTERS);

    info!("Updating printer cache with {} printers", printers.len());

    for (i, printer) in printers.iter().take(MAX_PRINTERS).enumerate() {
        let cached = &mut manager.printers[i];

        info!("Printer {}: serial={}, name={:?}, connected={}",
              i, printer.serial, printer.name, printer.connected);

        // Copy name
        cached.name = [0; 32];
        if let Some(ref name) = printer.name {
            let bytes = name.as_bytes();
            let len = bytes.len().min(31);
            cached.name[..len].copy_from_slice(&bytes[..len]);
        }

        // Copy serial
        cached.serial = [0; 20];
        let serial_bytes = printer.serial.as_bytes();
        let serial_len = serial_bytes.len().min(19);
        cached.serial[..serial_len].copy_from_slice(&serial_bytes[..serial_len]);

        // Copy IP address
        cached.ip_address = [0; 20];
        if let Some(ref ip) = printer.ip_address {
            let bytes = ip.as_bytes();
            let len = bytes.len().min(19);
            cached.ip_address[..len].copy_from_slice(&bytes[..len]);
        }

        // Copy access code
        cached.access_code = [0; 16];
        if let Some(ref code) = printer.access_code {
            let bytes = code.as_bytes();
            let len = bytes.len().min(15);
            cached.access_code[..len].copy_from_slice(&bytes[..len]);
        }

        // Copy state
        cached.connected = printer.connected;
        cached.gcode_state = [0; 16];
        cached.print_progress = 0;
        cached.subtask_name = [0; 64];
        cached.remaining_time_min = 0;

        if let Some(ref gcode) = printer.gcode_state {
            let bytes = gcode.as_bytes();
            let len = bytes.len().min(15);
            cached.gcode_state[..len].copy_from_slice(&bytes[..len]);
        }
        if let Some(progress) = printer.print_progress {
            cached.print_progress = progress;
        }
        if let Some(ref subtask) = printer.subtask_name {
            let bytes = subtask.as_bytes();
            let len = bytes.len().min(63);
            cached.subtask_name[..len].copy_from_slice(&bytes[..len]);
        }
        if let Some(time) = printer.mc_remaining_time {
            cached.remaining_time_min = time;
        }

        // Copy stage info
        cached.stg_cur = printer.stg_cur.unwrap_or(-1);
        cached.stg_cur_name = [0; 48];
        if let Some(ref stg_name) = printer.stg_cur_name {
            let bytes = stg_name.as_bytes();
            let len = bytes.len().min(47);
            cached.stg_cur_name[..len].copy_from_slice(&bytes[..len]);
        }

        // Copy active tray info
        cached.tray_now = printer.tray_now.unwrap_or(-1);
        cached.tray_now_left = printer.tray_now_left.unwrap_or(-1);
        cached.tray_now_right = printer.tray_now_right.unwrap_or(-1);
        cached.active_extruder = printer.active_extruder.unwrap_or(-1);

        // Copy AMS units
        cached.ams_unit_count = printer.ams_units.len().min(MAX_AMS_UNITS) as u8;
        cached.ams_units = [EMPTY_AMS_UNIT; MAX_AMS_UNITS];

        info!("Printer {} has {} AMS units, tray_now={}, active_extruder={}",
              i, printer.ams_units.len(), cached.tray_now, cached.active_extruder);

        for (j, ams) in printer.ams_units.iter().take(MAX_AMS_UNITS).enumerate() {
            let cached_ams = &mut cached.ams_units[j];
            cached_ams.id = ams.id;
            cached_ams.humidity = ams.humidity.unwrap_or(-1);
            cached_ams.temperature = ams.temperature.map(|t| (t * 10.0) as i16).unwrap_or(-1);
            cached_ams.extruder = ams.extruder.map(|e| e as i8).unwrap_or(-1);
            cached_ams.tray_count = ams.trays.len().min(4) as u8;

            info!("  AMS[{}] id={} extruder={:?} trays={}", j, ams.id, ams.extruder, ams.trays.len());

            for (k, tray) in ams.trays.iter().take(4).enumerate() {
                let cached_tray = &mut cached_ams.trays[k];

                // Copy tray type
                cached_tray.tray_type = [0; 16];
                if let Some(ref tray_type) = tray.tray_type {
                    let bytes = tray_type.as_bytes();
                    let len = bytes.len().min(15);
                    cached_tray.tray_type[..len].copy_from_slice(&bytes[..len]);
                }

                // Parse color
                cached_tray.tray_color = tray.tray_color
                    .as_ref()
                    .map(|c| parse_rgba_color(c))
                    .unwrap_or(0);

                // Remaining percentage (clamp negative to 0)
                cached_tray.remain = tray.remain.unwrap_or(0).max(0) as u8;
            }
        }
    }

}

/// Check if cover URL changed and return the new URL if so
fn check_cover_url_changed(printers: &[ApiPrinter], base_url: &str) -> Option<String> {
    if let Some(printer) = printers.first() {
        if let Some(ref cover_url) = printer.cover_url {
            let mut last_url = LAST_COVER_URL.lock().unwrap();
            if *last_url != *cover_url {
                // Cover URL changed
                *last_url = cover_url.clone();
                return Some(format!("{}{}", base_url, cover_url));
            }
        } else {
            // No cover URL, invalidate cover
            COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
            let mut last_url = LAST_COVER_URL.lock().unwrap();
            last_url.clear();
        }
    }
    None
}

/// Fetch cover image from URL
fn fetch_cover_image(url: &str) {
    info!("Fetching cover image from: {}", url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(10000)), // 10s timeout for image
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Cover fetch connection failed: {:?}", e);
            COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let request = match client.get(url) {
        Ok(r) => r,
        Err(e) => {
            warn!("Cover fetch request failed: {:?}", e);
            COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Cover fetch submit failed: {:?}", e);
            COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
            return;
        }
    };

    if response.status() != 200 {
        warn!("Cover fetch HTTP error: {}", response.status());
        COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
        return;
    }

    // Read image data
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if data.len() + n > MAX_COVER_SIZE {
                    warn!("Cover image too large, truncating");
                    break;
                }
                data.extend_from_slice(&buf[..n]);
            }
            Err(e) => {
                warn!("Cover read error: {:?}", e);
                COVER_VALID.store(false, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        }
    }

    info!("Downloaded cover image: {} bytes", data.len());

    // Store cover data
    let mut cover = COVER_DATA.lock().unwrap();
    *cover = data;
    COVER_VALID.store(true, std::sync::atomic::Ordering::Relaxed);
}

// ============================================================================
// C-callable interface
// ============================================================================

/// Backend status for C interface
#[repr(C)]
pub struct BackendStatus {
    /// 0=Disconnected, 1=Discovering, 2=Connected, 3=Error
    pub state: c_int,
    /// Server IP address (valid when state=2)
    pub server_ip: [u8; 4],
    /// Server port (valid when state=2)
    pub server_port: u16,
    /// Number of printers cached
    pub printer_count: u8,
}

/// Printer info for C interface
#[repr(C)]
pub struct PrinterInfo {
    pub name: [c_char; 32],           // 32 bytes
    pub serial: [c_char; 20],         // 20 bytes
    pub ip_address: [c_char; 20],     // 20 bytes - for settings sync
    pub access_code: [c_char; 16],    // 16 bytes - for settings sync
    pub gcode_state: [c_char; 16],    // 16 bytes
    pub subtask_name: [c_char; 64],   // 64 bytes
    pub stg_cur_name: [c_char; 48],   // 48 bytes - detailed stage name
    pub remaining_time_min: u16,      // 2 bytes
    pub print_progress: u8,           // 1 byte
    pub stg_cur: i8,                  // 1 byte - stage number (-1 = idle)
    pub connected: bool,              // 1 byte
    pub _pad: [u8; 3],                // 3 bytes padding for alignment
}

/// Get backend connection status
#[no_mangle]
pub extern "C" fn backend_get_status(status: *mut BackendStatus) {
    if status.is_null() {
        return;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();

    unsafe {
        match &manager.state {
            BackendState::Disconnected => {
                (*status).state = 0;
                (*status).server_ip = [0; 4];
                (*status).server_port = 0;
            }
            BackendState::Discovering => {
                (*status).state = 1;
                (*status).server_ip = [0; 4];
                (*status).server_port = 0;
            }
            BackendState::Connected { ip, port } => {
                (*status).state = 2;
                (*status).server_ip = *ip;
                (*status).server_port = *port;
            }
            BackendState::Error(_) => {
                (*status).state = 3;
                (*status).server_ip = [0; 4];
                (*status).server_port = 0;
            }
        }
        (*status).printer_count = manager.printer_count as u8;
    }
}

/// Get printer info by index
/// Returns 0 on success, -1 if index out of range
#[no_mangle]
pub extern "C" fn backend_get_printer(index: c_int, info: *mut PrinterInfo) -> c_int {
    if info.is_null() || index < 0 {
        return -1;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    let idx = index as usize;

    if idx >= manager.printer_count {
        return -1;
    }

    let cached = &manager.printers[idx];

    unsafe {
        // Copy name (already null-terminated due to zero init)
        for (i, &b) in cached.name.iter().enumerate() {
            (*info).name[i] = b as c_char;
        }

        // Copy serial
        for (i, &b) in cached.serial.iter().enumerate() {
            (*info).serial[i] = b as c_char;
        }

        // Copy IP address
        for (i, &b) in cached.ip_address.iter().enumerate() {
            (*info).ip_address[i] = b as c_char;
        }

        // Copy access code
        for (i, &b) in cached.access_code.iter().enumerate() {
            (*info).access_code[i] = b as c_char;
        }

        // Copy state
        for (i, &b) in cached.gcode_state.iter().enumerate() {
            (*info).gcode_state[i] = b as c_char;
        }

        // Copy subtask name
        for (i, &b) in cached.subtask_name.iter().enumerate() {
            (*info).subtask_name[i] = b as c_char;
        }

        (*info).connected = cached.connected;
        (*info).print_progress = cached.print_progress;
        (*info).remaining_time_min = cached.remaining_time_min;

        // Copy stage info
        (*info).stg_cur = cached.stg_cur;
        for (i, &b) in cached.stg_cur_name.iter().enumerate() {
            (*info).stg_cur_name[i] = b as c_char;
        }

        // Initialize padding
        (*info)._pad = [0; 3];
    }

    0
}

/// Set backend server URL from C
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn backend_set_url(url: *const c_char) -> c_int {
    if url.is_null() {
        return -1;
    }

    let url_str = unsafe {
        match std::ffi::CStr::from_ptr(url).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    set_server_url(url_str);
    0
}

/// Trigger mDNS discovery for backend server
/// Returns 0 if discovery started, -1 on error
#[no_mangle]
pub extern "C" fn backend_discover_server() -> c_int {
    // TODO: Implement mDNS browsing for _spoolbuddy-server._tcp
    // For now, this is a placeholder
    info!("Backend server discovery requested (not yet implemented)");

    let mut manager = BACKEND_MANAGER.lock().unwrap();
    manager.state = BackendState::Discovering;

    0
}

/// Check if backend is connected
/// Returns 1 if connected, 0 otherwise
#[no_mangle]
pub extern "C" fn backend_is_connected() -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    match manager.state {
        BackendState::Connected { .. } => 1,
        _ => 0,
    }
}

/// Get number of cached printers
#[no_mangle]
pub extern "C" fn backend_get_printer_count() -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    manager.printer_count as c_int
}

/// Check if cover image is available
/// Returns 1 if valid cover exists, 0 otherwise
#[no_mangle]
pub extern "C" fn backend_has_cover() -> c_int {
    if COVER_VALID.load(std::sync::atomic::Ordering::Relaxed) {
        1
    } else {
        0
    }
}

/// Get cover image data
/// Returns pointer to PNG data, or null if not available
/// Size is written to size_out if provided
#[no_mangle]
pub extern "C" fn backend_get_cover_data(size_out: *mut u32) -> *const u8 {
    if !COVER_VALID.load(std::sync::atomic::Ordering::Relaxed) {
        if !size_out.is_null() {
            unsafe { *size_out = 0; }
        }
        return std::ptr::null();
    }

    let cover = COVER_DATA.lock().unwrap();
    if cover.is_empty() {
        if !size_out.is_null() {
            unsafe { *size_out = 0; }
        }
        return std::ptr::null();
    }

    if !size_out.is_null() {
        unsafe { *size_out = cover.len() as u32; }
    }

    cover.as_ptr()
}

// ============================================================================
// AMS FFI functions
// ============================================================================

/// AMS tray info for C interface
#[repr(C)]
pub struct AmsTrayCInfo {
    pub tray_type: [c_char; 16],  // Material type
    pub tray_color: u32,          // RGBA packed
    pub remain: u8,               // 0-100 percentage
}

/// AMS unit info for C interface
#[repr(C)]
pub struct AmsUnitCInfo {
    pub id: c_int,
    pub humidity: c_int,          // -1 if not available
    pub temperature: i16,         // Celsius * 10, -1 if not available
    pub extruder: i8,             // -1 if not available, 0=right, 1=left
    pub tray_count: u8,
    pub trays: [AmsTrayCInfo; 4],
}

/// Get number of AMS units for a printer
#[no_mangle]
pub extern "C" fn backend_get_ams_count(printer_index: c_int) -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return 0;
    }
    let count = manager.printers[printer_index as usize].ams_unit_count as c_int;
    info!("backend_get_ams_count({}) = {}", printer_index, count);
    count
}

/// Get AMS unit info
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn backend_get_ams_unit(
    printer_index: c_int,
    ams_index: c_int,
    info: *mut AmsUnitCInfo,
) -> c_int {
    if info.is_null() {
        return -1;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }

    let printer = &manager.printers[printer_index as usize];
    if ams_index < 0 || ams_index as usize >= printer.ams_unit_count as usize {
        return -1;
    }

    let ams = &printer.ams_units[ams_index as usize];

    unsafe {
        let out = &mut *info;
        out.id = ams.id;
        out.humidity = ams.humidity;
        out.temperature = ams.temperature;
        out.extruder = ams.extruder;
        out.tray_count = ams.tray_count;

        for (i, tray) in ams.trays.iter().enumerate() {
            out.trays[i].tray_type = [0; 16];
            for (j, &byte) in tray.tray_type.iter().enumerate() {
                out.trays[i].tray_type[j] = byte as c_char;
            }
            out.trays[i].tray_color = tray.tray_color;
            out.trays[i].remain = tray.remain;
        }
    }

    0
}

/// AMS tray info with string color (for status_bar.c hex parsing)
#[repr(C)]
pub struct AmsTrayInfo {
    pub tray_type: [c_char; 16],
    pub tray_color: [c_char; 16],  // Hex string like "FF0000FF"
    pub remain: u8,
}

/// Get AMS tray info with color as hex string
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn backend_get_ams_tray(
    printer_index: c_int,
    ams_index: c_int,
    tray_index: c_int,
    info: *mut AmsTrayInfo,
) -> c_int {
    if info.is_null() {
        return -1;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }

    let printer = &manager.printers[printer_index as usize];
    if ams_index < 0 || ams_index as usize >= printer.ams_unit_count as usize {
        return -1;
    }

    let ams = &printer.ams_units[ams_index as usize];
    if tray_index < 0 || tray_index as usize >= ams.tray_count as usize {
        return -1;
    }

    let tray = &ams.trays[tray_index as usize];

    unsafe {
        let out = &mut *info;

        // Copy tray_type
        out.tray_type = [0; 16];
        for (j, &byte) in tray.tray_type.iter().enumerate() {
            out.tray_type[j] = byte as c_char;
        }

        // Convert packed RGBA to hex string
        out.tray_color = [0; 16];
        let color = tray.tray_color;
        let hex_str = format!("{:08X}", color);
        copy_to_c_buf_signed(&hex_str, &mut out.tray_color);

        out.remain = tray.remain;
    }

    0
}

/// Get active tray for single-nozzle printer
/// Returns -1 if not available
#[no_mangle]
pub extern "C" fn backend_get_tray_now(printer_index: c_int) -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }
    manager.printers[printer_index as usize].tray_now
}

/// Get active tray for left nozzle (dual-nozzle printer)
/// Returns -1 if not available
#[no_mangle]
pub extern "C" fn backend_get_tray_now_left(printer_index: c_int) -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }
    manager.printers[printer_index as usize].tray_now_left
}

/// Get active tray for right nozzle (dual-nozzle printer)
/// Returns -1 if not available
#[no_mangle]
pub extern "C" fn backend_get_tray_now_right(printer_index: c_int) -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }
    manager.printers[printer_index as usize].tray_now_right
}

/// Get currently active extruder (dual-nozzle printer)
/// Returns -1 if not available, 0=right, 1=left
#[no_mangle]
pub extern "C" fn backend_get_active_extruder(printer_index: c_int) -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    if printer_index < 0 || printer_index as usize >= manager.printer_count {
        return -1;
    }
    manager.printers[printer_index as usize].active_extruder
}

/// Check if firmware update is available
/// Returns 1 if update available, 0 otherwise
#[no_mangle]
pub extern "C" fn ota_is_update_available() -> c_int {
    if crate::ota_manager::is_update_available() {
        1
    } else {
        0
    }
}

/// Get current firmware version
/// Copies version string to buffer, returns length or -1 on error
#[no_mangle]
pub extern "C" fn ota_get_current_version(buf: *mut c_char, buf_len: c_int) -> c_int {
    if buf.is_null() || buf_len <= 0 {
        return -1;
    }
    let version = crate::ota_manager::get_version();
    let bytes = version.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), (buf_len - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
        *buf.add(copy_len) = 0; // Null terminate
    }
    copy_len as c_int
}

/// Get available update version
/// Copies version string to buffer, returns length or -1 on error
#[no_mangle]
pub extern "C" fn ota_get_update_version(buf: *mut c_char, buf_len: c_int) -> c_int {
    if buf.is_null() || buf_len <= 0 {
        return -1;
    }
    let version = crate::ota_manager::get_update_version();
    let bytes = version.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), (buf_len - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
        *buf.add(copy_len) = 0; // Null terminate
    }
    copy_len as c_int
}

/// Get OTA state
/// Returns: 0=Idle, 1=Checking, 2=Downloading, 3=Validating, 4=Flashing, 5=Complete, 6=Error
#[no_mangle]
pub extern "C" fn ota_get_state() -> c_int {
    match crate::ota_manager::get_state() {
        crate::ota_manager::OtaState::Idle => 0,
        crate::ota_manager::OtaState::Checking => 1,
        crate::ota_manager::OtaState::Downloading { .. } => 2,
        crate::ota_manager::OtaState::Validating => 3,
        crate::ota_manager::OtaState::Flashing { .. } => 4,
        crate::ota_manager::OtaState::Complete => 5,
        crate::ota_manager::OtaState::Error(_) => 6,
    }
}

/// Get OTA download/flash progress (0-100)
/// Returns -1 if not in a progress state
#[no_mangle]
pub extern "C" fn ota_get_progress() -> c_int {
    match crate::ota_manager::get_state() {
        crate::ota_manager::OtaState::Downloading { progress } => progress as c_int,
        crate::ota_manager::OtaState::Flashing { progress } => progress as c_int,
        _ => -1,
    }
}

/// Trigger OTA update check (non-blocking, spawns thread)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn ota_check_for_update() -> c_int {
    // Get backend URL
    let manager = BACKEND_MANAGER.lock().unwrap();
    let url = match &manager.state {
        BackendState::Connected { ip, port } => {
            format!("http://{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port)
        }
        _ => return -1, // Not connected
    };
    drop(manager);

    // Spawn thread with larger stack (HTTP client needs more stack space)
    std::thread::Builder::new()
        .name("ota_check".into())
        .stack_size(8192)  // 8KB stack
        .spawn(move || {
            match crate::ota_manager::check_for_update(&url) {
                Ok(info) => {
                    crate::ota_manager::set_update_available(info.available, &info.version);
                    info!("OTA check complete: available={}, version={}", info.available, info.version);
                }
                Err(e) => {
                    warn!("OTA check failed: {}", e);
                }
            }
        })
        .ok();
    0
}

/// Trigger OTA update (non-blocking, spawns thread)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub extern "C" fn ota_start_update() -> c_int {
    // Get backend URL
    let manager = BACKEND_MANAGER.lock().unwrap();
    let url = match &manager.state {
        BackendState::Connected { ip, port } => {
            format!("http://{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port)
        }
        _ => return -1, // Not connected
    };
    drop(manager);

    // Spawn thread to perform update
    std::thread::spawn(move || {
        if let Err(e) = crate::ota_manager::perform_update(&url) {
            log::error!("OTA update failed: {}", e);
        }
        // Note: perform_update reboots on success, so we only get here on error
    });
    0
}

// =============================================================================
// Spool API Functions (for AMS assignment and K-profile lookup)
// =============================================================================

/// C-compatible spool info structure
#[repr(C)]
pub struct SpoolInfoC {
    pub id: [u8; 64],           // Spool UUID
    pub tag_id: [u8; 32],       // NFC tag UID
    pub brand: [u8; 32],        // Vendor/brand name
    pub material: [u8; 16],     // Material type (PLA, PETG, etc.)
    pub subtype: [u8; 32],      // Material subtype (Basic, Matte, etc.)
    pub color_name: [u8; 32],   // Color name
    pub color_rgba: u32,        // RGBA packed color
    pub label_weight: i32,      // Label weight in grams
    pub weight_current: i32,    // Current weight from inventory (grams)
    pub slicer_filament: [u8; 32], // Slicer filament ID
    pub valid: bool,            // True if spool was found
}

/// C-compatible K-profile structure
#[repr(C)]
pub struct SpoolKProfileC {
    pub cali_idx: i32,          // Calibration index (-1 if not found)
    pub k_value: [u8; 16],      // K-factor value as string
    pub name: [u8; 64],         // Profile name
    pub printer_serial: [u8; 32], // Printer serial this profile is for
}

/// API response for spool listing
#[derive(Debug, Deserialize)]
struct ApiSpool {
    id: String,
    tag_id: Option<String>,
    brand: Option<String>,
    material: Option<String>,
    subtype: Option<String>,
    color_name: Option<String>,
    rgba: Option<String>,
    label_weight: Option<i32>,
    weight_current: Option<i32>,
    slicer_filament: Option<String>,
}

/// API response for K-profile
#[derive(Debug, Deserialize)]
struct ApiKProfile {
    cali_idx: Option<i32>,
    k_value: Option<String>,
    name: Option<String>,
    printer_serial: Option<String>,
}

/// API response for assign result
#[derive(Debug, Deserialize)]
struct ApiAssignResponse {
    status: Option<String>,     // "configured" or "staged"
    #[allow(dead_code)]
    message: Option<String>,
}

/// Helper to copy string to fixed-size C buffer
fn copy_to_c_buf(src: &str, dst: &mut [u8]) {
    let bytes = src.as_bytes();
    let copy_len = std::cmp::min(bytes.len(), dst.len() - 1);
    dst[..copy_len].copy_from_slice(&bytes[..copy_len]);
    dst[copy_len] = 0; // Null terminate
}

/// Helper to parse RGBA hex string to u32
fn parse_rgba_hex(hex: &str) -> u32 {
    let hex = hex.trim_start_matches('#');
    let padded = if hex.len() == 6 {
        format!("{}FF", hex)
    } else {
        hex.to_string()
    };
    u32::from_str_radix(&padded, 16).unwrap_or(0)
}

/// Get spool info by NFC tag ID
/// Returns true if found, fills info struct
#[no_mangle]
pub extern "C" fn spool_get_by_tag(tag_id: *const c_char, info: *mut SpoolInfoC) -> bool {
    if tag_id.is_null() || info.is_null() {
        return false;
    }

    let tag_id_str = unsafe {
        match std::ffi::CStr::from_ptr(tag_id).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    // Get backend URL
    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // GET /api/spools to list all spools
    let url = format!("{}/api/spools", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Read response body
    let mut buf = vec![0u8; 8192];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return false;
    }

    // Parse JSON array of spools
    let body = String::from_utf8_lossy(&buf[..total]);
    let spools: Vec<ApiSpool> = match serde_json::from_str(&body) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Find spool with matching tag_id
    for spool in spools {
        if let Some(ref tid) = spool.tag_id {
            if tid == tag_id_str {
                // Found - fill info struct
                let info_ref = unsafe { &mut *info };
                *info_ref = SpoolInfoC {
                    id: [0; 64],
                    tag_id: [0; 32],
                    brand: [0; 32],
                    material: [0; 16],
                    subtype: [0; 32],
                    color_name: [0; 32],
                    color_rgba: 0,
                    label_weight: 0,
                    weight_current: 0,
                    slicer_filament: [0; 32],
                    valid: true,
                };

                copy_to_c_buf(&spool.id, &mut info_ref.id);
                copy_to_c_buf(tid, &mut info_ref.tag_id);
                if let Some(ref b) = spool.brand {
                    copy_to_c_buf(b, &mut info_ref.brand);
                }
                if let Some(ref m) = spool.material {
                    copy_to_c_buf(m, &mut info_ref.material);
                }
                if let Some(ref s) = spool.subtype {
                    copy_to_c_buf(s, &mut info_ref.subtype);
                }
                if let Some(ref c) = spool.color_name {
                    copy_to_c_buf(c, &mut info_ref.color_name);
                }
                if let Some(ref rgba) = spool.rgba {
                    info_ref.color_rgba = parse_rgba_hex(rgba);
                }
                if let Some(w) = spool.label_weight {
                    info_ref.label_weight = w;
                }
                if let Some(w) = spool.weight_current {
                    info_ref.weight_current = w;
                }
                if let Some(ref sf) = spool.slicer_filament {
                    copy_to_c_buf(sf, &mut info_ref.slicer_filament);
                }

                info!("spool_get_by_tag: found spool {} for tag {}", spool.id, tag_id_str);
                return true;
            }
        }
    }

    info!("spool_get_by_tag: no spool found for tag {}", tag_id_str);
    false
}

/// Get K-profile for a spool on a specific printer
/// Returns true if found, fills profile struct
#[no_mangle]
pub extern "C" fn spool_get_k_profile_for_printer(
    spool_id: *const c_char,
    printer_serial: *const c_char,
    profile: *mut SpoolKProfileC,
) -> bool {
    if spool_id.is_null() || printer_serial.is_null() || profile.is_null() {
        return false;
    }

    let spool_id_str = unsafe {
        match std::ffi::CStr::from_ptr(spool_id).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    let printer_serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    // Get backend URL
    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // GET /api/spools/{id}/k-profiles
    let url = format!("{}/api/spools/{}/k-profiles", base_url, spool_id_str);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return false,
    };

    // Read response body
    let mut buf = vec![0u8; 4096];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return false;
    }

    // Parse JSON array of K-profiles
    let body = String::from_utf8_lossy(&buf[..total]);
    let profiles: Vec<ApiKProfile> = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(_) => return false,
    };

    // Find profile matching the printer serial
    for p in profiles {
        if let Some(ref serial) = p.printer_serial {
            if serial == printer_serial_str {
                // Found - fill profile struct
                let profile_ref = unsafe { &mut *profile };
                *profile_ref = SpoolKProfileC {
                    cali_idx: p.cali_idx.unwrap_or(-1),
                    k_value: [0; 16],
                    name: [0; 64],
                    printer_serial: [0; 32],
                };

                if let Some(ref kv) = p.k_value {
                    copy_to_c_buf(kv, &mut profile_ref.k_value);
                }
                if let Some(ref n) = p.name {
                    copy_to_c_buf(n, &mut profile_ref.name);
                }
                copy_to_c_buf(serial, &mut profile_ref.printer_serial);

                info!("spool_get_k_profile: found profile for {} on {}", spool_id_str, printer_serial_str);
                return true;
            }
        }
    }

    info!("spool_get_k_profile: no profile for {} on {}", spool_id_str, printer_serial_str);
    false
}

/// Check if a spool with given tag_id exists in inventory
#[no_mangle]
pub extern "C" fn spool_exists_by_tag(tag_id: *const c_char) -> bool {
    if tag_id.is_null() {
        return false;
    }

    let tag_id_str = unsafe {
        match std::ffi::CStr::from_ptr(tag_id).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // GET /api/spools - get all spools and search for matching tag
    let url = format!("{}/api/spools", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return false,
    };

    if response.status() != 200 {
        return false;
    }

    // Read response - use larger buffer for full spool list
    let mut buf = vec![0u8; 16384];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return false;
    }

    // Parse as JSON array of spools
    let body = String::from_utf8_lossy(&buf[..total]);
    let spools: Vec<ApiSpool> = match serde_json::from_str(&body) {
        Ok(s) => s,
        Err(e) => {
            warn!("spool_exists_by_tag: JSON parse error: {:?}", e);
            return false;
        }
    };

    // Search for matching tag_id
    for spool in spools {
        if let Some(ref tid) = spool.tag_id {
            if tid == tag_id_str {
                info!("spool_exists_by_tag: found spool for tag {}", tag_id_str);
                return true;
            }
        }
    }

    info!("spool_exists_by_tag: no spool found for tag {}", tag_id_str);
    false
}

/// Add a new spool to inventory
#[no_mangle]
pub extern "C" fn spool_add_to_inventory(
    tag_id: *const c_char,
    vendor: *const c_char,
    material: *const c_char,
    subtype: *const c_char,
    color_name: *const c_char,
    color_rgba: u32,
    label_weight: c_int,
    weight_current: c_int,
    data_origin: *const c_char,
    tag_type: *const c_char,
    slicer_filament: *const c_char,
) -> bool {
    fn c_str_to_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            }
        }
    }

    let tag_id_str = c_str_to_string(tag_id);
    let vendor_str = c_str_to_string(vendor);
    let material_str = c_str_to_string(material);
    let subtype_str = c_str_to_string(subtype);
    let color_name_str = c_str_to_string(color_name);
    let data_origin_str = c_str_to_string(data_origin);
    let tag_type_str = c_str_to_string(tag_type);
    let slicer_filament_str = c_str_to_string(slicer_filament);

    // Convert RGBA to hex string
    let rgba_hex = format!("{:08X}", color_rgba);

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // POST /api/spools
    let url = format!("{}/api/spools", base_url);

    let body = format!(
        r#"{{"tag_id":"{}","brand":"{}","material":"{}","subtype":"{}","color_name":"{}","rgba":"{}","label_weight":{},"weight_current":{},"data_origin":"{}","tag_type":"{}","slicer_filament":"{}"}}"#,
        tag_id_str, vendor_str, material_str, subtype_str, color_name_str,
        rgba_hex, label_weight, weight_current, data_origin_str, tag_type_str, slicer_filament_str
    );

    info!("spool_add_to_inventory: POST {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return false;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Post, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create POST request: {:?}", e);
            return false;
        }
    };

    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return false;
    }

    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return false;
    }

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return false;
        }
    };

    let status = response.status();
    if status != 200 && status != 201 {
        warn!("spool_add_to_inventory failed with status {}", status);
        return false;
    }

    info!("spool_add_to_inventory: success");
    true
}

/// Untagged spool info for FFI
#[repr(C)]
pub struct UntaggedSpoolInfo {
    pub id: [c_char; 64],
    pub brand: [c_char; 32],
    pub material: [c_char; 32],
    pub color_name: [c_char; 32],
    pub color_rgba: u32,
    pub label_weight: i32,
    pub spool_number: i32,
    pub valid: bool,
}

/// API response for untagged spools
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiUntaggedSpool {
    id: String,
    brand: Option<String>,
    material: Option<String>,
    color_name: Option<String>,
    rgba: Option<String>,
    label_weight: Option<i32>,
    spool_number: Option<i32>,
}

/// Get list of spools without NFC tags assigned
#[no_mangle]
pub extern "C" fn spool_get_untagged_list(
    spools: *mut UntaggedSpoolInfo,
    max_count: c_int,
) -> c_int {
    if spools.is_null() || max_count <= 0 {
        return -1;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return -1;
    }

    // GET /api/spools?untagged=true
    let url = format!("{}/api/spools?untagged=true", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    if response.status() != 200 {
        return -1;
    }

    let mut buf = vec![0u8; 8192];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return 0;
    }

    let body = String::from_utf8_lossy(&buf[..total]);
    let api_spools: Vec<ApiUntaggedSpool> = match serde_json::from_str(&body) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let count = api_spools.len().min(max_count as usize);

    for (i, spool) in api_spools.iter().take(count).enumerate() {
        let spool_ref = unsafe { &mut *spools.add(i) };

        spool_ref.id = [0; 64];
        spool_ref.brand = [0; 32];
        spool_ref.material = [0; 32];
        spool_ref.color_name = [0; 32];
        spool_ref.color_rgba = 0;
        spool_ref.label_weight = spool.label_weight.unwrap_or(0);
        spool_ref.spool_number = spool.spool_number.unwrap_or(0);
        spool_ref.valid = true;

        copy_to_c_buf_signed(&spool.id, &mut spool_ref.id);
        if let Some(ref b) = spool.brand {
            copy_to_c_buf_signed(b, &mut spool_ref.brand);
        }
        if let Some(ref m) = spool.material {
            copy_to_c_buf_signed(m, &mut spool_ref.material);
        }
        if let Some(ref c) = spool.color_name {
            copy_to_c_buf_signed(c, &mut spool_ref.color_name);
        }
        if let Some(ref rgba) = spool.rgba {
            spool_ref.color_rgba = parse_rgba_hex(rgba);
        }
    }

    count as c_int
}

/// Get count of spools without NFC tags
#[no_mangle]
pub extern "C" fn spool_get_untagged_count() -> c_int {
    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return -1;
    }

    // GET /api/spools?untagged=true (just count results)
    let url = format!("{}/api/spools?untagged=true", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    if response.status() != 200 {
        return -1;
    }

    let mut buf = vec![0u8; 8192];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return 0;
    }

    let body = String::from_utf8_lossy(&buf[..total]);
    let spools: Vec<serde_json::Value> = match serde_json::from_str(&body) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    spools.len() as c_int
}

/// Link an NFC tag to an existing spool
/// Returns: 0 = success, -1 = connection error, or HTTP status code on failure (e.g., 409 = already assigned)
#[no_mangle]
pub extern "C" fn spool_link_tag(
    spool_id: *const c_char,
    tag_id: *const c_char,
    tag_type: *const c_char,
) -> c_int {
    if spool_id.is_null() || tag_id.is_null() {
        return -1;
    }

    fn c_str_to_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            }
        }
    }

    let spool_id_str = c_str_to_string(spool_id);
    let tag_id_str = c_str_to_string(tag_id);
    let tag_type_str = c_str_to_string(tag_type);

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return -1;
    }

    // PATCH /api/spools/{spool_id}/link-tag
    let url = format!("{}/api/spools/{}/link-tag", base_url, spool_id_str);

    let body = format!(
        r#"{{"tag_id":"{}","tag_type":"{}"}}"#,
        tag_id_str, tag_type_str
    );

    info!("spool_link_tag: PATCH {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return -1;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Patch, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create PATCH request: {:?}", e);
            return -1;
        }
    };

    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return -1;
    }

    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return -1;
    }

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return -1;
        }
    };

    let status = response.status();
    if status != 200 {
        warn!("spool_link_tag failed with status {}", status);
        return status as c_int;
    }

    info!("spool_link_tag: success");
    0
}

/// Sync spool weight to backend
#[no_mangle]
pub extern "C" fn spool_sync_weight(
    spool_id: *const c_char,
    weight: c_int,
) -> bool {
    if spool_id.is_null() {
        return false;
    }

    let spool_id_str = unsafe {
        std::ffi::CStr::from_ptr(spool_id)
            .to_str()
            .unwrap_or("")
            .to_string()
    };

    if spool_id_str.is_empty() {
        return false;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // PUT /api/spools/{spool_id}
    let url = format!("{}/api/spools/{}", base_url, spool_id_str);

    let body = format!(r#"{{"weight_current":{}}}"#, weight);

    info!("spool_sync_weight: PUT {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return false;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Put, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create PUT request: {:?}", e);
            return false;
        }
    };

    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return false;
    }

    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return false;
    }

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return false;
        }
    };

    let status = response.status();
    if status != 200 {
        warn!("spool_sync_weight failed with status {}", status);
        return false;
    }

    info!("spool_sync_weight: success");
    true
}

/// Assign result enum (matches simulator)
/// 0 = Error, 1 = Configured, 2 = Staged, 3 = StagedReplace
#[no_mangle]
pub extern "C" fn backend_assign_spool_to_tray(
    printer_serial: *const c_char,
    ams_id: c_int,
    tray_id: c_int,
    spool_id: *const c_char,
) -> c_int {
    if printer_serial.is_null() || spool_id.is_null() {
        return 0; // Error
    }

    let printer_serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let spool_id_str = unsafe {
        match std::ffi::CStr::from_ptr(spool_id).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    // Get backend URL
    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return 0;
    }

    // POST /api/printers/{serial}/ams/{ams_id}/tray/{tray_id}/assign
    let url = format!(
        "{}/api/printers/{}/ams/{}/tray/{}/assign",
        base_url, printer_serial_str, ams_id, tray_id
    );

    // Build JSON body
    let body = format!(r#"{{"spool_id":"{}"}}"#, spool_id_str);

    info!("backend_assign_spool_to_tray: POST {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return 0;
        }
    };

    let mut client = HttpClient::wrap(connection);

    // POST request with JSON body
    // Use request() method with headers that include Content-Length
    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Post, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create POST request: {:?}", e);
            return 0;
        }
    };

    // Write body
    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return 0;
    }

    // Flush to ensure body is sent
    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return 0;
    }

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return 0;
        }
    };

    let status = response.status();

    // Read response body
    let mut buf = vec![0u8; 1024];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if status != 200 && status != 201 {
        warn!("Assign failed with status {}", status);
        return 0;
    }

    // Parse response
    if total > 0 {
        let body = String::from_utf8_lossy(&buf[..total]);
        if let Ok(resp) = serde_json::from_str::<ApiAssignResponse>(&body) {
            if let Some(ref s) = resp.status {
                match s.as_str() {
                    "configured" => {
                        info!("Assign result: configured");
                        return 1;
                    }
                    "staged" => {
                        info!("Assign result: staged");
                        return 2;
                    }
                    _ => {}
                }
            }
        }
    }

    // Default to configured if status was OK
    info!("Assign result: assuming configured (status {})", status);
    1
}

// =============================================================================
// AMS Slot Configuration API (for Configure Slot modal)
// =============================================================================

/// Slicer preset from cloud API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiSlicerPreset {
    setting_id: String,
    name: String,
    #[serde(rename = "type")]
    preset_type: Option<String>,
    is_custom: Option<bool>,
}

/// Slicer settings response from cloud API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiSlicerSettingsResponse {
    filament: Option<Vec<ApiSlicerPreset>>,
    #[allow(dead_code)]
    printer: Option<Vec<ApiSlicerPreset>>,
    #[allow(dead_code)]
    process: Option<Vec<ApiSlicerPreset>>,
}

/// Preset detail from cloud API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiPresetDetail {
    filament_id: Option<String>,
    base_id: Option<String>,
    setting: Option<ApiPresetDetailSetting>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ApiPresetDetailSetting {
    filament_id: Option<String>,
    base_id: Option<String>,
}

/// K-profile from calibrations API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiKProfileInfo {
    cali_idx: Option<i32>,
    name: Option<String>,
    k_value: Option<f64>,
    filament_id: Option<String>,
    setting_id: Option<String>,
    extruder_id: Option<i32>,
    nozzle_temp: Option<i32>,
}

/// Color catalog entry from colors API
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiColorEntry {
    id: Option<i32>,
    manufacturer: Option<String>,
    color_name: Option<String>,
    hex_color: Option<String>,
    material: Option<String>,
}

// C-compatible structs for FFI (names match ui_internal.h)

/// Slicer preset (C-compatible, matches ui_internal.h SlicerPreset)
#[repr(C)]
pub struct SlicerPreset {
    pub setting_id: [c_char; 64],
    pub name: [c_char; 64],
    pub preset_type: [c_char; 16],  // Called 'type' in C, but 'type' is reserved in Rust
    pub is_custom: bool,
}

/// Preset detail (C-compatible, matches ui_internal.h PresetDetail)
#[repr(C)]
pub struct PresetDetail {
    pub filament_id: [c_char; 64],
    pub base_id: [c_char; 64],
    pub has_filament_id: bool,
    pub has_base_id: bool,
}

/// K-profile info (C-compatible, matches ui_internal.h KProfileInfo)
#[repr(C)]
pub struct KProfileInfo {
    pub cali_idx: i32,
    pub name: [c_char; 64],
    pub k_value: [c_char; 16],
    pub filament_id: [c_char; 32],
    pub setting_id: [c_char; 64],
    pub extruder_id: i32,
    pub nozzle_temp: i32,
}

/// Color catalog entry (C-compatible, matches ui_internal.h ColorCatalogEntry)
#[repr(C)]
pub struct ColorCatalogEntry {
    pub id: i32,
    pub manufacturer: [c_char; 64],
    pub color_name: [c_char; 64],
    pub hex_color: [c_char; 16],
    pub material: [c_char; 32],
}

/// Get slicer filament presets from Bambu Cloud (via backend)
/// Returns number of presets found (up to max_count), -1 on error
#[no_mangle]
pub extern "C" fn backend_get_slicer_presets(
    presets: *mut SlicerPreset,
    max_count: c_int,
) -> c_int {
    info!("backend_get_slicer_presets called");

    if presets.is_null() || max_count <= 0 {
        return -1;
    }

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    let is_connected = matches!(manager.state, BackendState::Connected { .. });
    drop(manager);

    if base_url.is_empty() {
        info!("backend_get_slicer_presets: no server URL configured");
        return 0;  // Return 0 presets instead of -1 if not configured
    }

    if !is_connected {
        info!("backend_get_slicer_presets: backend not connected, skipping");
        return 0;  // Don't block UI if backend not connected
    }

    // GET /api/cloud/settings
    let url = format!("{}/api/cloud/settings", base_url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    if response.status() != 200 {
        // Might be 401 (not authenticated) - return 0 presets
        if response.status() == 401 {
            return 0;
        }
        return -1;
    }

    // Read response body
    let mut buf = vec![0u8; 262144]; // 256KB buffer for presets list
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return 0;
    }

    // Parse response
    let body = String::from_utf8_lossy(&buf[..total]);
    let settings: ApiSlicerSettingsResponse = match serde_json::from_str(&body) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to parse slicer presets: {:?}", e);
            return -1;
        }
    };

    // Extract filament presets only
    let filaments = settings.filament.unwrap_or_default();
    let count = filaments.len().min(max_count as usize);

    for (i, preset) in filaments.iter().take(count).enumerate() {
        let preset_ref = unsafe { &mut *presets.add(i) };

        // Initialize with zeros
        preset_ref.setting_id = [0; 64];
        preset_ref.name = [0; 64];
        preset_ref.preset_type = [0; 16];
        preset_ref.is_custom = preset.is_custom.unwrap_or(false);

        // Copy strings
        copy_to_c_buf_signed(&preset.setting_id, &mut preset_ref.setting_id);
        copy_to_c_buf_signed(&preset.name, &mut preset_ref.name);
        if let Some(ref t) = preset.preset_type {
            copy_to_c_buf_signed(t, &mut preset_ref.preset_type);
        }
    }

    info!("backend_get_slicer_presets: returning {} presets", count);
    count as c_int
}

/// Get detailed preset info including filament_id and base_id
/// Returns true on success, false on failure
#[no_mangle]
pub extern "C" fn backend_get_preset_detail(
    setting_id: *const c_char,
    detail: *mut PresetDetail,
) -> bool {
    if setting_id.is_null() || detail.is_null() {
        return false;
    }

    let setting_id_str = unsafe {
        match std::ffi::CStr::from_ptr(setting_id).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    let is_connected = matches!(manager.state, BackendState::Connected { .. });
    drop(manager);

    if base_url.is_empty() || !is_connected {
        info!("backend_get_preset_detail: backend not available, skipping");
        return false;
    }

    // GET /api/cloud/settings/{setting_id}
    let url = format!("{}/api/cloud/settings/{}", base_url, setting_id_str);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return false,
    };

    if response.status() != 200 {
        return false;
    }

    // Read response body
    let mut buf = vec![0u8; 4096];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return false;
    }

    // Parse response
    let body = String::from_utf8_lossy(&buf[..total]);
    let api_detail: ApiPresetDetail = match serde_json::from_str(&body) {
        Ok(d) => d,
        Err(_) => return false,
    };

    // Fill detail struct
    let detail_ref = unsafe { &mut *detail };
    detail_ref.filament_id = [0; 64];
    detail_ref.base_id = [0; 64];
    detail_ref.has_filament_id = false;
    detail_ref.has_base_id = false;

    // Check top-level first, then nested setting object
    if let Some(ref fid) = api_detail.filament_id {
        copy_to_c_buf_signed(fid, &mut detail_ref.filament_id);
        detail_ref.has_filament_id = true;
    } else if let Some(ref setting) = api_detail.setting {
        if let Some(ref fid) = setting.filament_id {
            copy_to_c_buf_signed(fid, &mut detail_ref.filament_id);
            detail_ref.has_filament_id = true;
        }
    }

    if let Some(ref bid) = api_detail.base_id {
        copy_to_c_buf_signed(bid, &mut detail_ref.base_id);
        detail_ref.has_base_id = true;
    } else if let Some(ref setting) = api_detail.setting {
        if let Some(ref bid) = setting.base_id {
            copy_to_c_buf_signed(bid, &mut detail_ref.base_id);
            detail_ref.has_base_id = true;
        }
    }

    info!("backend_get_preset_detail: {} -> has_fid={}, has_bid={}",
          setting_id_str, detail_ref.has_filament_id, detail_ref.has_base_id);
    true
}

/// Get K-profiles (calibration profiles) for a printer
/// Returns number of profiles found, -1 on error
#[no_mangle]
pub extern "C" fn backend_get_k_profiles(
    printer_serial: *const c_char,
    nozzle_diameter: *const c_char,
    profiles: *mut KProfileInfo,
    max_count: c_int,
) -> c_int {
    info!("backend_get_k_profiles called");

    if printer_serial.is_null() || profiles.is_null() || max_count <= 0 {
        return -1;
    }

    let serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    let nozzle_str = if nozzle_diameter.is_null() {
        "0.4"
    } else {
        unsafe {
            match std::ffi::CStr::from_ptr(nozzle_diameter).to_str() {
                Ok(s) => s,
                Err(_) => "0.4",
            }
        }
    };

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    let is_connected = matches!(manager.state, BackendState::Connected { .. });
    drop(manager);

    if base_url.is_empty() {
        info!("backend_get_k_profiles: no server URL configured");
        return 0;  // Return 0 profiles instead of -1 if not configured
    }

    if !is_connected {
        info!("backend_get_k_profiles: backend not connected, skipping");
        return 0;  // Don't block UI if backend not connected
    }

    info!("backend_get_k_profiles: fetching from {}", base_url);

    // GET /api/printers/{serial}/calibrations?nozzle_diameter=X
    let url = format!("{}/api/printers/{}/calibrations?nozzle_diameter={}",
                      base_url, serial_str, nozzle_str);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    if response.status() != 200 {
        return -1;
    }

    // Read response body
    let mut buf = vec![0u8; 8192];
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return 0;
    }

    // Parse response
    let body = String::from_utf8_lossy(&buf[..total]);
    let api_profiles: Vec<ApiKProfileInfo> = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to parse K-profiles: {:?}", e);
            return -1;
        }
    };

    let count = api_profiles.len().min(max_count as usize);

    for (i, prof) in api_profiles.iter().take(count).enumerate() {
        let prof_ref = unsafe { &mut *profiles.add(i) };

        prof_ref.cali_idx = prof.cali_idx.unwrap_or(-1);
        prof_ref.name = [0; 64];
        prof_ref.k_value = [0; 16];
        prof_ref.filament_id = [0; 32];
        prof_ref.setting_id = [0; 64];
        prof_ref.extruder_id = prof.extruder_id.unwrap_or(-1);
        prof_ref.nozzle_temp = prof.nozzle_temp.unwrap_or(0);

        if let Some(ref n) = prof.name {
            copy_to_c_buf_signed(n, &mut prof_ref.name);
        }
        if let Some(k) = prof.k_value {
            let k_str = format!("{:.3}", k);
            copy_to_c_buf_signed(&k_str, &mut prof_ref.k_value);
        }
        if let Some(ref fid) = prof.filament_id {
            copy_to_c_buf_signed(fid, &mut prof_ref.filament_id);
        }
        if let Some(ref sid) = prof.setting_id {
            copy_to_c_buf_signed(sid, &mut prof_ref.setting_id);
        }
    }

    info!("backend_get_k_profiles: returning {} profiles for {}", count, serial_str);
    count as c_int
}

/// Set filament in an AMS slot
/// Returns true on success
#[no_mangle]
pub extern "C" fn backend_set_slot_filament(
    printer_serial: *const c_char,
    ams_id: c_int,
    tray_id: c_int,
    tray_info_idx: *const c_char,
    setting_id: *const c_char,
    tray_type: *const c_char,
    tray_sub_brands: *const c_char,
    tray_color: *const c_char,
    nozzle_temp_min: c_int,
    nozzle_temp_max: c_int,
) -> bool {
    if printer_serial.is_null() {
        return false;
    }

    let serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    // Helper to convert C string to Rust string
    fn c_str_to_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            }
        }
    }

    let tray_info_idx_str = c_str_to_string(tray_info_idx);
    let setting_id_str = c_str_to_string(setting_id);
    let tray_type_str = c_str_to_string(tray_type);
    let tray_sub_brands_str = c_str_to_string(tray_sub_brands);
    let tray_color_str = c_str_to_string(tray_color);

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // POST /api/printers/{serial}/ams/{ams_id}/tray/{tray_id}/filament
    let url = format!("{}/api/printers/{}/ams/{}/tray/{}/filament",
                      base_url, serial_str, ams_id, tray_id);

    // Build JSON body
    let body = format!(
        r#"{{"tray_info_idx":"{}","setting_id":"{}","tray_type":"{}","tray_sub_brands":"{}","tray_color":"{}","nozzle_temp_min":{},"nozzle_temp_max":{}}}"#,
        tray_info_idx_str, setting_id_str, tray_type_str, tray_sub_brands_str,
        tray_color_str, nozzle_temp_min, nozzle_temp_max
    );

    info!("backend_set_slot_filament: POST {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return false;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Post, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create POST request: {:?}", e);
            return false;
        }
    };

    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return false;
    }

    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return false;
    }

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return false;
        }
    };

    let status = response.status();
    if status != 200 && status != 204 {
        warn!("set_slot_filament failed with status {}", status);
        return false;
    }

    info!("backend_set_slot_filament: success");
    true
}

/// Set calibration (K-profile) for an AMS slot
/// Returns true on success
#[no_mangle]
pub extern "C" fn backend_set_slot_calibration(
    printer_serial: *const c_char,
    ams_id: c_int,
    tray_id: c_int,
    cali_idx: c_int,
    filament_id: *const c_char,
    setting_id: *const c_char,
    nozzle_diameter: *const c_char,
    k_value: f32,
    nozzle_temp: c_int,
) -> bool {
    if printer_serial.is_null() {
        return false;
    }

    let serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    fn c_str_to_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            String::new()
        } else {
            unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            }
        }
    }

    let filament_id_str = c_str_to_string(filament_id);
    let setting_id_str = c_str_to_string(setting_id);
    let nozzle_diameter_str = if nozzle_diameter.is_null() {
        "0.4".to_string()
    } else {
        c_str_to_string(nozzle_diameter)
    };

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // POST /api/printers/{serial}/ams/{ams_id}/tray/{tray_id}/calibration
    let url = format!("{}/api/printers/{}/ams/{}/tray/{}/calibration",
                      base_url, serial_str, ams_id, tray_id);

    // Build JSON body
    let body = format!(
        r#"{{"cali_idx":{},"filament_id":"{}","setting_id":"{}","nozzle_diameter":"{}","k_value":{},"nozzle_temp_max":{}}}"#,
        cali_idx, filament_id_str, setting_id_str, nozzle_diameter_str, k_value, nozzle_temp
    );

    info!("backend_set_slot_calibration: POST {} with {}", url, body);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return false;
        }
    };

    let mut client = HttpClient::wrap(connection);

    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", &body.len().to_string()),
    ];

    let mut request = match client.request(embedded_svc::http::Method::Post, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create POST request: {:?}", e);
            return false;
        }
    };

    if let Err(e) = request.write(body.as_bytes()) {
        warn!("Failed to write request body: {:?}", e);
        return false;
    }

    if let Err(e) = request.flush() {
        warn!("Failed to flush request: {:?}", e);
        return false;
    }

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return false;
        }
    };

    let status = response.status();
    if status != 200 && status != 204 {
        warn!("set_slot_calibration failed with status {}", status);
        return false;
    }

    info!("backend_set_slot_calibration: success");
    true
}

/// Reset/clear an AMS slot (triggers RFID re-read)
/// Returns true on success
#[no_mangle]
pub extern "C" fn backend_reset_slot(
    printer_serial: *const c_char,
    ams_id: c_int,
    tray_id: c_int,
) -> bool {
    if printer_serial.is_null() {
        return false;
    }

    let serial_str = unsafe {
        match std::ffi::CStr::from_ptr(printer_serial).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return false;
    }

    // POST /api/printers/{serial}/ams/{ams_id}/tray/{tray_id}/reset
    let url = format!("{}/api/printers/{}/ams/{}/tray/{}/reset",
                      base_url, serial_str, ams_id, tray_id);

    info!("backend_reset_slot: POST {}", url);

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create HTTP connection: {:?}", e);
            return false;
        }
    };

    let mut client = HttpClient::wrap(connection);

    // Empty body POST
    let headers = [
        ("Content-Type", "application/json"),
        ("Content-Length", "0"),
    ];

    let request = match client.request(embedded_svc::http::Method::Post, &url, &headers) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to create POST request: {:?}", e);
            return false;
        }
    };

    let response = match request.submit() {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to submit request: {:?}", e);
            return false;
        }
    };

    let status = response.status();
    if status != 200 && status != 204 {
        warn!("reset_slot failed with status {}", status);
        return false;
    }

    info!("backend_reset_slot: success");
    true
}

/// Search color catalog by manufacturer and/or material
/// Returns number of colors found (up to max_count), -1 on error
#[no_mangle]
pub extern "C" fn backend_search_colors(
    manufacturer: *const c_char,
    material: *const c_char,
    colors: *mut ColorCatalogEntry,
    max_count: c_int,
) -> c_int {
    if colors.is_null() || max_count <= 0 {
        return -1;
    }

    fn c_str_to_option(ptr: *const c_char) -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            let s = unsafe {
                std::ffi::CStr::from_ptr(ptr)
                    .to_str()
                    .unwrap_or("")
                    .to_string()
            };
            if s.is_empty() { None } else { Some(s) }
        }
    }

    let manufacturer_opt = c_str_to_option(manufacturer);
    let material_opt = c_str_to_option(material);

    let manager = BACKEND_MANAGER.lock().unwrap();
    let base_url = manager.server_url.clone();
    drop(manager);

    if base_url.is_empty() {
        return -1;
    }

    // GET /api/colors/search?manufacturer=X&material=Y
    let mut url = format!("{}/api/colors/search", base_url);
    let mut has_param = false;

    if let Some(ref m) = manufacturer_opt {
        url.push_str(&format!("?manufacturer={}", m.replace(' ', "%20")));
        has_param = true;
    }
    if let Some(ref m) = material_opt {
        url.push_str(&format!("{}material={}", if has_param { "&" } else { "?" }, m));
    }

    let config = HttpConfig {
        timeout: Some(std::time::Duration::from_millis(HTTP_TIMEOUT_MS)),
        ..Default::default()
    };

    let connection = match EspHttpConnection::new(&config) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut client = HttpClient::wrap(connection);
    let request = match client.get(&url) {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let mut response = match request.submit() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    if response.status() != 200 {
        return -1;
    }

    // Read response body
    let mut buf = vec![0u8; 65536]; // 64KB buffer for colors list
    let mut total = 0;
    loop {
        match response.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return 0;
    }

    // Parse response
    let body = String::from_utf8_lossy(&buf[..total]);
    let api_colors: Vec<ApiColorEntry> = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse color catalog: {:?}", e);
            return -1;
        }
    };

    let count = api_colors.len().min(max_count as usize);

    for (i, color) in api_colors.iter().take(count).enumerate() {
        let color_ref = unsafe { &mut *colors.add(i) };

        color_ref.id = color.id.unwrap_or(0);
        color_ref.manufacturer = [0; 64];
        color_ref.color_name = [0; 64];
        color_ref.hex_color = [0; 16];
        color_ref.material = [0; 32];

        if let Some(ref m) = color.manufacturer {
            copy_to_c_buf_signed(m, &mut color_ref.manufacturer);
        }
        if let Some(ref c) = color.color_name {
            copy_to_c_buf_signed(c, &mut color_ref.color_name);
        }
        if let Some(ref h) = color.hex_color {
            copy_to_c_buf_signed(h, &mut color_ref.hex_color);
        }
        if let Some(ref m) = color.material {
            copy_to_c_buf_signed(m, &mut color_ref.material);
        }
    }

    info!("backend_search_colors: returning {} colors", count);
    count as c_int
}

/// Helper to copy string to c_char buffer (signed char)
fn copy_to_c_buf_signed(src: &str, dest: &mut [c_char]) {
    let bytes = src.as_bytes();
    let len = bytes.len().min(dest.len() - 1);
    for i in 0..len {
        dest[i] = bytes[i] as c_char;
    }
    dest[len] = 0; // Null terminate
}

//! WiFi initialization using esp-idf-svc
//!
//! Connects to a configured WiFi network and returns the IP address.

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use esp_idf_hal::modem::Modem;
use log::{info, warn};

/// WiFi credentials - hardcoded for development
/// TODO: Load from NVS or config portal in production
const WIFI_SSID: &str = "NYHC!";
const WIFI_PASSWORD: &str = "335288888";

/// Initialize and connect to WiFi
/// Returns the IP address on success
/// Note: WiFi handle is leaked to keep connection alive for program duration
pub fn connect_wifi(
    modem: Modem,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspDefaultNvsPartition>,
) -> anyhow::Result<[u8; 4]> {
    info!("Initializing WiFi...");

    // Leak modem to get 'static lifetime (WiFi stays alive for duration of program)
    let modem: Modem<'static> = unsafe { std::mem::transmute(modem) };

    let esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs)?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop)?;

    let wifi_configuration = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: WIFI_PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    info!("Starting WiFi...");
    wifi.start()?;

    info!("Connecting to WiFi: {}", WIFI_SSID);
    match wifi.connect() {
        Ok(_) => info!("WiFi connected!"),
        Err(e) => {
            warn!("WiFi connection failed: {:?}", e);
            return Err(anyhow::anyhow!("WiFi connection failed"));
        }
    }

    info!("Waiting for DHCP...");
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    let ip = ip_info.ip;
    let ip_bytes = [
        ip.octets()[0],
        ip.octets()[1],
        ip.octets()[2],
        ip.octets()[3],
    ];

    info!("WiFi connected!");
    info!("  IP address: {}.{}.{}.{}", ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]);
    info!("  Gateway:    {}", ip_info.subnet.gateway);
    info!("  Subnet:     {}", ip_info.subnet.mask);

    // Leak WiFi handle to keep connection alive for program duration
    // This is intentional - the WiFi connection should persist
    std::mem::forget(wifi);

    Ok(ip_bytes)
}

/// Check if WiFi credentials are configured
#[allow(dead_code)]
pub fn is_configured() -> bool {
    !WIFI_SSID.is_empty() && WIFI_SSID != "YOUR_WIFI_SSID"
}

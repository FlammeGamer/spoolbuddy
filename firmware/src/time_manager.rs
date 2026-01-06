//! Time Manager with SNTP synchronization and backend fallback
//!
//! Provides NTP time sync and C-callable interface for UI clock display.
//! Falls back to backend server time if SNTP is unavailable.

use esp_idf_svc::sntp::{EspSntp, SyncStatus, SntpConf};
use log::{info, warn};
use std::ffi::c_int;
use std::sync::Mutex;
use std::time::SystemTime;

/// Time sync state
static TIME_SYNCED: Mutex<bool> = Mutex::new(false);
static SNTP_HANDLE: Mutex<Option<EspSntp<'static>>> = Mutex::new(None);

/// Backend time (hour, minute) - used when SNTP isn't available
static BACKEND_TIME: Mutex<Option<(u8, u8)>> = Mutex::new(None);

/// Initialize SNTP time synchronization
/// Call this after WiFi is connected
pub fn init_sntp() {
    let mut handle = SNTP_HANDLE.lock().unwrap();
    if handle.is_some() {
        return; // Already initialized
    }

    info!("Initializing SNTP time sync...");

    let conf = SntpConf::default();
    match EspSntp::new(&conf) {
        Ok(sntp) => {
            *handle = Some(sntp);
            info!("SNTP initialized, waiting for time sync...");
        }
        Err(e) => {
            warn!("Failed to initialize SNTP: {:?}", e);
        }
    }
}

/// Check if time is synchronized
pub fn is_time_synced() -> bool {
    let handle = SNTP_HANDLE.lock().unwrap();
    if let Some(ref sntp) = *handle {
        let synced = sntp.get_sync_status() == SyncStatus::Completed;
        if synced {
            let mut time_synced = TIME_SYNCED.lock().unwrap();
            if !*time_synced {
                info!("SNTP time synchronized");
                *time_synced = true;
            }
        }
        synced
    } else {
        false
    }
}

/// Set time from backend server
/// Called when we receive time from the backend API
pub fn set_backend_time(hour: u8, minute: u8) {
    let mut backend_time = BACKEND_TIME.lock().unwrap();
    *backend_time = Some((hour, minute));
}

// Timezone offset in seconds (CET = UTC+1 = 3600, CEST = UTC+2 = 7200)
// TODO: Make this configurable via backend
const TIMEZONE_OFFSET_SECS: u64 = 3600; // CET (UTC+1)

/// Get current time components (for UI display)
/// Returns (hour, minute) - tries SNTP first, falls back to backend time
pub fn get_time() -> Option<(u8, u8)> {
    // Try SNTP first
    if is_time_synced() {
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs() + TIMEZONE_OFFSET_SECS;
                // Local time calculation with timezone offset
                let day_secs = secs % 86400;
                let hour = (day_secs / 3600) as u8;
                let minute = ((day_secs % 3600) / 60) as u8;
                return Some((hour, minute));
            }
            Err(_) => {}
        }
    }

    // Fall back to backend time
    let backend_time = BACKEND_TIME.lock().unwrap();
    *backend_time
}

// ============================================================================
// C-callable interface
// ============================================================================

/// Get current time for UI display
/// Returns hour in upper 8 bits, minute in lower 8 bits
/// Returns -1 if time not synced
#[no_mangle]
pub extern "C" fn time_get_hhmm() -> c_int {
    match get_time() {
        Some((hour, minute)) => ((hour as c_int) << 8) | (minute as c_int),
        None => -1,
    }
}

/// Check if time is synchronized
/// Returns 1 if synced, 0 otherwise
#[no_mangle]
pub extern "C" fn time_is_synced() -> c_int {
    if is_time_synced() { 1 } else { 0 }
}

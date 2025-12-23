//! UI module for the 4.3" 800x480 touch display using LVGL.
//!
//! The Waveshare ESP32-S3-Touch-LCD-4.3 has:
//! - 800x480 RGB565 display (parallel RGB interface)
//! - GT911 capacitive touch controller (I2C)
//! - 5-point multi-touch support
//!
//! This module uses LVGL for the GUI framework, providing:
//! - Professional-looking widgets
//! - Touch input handling
//! - Screen transitions
//! - Theme support

#![allow(dead_code)]

pub mod display;
pub mod theme;
pub mod touch;

pub mod screens;
pub mod widgets;

use core::cell::RefCell;
use critical_section::Mutex;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use heapless::String;
use log::info;

/// Display dimensions
pub const DISPLAY_WIDTH: u32 = 800;
pub const DISPLAY_HEIGHT: u32 = 480;

/// UI refresh rate in Hz
pub const UI_REFRESH_RATE_HZ: u32 = 30;

/// UI Manager handles all GUI state and rendering
pub struct UiManager {
    /// Current screen
    current_screen: Screen,
    /// Shared state for UI updates
    state: UiState,
    /// Whether the UI needs to be redrawn
    dirty: bool,
}

/// Current active screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    SpoolInfo,
    AmsOverview,
    ScanResult,
    SpoolDetail,
    Catalog,
    AmsSelect,
    Settings,
    NfcReader,
    DisplayBrightness,
    About,
    Calibration,
    WifiSetup,
}

/// Settings tab for consolidated settings screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Network,
    Hardware,
    System,
}

/// Catalog filter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CatalogFilter {
    #[default]
    All,
    InAms,
    Pla,
    Petg,
    Other,
}

/// NFC reader status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NfcStatus {
    #[default]
    Ready,
    Reading,
    Success,
    Error,
    NotConnected,
}

/// Shared UI state
#[derive(Clone)]
pub struct UiState {
    /// Current weight in grams
    pub weight: f32,
    /// Is weight stable?
    pub weight_stable: bool,
    /// Current spool info (if any)
    pub spool: Option<SpoolDisplay>,
    /// WiFi connection status
    pub wifi_connected: bool,
    /// WiFi SSID (if connected)
    pub wifi_ssid: String<32>,
    /// Server connection status
    pub server_connected: bool,
    /// Display brightness (0-100)
    pub brightness: u8,
    /// Firmware version
    pub firmware_version: String<16>,
    /// Device ID
    pub device_id: String<32>,

    // Settings screen state
    /// Current settings tab
    pub settings_tab: SettingsTab,

    // Catalog state
    /// Current catalog filter
    pub catalog_filter: CatalogFilter,

    // NFC state
    /// NFC reader status
    pub nfc_status: NfcStatus,
    /// Last NFC tag ID read
    pub nfc_last_tag: String<32>,

    // Display settings
    /// Auto brightness enabled
    pub auto_brightness: bool,
    /// Screen timeout enabled
    pub screen_timeout: bool,
    /// Timeout duration in seconds
    pub timeout_seconds: u16,

    // AMS selection state
    /// Selected AMS slot (ams_id, slot_id) for assignment
    pub selected_ams_slot: Option<(u8, u8)>,
}

impl Default for UiState {
    fn default() -> Self {
        let mut firmware_version = String::new();
        let _ = firmware_version.push_str("v0.1.0");

        let mut device_id = String::new();
        let _ = device_id.push_str("SPOOLBUDDY-XXXX");

        Self {
            weight: 0.0,
            weight_stable: false,
            spool: None,
            wifi_connected: false,
            wifi_ssid: String::new(),
            server_connected: false,
            brightness: 80,
            firmware_version,
            device_id,
            settings_tab: SettingsTab::default(),
            catalog_filter: CatalogFilter::default(),
            nfc_status: NfcStatus::default(),
            nfc_last_tag: String::new(),
            auto_brightness: false,
            screen_timeout: true,
            timeout_seconds: 60,
            selected_ams_slot: None,
        }
    }
}

/// Spool information for display
#[derive(Clone)]
pub struct SpoolDisplay {
    pub id: String<64>,
    pub material: String<32>,
    pub color_name: String<32>,
    pub brand: String<32>,
    pub color_rgba: u32,
    pub weight_current: f32,
    pub weight_label: f32,
    pub k_value: Option<f32>,
    pub source: SpoolSource,
}

/// Where the spool data came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpoolSource {
    Bambu,
    Manual,
    Nfc,
}

impl UiManager {
    /// Create a new UI manager
    pub fn new() -> Self {
        info!("Creating UI manager");
        Self {
            current_screen: Screen::Home,
            state: UiState::default(),
            dirty: true,
        }
    }

    /// Get the current screen
    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    /// Navigate to a different screen
    pub fn navigate(&mut self, screen: Screen) {
        if self.current_screen != screen {
            info!("Navigating to {:?}", screen);
            self.current_screen = screen;
            self.dirty = true;
        }
    }

    /// Update the weight display
    pub fn set_weight(&mut self, grams: f32, stable: bool) {
        if (self.state.weight - grams).abs() > 0.05 || self.state.weight_stable != stable {
            self.state.weight = grams;
            self.state.weight_stable = stable;
            self.dirty = true;
        }
    }

    /// Update spool information
    pub fn set_spool(&mut self, spool: Option<SpoolDisplay>) {
        self.state.spool = spool;
        self.dirty = true;

        // Auto-navigate to spool info screen when spool detected
        if self.state.spool.is_some() && self.current_screen == Screen::Home {
            self.navigate(Screen::SpoolInfo);
        }
    }

    /// Update WiFi status
    pub fn set_wifi_status(&mut self, connected: bool, ssid: Option<&str>) {
        self.state.wifi_connected = connected;
        self.state.wifi_ssid.clear();
        if let Some(s) = ssid {
            let _ = self.state.wifi_ssid.push_str(s);
        }
        self.dirty = true;
    }

    /// Update server connection status
    pub fn set_server_connected(&mut self, connected: bool) {
        if self.state.server_connected != connected {
            self.state.server_connected = connected;
            self.dirty = true;
        }
    }

    /// Set display brightness
    pub fn set_brightness(&mut self, brightness: u8) {
        self.state.brightness = brightness.min(100);
        self.dirty = true;
    }

    /// Check if UI needs redraw
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark UI as clean after rendering
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Get current state (for rendering)
    pub fn state(&self) -> &UiState {
        &self.state
    }

    /// Handle touch event
    pub fn handle_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        match self.current_screen {
            Screen::Home => self.handle_home_touch(event),
            Screen::SpoolInfo => self.handle_spool_info_touch(event),
            Screen::Settings => self.handle_settings_touch(event),
            Screen::AmsSelect => self.handle_ams_select_touch(event),
            Screen::Calibration => self.handle_calibration_touch(event),
            Screen::WifiSetup => self.handle_wifi_setup_touch(event),
            Screen::AmsOverview => self.handle_ams_overview_touch(event),
            Screen::ScanResult => self.handle_scan_result_touch(event),
            Screen::SpoolDetail => self.handle_spool_detail_touch(event),
            Screen::Catalog => self.handle_catalog_touch(event),
            Screen::NfcReader => self.handle_nfc_reader_touch(event),
            Screen::DisplayBrightness => self.handle_display_brightness_touch(event),
            Screen::About => self.handle_about_touch(event),
        }
    }

    // Touch handlers for each screen
    fn handle_home_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Tare button (bottom left)
            if x < 200 && y > 400 {
                return Some(UiAction::TareScale);
            }
            // Settings button (bottom right)
            if x > 600 && y > 400 {
                self.navigate(Screen::Settings);
                return None;
            }
        }
        None
    }

    fn handle_spool_info_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Bottom button row
            if y > 400 {
                if x < 200 {
                    return Some(UiAction::AssignToAms);
                } else if x < 400 {
                    return Some(UiAction::UpdateWeight);
                } else if x < 600 {
                    return Some(UiAction::WriteTag);
                } else {
                    self.navigate(Screen::Settings);
                }
            }
        }
        None
    }

    fn handle_settings_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button (top left)
            if x < 60 && y < 48 {
                self.navigate(Screen::Home);
                return None;
            }

            // Tab bar (y 48-84)
            if y >= 48 && y < 84 {
                let tab_width = DISPLAY_WIDTH as u16 / 3;
                let tab_index = x / tab_width;
                let new_tab = match tab_index {
                    0 => SettingsTab::Network,
                    1 => SettingsTab::Hardware,
                    2 => SettingsTab::System,
                    _ => return None,
                };
                if self.state.settings_tab != new_tab {
                    self.state.settings_tab = new_tab;
                    self.dirty = true;
                }
                return None;
            }

            // Content rows (y 92+, each row is 48px)
            // Row 1: y 92-140, Row 2: y 140-188, Row 3: y 188-236
            if y >= 92 && y < 236 {
                let row = ((y - 92) / 48) as u8;

                match self.state.settings_tab {
                    SettingsTab::Network => {
                        // Network tab: WiFi, Backend Server, Printers
                        match row {
                            0 => self.navigate(Screen::WifiSetup),
                            // 1 => Backend server config (not implemented)
                            // 2 => Printers list (not implemented)
                            _ => {}
                        }
                    }
                    SettingsTab::Hardware => {
                        // Hardware tab: Scale Calibration, NFC Reader, Display
                        match row {
                            0 => self.navigate(Screen::Calibration),
                            1 => self.navigate(Screen::NfcReader),
                            2 => self.navigate(Screen::DisplayBrightness),
                            _ => {}
                        }
                    }
                    SettingsTab::System => {
                        // System tab: Check for Updates, Advanced Settings, About
                        match row {
                            0 => return Some(UiAction::CheckForUpdates),
                            // 1 => Advanced settings (not implemented)
                            2 => self.navigate(Screen::About),
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    fn handle_ams_select_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        // TODO: Implement AMS slot selection
        None
    }

    fn handle_calibration_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        // TODO: Implement calibration flow
        None
    }

    fn handle_wifi_setup_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        // TODO: Implement WiFi setup
        None
    }

    fn handle_ams_overview_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button (top left)
            if x < 60 && y < 48 {
                self.navigate(Screen::Home);
                return None;
            }
            // Action buttons (right sidebar)
            if x > 600 {
                if y >= 48 && y < 140 {
                    // Scan button
                    self.navigate(Screen::ScanResult);
                } else if y >= 140 && y < 232 {
                    // Catalog button
                    self.navigate(Screen::Catalog);
                } else if y >= 232 && y < 324 {
                    // Calibrate button
                    self.navigate(Screen::Calibration);
                } else if y >= 324 && y < 416 {
                    // Settings button
                    self.navigate(Screen::Settings);
                }
            }
        }
        None
    }

    fn handle_scan_result_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::Home);
                return None;
            }
            // Assign & Save button (bottom)
            if y > 400 && x > 300 && x < 500 {
                return Some(UiAction::AssignAndSave);
            }
        }
        None
    }

    fn handle_spool_detail_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::AmsOverview);
                return None;
            }
        }
        None
    }

    fn handle_catalog_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::AmsOverview);
                return None;
            }
            // Filter pills (y ~60-90)
            if y >= 56 && y < 90 {
                // TODO: Calculate which filter pill was pressed
            }
        }
        None
    }

    fn handle_nfc_reader_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::Settings);
                return None;
            }
            // Test button (bottom)
            if y > 380 && x > 300 && x < 500 {
                return Some(UiAction::TestNfc);
            }
        }
        None
    }

    fn handle_display_brightness_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::Settings);
                return None;
            }
            // Brightness slider area (y ~100-140)
            if y >= 100 && y < 160 && x >= 100 && x < 700 {
                // Calculate brightness from x position
                let brightness = ((x - 100) * 100 / 600).min(100) as u8;
                return Some(UiAction::SetBrightness(brightness));
            }
            // Auto brightness toggle (~y 200)
            if y >= 180 && y < 230 && x > 650 {
                return Some(UiAction::ToggleAutoBrightness);
            }
            // Screen timeout toggle (~y 250)
            if y >= 240 && y < 290 && x > 650 {
                return Some(UiAction::ToggleScreenTimeout);
            }
        }
        None
    }

    fn handle_about_touch(&mut self, event: TouchEvent) -> Option<UiAction> {
        if let TouchEvent::Press { x, y } = event {
            // Back button
            if x < 60 && y < 48 {
                self.navigate(Screen::Settings);
                return None;
            }
        }
        None
    }
}

/// Touch event types
#[derive(Debug, Clone, Copy)]
pub enum TouchEvent {
    Press { x: u16, y: u16 },
    Release { x: u16, y: u16 },
    Move { x: u16, y: u16 },
}

/// Actions the UI can request
#[derive(Debug, Clone)]
pub enum UiAction {
    TareScale,
    CalibrateScale { weight_grams: f32 },
    AssignToAms,
    UpdateWeight,
    WriteTag,
    ConfigureWifi,
    SetBrightness(u8),
    // New actions for added screens
    ChangeSettingsTab(SettingsTab),
    SetCatalogFilter(CatalogFilter),
    SelectAmsSlot { ams_id: u8, slot_id: u8 },
    AssignAndSave,
    TestNfc,
    CheckForUpdates,
    ToggleAutoBrightness,
    ToggleScreenTimeout,
    SetTimeoutDuration(u16),
    OpenSpoolDetail,
    NavigateBack,
}

/// Display errors
#[derive(Debug, Clone, Copy)]
pub enum DisplayError {
    InitFailed,
    I2cError,
    SpiError,
    InvalidConfig,
    BufferOverflow,
}

/// Global UI manager instance (for interrupt handlers)
static UI_MANAGER: Mutex<RefCell<Option<UiManager>>> = Mutex::new(RefCell::new(None));

/// Initialize the global UI manager
pub fn init_ui_manager() {
    critical_section::with(|cs| {
        *UI_MANAGER.borrow_ref_mut(cs) = Some(UiManager::new());
    });
}

/// Access the UI manager from any context
pub fn with_ui<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut UiManager) -> R,
{
    critical_section::with(|cs| {
        if let Some(ref mut ui) = *UI_MANAGER.borrow_ref_mut(cs) {
            Some(f(ui))
        } else {
            None
        }
    })
}

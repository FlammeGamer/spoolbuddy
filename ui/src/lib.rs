//! SpoolBuddy UI - Shared UI components for embedded and simulator.
//!
//! This crate provides platform-agnostic UI components using embedded-graphics.
//! It can be used with any display that implements DrawTarget<Color = Rgb565>.
//!
//! # Supported Platforms
//! - ESP32-S3 with RGB LCD (firmware)
//! - PC with SDL2 (simulator)

#![allow(dead_code)]

pub mod theme;
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
    AmsSelect,
    Settings,
    Calibration,
    WifiSetup,
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
            if x < 100 && y < 60 {
                self.navigate(Screen::Home);
                return None;
            }
            // WiFi configure button
            if y > 80 && y < 140 {
                self.navigate(Screen::WifiSetup);
                return None;
            }
            // Tare scale button
            if y > 200 && y < 240 {
                return Some(UiAction::TareScale);
            }
            // Calibrate scale button
            if y > 240 && y < 280 {
                self.navigate(Screen::Calibration);
                return None;
            }
        }
        None
    }

    fn handle_ams_select_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        None
    }

    fn handle_calibration_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        None
    }

    fn handle_wifi_setup_touch(&mut self, _event: TouchEvent) -> Option<UiAction> {
        None
    }
}

impl Default for UiManager {
    fn default() -> Self {
        Self::new()
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

/// Render the current screen to a display
pub fn render<D>(display: &mut D, ui: &UiManager) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    match ui.current_screen() {
        Screen::Home => screens::HomeScreen::render(display, ui.state()),
        Screen::SpoolInfo => screens::SpoolInfoScreen::render(display, ui.state()),
        Screen::Settings => screens::SettingsScreen::render(display, ui.state()),
        Screen::AmsSelect => screens::AmsSelectScreen::render(display, ui.state()),
        Screen::Calibration => screens::CalibrationScreen::render(display, ui.state()),
        Screen::WifiSetup => {
            // Use settings screen as placeholder
            screens::SettingsScreen::render(display, ui.state())
        }
    }
}

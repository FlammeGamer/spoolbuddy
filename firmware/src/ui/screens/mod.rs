//! Screen implementations for SpoolBuddy.
//!
//! Each screen module contains the layout and rendering logic for a specific view.

pub mod about;
pub mod ams_overview;
pub mod ams_select;
pub mod calibration;
pub mod catalog;
pub mod display_brightness;
pub mod home;
pub mod nfc_reader;
pub mod scan_result;
pub mod settings;
pub mod spool_detail;
pub mod spool_info;

pub use about::AboutScreen;
pub use ams_overview::AmsOverviewScreen;
pub use ams_select::AmsSelectScreen;
pub use calibration::CalibrationScreen;
pub use catalog::CatalogScreen;
pub use display_brightness::DisplayBrightnessScreen;
pub use home::HomeScreen;
pub use nfc_reader::NfcReaderScreen;
pub use scan_result::ScanResultScreen;
pub use settings::SettingsScreen;
pub use spool_detail::SpoolDetailScreen;
pub use spool_info::SpoolInfoScreen;

use crate::ui::{Screen, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

/// Render the current screen
pub fn render_screen<D>(display: &mut D, screen: Screen, state: &UiState) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    match screen {
        Screen::Home => HomeScreen::render(display, state),
        Screen::SpoolInfo => SpoolInfoScreen::render(display, state),
        Screen::AmsOverview => AmsOverviewScreen::render(display, state),
        Screen::ScanResult => ScanResultScreen::render(display, state),
        Screen::SpoolDetail => SpoolDetailScreen::render(display, state),
        Screen::Catalog => CatalogScreen::render(display, state),
        Screen::AmsSelect => AmsSelectScreen::render(display, state),
        Screen::Settings => SettingsScreen::render(display, state),
        Screen::NfcReader => NfcReaderScreen::render(display, state),
        Screen::DisplayBrightness => DisplayBrightnessScreen::render(display, state),
        Screen::About => AboutScreen::render(display, state),
        Screen::Calibration => CalibrationScreen::render(display, state),
        Screen::WifiSetup => {
            // WiFi setup uses settings screen for now
            SettingsScreen::render(display, state)
        }
    }
}

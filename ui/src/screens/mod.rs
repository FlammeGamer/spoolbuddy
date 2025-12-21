//! Screen implementations for SpoolBuddy.
//!
//! Each screen module contains the layout and rendering logic for a specific view.

pub mod home;
pub mod spool_info;
pub mod settings;
pub mod ams_select;
pub mod calibration;

pub use home::HomeScreen;
pub use spool_info::SpoolInfoScreen;
pub use settings::SettingsScreen;
pub use ams_select::AmsSelectScreen;
pub use calibration::CalibrationScreen;

use crate::{Screen, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
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
        Screen::Settings => SettingsScreen::render(display, state),
        Screen::AmsSelect => AmsSelectScreen::render(display, state),
        Screen::Calibration => CalibrationScreen::render(display, state),
        Screen::WifiSetup => {
            // WiFi setup is similar to settings for now
            SettingsScreen::render(display, state)
        }
    }
}

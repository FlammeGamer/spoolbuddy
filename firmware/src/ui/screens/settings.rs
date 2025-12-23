//! Settings screen with consolidated tabs.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < Settings                                                 │
//! ├────────────────────────────────────────────────────────────┤
//! │  [Network]  [Hardware]  [System]                          │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  Network Tab:                                              │
//! │  ┌──────────────────────────────────────────────────────┐ │
//! │  │ ● WiFi                              MyNetwork  >     │ │
//! │  │ ● Backend Server                   Connected   >     │ │
//! │  │   Printers                                     >     │ │
//! │  └──────────────────────────────────────────────────────┘ │
//! │                                                            │
//! │  Hardware Tab:                                             │
//! │  ┌──────────────────────────────────────────────────────┐ │
//! │  │   Scale Calibration                            >     │ │
//! │  │   NFC Reader                                   >     │ │
//! │  │   Display                                      >     │ │
//! │  └──────────────────────────────────────────────────────┘ │
//! │                                                            │
//! │  System Tab:                                               │
//! │  ┌──────────────────────────────────────────────────────┐ │
//! │  │   Check for Updates                            >     │ │
//! │  │   Advanced Settings                            >     │ │
//! │  │   About SpoolBuddy                            >     │ │
//! │  └──────────────────────────────────────────────────────┘ │
//! │                                                            │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::icon::Icon;
use crate::ui::widgets::{SettingsRow, StatusBar, StatusDot, TabBar};
use crate::ui::{Screen, SettingsTab, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
};

/// Settings screen renderer
pub struct SettingsScreen;

impl SettingsScreen {
    /// Render the settings screen
    pub fn render<D>(display: &mut D, state: &UiState) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Clear background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.bg))
            .draw(display)?;

        // Status bar with back button
        let mut status_bar = StatusBar::new("< Settings");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        // Tab bar
        let tabs = ["Network", "Hardware", "System"];
        let active_tab = match state.settings_tab {
            SettingsTab::Network => 0,
            SettingsTab::Hardware => 1,
            SettingsTab::System => 2,
        };

        TabBar::new(&tabs, Point::new(0, 48), DISPLAY_WIDTH)
            .with_active(active_tab)
            .draw(display)?;

        let content_y = 92;
        let card_x = spacing::MD;
        let card_width = DISPLAY_WIDTH - (spacing::MD * 2) as u32;

        // Render content based on active tab
        match state.settings_tab {
            SettingsTab::Network => Self::render_network_tab(display, state, card_x, content_y, card_width)?,
            SettingsTab::Hardware => Self::render_hardware_tab(display, state, card_x, content_y, card_width)?,
            SettingsTab::System => Self::render_system_tab(display, state, card_x, content_y, card_width)?,
        }

        Ok(())
    }

    /// Render Network tab content
    fn render_network_tab<D>(
        display: &mut D,
        state: &UiState,
        card_x: i32,
        content_y: i32,
        card_width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let card_height = 160u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, content_y),
                Size::new(card_width, card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        let row_width = card_width - spacing::SM as u32;
        let mut row_y = content_y;

        // WiFi row
        let wifi_status = if state.wifi_connected {
            StatusDot::Green
        } else {
            StatusDot::Gray
        };
        let wifi_value = if state.wifi_connected {
            state.wifi_ssid.as_str()
        } else {
            "Not connected"
        };

        SettingsRow::new(Point::new(card_x, row_y), row_width, "WiFi")
            .with_icon(Icon::Setting)
            .with_status(wifi_status)
            .with_value(wifi_value)
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // Backend Server row
        let server_status = if state.server_connected {
            StatusDot::Green
        } else {
            StatusDot::Gray
        };
        let server_value = if state.server_connected {
            "Connected"
        } else {
            "Disconnected"
        };

        SettingsRow::new(Point::new(card_x, row_y), row_width, "Backend Server")
            .with_icon(Icon::Setting)
            .with_status(server_status)
            .with_value(server_value)
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // Printers row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "Printers")
            .with_icon(Icon::Setting)
            .with_value("1 connected")
            .without_separator()
            .draw(display)?;

        Ok(())
    }

    /// Render Hardware tab content
    fn render_hardware_tab<D>(
        display: &mut D,
        _state: &UiState,
        card_x: i32,
        content_y: i32,
        card_width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let card_height = 160u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, content_y),
                Size::new(card_width, card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        let row_width = card_width - spacing::SM as u32;
        let mut row_y = content_y;

        // Scale Calibration row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "Scale Calibration")
            .with_icon(Icon::Weight)
            .with_value("Calibrated")
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // NFC Reader row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "NFC Reader")
            .with_icon(Icon::Nfc)
            .with_value("Ready")
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // Display row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "Display")
            .with_icon(Icon::Setting)
            .with_value("80%")
            .without_separator()
            .draw(display)?;

        Ok(())
    }

    /// Render System tab content
    fn render_system_tab<D>(
        display: &mut D,
        state: &UiState,
        card_x: i32,
        content_y: i32,
        card_width: u32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let card_height = 160u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, content_y),
                Size::new(card_width, card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        let row_width = card_width - spacing::SM as u32;
        let mut row_y = content_y;

        // Check for Updates row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "Check for Updates")
            .with_icon(Icon::Setting)
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // Advanced Settings row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "Advanced Settings")
            .with_icon(Icon::Setting)
            .draw(display)?;
        row_y += SettingsRow::HEIGHT as i32;

        // About SpoolBuddy row
        SettingsRow::new(Point::new(card_x, row_y), row_width, "About SpoolBuddy")
            .with_icon(Icon::Setting)
            .with_value(state.firmware_version.as_str())
            .without_separator()
            .draw(display)?;

        Ok(())
    }

    /// Get back button bounds
    pub fn get_back_button_bounds() -> Rectangle {
        Rectangle::new(Point::new(0, 0), Size::new(60, 48))
    }

    /// Get tab bar bounds
    pub fn get_tab_bar_y() -> i32 {
        48
    }

    /// Calculate which tab was pressed
    pub fn get_tab_from_point(point: Point) -> Option<SettingsTab> {
        if point.y < 48 || point.y >= 84 {
            return None;
        }

        let tab_width = DISPLAY_WIDTH as i32 / 3;
        let index = point.x / tab_width;

        match index {
            0 => Some(SettingsTab::Network),
            1 => Some(SettingsTab::Hardware),
            2 => Some(SettingsTab::System),
            _ => None,
        }
    }
}

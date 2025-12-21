//! Settings screen.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ ← Settings                                                │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  WiFi                                                      │
//! │  ├── Network: NYHC! (Connected)                           │
//! │  └── [Configure WiFi]                                     │
//! │                                                            │
//! │  Server                                                    │
//! │  ├── URL: spoolbuddy.local:3000                           │
//! │  └── Status: Connected                                    │
//! │                                                            │
//! │  Scale                                                     │
//! │  ├── [Tare Scale]                                         │
//! │  └── [Calibrate Scale]                                    │
//! │                                                            │
//! │  Display                                                   │
//! │  └── Brightness: [━━━━━━━━░░] 80%                         │
//! │  └── Theme: [Dark] / Light                                │
//! │                                                            │
//! │  About                                                     │
//! │  ├── Firmware: v0.1.0                                     │
//! │  └── Device ID: SPOOLBUDDY-A1B2C3                         │
//! │                                                            │
//! └────────────────────────────────────────────────────────────┘

use crate::theme::{self, spacing, ThemeMode};
use crate::widgets::{Button, StatusBar};
use crate::widgets::button::ButtonStyle;
use crate::widgets::icon::Icon;
use crate::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
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

        // Header with back button
        let header_height = 50;
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, header_height))
            .into_styled(PrimitiveStyle::with_fill(theme.status_bar_bg))
            .draw(display)?;

        // Back icon
        Icon::Back.draw(display, Point::new(spacing::MD, 15), 24, theme.text_primary)?;

        // Title
        let title_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        Text::new("Settings", Point::new(spacing::MD + 36, 32), title_style).draw(display)?;

        // Settings sections
        let mut y = header_height as i32 + spacing::MD;
        let section_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let value_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);

        // WiFi section
        y = Self::draw_section(display, "WiFi", y)?;
        y = Self::draw_setting_row(
            display,
            "Network",
            if state.wifi_connected {
                state.wifi_ssid.as_str()
            } else {
                "Not connected"
            },
            y,
        )?;

        let wifi_button = Button::new(
            Point::new(spacing::MD + 20, y),
            Size::new(140, 32),
            "Configure WiFi",
        )
        .with_style(ButtonStyle::Secondary);
        wifi_button.draw(display)?;
        y += 44;

        // Server section
        y = Self::draw_section(display, "Server", y)?;
        y = Self::draw_setting_row(display, "URL", "spoolbuddy.local:3000", y)?;
        y = Self::draw_setting_row(
            display,
            "Status",
            if state.server_connected {
                "Connected"
            } else {
                "Disconnected"
            },
            y,
        )?;

        // Scale section
        y = Self::draw_section(display, "Scale", y)?;

        let tare_button = Button::new(
            Point::new(spacing::MD + 20, y),
            Size::new(100, 32),
            "Tare",
        )
        .with_style(ButtonStyle::Secondary);
        tare_button.draw(display)?;

        let cal_button = Button::new(
            Point::new(spacing::MD + 140, y),
            Size::new(100, 32),
            "Calibrate",
        )
        .with_style(ButtonStyle::Secondary);
        cal_button.draw(display)?;
        y += 44;

        // Display section
        y = Self::draw_section(display, "Display", y)?;

        // Brightness slider
        Text::new("Brightness", Point::new(spacing::MD + 20, y + 12), label_style).draw(display)?;

        // Progress bar for brightness
        let slider_x = spacing::MD + 100;
        let slider_width = 200;
        let slider_height = 12;

        // Track
        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(slider_x, y + 4), Size::new(slider_width, slider_height)),
            Size::new(6, 6),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.progress_bg))
        .draw(display)?;

        // Fill
        let fill_width = ((slider_width as u32) * (state.brightness as u32) / 100) as u32;
        RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(slider_x, y + 4), Size::new(fill_width, slider_height)),
            Size::new(6, 6),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.primary))
        .draw(display)?;

        // Percentage label
        let mut pct: heapless::String<8> = heapless::String::new();
        let _ = core::fmt::write(&mut pct, format_args!("{}%", state.brightness));
        Text::new(&pct, Point::new(slider_x + slider_width as i32 + 8, y + 14), value_style)
            .draw(display)?;
        y += 28;

        // Theme toggle
        Text::new("Theme", Point::new(spacing::MD + 20, y + 12), label_style).draw(display)?;

        let current_mode = theme::theme_mode();
        let dark_button = Button::new(
            Point::new(slider_x, y),
            Size::new(60, 28),
            "Dark",
        )
        .with_style(if current_mode == ThemeMode::Dark {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Secondary
        });
        dark_button.draw(display)?;

        let light_button = Button::new(
            Point::new(slider_x + 70, y),
            Size::new(60, 28),
            "Light",
        )
        .with_style(if current_mode == ThemeMode::Light {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Secondary
        });
        light_button.draw(display)?;
        y += 40;

        // About section
        y = Self::draw_section(display, "About", y)?;
        y = Self::draw_setting_row(display, "Firmware", state.firmware_version.as_str(), y)?;
        Self::draw_setting_row(display, "Device ID", state.device_id.as_str(), y)?;

        Ok(())
    }

    /// Draw a section header
    fn draw_section<D>(display: &mut D, title: &str, y: i32) -> Result<i32, D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let section_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);

        Text::new(title, Point::new(spacing::MD, y + 16), section_style).draw(display)?;

        // Divider line
        Rectangle::new(
            Point::new(spacing::MD, y + 24),
            Size::new(DISPLAY_WIDTH - (spacing::MD as u32 * 2), 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        Ok(y + 32)
    }

    /// Draw a setting row with label and value
    fn draw_setting_row<D>(
        display: &mut D,
        label: &str,
        value: &str,
        y: i32,
    ) -> Result<i32, D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let value_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);

        // Tree line indicator
        Text::new("├─", Point::new(spacing::MD + 4, y + 10), label_style).draw(display)?;

        // Label
        Text::new(label, Point::new(spacing::MD + 24, y + 10), label_style).draw(display)?;

        // Value (right side)
        let value_x = DISPLAY_WIDTH as i32 - spacing::MD - (value.len() as i32 * 6);
        Text::new(value, Point::new(value_x, y + 10), value_style).draw(display)?;

        Ok(y + 20)
    }

    /// Get back button bounds
    pub fn get_back_button_bounds() -> Rectangle {
        Rectangle::new(Point::new(0, 0), Size::new(100, 50))
    }

    /// Check if point is in brightness slider
    pub fn is_in_brightness_slider(point: Point) -> bool {
        let slider_x = spacing::MD + 100;
        let slider_width = 200;
        let slider_y = 50 + spacing::MD + 32 + 20 + 44 + 20 + 20 + 44 + 32 + 4; // Approximate y position

        point.x >= slider_x
            && point.x < slider_x + slider_width as i32
            && point.y >= slider_y as i32
            && point.y < (slider_y + 20) as i32
    }

    /// Get brightness from slider position
    pub fn get_brightness_from_point(point: Point) -> u8 {
        let slider_x = spacing::MD + 100;
        let slider_width = 200;

        let relative_x = (point.x - slider_x).max(0).min(slider_width as i32);
        ((relative_x as u32) * 100 / slider_width as u32) as u8
    }
}

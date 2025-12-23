//! About screen - displays firmware version and device info.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < About                                                     │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │     ┌──────────────────────────────────────────────────┐  │
//! │     │                  SpoolBuddy                      │  │
//! │     │            Filament Management System            │  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! │     ┌──────────────────────────────────────────────────┐  │
//! │     │  Version          v0.1.0                         │  │
//! │     │  Build            2024.12.21                     │  │
//! │     │  Hardware         ESP32-S3                       │  │
//! │     │  Device ID        SPOOLBUDDY-XXXX                │  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! │            Made with care for the 3D printing             │
//! │                      community                            │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::{InfoRow, StatusBar};
use crate::ui::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// About screen renderer
pub struct AboutScreen;

impl AboutScreen {
    /// Render the about screen
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
        let mut status_bar = StatusBar::new("< About");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        let content_y = 60;

        // Title card
        let title_card_width = 600u32;
        let title_card_height = 80u32;
        let title_card_x = (DISPLAY_WIDTH - title_card_width) as i32 / 2;
        let title_card_y = content_y;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(title_card_x, title_card_y),
                Size::new(title_card_width, title_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // SpoolBuddy title
        let title_style = MonoTextStyle::new(&FONT_10X20, theme.primary);
        Text::with_alignment(
            "SpoolBuddy",
            Point::new(
                DISPLAY_WIDTH as i32 / 2,
                title_card_y + 35,
            ),
            title_style,
            Alignment::Center,
        )
        .draw(display)?;

        // Subtitle
        let subtitle_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        Text::with_alignment(
            "Filament Management System",
            Point::new(
                DISPLAY_WIDTH as i32 / 2,
                title_card_y + 55,
            ),
            subtitle_style,
            Alignment::Center,
        )
        .draw(display)?;

        // Info card
        let info_card_y = title_card_y + title_card_height as i32 + spacing::LG;
        let info_card_height = 160u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(title_card_x, info_card_y),
                Size::new(title_card_width, info_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Info rows
        let row_width = title_card_width - spacing::MD as u32 * 2;
        let row_x = title_card_x + spacing::MD;
        let mut row_y = info_card_y + spacing::SM;

        // Version row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Version",
            state.firmware_version.as_str(),
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Build row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Build",
            "2024.12.21",
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Hardware row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Hardware",
            "ESP32-S3 Touch LCD 4.3",
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Device ID row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Device ID",
            state.device_id.as_str(),
        )
        .without_separator()
        .draw(display)?;

        // Footer text
        let footer_y = info_card_y + info_card_height as i32 + spacing::XL;
        Text::with_alignment(
            "Made with care for the 3D printing community",
            Point::new(DISPLAY_WIDTH as i32 / 2, footer_y),
            subtitle_style,
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}

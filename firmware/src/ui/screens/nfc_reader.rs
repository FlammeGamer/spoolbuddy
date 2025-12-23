//! NFC reader screen - displays NFC reader status and controls.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < NFC Reader                                               │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │     ┌──────────────────────────────────────────────────┐  │
//! │     │                   [NFC Icon]                     │  │
//! │     │                     Ready                        │  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! │     ┌──────────────────────────────────────────────────┐  │
//! │     │  Reader Type                         PN532       │  │
//! │     │  Connection                           I2C        │  │
//! │     │  Tags Read                             12        │  │
//! │     │  Last Read                    spool_123          │  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! │                      [Test Reader]                        │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::icon::Icon;
use crate::ui::widgets::{Button, InfoRow, StatusBar};
use crate::ui::widgets::button::ButtonStyle;
use crate::ui::{NfcStatus, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// NFC reader screen renderer
pub struct NfcReaderScreen;

impl NfcReaderScreen {
    /// Render the NFC reader screen
    pub fn render<D>(display: &mut D, state: &UiState) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Clear background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.bg))
            .draw(display)?;

        // Status bar
        let mut status_bar = StatusBar::new("< NFC Reader");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        let card_width = 600u32;
        let card_x = (DISPLAY_WIDTH - card_width) as i32 / 2;
        let mut y = 60;

        // Status card
        let status_card_height = 100u32;
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, y),
                Size::new(card_width, status_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // NFC icon
        let (icon_color, status_text) = match state.nfc_status {
            NfcStatus::Ready => (theme.success, "Ready"),
            NfcStatus::Reading => (theme.primary, "Reading..."),
            NfcStatus::Success => (theme.success, "Tag Read!"),
            NfcStatus::Error => (theme.error, "Error"),
            NfcStatus::NotConnected => (theme.disabled, "Not Connected"),
        };

        Icon::Nfc.draw(
            display,
            Point::new(DISPLAY_WIDTH as i32 / 2 - 24, y + 20),
            48,
            icon_color,
        )?;

        // Status text
        let status_style = MonoTextStyle::new(&FONT_10X20, icon_color);
        Text::with_alignment(
            status_text,
            Point::new(DISPLAY_WIDTH as i32 / 2, y + 85),
            status_style,
            Alignment::Center,
        )
        .draw(display)?;

        y += status_card_height as i32 + spacing::LG;

        // Info card
        let info_card_height = 140u32;
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, y),
                Size::new(card_width, info_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        let row_x = card_x + spacing::MD;
        let row_width = card_width - (spacing::MD * 2) as u32;
        let mut row_y = y + spacing::SM;

        // Reader Type row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Reader Type",
            "PN532",
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Connection row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Connection",
            "I2C",
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Tags Read row
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Tags Read",
            "0",
        )
        .draw(display)?;
        row_y += InfoRow::HEIGHT as i32;

        // Last Read row
        let last_tag = if state.nfc_last_tag.is_empty() {
            "-"
        } else {
            state.nfc_last_tag.as_str()
        };
        InfoRow::new(
            Point::new(row_x, row_y),
            row_width,
            "Last Read",
            last_tag,
        )
        .without_separator()
        .draw(display)?;

        y += info_card_height as i32 + spacing::LG;

        // Test button
        let button_width = 200u32;
        let button_x = (DISPLAY_WIDTH - button_width) as i32 / 2;
        Button::new(
            Point::new(button_x, y),
            Size::new(button_width, 48),
            "Test Reader",
        )
        .with_style(ButtonStyle::Primary)
        .with_large_font()
        .draw(display)?;

        Ok(())
    }
}

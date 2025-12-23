//! Spool detail screen - detailed spool information and settings.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < Spool Detail                                             │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  ┌──────────┐  ┌────────────────────────────────────────┐ │
//! │  │  ████    │  │  Print Settings                        │ │
//! │  │  ████ A1 │  │  Nozzle Temp            220°C          │ │
//! │  │  ████    │  │  Bed Temp               60°C           │ │
//! │  │          │  │  K Factor               0.024          │ │
//! │  │  PLA     │  │  Max Speed              100%           │ │
//! │  │  Yellow  │  └────────────────────────────────────────┘ │
//! │  │  850g    │                                             │
//! │  └──────────┘  ┌────────────────────────────────────────┐ │
//! │                │  Spool Information                     │ │
//! │                │  Tag ID              NTAG215-ABC123    │ │
//! │                │  Net Weight          1000g             │ │
//! │                │  Current Weight      850g              │ │
//! │                │  Source              Bambu Lab         │ │
//! │                └────────────────────────────────────────┘ │
//! │                                                            │
//! │  [Assign Slot]    [Edit Info]    [Remove]                 │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::{Button, InfoRow, StatusBar};
use crate::ui::widgets::button::ButtonStyle;
use crate::ui::{SpoolSource, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// Spool detail screen renderer
pub struct SpoolDetailScreen;

impl SpoolDetailScreen {
    /// Render the spool detail screen
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
        let mut status_bar = StatusBar::new("< Spool Detail");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        let content_y = 56;

        // Get spool info or use defaults
        let (material, color_name, color, weight_current, weight_label, source) =
            if let Some(ref spool) = state.spool {
                let source_str = match spool.source {
                    SpoolSource::Bambu => "Bambu Lab",
                    SpoolSource::Manual => "Manual",
                    SpoolSource::Nfc => "NFC Tag",
                };
                (
                    spool.material.as_str(),
                    spool.color_name.as_str(),
                    theme::rgba_to_rgb565(spool.color_rgba),
                    spool.weight_current,
                    spool.weight_label,
                    source_str,
                )
            } else {
                ("PLA", "Yellow", theme::rgba_to_rgb565(0xF5C518FF), 850.0, 1000.0, "Unknown")
            };

        // Left side: Spool visualization
        let spool_card_x = spacing::LG;
        let spool_card_y = content_y + spacing::SM;
        let spool_card_width = 160u32;
        let spool_card_height = 220u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(spool_card_x, spool_card_y),
                Size::new(spool_card_width, spool_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Color swatch
        let swatch_size = 100u32;
        let swatch_x = spool_card_x + (spool_card_width - swatch_size) as i32 / 2;
        let swatch_y = spool_card_y + spacing::MD;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(swatch_x, swatch_y),
                Size::new(swatch_size, swatch_size),
            ),
            Size::new(radius::MD, radius::MD),
        )
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(display)?;

        // Slot badge (if assigned)
        if state.selected_ams_slot.is_some() {
            let badge_text = "A1"; // Would come from state
            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(swatch_x + swatch_size as i32 - 28, swatch_y + 4),
                    Size::new(24, 18),
                ),
                Size::new(radius::SM, radius::SM),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.primary))
            .draw(display)?;

            let badge_style = MonoTextStyle::new(&FONT_6X10, theme.bg);
            Text::new(
                badge_text,
                Point::new(swatch_x + swatch_size as i32 - 24, swatch_y + 16),
                badge_style,
            )
            .draw(display)?;
        }

        // Material and color text
        let text_y = swatch_y + swatch_size as i32 + spacing::MD;
        let text_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let secondary_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        Text::new(material, Point::new(spool_card_x + spacing::MD, text_y), text_style)
            .draw(display)?;

        Text::new(color_name, Point::new(spool_card_x + spacing::MD, text_y + 20), secondary_style)
            .draw(display)?;

        let weight_text: heapless::String<16> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{:.0}g", weight_current));
            s
        };
        Text::new(&weight_text, Point::new(spool_card_x + spacing::MD, text_y + 36), secondary_style)
            .draw(display)?;

        // Right side: Info cards
        let right_x = spool_card_x + spool_card_width as i32 + spacing::LG;
        let right_width = DISPLAY_WIDTH as i32 - right_x - spacing::LG;

        // Print Settings card
        let print_card_height = 130u32;
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(right_x, content_y + spacing::SM),
                Size::new(right_width as u32, print_card_height),
            ),
            Size::new(radius::MD, radius::MD),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Print Settings title
        Text::new(
            "Print Settings",
            Point::new(right_x + spacing::MD, content_y + spacing::SM + 20),
            MonoTextStyle::new(&FONT_6X10, theme.primary),
        )
        .draw(display)?;

        let row_width = (right_width - spacing::MD * 2) as u32;
        let mut row_y = content_y + spacing::SM + 28;

        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "Nozzle Temp", "220°C")
            .draw(display)?;
        row_y += 26;

        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "Bed Temp", "60°C")
            .draw(display)?;
        row_y += 26;

        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "K Factor", "0.024")
            .without_separator()
            .draw(display)?;

        // Spool Information card
        let info_card_y = content_y + spacing::SM + print_card_height as i32 + spacing::MD;
        let info_card_height = 130u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(right_x, info_card_y),
                Size::new(right_width as u32, info_card_height),
            ),
            Size::new(radius::MD, radius::MD),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Spool Information title
        Text::new(
            "Spool Information",
            Point::new(right_x + spacing::MD, info_card_y + 20),
            MonoTextStyle::new(&FONT_6X10, theme.primary),
        )
        .draw(display)?;

        let mut row_y = info_card_y + 28;

        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "Tag ID", "NTAG215-ABC")
            .draw(display)?;
        row_y += 26;

        let net_weight: heapless::String<16> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{:.0}g", weight_label));
            s
        };
        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "Net Weight", &net_weight)
            .draw(display)?;
        row_y += 26;

        InfoRow::new(Point::new(right_x + spacing::SM, row_y), row_width, "Source", source)
            .without_separator()
            .draw(display)?;

        // Bottom buttons
        let button_y = DISPLAY_HEIGHT as i32 - 60;
        let button_width = 180u32;
        let button_gap = spacing::MD;

        Button::new(
            Point::new(spacing::LG, button_y),
            Size::new(button_width, 44),
            "Assign Slot",
        )
        .with_style(ButtonStyle::Primary)
        .draw(display)?;

        Button::new(
            Point::new(spacing::LG + button_width as i32 + button_gap, button_y),
            Size::new(button_width, 44),
            "Edit Info",
        )
        .with_style(ButtonStyle::Secondary)
        .draw(display)?;

        Button::new(
            Point::new(spacing::LG + (button_width as i32 + button_gap) * 2, button_y),
            Size::new(button_width, 44),
            "Remove",
        )
        .with_style(ButtonStyle::Danger)
        .draw(display)?;

        Ok(())
    }
}

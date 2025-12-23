//! Scan result screen - displays scanned spool info and AMS slot assignment.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < Scan Result                                              │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  ┌──────────────────────────────┐  ┌─────────────────────┐│
//! │  │          ████████            │  │  Select AMS Slot    ││
//! │  │          ████████            │  │ ┌───┬───┬───┬───┐   ││
//! │  │          ████████            │  │ │A1 │A2 │A3 │A4 │   ││
//! │  │                              │  │ └───┴───┴───┴───┘   ││
//! │  │  PLA - Yellow                │  │ ┌───┬───┬───┬───┐   ││
//! │  │  Bambu Lab Basic PLA         │  │ │B1 │B2 │B3 │B4 │   ││
//! │  │                              │  │ └───┴───┴───┴───┘   ││
//! │  │  ████████████████░░░  85%    │  │ ┌───┬───┬───┬───┐   ││
//! │  │  850g / 1000g                │  │ │C1 │C2 │C3 │C4 │   ││
//! │  │                              │  │ └───┴───┴───┴───┘   ││
//! │  │  Nozzle: 220°C  Bed: 60°C    │  │ ┌───┬───┬───┬───┐   ││
//! │  │  K-Factor: 0.024             │  │ │D1 │D2 │D3 │D4 │   ││
//! │  └──────────────────────────────┘  │ └───┴───┴───┴───┘   ││
//! │                                    └─────────────────────┘│
//! │                                                            │
//! │                    [Assign & Save]                        │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::{Button, ProgressBar, StatusBar};
use crate::ui::widgets::button::ButtonStyle;
use crate::ui::{SpoolSource, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// Scan result screen renderer
pub struct ScanResultScreen;

impl ScanResultScreen {
    /// Render the scan result screen
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
        let mut status_bar = StatusBar::new("< Scan Result");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        let content_y = 56;

        // Get spool info or use defaults
        let (material, color_name, brand, color, weight_current, weight_label) =
            if let Some(ref spool) = state.spool {
                (
                    spool.material.as_str(),
                    spool.color_name.as_str(),
                    spool.brand.as_str(),
                    theme::rgba_to_rgb565(spool.color_rgba),
                    spool.weight_current,
                    spool.weight_label,
                )
            } else {
                ("PLA", "Yellow", "Bambu Lab Basic PLA", theme::rgba_to_rgb565(0xF5C518FF), 850.0, 1000.0)
            };

        // Left side: Spool info card
        let info_card_x = spacing::LG;
        let info_card_width = 350u32;
        let info_card_height = 300u32;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(info_card_x, content_y),
                Size::new(info_card_width, info_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Color swatch (large)
        let swatch_width = 200u32;
        let swatch_height = 100u32;
        let swatch_x = info_card_x + (info_card_width - swatch_width) as i32 / 2;
        let swatch_y = content_y + spacing::MD;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(swatch_x, swatch_y),
                Size::new(swatch_width, swatch_height),
            ),
            Size::new(radius::MD, radius::MD),
        )
        .into_styled(PrimitiveStyle::with_fill(color))
        .draw(display)?;

        // Material and brand
        let text_x = info_card_x + spacing::MD;
        let mut text_y = swatch_y + swatch_height as i32 + spacing::MD;

        let title_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let subtitle_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        let material_text: heapless::String<32> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{} - {}", material, color_name));
            s
        };
        Text::new(&material_text, Point::new(text_x, text_y), title_style)
            .draw(display)?;

        text_y += 22;
        Text::new(brand, Point::new(text_x, text_y), subtitle_style)
            .draw(display)?;

        // Weight progress
        text_y += 30;
        let progress_width = info_card_width - (spacing::MD * 2) as u32;
        let percentage = theme::weight_percentage(weight_current, weight_label);

        ProgressBar::new(
            Point::new(text_x, text_y),
            Size::new(progress_width - 50, 16),
        )
        .with_value(percentage)
        .draw(display)?;

        let percent_text: heapless::String<8> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{}%", percentage));
            s
        };
        Text::new(
            &percent_text,
            Point::new(text_x + progress_width as i32 - 40, text_y + 12),
            subtitle_style,
        )
        .draw(display)?;

        text_y += 24;
        let weight_text: heapless::String<24> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{:.0}g / {:.0}g", weight_current, weight_label));
            s
        };
        Text::new(&weight_text, Point::new(text_x, text_y), subtitle_style)
            .draw(display)?;

        // Print settings
        text_y += 30;
        Text::new("Nozzle: 220°C  Bed: 60°C", Point::new(text_x, text_y), subtitle_style)
            .draw(display)?;

        text_y += 16;
        Text::new("K-Factor: 0.024", Point::new(text_x, text_y), subtitle_style)
            .draw(display)?;

        // Right side: AMS slot selection
        let slot_card_x = info_card_x + info_card_width as i32 + spacing::LG;
        let slot_card_width = DISPLAY_WIDTH as i32 - slot_card_x - spacing::LG;

        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(slot_card_x, content_y),
                Size::new(slot_card_width as u32, info_card_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // Title
        Text::new(
            "Select AMS Slot",
            Point::new(slot_card_x + spacing::MD, content_y + 20),
            MonoTextStyle::new(&FONT_6X10, theme.primary),
        )
        .draw(display)?;

        // AMS slot grid (4 rows of 4 slots each)
        let slot_size = 44u32;
        let slot_gap = 8;
        let grid_x = slot_card_x + spacing::MD;
        let grid_y = content_y + 32;

        let labels = [
            ["A1", "A2", "A3", "A4"],
            ["B1", "B2", "B3", "B4"],
            ["C1", "C2", "C3", "C4"],
            ["D1", "D2", "D3", "D4"],
        ];

        // Sample occupied slots
        let occupied: [(usize, usize); 5] = [(0, 0), (0, 1), (1, 0), (2, 0), (3, 0)];
        let selected = state.selected_ams_slot.map(|(ams, slot)| (ams as usize, slot as usize));

        for (row, row_labels) in labels.iter().enumerate() {
            for (col, label) in row_labels.iter().enumerate() {
                let x = grid_x + (col as i32 * (slot_size as i32 + slot_gap));
                let y = grid_y + (row as i32 * (slot_size as i32 + slot_gap));

                let is_occupied = occupied.contains(&(row, col));
                let is_selected = selected == Some((row, col));

                // Slot background
                let bg_color = if is_selected {
                    theme.primary
                } else if is_occupied {
                    theme.progress_bg
                } else {
                    theme.bg
                };

                RoundedRectangle::with_equal_corners(
                    Rectangle::new(Point::new(x, y), Size::new(slot_size, slot_size)),
                    Size::new(radius::SM, radius::SM),
                )
                .into_styled(PrimitiveStyle::with_fill(bg_color))
                .draw(display)?;

                // Border
                if !is_selected {
                    RoundedRectangle::with_equal_corners(
                        Rectangle::new(Point::new(x, y), Size::new(slot_size, slot_size)),
                        Size::new(radius::SM, radius::SM),
                    )
                    .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
                    .draw(display)?;
                }

                // Label
                let text_color = if is_selected {
                    theme.bg
                } else if is_occupied {
                    theme.text_secondary
                } else {
                    theme.text_primary
                };

                Text::new(
                    label,
                    Point::new(x + 14, y + 28),
                    MonoTextStyle::new(&FONT_6X10, text_color),
                )
                .draw(display)?;
            }
        }

        // Assign & Save button
        let button_width = 200u32;
        let button_x = (DISPLAY_WIDTH - button_width) as i32 / 2;
        let button_y = DISPLAY_HEIGHT as i32 - 60;

        Button::new(
            Point::new(button_x, button_y),
            Size::new(button_width, 48),
            "Assign & Save",
        )
        .with_style(ButtonStyle::Primary)
        .with_large_font()
        .draw(display)?;

        Ok(())
    }
}

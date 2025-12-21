//! Spool info screen - shown when a spool is detected.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ SpoolBuddy                        [WiFi] [Server] [12:34] │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  ┌────┐  Bambu Lab PLA Basic                              │
//! │  │████│  Jade White                                       │
//! │  │████│  ──────────────────────────── 85%                 │
//! │  └────┘  850g / 1000g remaining                           │
//! │          K: 0.022 (calibrated)                            │
//! │                                                            │
//! │  ┌──────────────────────┐                                 │
//! │  │    1,098.5 g   ✓     │                                 │
//! │  └──────────────────────┘                                 │
//! │                                                            │
//! │  [ASSIGN TO AMS]  [UPDATE WEIGHT]  [WRITE TAG]  [DETAILS] │
//! │                                                            │
//! └────────────────────────────────────────────────────────────┘

use crate::theme::{self, spacing};
use crate::widgets::{Button, SpoolCard, StatusBar, WeightDisplay};
use crate::widgets::button::{ButtonBar, ButtonStyle};
use crate::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

/// Spool info screen renderer
pub struct SpoolInfoScreen;

impl SpoolInfoScreen {
    /// Action button labels
    const BUTTONS: [&'static str; 4] = ["ASSIGN AMS", "UPDATE WT", "WRITE TAG", "DETAILS"];

    /// Render the spool info screen
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
        let mut status_bar = StatusBar::new("SpoolBuddy");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        // Spool card
        let card_y = 60;
        let card_height = 120u32;
        let card = SpoolCard::new(
            Point::new(spacing::MD, card_y),
            Size::new(DISPLAY_WIDTH - (spacing::MD as u32 * 2), card_height),
        );

        if let Some(ref spool) = state.spool {
            card.draw(display, spool)?;
        } else {
            card.draw_empty(display, "No spool data")?;
        }

        // Weight display widget
        let weight_y = card_y + card_height as i32 + spacing::LG;
        let weight_width = 400u32;
        let weight_height = 70u32;
        let weight_x = (DISPLAY_WIDTH - weight_width) as i32 / 2;

        let mut weight_display = WeightDisplay::new(
            Point::new(weight_x, weight_y),
            Size::new(weight_width, weight_height),
        );
        weight_display.set_weight(state.weight, state.weight_stable);
        weight_display.draw(display)?;

        // Additional weight info
        if let Some(ref spool) = state.spool {
            let diff = state.weight - spool.weight_current;
            let diff_text = if diff.abs() > 1.0 {
                let mut s: heapless::String<32> = heapless::String::new();
                let sign = if diff > 0.0 { "+" } else { "" };
                let _ = core::fmt::write(&mut s, format_args!("Scale diff: {}{:.1}g", sign, diff));
                s
            } else {
                let mut s: heapless::String<32> = heapless::String::new();
                let _ = s.push_str("Weight matches");
                s
            };

            let info_style = MonoTextStyle::new(
                &embedded_graphics::mono_font::ascii::FONT_6X10,
                theme.text_secondary,
            );
            Text::with_alignment(
                &diff_text,
                Point::new(
                    DISPLAY_WIDTH as i32 / 2,
                    weight_y + weight_height as i32 + spacing::SM,
                ),
                info_style,
                Alignment::Center,
            )
            .draw(display)?;
        }

        // Bottom action buttons
        let button_y = DISPLAY_HEIGHT as i32 - 60;
        let button_height = 48u32;
        let button_bar = ButtonBar::new(button_y, button_height, &Self::BUTTONS);
        button_bar.draw(display, DISPLAY_WIDTH)?;

        Ok(())
    }

    /// Get which action button was pressed
    pub fn get_button_at(point: Point) -> Option<usize> {
        let button_y = DISPLAY_HEIGHT as i32 - 60;
        let button_bar = ButtonBar::new(button_y, 48, &Self::BUTTONS);
        button_bar.button_at(point, DISPLAY_WIDTH)
    }
}

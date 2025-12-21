//! Home screen - default view when no spool is detected.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ SpoolBuddy                        [WiFi] [Server] [12:34] │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │     ┌─────────────────────────────────────────────┐       │
//! │     │                                              │       │
//! │     │         PLACE SPOOL ON SCALE                │       │
//! │     │              [NFC icon]                      │       │
//! │     │                                              │       │
//! │     └─────────────────────────────────────────────┘       │
//! │                                                            │
//! │     ┌──────────────────────┐                              │
//! │     │    1,234.5 g   ✓     │  ← Weight display            │
//! │     └──────────────────────┘                              │
//! │                                                            │
//! │     [TARE]              [SETTINGS]                        │
//! │                                                            │
//! └────────────────────────────────────────────────────────────┘

use crate::theme::{self, spacing};
use crate::widgets::{Button, StatusBar, WeightDisplay};
use crate::widgets::button::ButtonStyle;
use crate::widgets::icon::Icon;
use crate::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// Home screen renderer
pub struct HomeScreen;

impl HomeScreen {
    /// Render the home screen
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

        // Main content area
        let content_y = 60;
        let content_height = DISPLAY_HEIGHT - 60 - 80; // Leave room for buttons

        // "Place spool" prompt card
        let card_width = 500u32;
        let card_height = 200u32;
        let card_x = (DISPLAY_WIDTH - card_width) as i32 / 2;
        let card_y = content_y + ((content_height - card_height - 80) as i32) / 2;

        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(card_x, card_y), Size::new(card_width, card_height)),
            Size::new(theme::radius::LG, theme::radius::LG),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // Prompt text
        let text_style = MonoTextStyle::new(&FONT_10X20, theme.text_secondary);
        Text::with_alignment(
            "PLACE SPOOL ON SCALE",
            Point::new(
                card_x + card_width as i32 / 2,
                card_y + card_height as i32 / 2 - 20,
            ),
            text_style,
            Alignment::Center,
        )
        .draw(display)?;

        // NFC icon
        Icon::Nfc.draw(
            display,
            Point::new(
                card_x + card_width as i32 / 2 - 24,
                card_y + card_height as i32 / 2 + 10,
            ),
            48,
            theme.primary,
        )?;

        // Weight display widget
        let weight_width = 300u32;
        let weight_height = 60u32;
        let weight_x = (DISPLAY_WIDTH - weight_width) as i32 / 2;
        let weight_y = card_y + card_height as i32 + spacing::LG;

        let mut weight_display = WeightDisplay::new(
            Point::new(weight_x, weight_y),
            Size::new(weight_width, weight_height),
        );
        weight_display.set_weight(state.weight, state.weight_stable);
        weight_display.draw(display)?;

        // Bottom buttons
        let button_y = DISPLAY_HEIGHT as i32 - 60;
        let button_height = 48u32;
        let button_width = 150u32;

        // Tare button (left)
        let tare_button = Button::new(
            Point::new(spacing::LG, button_y),
            Size::new(button_width, button_height),
            "TARE",
        )
        .with_style(ButtonStyle::Secondary)
        .with_large_font();
        tare_button.draw(display)?;

        // Settings button (right)
        let settings_button = Button::new(
            Point::new(
                DISPLAY_WIDTH as i32 - spacing::LG - button_width as i32,
                button_y,
            ),
            Size::new(button_width, button_height),
            "SETTINGS",
        )
        .with_style(ButtonStyle::Secondary)
        .with_large_font();
        settings_button.draw(display)?;

        Ok(())
    }

    /// Get button bounds for touch handling
    pub fn get_tare_button_bounds() -> Rectangle {
        Rectangle::new(
            Point::new(spacing::LG, DISPLAY_HEIGHT as i32 - 60),
            Size::new(150, 48),
        )
    }

    pub fn get_settings_button_bounds() -> Rectangle {
        Rectangle::new(
            Point::new(DISPLAY_WIDTH as i32 - spacing::LG - 150, DISPLAY_HEIGHT as i32 - 60),
            Size::new(150, 48),
        )
    }
}

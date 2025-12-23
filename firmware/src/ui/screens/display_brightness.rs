//! Display brightness screen - adjusts screen brightness and timeout settings.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < Display                                                  │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │     Brightness                                    80%      │
//! │     ┌──────────────────────────────────●───────────────┐  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! │     ┌──────────────────────────────────────────────────┐  │
//! │     │  Auto Brightness                         [OFF]   │  │
//! │     │  Screen Timeout                          [ON]    │  │
//! │     │  Timeout Duration                        60s     │  │
//! │     └──────────────────────────────────────────────────┘  │
//! │                                                            │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::{Slider, StatusBar, Toggle};
use crate::ui::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// Display brightness screen renderer
pub struct DisplayBrightnessScreen;

impl DisplayBrightnessScreen {
    /// Render the display brightness screen
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
        let mut status_bar = StatusBar::new("< Display");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        let content_x = spacing::LG;
        let content_width = DISPLAY_WIDTH - (spacing::LG * 2) as u32;
        let mut y = 70;

        // Brightness section
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        let value_style = MonoTextStyle::new(&FONT_10X20, theme.primary);

        // Label and value
        Text::new("Brightness", Point::new(content_x, y), label_style)
            .draw(display)?;

        let percent_text: heapless::String<8> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{}%", state.brightness));
            s
        };
        let value_x = DISPLAY_WIDTH as i32 - content_x - (percent_text.len() as i32 * 10);
        Text::new(&percent_text, Point::new(value_x, y + 5), value_style)
            .draw(display)?;

        y += 30;

        // Brightness slider
        let slider = Slider::new(Point::new(content_x, y), content_width)
            .with_value(state.brightness);
        slider.draw(display)?;

        y += 60;

        // Options card
        let card_height = 140u32;
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(content_x, y),
                Size::new(content_width, card_height),
            ),
            Size::new(radius::MD, radius::MD),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        let row_x = content_x + spacing::MD;
        let row_width = content_width - (spacing::MD * 2) as u32;
        let mut row_y = y + spacing::MD;

        // Auto Brightness row
        Text::new("Auto Brightness", Point::new(row_x, row_y + 16), label_style)
            .draw(display)?;

        let toggle_x = content_x + content_width as i32 - spacing::MD - 44;
        Toggle::new(Point::new(toggle_x, row_y + 4))
            .with_enabled(state.auto_brightness)
            .draw(display)?;

        // Separator
        row_y += 40;
        Rectangle::new(
            Point::new(row_x, row_y),
            Size::new(row_width, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        row_y += spacing::MD;

        // Screen Timeout row
        Text::new("Screen Timeout", Point::new(row_x, row_y + 16), label_style)
            .draw(display)?;

        Toggle::new(Point::new(toggle_x, row_y + 4))
            .with_enabled(state.screen_timeout)
            .draw(display)?;

        // Separator
        row_y += 40;
        Rectangle::new(
            Point::new(row_x, row_y),
            Size::new(row_width, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        row_y += spacing::MD;

        // Timeout Duration row
        Text::new("Timeout Duration", Point::new(row_x, row_y + 16), label_style)
            .draw(display)?;

        let duration_text: heapless::String<8> = {
            let mut s = heapless::String::new();
            let _ = core::fmt::write(&mut s, format_args!("{}s", state.timeout_seconds));
            s
        };
        let secondary_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let duration_x = toggle_x + 20 - (duration_text.len() as i32 * 6);
        Text::new(&duration_text, Point::new(duration_x, row_y + 16), secondary_style)
            .draw(display)?;

        Ok(())
    }
}

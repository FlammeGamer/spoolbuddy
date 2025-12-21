//! Weight display widget - large, prominent weight readout.

use crate::theme::{self, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// Weight display widget showing current weight with stability indicator
pub struct WeightDisplay {
    /// Position (top-left corner)
    pub position: Point,
    /// Size of the widget
    pub size: Size,
    /// Current weight in grams
    pub weight: f32,
    /// Whether the weight is stable
    pub stable: bool,
}

impl WeightDisplay {
    /// Create a new weight display widget
    pub fn new(position: Point, size: Size) -> Self {
        Self {
            position,
            size,
            weight: 0.0,
            stable: false,
        }
    }

    /// Set the weight value
    pub fn set_weight(&mut self, grams: f32, stable: bool) {
        self.weight = grams;
        self.stable = stable;
    }

    /// Draw the widget
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Background card
        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            Size::new(theme::radius::MD, theme::radius::MD),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // Weight text
        let weight_text = theme::format_weight(self.weight);
        let text_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);

        // Center the text in the widget
        let text_pos = Point::new(
            self.position.x + (self.size.width as i32) / 2,
            self.position.y + (self.size.height as i32) / 2 + 8,
        );

        Text::with_alignment(&weight_text, text_pos, text_style, Alignment::Center)
            .draw(display)?;

        // Stability indicator (checkmark or dot)
        if self.stable {
            // Draw green checkmark indicator
            let indicator_pos = Point::new(
                self.position.x + self.size.width as i32 - spacing::MD - 20,
                self.position.y + (self.size.height as i32) / 2 - 8,
            );

            // Simple checkmark using rectangles
            let check_color = theme.success;
            let check_style = PrimitiveStyle::with_fill(check_color);

            // Checkmark left part (/)
            Rectangle::new(indicator_pos, Size::new(4, 12))
                .into_styled(check_style)
                .draw(display)?;

            // Checkmark right part (\)
            Rectangle::new(
                Point::new(indicator_pos.x + 6, indicator_pos.y - 6),
                Size::new(4, 18),
            )
            .into_styled(check_style)
            .draw(display)?;
        } else {
            // Draw pulsing dot for unstable
            let indicator_pos = Point::new(
                self.position.x + self.size.width as i32 - spacing::MD - 12,
                self.position.y + (self.size.height as i32) / 2 - 6,
            );

            embedded_graphics::primitives::Circle::new(indicator_pos, 12)
                .into_styled(PrimitiveStyle::with_fill(theme.warning))
                .draw(display)?;
        }

        Ok(())
    }
}

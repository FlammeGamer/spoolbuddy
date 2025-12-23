//! Info row widget for displaying label-value pairs.

use crate::ui::theme::{self, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

/// Info row widget for label-value display
pub struct InfoRow<'a> {
    /// Position (top-left corner)
    position: Point,
    /// Width of the row
    width: u32,
    /// Row height
    height: u32,
    /// Label text
    label: &'a str,
    /// Value text
    value: &'a str,
    /// Show bottom separator
    show_separator: bool,
}

impl<'a> InfoRow<'a> {
    /// Default row height
    pub const HEIGHT: u32 = 32;

    /// Create a new info row
    pub fn new(position: Point, width: u32, label: &'a str, value: &'a str) -> Self {
        Self {
            position,
            width,
            height: Self::HEIGHT,
            label,
            value,
            show_separator: true,
        }
    }

    /// Hide separator
    pub fn without_separator(mut self) -> Self {
        self.show_separator = false;
        self
    }

    /// Draw the info row
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let value_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);

        let center_y = self.position.y + (self.height as i32 / 2) + 4;

        // Label (left)
        Text::new(
            self.label,
            Point::new(self.position.x + spacing::MD, center_y),
            label_style,
        )
        .draw(display)?;

        // Value (right-aligned)
        let value_x = self.position.x + self.width as i32 - spacing::MD - (self.value.len() as i32 * 6);
        Text::new(self.value, Point::new(value_x, center_y), value_style)
            .draw(display)?;

        // Bottom separator
        if self.show_separator {
            Rectangle::new(
                Point::new(
                    self.position.x + spacing::MD,
                    self.position.y + self.height as i32 - 1,
                ),
                Size::new(self.width - (spacing::MD * 2) as u32, 1),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.border))
            .draw(display)?;
        }

        Ok(())
    }
}

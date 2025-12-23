//! Slider widget for value selection (e.g., brightness).

use crate::ui::theme::{self, radius};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
};

/// Slider widget
pub struct Slider {
    /// Position (top-left corner)
    position: Point,
    /// Width of the slider track
    width: u32,
    /// Current value (0-100)
    value: u8,
    /// Track height
    track_height: u32,
    /// Knob size
    knob_size: u32,
}

impl Slider {
    /// Create a new slider
    pub fn new(position: Point, width: u32) -> Self {
        Self {
            position,
            width,
            value: 50,
            track_height: 8,
            knob_size: 24,
        }
    }

    /// Set current value
    pub fn with_value(mut self, value: u8) -> Self {
        self.value = value.min(100);
        self
    }

    /// Get the full bounds (including knob overhang)
    pub fn bounds(&self) -> Rectangle {
        let height = self.knob_size.max(self.track_height);
        Rectangle::new(
            Point::new(self.position.x, self.position.y - (height as i32 - self.track_height as i32) / 2),
            Size::new(self.width, height),
        )
    }

    /// Check if point is within slider track area
    pub fn contains(&self, point: Point) -> bool {
        // Expand hit area vertically for easier touch
        let hit_height = self.knob_size + 16;
        let hit_y = self.position.y - (hit_height as i32 - self.track_height as i32) / 2;

        point.x >= self.position.x
            && point.x < self.position.x + self.width as i32
            && point.y >= hit_y
            && point.y < hit_y + hit_height as i32
    }

    /// Calculate value from touch x position
    pub fn value_from_x(&self, x: i32) -> u8 {
        let effective_width = self.width as i32 - self.knob_size as i32;
        let rel_x = (x - self.position.x - self.knob_size as i32 / 2).clamp(0, effective_width);
        ((rel_x as u32 * 100) / effective_width as u32).min(100) as u8
    }

    /// Draw the slider
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        let track_y = self.position.y;

        // Background track
        let track_bg = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(self.position.x, track_y),
                Size::new(self.width, self.track_height),
            ),
            Size::new(self.track_height / 2, self.track_height / 2),
        );
        track_bg
            .into_styled(PrimitiveStyle::with_fill(theme.progress_bg))
            .draw(display)?;

        // Filled portion
        let filled_width = ((self.width - self.knob_size) as u32 * self.value as u32) / 100 + self.knob_size / 2;
        if filled_width > 0 {
            let track_fill = RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(self.position.x, track_y),
                    Size::new(filled_width, self.track_height),
                ),
                Size::new(self.track_height / 2, self.track_height / 2),
            );
            track_fill
                .into_styled(PrimitiveStyle::with_fill(theme.primary))
                .draw(display)?;
        }

        // Knob position
        let effective_width = self.width - self.knob_size;
        let knob_x = self.position.x + ((effective_width as u32 * self.value as u32) / 100) as i32;
        let knob_y = track_y - (self.knob_size as i32 - self.track_height as i32) / 2;

        // Knob shadow (subtle)
        Circle::new(
            Point::new(knob_x + 1, knob_y + 1),
            self.knob_size,
        )
        .into_styled(PrimitiveStyle::with_fill(theme::darken(theme.card_bg, 30)))
        .draw(display)?;

        // Knob
        Circle::new(Point::new(knob_x, knob_y), self.knob_size)
            .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
            .draw(display)?;

        // Knob border
        Circle::new(Point::new(knob_x, knob_y), self.knob_size)
            .into_styled(PrimitiveStyle::with_stroke(theme.border, 1))
            .draw(display)?;

        Ok(())
    }
}

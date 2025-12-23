//! Toggle switch widget for boolean settings.

use crate::ui::theme::{self};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
};

/// Toggle switch widget
pub struct Toggle {
    /// Position (top-left corner)
    position: Point,
    /// Whether the toggle is enabled
    enabled: bool,
    /// Width of the toggle track
    width: u32,
    /// Height of the toggle track
    height: u32,
}

impl Toggle {
    /// Standard toggle dimensions
    const DEFAULT_WIDTH: u32 = 44;
    const DEFAULT_HEIGHT: u32 = 24;

    /// Create a new toggle switch
    pub fn new(position: Point) -> Self {
        Self {
            position,
            enabled: false,
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
        }
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get bounds for hit testing
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(self.position, Size::new(self.width, self.height))
    }

    /// Check if point is within toggle
    pub fn contains(&self, point: Point) -> bool {
        self.bounds().contains(point)
    }

    /// Draw the toggle
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Track colors
        let track_color = if self.enabled {
            theme.primary
        } else {
            theme.progress_bg
        };

        // Draw track (rounded rectangle)
        let track = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, Size::new(self.width, self.height)),
            Size::new(self.height / 2, self.height / 2),
        );
        track
            .into_styled(PrimitiveStyle::with_fill(track_color))
            .draw(display)?;

        // Draw knob
        let knob_diameter = self.height - 4;
        let knob_y = self.position.y + 2;
        let knob_x = if self.enabled {
            self.position.x + (self.width - knob_diameter - 2) as i32
        } else {
            self.position.x + 2
        };

        // Knob circle
        Circle::new(
            Point::new(knob_x, knob_y),
            knob_diameter,
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

        Ok(())
    }
}

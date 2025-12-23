//! Filter pill widget for catalog filtering.

use crate::ui::theme::{self, radius, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// Filter pill widget
pub struct FilterPill<'a> {
    /// Position (top-left corner)
    position: Point,
    /// Label text
    label: &'a str,
    /// Whether the pill is active/selected
    active: bool,
    /// Minimum width
    min_width: u32,
    /// Height
    height: u32,
}

impl<'a> FilterPill<'a> {
    /// Default height
    pub const HEIGHT: u32 = 28;

    /// Create a new filter pill
    pub fn new(position: Point, label: &'a str) -> Self {
        Self {
            position,
            label,
            active: false,
            min_width: 60,
            height: Self::HEIGHT,
        }
    }

    /// Set active state
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Calculate width based on label
    pub fn width(&self) -> u32 {
        let text_width = (self.label.len() as u32 * 6) + (spacing::MD as u32 * 2);
        text_width.max(self.min_width)
    }

    /// Get bounds for hit testing
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(self.position, Size::new(self.width(), self.height))
    }

    /// Check if point is within pill
    pub fn contains(&self, point: Point) -> bool {
        self.bounds().contains(point)
    }

    /// Draw the filter pill
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let width = self.width();

        let (bg_color, text_color, border_color) = if self.active {
            (theme.primary, theme.bg, theme.primary)
        } else {
            (theme.card_bg, theme.text_secondary, theme.border)
        };

        // Background
        let pill = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, Size::new(width, self.height)),
            Size::new(radius::PILL, radius::PILL),
        );
        pill.into_styled(PrimitiveStyle::with_fill(bg_color))
            .draw(display)?;

        // Border (only when not active)
        if !self.active {
            pill.into_styled(PrimitiveStyle::with_stroke(border_color, 1))
                .draw(display)?;
        }

        // Label
        let text_style = MonoTextStyle::new(&FONT_6X10, text_color);
        let text_x = self.position.x + (width as i32 / 2);
        let text_y = self.position.y + (self.height as i32 / 2) + 4;

        Text::with_alignment(self.label, Point::new(text_x, text_y), text_style, Alignment::Center)
            .draw(display)?;

        Ok(())
    }
}

/// A row of filter pills
pub struct FilterPillRow<'a> {
    /// Position (top-left corner)
    position: Point,
    /// Labels for pills
    labels: &'a [&'a str],
    /// Active pill index
    active: usize,
    /// Gap between pills
    gap: i32,
}

impl<'a> FilterPillRow<'a> {
    /// Create a new filter pill row
    pub fn new(position: Point, labels: &'a [&'a str]) -> Self {
        Self {
            position,
            labels,
            active: 0,
            gap: spacing::SM,
        }
    }

    /// Set active pill
    pub fn with_active(mut self, index: usize) -> Self {
        self.active = index.min(self.labels.len().saturating_sub(1));
        self
    }

    /// Get which pill was pressed, if any
    pub fn pill_at(&self, point: Point) -> Option<usize> {
        let mut x = self.position.x;

        for (i, label) in self.labels.iter().enumerate() {
            let pill = FilterPill::new(Point::new(x, self.position.y), label);
            if pill.contains(point) {
                return Some(i);
            }
            x += pill.width() as i32 + self.gap;
        }
        None
    }

    /// Draw the pill row
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let mut x = self.position.x;

        for (i, label) in self.labels.iter().enumerate() {
            let pill = FilterPill::new(Point::new(x, self.position.y), label)
                .with_active(i == self.active);
            pill.draw(display)?;
            x += pill.width() as i32 + self.gap;
        }

        Ok(())
    }
}

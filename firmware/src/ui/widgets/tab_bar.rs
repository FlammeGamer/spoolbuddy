//! Tab bar widget for navigation between sections.

use crate::ui::theme::{self, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// Tab bar widget for section navigation
pub struct TabBar<'a> {
    /// Tab labels
    tabs: &'a [&'a str],
    /// Currently active tab index
    active: usize,
    /// Position (top-left corner)
    position: Point,
    /// Total width
    width: u32,
    /// Height
    height: u32,
}

impl<'a> TabBar<'a> {
    /// Create a new tab bar
    pub fn new(tabs: &'a [&'a str], position: Point, width: u32) -> Self {
        Self {
            tabs,
            active: 0,
            position,
            width,
            height: 36,
        }
    }

    /// Set active tab
    pub fn with_active(mut self, index: usize) -> Self {
        self.active = index.min(self.tabs.len().saturating_sub(1));
        self
    }

    /// Get tab bounds for hit testing
    pub fn tab_bounds(&self, index: usize) -> Option<Rectangle> {
        if index >= self.tabs.len() {
            return None;
        }

        let tab_width = self.width / self.tabs.len() as u32;
        let x = self.position.x + (tab_width as i32 * index as i32);

        Some(Rectangle::new(
            Point::new(x, self.position.y),
            Size::new(tab_width, self.height),
        ))
    }

    /// Get which tab was pressed, if any
    pub fn tab_at(&self, point: Point) -> Option<usize> {
        for i in 0..self.tabs.len() {
            if let Some(bounds) = self.tab_bounds(i) {
                if bounds.contains(point) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Draw the tab bar
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let tab_width = self.width / self.tabs.len() as u32;

        // Background
        Rectangle::new(self.position, Size::new(self.width, self.height))
            .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        for (i, label) in self.tabs.iter().enumerate() {
            let x = self.position.x + (tab_width as i32 * i as i32);
            let is_active = i == self.active;

            // Active tab indicator (bottom border)
            if is_active {
                let indicator = Rectangle::new(
                    Point::new(x + spacing::XS, self.position.y + self.height as i32 - 3),
                    Size::new(tab_width - (spacing::XS * 2) as u32, 3),
                );
                RoundedRectangle::with_equal_corners(indicator, Size::new(2, 2))
                    .into_styled(PrimitiveStyle::with_fill(theme.primary))
                    .draw(display)?;
            }

            // Tab text
            let text_color = if is_active {
                theme.primary
            } else {
                theme.text_secondary
            };

            let text_style = MonoTextStyle::new(&FONT_6X10, text_color);
            let text_x = x + (tab_width as i32 / 2);
            let text_y = self.position.y + (self.height as i32 / 2) + 4;

            Text::with_alignment(*label, Point::new(text_x, text_y), text_style, Alignment::Center)
                .draw(display)?;
        }

        // Bottom separator line
        Rectangle::new(
            Point::new(self.position.x, self.position.y + self.height as i32 - 1),
            Size::new(self.width, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        Ok(())
    }
}

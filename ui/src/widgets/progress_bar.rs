//! Progress bar widget for showing percentages.

use crate::theme::{self, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};
use heapless::String;

/// Progress bar widget
pub struct ProgressBar {
    /// Position (top-left corner)
    pub position: Point,
    /// Size of the widget
    pub size: Size,
    /// Current value (0-100)
    pub value: u8,
    /// Whether to show percentage label
    pub show_label: bool,
    /// Custom fill color (if None, uses theme primary)
    pub fill_color: Option<Rgb565>,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(position: Point, size: Size) -> Self {
        Self {
            position,
            size,
            value: 0,
            show_label: true,
            fill_color: None,
        }
    }

    /// Set the progress value (0-100)
    pub fn set_value(&mut self, value: u8) {
        self.value = value.min(100);
    }

    /// Set whether to show the percentage label
    pub fn set_show_label(&mut self, show: bool) {
        self.show_label = show;
    }

    /// Set a custom fill color
    pub fn set_fill_color(&mut self, color: Rgb565) {
        self.fill_color = Some(color);
    }

    /// Get color based on value (red for low, yellow for medium, green for high)
    fn value_color(&self) -> Rgb565 {
        let theme = theme::theme();
        match self.value {
            0..=20 => theme.error,
            21..=40 => theme.warning,
            _ => theme.primary,
        }
    }

    /// Draw the widget
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let corner_radius = Size::new(self.size.height / 2, self.size.height / 2);

        // Background track
        let track = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            corner_radius,
        );
        track
            .into_styled(PrimitiveStyle::with_fill(theme.progress_bg))
            .draw(display)?;

        // Fill bar
        if self.value > 0 {
            let fill_width = ((self.size.width as u32) * (self.value as u32) / 100) as u32;
            let fill_width = fill_width.max(self.size.height); // Minimum width for rounded corners

            let fill_color = self.fill_color.unwrap_or_else(|| self.value_color());

            let fill = RoundedRectangle::with_equal_corners(
                Rectangle::new(self.position, Size::new(fill_width, self.size.height)),
                corner_radius,
            );
            fill.into_styled(PrimitiveStyle::with_fill(fill_color))
                .draw(display)?;
        }

        // Percentage label
        if self.show_label {
            let mut label: String<8> = String::new();
            let _ = core::fmt::write(&mut label, format_args!("{}%", self.value));

            let text_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
            let text_pos = Point::new(
                self.position.x + (self.size.width as i32) / 2,
                self.position.y + (self.size.height as i32) / 2 + 3,
            );

            Text::with_alignment(&label, text_pos, text_style, Alignment::Center)
                .draw(display)?;
        }

        Ok(())
    }
}

/// Vertical progress bar (for AMS slot indicators)
pub struct VerticalProgressBar {
    /// Position (top-left corner)
    pub position: Point,
    /// Size of the widget
    pub size: Size,
    /// Current value (0-100)
    pub value: u8,
    /// Fill color
    pub fill_color: Rgb565,
}

impl VerticalProgressBar {
    /// Create a new vertical progress bar
    pub fn new(position: Point, size: Size, fill_color: Rgb565) -> Self {
        Self {
            position,
            size,
            value: 0,
            fill_color,
        }
    }

    /// Set the progress value (0-100)
    pub fn set_value(&mut self, value: u8) {
        self.value = value.min(100);
    }

    /// Draw the widget
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let corner_radius = Size::new(self.size.width / 2, self.size.width / 2);

        // Background track
        let track = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            corner_radius,
        );
        track
            .into_styled(PrimitiveStyle::with_fill(theme.progress_bg))
            .draw(display)?;

        // Fill bar (from bottom)
        if self.value > 0 {
            let fill_height = ((self.size.height as u32) * (self.value as u32) / 100) as u32;
            let fill_height = fill_height.max(self.size.width); // Minimum height for rounded corners

            let fill_y = self.position.y + (self.size.height - fill_height) as i32;
            let fill = RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(self.position.x, fill_y),
                    Size::new(self.size.width, fill_height),
                ),
                corner_radius,
            );
            fill.into_styled(PrimitiveStyle::with_fill(self.fill_color))
                .draw(display)?;
        }

        Ok(())
    }
}

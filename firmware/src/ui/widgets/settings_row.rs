//! Settings row widget for settings screens.

use crate::ui::theme::{self, spacing};
use crate::ui::widgets::icon::Icon;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::Text,
};

/// Status indicator dot color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusDot {
    /// Green dot (connected/active)
    Green,
    /// Gray dot (disconnected/inactive)
    Gray,
    /// No status indicator
    None,
}

/// Settings row widget
pub struct SettingsRow<'a> {
    /// Position (top-left corner)
    position: Point,
    /// Width of the row
    width: u32,
    /// Row height
    height: u32,
    /// Icon to display (optional)
    icon: Option<Icon>,
    /// Title text
    title: &'a str,
    /// Value/subtitle text (optional)
    value: Option<&'a str>,
    /// Status indicator
    status: StatusDot,
    /// Show arrow indicator
    show_arrow: bool,
    /// Show bottom separator
    show_separator: bool,
}

impl<'a> SettingsRow<'a> {
    /// Default row height
    pub const HEIGHT: u32 = 48;

    /// Create a new settings row
    pub fn new(position: Point, width: u32, title: &'a str) -> Self {
        Self {
            position,
            width,
            height: Self::HEIGHT,
            icon: None,
            title,
            value: None,
            status: StatusDot::None,
            show_arrow: true,
            show_separator: true,
        }
    }

    /// Set icon
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set value text
    pub fn with_value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }

    /// Set status indicator
    pub fn with_status(mut self, status: StatusDot) -> Self {
        self.status = status;
        self
    }

    /// Hide arrow
    pub fn without_arrow(mut self) -> Self {
        self.show_arrow = false;
        self
    }

    /// Hide separator
    pub fn without_separator(mut self) -> Self {
        self.show_separator = false;
        self
    }

    /// Get bounds for hit testing
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(self.position, Size::new(self.width, self.height))
    }

    /// Check if point is within row
    pub fn contains(&self, point: Point) -> bool {
        self.bounds().contains(point)
    }

    /// Draw the settings row
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let text_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        let value_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        let mut x = self.position.x + spacing::MD;
        let center_y = self.position.y + (self.height as i32 / 2);

        // Icon
        if let Some(icon) = self.icon {
            icon.draw(display, Point::new(x, center_y - 10), 20, theme.text_secondary)?;
            x += 28;
        }

        // Status dot
        if self.status != StatusDot::None {
            let dot_color = match self.status {
                StatusDot::Green => theme.success,
                StatusDot::Gray => theme.disabled,
                StatusDot::None => theme.disabled, // Won't reach here
            };
            Circle::new(Point::new(x, center_y - 4), 8)
                .into_styled(PrimitiveStyle::with_fill(dot_color))
                .draw(display)?;
            x += 16;
        }

        // Title
        Text::new(self.title, Point::new(x, center_y + 4), text_style)
            .draw(display)?;

        // Value and arrow on right side
        let right_x = self.position.x + self.width as i32 - spacing::MD;

        if self.show_arrow {
            // Draw arrow (simple >)
            let arrow_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
            Text::new(">", Point::new(right_x - 6, center_y + 4), arrow_style)
                .draw(display)?;
        }

        if let Some(value) = self.value {
            let value_x = if self.show_arrow {
                right_x - 20 - (value.len() as i32 * 6)
            } else {
                right_x - (value.len() as i32 * 6)
            };
            Text::new(value, Point::new(value_x, center_y + 4), value_style)
                .draw(display)?;
        }

        // Bottom separator
        if self.show_separator {
            Rectangle::new(
                Point::new(self.position.x + spacing::MD, self.position.y + self.height as i32 - 1),
                Size::new(self.width - (spacing::MD * 2) as u32, 1),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.border))
            .draw(display)?;
        }

        Ok(())
    }
}

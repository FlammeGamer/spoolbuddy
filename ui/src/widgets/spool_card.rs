//! Spool card widget for displaying spool information.

use crate::theme::{self, spacing};
use crate::{SpoolDisplay, SpoolSource};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};
use super::progress_bar::ProgressBar;

/// Spool card widget showing complete spool information
pub struct SpoolCard {
    /// Position (top-left corner)
    pub position: Point,
    /// Size of the widget
    pub size: Size,
}

impl SpoolCard {
    /// Create a new spool card
    pub fn new(position: Point, size: Size) -> Self {
        Self { position, size }
    }

    /// Draw the spool card with spool data
    pub fn draw<D>(&self, display: &mut D, spool: &SpoolDisplay) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let p = self.position;

        // Card background
        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            Size::new(theme::radius::MD, theme::radius::MD),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // Color swatch (left side)
        let swatch_size = 60u32;
        let swatch_pos = Point::new(p.x + spacing::MD, p.y + spacing::MD);
        let swatch_color = theme::rgba_to_rgb565(spool.color_rgba);

        let swatch = RoundedRectangle::with_equal_corners(
            Rectangle::new(swatch_pos, Size::new(swatch_size, swatch_size)),
            Size::new(theme::radius::SM, theme::radius::SM),
        );
        swatch
            .into_styled(PrimitiveStyle::with_fill(swatch_color))
            .draw(display)?;

        // Brand and material (top right of swatch)
        let text_x = swatch_pos.x + swatch_size as i32 + spacing::MD;
        let text_y = swatch_pos.y + 14;

        let title_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let subtitle_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        // Brand + Material
        let mut title: heapless::String<64> = heapless::String::new();
        let _ = core::fmt::write(&mut title, format_args!("{} {}", spool.brand, spool.material));
        Text::new(&title, Point::new(text_x, text_y), title_style).draw(display)?;

        // Color name
        Text::new(&spool.color_name, Point::new(text_x, text_y + 20), subtitle_style).draw(display)?;

        // Weight progress bar
        let progress_y = text_y + 36;
        let progress_width = self.size.width - swatch_size - spacing::MD as u32 * 3 - 60;
        let percentage = theme::weight_percentage(spool.weight_current, spool.weight_label);

        let mut progress = ProgressBar::new(
            Point::new(text_x, progress_y),
            Size::new(progress_width, 16),
        );
        progress.set_value(percentage);
        progress.draw(display)?;

        // Weight text
        let mut weight_text: heapless::String<32> = heapless::String::new();
        let _ = core::fmt::write(
            &mut weight_text,
            format_args!("{:.0}g / {:.0}g", spool.weight_current, spool.weight_label),
        );
        Text::new(
            &weight_text,
            Point::new(text_x, progress_y + 32),
            subtitle_style,
        )
        .draw(display)?;

        // K-value (if calibrated)
        if let Some(k) = spool.k_value {
            let mut k_text: heapless::String<32> = heapless::String::new();
            let _ = core::fmt::write(&mut k_text, format_args!("K: {:.3}", k));
            Text::new(
                &k_text,
                Point::new(text_x + 120, progress_y + 32),
                subtitle_style,
            )
            .draw(display)?;
        }

        // Source badge
        let badge_text = match spool.source {
            SpoolSource::Bambu => "BAMBU",
            SpoolSource::Manual => "MANUAL",
            SpoolSource::Nfc => "NFC",
        };
        let badge_color = match spool.source {
            SpoolSource::Bambu => theme.primary,
            SpoolSource::Manual => theme.warning,
            SpoolSource::Nfc => theme.success,
        };

        let badge_x = p.x + self.size.width as i32 - spacing::MD - 50;
        let badge_y = p.y + spacing::SM;

        // Badge background
        let badge = RoundedRectangle::with_equal_corners(
            Rectangle::new(Point::new(badge_x, badge_y), Size::new(48, 18)),
            Size::new(theme::radius::SM, theme::radius::SM),
        );
        badge
            .into_styled(PrimitiveStyle::with_fill(badge_color))
            .draw(display)?;

        // Badge text
        let badge_text_style = MonoTextStyle::new(&FONT_6X10, theme.bg);
        Text::new(
            badge_text,
            Point::new(badge_x + 4, badge_y + 12),
            badge_text_style,
        )
        .draw(display)?;

        Ok(())
    }

    /// Draw an empty/placeholder spool card
    pub fn draw_empty<D>(&self, display: &mut D, message: &str) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Card background
        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            Size::new(theme::radius::MD, theme::radius::MD),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // Centered message
        let text_style = MonoTextStyle::new(&FONT_10X20, theme.text_secondary);
        let text_pos = Point::new(
            self.position.x + (self.size.width as i32) / 2 - (message.len() as i32 * 5),
            self.position.y + (self.size.height as i32) / 2 + 8,
        );
        Text::new(message, text_pos, text_style).draw(display)?;

        Ok(())
    }
}

/// Compact spool card for lists
pub struct SpoolCardCompact {
    /// Position (top-left corner)
    pub position: Point,
    /// Width of the card
    pub width: u32,
}

impl SpoolCardCompact {
    /// Height of compact spool card
    pub const HEIGHT: u32 = 48;

    /// Create a new compact spool card
    pub fn new(position: Point, width: u32) -> Self {
        Self { position, width }
    }

    /// Draw the compact card
    pub fn draw<D>(&self, display: &mut D, spool: &SpoolDisplay, selected: bool) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let p = self.position;

        // Card background (with selection highlight)
        let bg_color = if selected {
            theme::lighten(theme.card_bg, 20)
        } else {
            theme.card_bg
        };

        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, Size::new(self.width, Self::HEIGHT)),
            Size::new(theme::radius::SM, theme::radius::SM),
        );
        card.into_styled(PrimitiveStyle::with_fill(bg_color))
            .draw(display)?;

        // Color swatch (small)
        let swatch_size = 32u32;
        let swatch_pos = Point::new(p.x + spacing::SM, p.y + (Self::HEIGHT as i32 - swatch_size as i32) / 2);
        let swatch_color = theme::rgba_to_rgb565(spool.color_rgba);

        Rectangle::new(swatch_pos, Size::new(swatch_size, swatch_size))
            .into_styled(PrimitiveStyle::with_fill(swatch_color))
            .draw(display)?;

        // Material and color name
        let text_x = swatch_pos.x + swatch_size as i32 + spacing::SM;
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        let sublabel_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        let mut label: heapless::String<32> = heapless::String::new();
        let _ = core::fmt::write(&mut label, format_args!("{} {}", spool.material, spool.color_name));
        Text::new(&label, Point::new(text_x, p.y + 18), label_style).draw(display)?;

        Text::new(&spool.brand, Point::new(text_x, p.y + 32), sublabel_style).draw(display)?;

        // Weight on right side
        let weight_text = theme::format_weight(spool.weight_current);
        let weight_x = p.x + self.width as i32 - spacing::SM - (weight_text.len() as i32 * 6);
        Text::new(&weight_text, Point::new(weight_x, p.y + 24), label_style).draw(display)?;

        Ok(())
    }
}

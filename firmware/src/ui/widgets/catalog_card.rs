//! Catalog card widget for filament grid display.

use crate::ui::theme::{self, radius, spacing};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// Catalog card widget
pub struct CatalogCard<'a> {
    /// Position (top-left corner)
    position: Point,
    /// Card size
    size: Size,
    /// Material type (e.g., "PLA", "PETG")
    material: &'a str,
    /// Color name
    color_name: &'a str,
    /// Spool color (RGB565)
    color: Rgb565,
    /// Weight text (e.g., "850g")
    weight: &'a str,
    /// AMS slot label (e.g., "A1") - optional
    slot: Option<&'a str>,
    /// Whether card is selected
    selected: bool,
}

impl<'a> CatalogCard<'a> {
    /// Standard card dimensions
    pub const WIDTH: u32 = 180;
    pub const HEIGHT: u32 = 100;

    /// Create a new catalog card
    pub fn new(position: Point, material: &'a str, color_name: &'a str, color: Rgb565) -> Self {
        Self {
            position,
            size: Size::new(Self::WIDTH, Self::HEIGHT),
            material,
            color_name,
            color,
            weight: "",
            slot: None,
            selected: false,
        }
    }

    /// Set weight text
    pub fn with_weight(mut self, weight: &'a str) -> Self {
        self.weight = weight;
        self
    }

    /// Set AMS slot
    pub fn with_slot(mut self, slot: &'a str) -> Self {
        self.slot = Some(slot);
        self
    }

    /// Set selected state
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Get bounds for hit testing
    pub fn bounds(&self) -> Rectangle {
        Rectangle::new(self.position, self.size)
    }

    /// Check if point is within card
    pub fn contains(&self, point: Point) -> bool {
        self.bounds().contains(point)
    }

    /// Draw the catalog card
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Card background
        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(self.position, self.size),
            Size::new(radius::MD, radius::MD),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // Selection border
        if self.selected {
            card.into_styled(PrimitiveStyle::with_stroke(theme.primary, 2))
                .draw(display)?;
        }

        // Color swatch (top portion)
        let swatch_height = 40u32;
        let swatch = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(self.position.x + 8, self.position.y + 8),
                Size::new(self.size.width - 16, swatch_height),
            ),
            Size::new(radius::SM, radius::SM),
        );
        swatch
            .into_styled(PrimitiveStyle::with_fill(self.color))
            .draw(display)?;

        // Slot badge (if assigned to AMS)
        if let Some(slot) = self.slot {
            let badge_x = self.position.x + self.size.width as i32 - 28;
            let badge_y = self.position.y + 12;

            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(badge_x, badge_y), Size::new(20, 16)),
                Size::new(radius::SM, radius::SM),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.primary))
            .draw(display)?;

            let badge_style = MonoTextStyle::new(&FONT_6X10, theme.bg);
            Text::new(
                slot,
                Point::new(badge_x + 4, badge_y + 12),
                badge_style,
            )
            .draw(display)?;
        }

        // Material and color name
        let text_y = self.position.y + swatch_height as i32 + 20;
        let text_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        let secondary_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);

        // Material type
        Text::new(
            self.material,
            Point::new(self.position.x + spacing::SM, text_y),
            text_style,
        )
        .draw(display)?;

        // Color name (below material)
        Text::new(
            self.color_name,
            Point::new(self.position.x + spacing::SM, text_y + 14),
            secondary_style,
        )
        .draw(display)?;

        // Weight (bottom right)
        if !self.weight.is_empty() {
            let weight_x = self.position.x + self.size.width as i32 - spacing::SM - (self.weight.len() as i32 * 6);
            Text::new(
                self.weight,
                Point::new(weight_x, text_y + 14),
                secondary_style,
            )
            .draw(display)?;
        }

        Ok(())
    }
}

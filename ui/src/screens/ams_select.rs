//! AMS slot selection screen.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ ← Select AMS Slot                                         │
//! ├────────────────────────────────────────────────────────────┤
//! │                                                            │
//! │  X1 Carbon (00M09A...)                                    │
//! │  ┌────────────────────────────────────────────────────┐   │
//! │  │  AMS A                                              │   │
//! │  │  ┌────┐ ┌────┐ ┌────┐ ┌────┐                       │   │
//! │  │  │ A1 │ │ A2 │ │ A3 │ │ A4 │                       │   │
//! │  │  │PLA │ │PETG│ │    │ │PLA │                       │   │
//! │  │  └────┘ └────┘ └────┘ └────┘                       │   │
//! │  └────────────────────────────────────────────────────┘   │
//! │                                                            │
//! │  ┌────────────────────────────────────────────────────┐   │
//! │  │  External Spool                                     │   │
//! │  │  ┌────┐                                            │   │
//! │  │  │EXT │                                            │   │
//! │  │  └────┘                                            │   │
//! │  └────────────────────────────────────────────────────┘   │
//! │                                                            │
//! │                                          [CANCEL]         │
//! └────────────────────────────────────────────────────────────┘

use crate::theme::{self, spacing};
use crate::widgets::{Button};
use crate::widgets::button::ButtonStyle;
use crate::widgets::icon::Icon;
use crate::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// AMS slot representation
#[derive(Debug, Clone)]
pub struct AmsSlot {
    pub id: u8,
    pub label: heapless::String<8>,
    pub material: Option<heapless::String<8>>,
    pub color: Rgb565,
    pub remaining_percent: u8,
    pub occupied: bool,
}

impl Default for AmsSlot {
    fn default() -> Self {
        Self {
            id: 0,
            label: heapless::String::new(),
            material: None,
            color: Rgb565::new(0x10, 0x10, 0x10),
            remaining_percent: 0,
            occupied: false,
        }
    }
}

/// AMS unit with 4 slots
#[derive(Debug, Clone)]
pub struct AmsUnit {
    pub id: u8,
    pub name: heapless::String<16>,
    pub slots: [AmsSlot; 4],
}

impl Default for AmsUnit {
    fn default() -> Self {
        let mut unit = Self {
            id: 0,
            name: heapless::String::new(),
            slots: [
                AmsSlot::default(),
                AmsSlot::default(),
                AmsSlot::default(),
                AmsSlot::default(),
            ],
        };
        let _ = unit.name.push_str("AMS A");

        for (i, slot) in unit.slots.iter_mut().enumerate() {
            slot.id = i as u8;
            let _ = core::fmt::write(&mut slot.label, format_args!("A{}", i + 1));
        }

        unit
    }
}

/// AMS select screen renderer
pub struct AmsSelectScreen;

impl AmsSelectScreen {
    /// Slot size
    const SLOT_SIZE: u32 = 70;
    /// Slot spacing
    const SLOT_SPACING: i32 = 16;

    /// Render the AMS selection screen
    pub fn render<D>(display: &mut D, state: &UiState) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Clear background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.bg))
            .draw(display)?;

        // Header with back button
        let header_height = 50;
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, header_height))
            .into_styled(PrimitiveStyle::with_fill(theme.status_bar_bg))
            .draw(display)?;

        // Back icon
        Icon::Back.draw(display, Point::new(spacing::MD, 15), 24, theme.text_primary)?;

        // Title
        let title_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        Text::new("Select AMS Slot", Point::new(spacing::MD + 36, 32), title_style).draw(display)?;

        let mut y = header_height as i32 + spacing::MD;

        // Printer name
        let printer_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        Text::new("X1 Carbon (00M09A...)", Point::new(spacing::MD, y + 12), printer_style)
            .draw(display)?;
        y += 24;

        // AMS unit card (placeholder - would come from actual printer data)
        let ams = AmsUnit::default();
        y = Self::draw_ams_unit(display, &ams, Point::new(spacing::MD, y))?;

        // External spool section
        y += spacing::MD;
        let ext_card = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(spacing::MD, y),
                Size::new(DISPLAY_WIDTH - (spacing::MD as u32 * 2), 100),
            ),
            Size::new(theme::radius::MD, theme::radius::MD),
        );
        ext_card
            .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // External spool label
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        Text::new("External Spool", Point::new(spacing::MD + 12, y + 20), label_style)
            .draw(display)?;

        // External slot
        Self::draw_slot(
            display,
            Point::new(spacing::MD + 12, y + 32),
            "EXT",
            None,
            theme.card_bg,
            0,
            false,
        )?;

        // Cancel button
        let cancel_button = Button::new(
            Point::new(DISPLAY_WIDTH as i32 - spacing::MD - 100, DISPLAY_HEIGHT as i32 - 60),
            Size::new(100, 44),
            "CANCEL",
        )
        .with_style(ButtonStyle::Secondary);
        cancel_button.draw(display)?;

        Ok(())
    }

    /// Draw an AMS unit card with 4 slots
    fn draw_ams_unit<D>(display: &mut D, ams: &AmsUnit, pos: Point) -> Result<i32, D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Card background
        let card_height = 130;
        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                pos,
                Size::new(DISPLAY_WIDTH - (spacing::MD as u32 * 2), card_height),
            ),
            Size::new(theme::radius::MD, theme::radius::MD),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        // AMS label
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        Text::new(&ams.name, Point::new(pos.x + 12, pos.y + 20), label_style).draw(display)?;

        // Draw 4 slots
        let slots_start_x = pos.x + 12;
        let slots_y = pos.y + 36;

        for (i, slot) in ams.slots.iter().enumerate() {
            let slot_x = slots_start_x + (i as i32) * (Self::SLOT_SIZE as i32 + Self::SLOT_SPACING);
            Self::draw_slot(
                display,
                Point::new(slot_x, slots_y),
                &slot.label,
                slot.material.as_ref().map(|s| s.as_str()),
                slot.color,
                slot.remaining_percent,
                slot.occupied,
            )?;
        }

        Ok(pos.y + card_height as i32)
    }

    /// Draw a single AMS slot
    fn draw_slot<D>(
        display: &mut D,
        pos: Point,
        label: &str,
        material: Option<&str>,
        color: Rgb565,
        remaining: u8,
        occupied: bool,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Slot background
        let slot = RoundedRectangle::with_equal_corners(
            Rectangle::new(pos, Size::new(Self::SLOT_SIZE, Self::SLOT_SIZE)),
            Size::new(theme::radius::SM, theme::radius::SM),
        );

        // Border style based on state
        let border_color = if occupied { theme.primary } else { theme.border };
        slot.into_styled(PrimitiveStyle::with_stroke(border_color, 2))
            .draw(display)?;

        // Color fill (partial based on remaining)
        if remaining > 0 {
            let fill_height = ((Self::SLOT_SIZE - 4) as u32 * remaining as u32 / 100) as u32;
            let fill_y = pos.y + (Self::SLOT_SIZE - 2 - fill_height) as i32;

            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(pos.x + 2, fill_y),
                    Size::new(Self::SLOT_SIZE - 4, fill_height),
                ),
                Size::new(theme::radius::SM - 2, theme::radius::SM - 2),
            )
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        }

        // Slot label
        let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        Text::new(
            label,
            Point::new(pos.x + Self::SLOT_SIZE as i32 / 2 - (label.len() as i32 * 3), pos.y + 20),
            label_style,
        )
        .draw(display)?;

        // Material label (if present)
        if let Some(mat) = material {
            let mat_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
            Text::new(
                mat,
                Point::new(pos.x + Self::SLOT_SIZE as i32 / 2 - (mat.len() as i32 * 3), pos.y + 50),
                mat_style,
            )
            .draw(display)?;
        }

        Ok(())
    }

    /// Get which slot was tapped
    pub fn get_slot_at(_point: Point) -> Option<(u8, u8)> {
        // Return (ams_id, slot_id) if a slot was tapped
        // TODO: Implement hit testing
        None
    }
}

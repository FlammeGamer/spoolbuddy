//! Catalog screen - filament inventory browser with filtering.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ < Catalog                                                  │
//! ├────────────────────────────────────────────────────────────┤
//! │  [All] [In AMS] [PLA] [PETG] [Other]                      │
//! ├────────────────────────────────────────────────────────────┤
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐         │
//! │  │ ████ A1 │ │ ████    │ │ ████ B2 │ │ ████    │         │
//! │  │ PLA     │ │ PETG    │ │ PLA     │ │ ASA     │         │
//! │  │ Yellow  │ │ Black   │ │ Red     │ │ White   │         │
//! │  │ 850g    │ │ 720g    │ │ 450g    │ │ 980g    │         │
//! │  └─────────┘ └─────────┘ └─────────┘ └─────────┘         │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐         │
//! │  │ ...     │ │ ...     │ │ ...     │ │ ...     │         │
//! │  └─────────┘ └─────────┘ └─────────┘ └─────────┘         │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, spacing};
use crate::ui::widgets::{CatalogCard, FilterPillRow, StatusBar};
use crate::ui::{CatalogFilter, UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

/// Catalog screen renderer
pub struct CatalogScreen;

impl CatalogScreen {
    /// Render the catalog screen
    pub fn render<D>(display: &mut D, state: &UiState) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Clear background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.bg))
            .draw(display)?;

        // Status bar
        let mut status_bar = StatusBar::new("< Catalog");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        // Filter pills
        let filters = ["All", "In AMS", "PLA", "PETG", "Other"];
        let active_filter = match state.catalog_filter {
            CatalogFilter::All => 0,
            CatalogFilter::InAms => 1,
            CatalogFilter::Pla => 2,
            CatalogFilter::Petg => 3,
            CatalogFilter::Other => 4,
        };

        FilterPillRow::new(Point::new(spacing::LG, 56), &filters)
            .with_active(active_filter)
            .draw(display)?;

        // Separator line
        Rectangle::new(
            Point::new(0, 92),
            Size::new(DISPLAY_WIDTH, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        // Catalog grid
        // 4 columns, variable rows
        let card_width = CatalogCard::WIDTH;
        let card_height = CatalogCard::HEIGHT;
        let gap = spacing::SM as u32;
        let grid_x = spacing::MD;
        let grid_y = 100;

        // Sample data - in real implementation, this would come from state
        let sample_spools: [(& str, &str, u32, &str, Option<&str>); 8] = [
            ("PLA", "Yellow", 0xF5C518FF, "850g", Some("A1")),
            ("PETG", "Black", 0x333333FF, "720g", None),
            ("PLA", "Red", 0xE91E63FF, "450g", Some("B2")),
            ("ASA", "White", 0xFFFFFFFF, "980g", None),
            ("PLA", "Blue", 0x2196F3FF, "600g", Some("C1")),
            ("TPU", "Clear", 0xECEFF1FF, "350g", None),
            ("PETG", "Orange", 0xFF9800FF, "900g", Some("A3")),
            ("ABS", "Purple", 0x673AB7FF, "780g", None),
        ];

        for (i, (material, color_name, rgba, weight, slot)) in sample_spools.iter().enumerate() {
            let col = i % 4;
            let row = i / 4;

            let x = grid_x + (col as i32 * (card_width as i32 + gap as i32));
            let y = grid_y + (row as i32 * (card_height as i32 + gap as i32));

            let color = theme::rgba_to_rgb565(*rgba);

            let mut card = CatalogCard::new(
                Point::new(x, y),
                material,
                color_name,
                color,
            )
            .with_weight(weight);

            if let Some(s) = slot {
                card = card.with_slot(s);
            }

            card.draw(display)?;
        }

        Ok(())
    }
}

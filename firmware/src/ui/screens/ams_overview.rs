//! AMS Overview screen - full visualization of all AMS units.
//!
//! Layout:
//! ┌────────────────────────────────────────────────────────────┐
//! │ SpoolBuddy                    [X1C-Studio]     [14:23]    │
//! ├────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────────────────────────────┐  ┌────┬────┐ │
//! │  │ AMS Units                               │  │Scan│Cat.│ │
//! │  │ ┌─────────┐ ┌─────────┐ ┌─────────┐    │  ├────┼────┤ │
//! │  │ │ AMS A   │ │ AMS B   │ │ AMS C   │    │  │Cal.│Set.│ │
//! │  │ │ ████████│ │ ████████│ │ ████████│    │  └────┴────┘ │
//! │  │ └─────────┘ └─────────┘ └─────────┘    │               │
//! │  │ ┌─────────┐ ┌────┐┌────┐┌────┐┌────┐  │               │
//! │  │ │ AMS D   │ │HT-A││HT-B││Ext1││Ext2│  │               │
//! │  │ │ ████████│ │ ██ ││ ██ ││ ██ ││ ██ │  │               │
//! │  │ └─────────┘ └────┘└────┘└────┘└────┘  │               │
//! │  └─────────────────────────────────────────┘               │
//! ├────────────────────────────────────────────────────────────┤
//! │  ● Connected        Printing           [Progress bar]     │
//! └────────────────────────────────────────────────────────────┘

use crate::ui::theme::{self, radius, spacing};
use crate::ui::widgets::{AmsSlot, AmsView, Button, StatusBar};
use crate::ui::widgets::button::ButtonStyle;
use crate::ui::widgets::icon::Icon;
use crate::ui::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::Text,
};

/// AMS Overview screen renderer
pub struct AmsOverviewScreen;

impl AmsOverviewScreen {
    /// Render the AMS overview screen
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
        let mut status_bar = StatusBar::new("SpoolBuddy");
        status_bar.set_wifi(state.wifi_connected, -60);
        status_bar.set_server(state.server_connected);
        status_bar.draw(display)?;

        // Main content area
        let content_y = 48;
        let panel_x = spacing::SM;
        let sidebar_x = 616;
        let panel_width = (sidebar_x - panel_x - spacing::SM) as u32;
        let panel_height = 388u32;

        // AMS Panel
        RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(panel_x, content_y),
                Size::new(panel_width, panel_height),
            ),
            Size::new(radius::LG, radius::LG),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
        .draw(display)?;

        // "AMS Units" title
        let title_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        Text::new(
            "AMS Units",
            Point::new(panel_x + spacing::MD, content_y + 16),
            title_style,
        )
        .draw(display)?;

        // Grid layout for AMS units
        // Row 1: AMS A, B, C
        let row1_y = content_y + 28;
        let ams_gap = spacing::SM;

        // Sample AMS data
        let ams_colors: [[Option<u32>; 4]; 4] = [
            [Some(0xF5C518FF), Some(0x333333FF), Some(0xFF9800FF), Some(0x9E9E9EFF)],  // AMS A
            [Some(0xE91E63FF), Some(0x2196F3FF), Some(0x4CAF50FF), None],              // AMS B
            [Some(0xFFFFFFFF), Some(0x212121FF), None, None],                          // AMS C
            [Some(0x00BCD4FF), Some(0xFF5722FF), None, None],                          // AMS D
        ];

        // Draw AMS A, B, C in row 1
        for (i, label) in ['A', 'B', 'C'].iter().enumerate() {
            let mut ams = AmsView::new(
                Point::new(
                    panel_x + spacing::SM + (i as i32 * (AmsView::new(Point::zero(), 'A').size().width as i32 + ams_gap)),
                    row1_y,
                ),
                *label,
            );

            for (slot_idx, color_opt) in ams_colors[i].iter().enumerate() {
                let slot = if let Some(rgba) = color_opt {
                    AmsSlot {
                        color: Some(theme::rgba_to_rgb565(*rgba)),
                        material: Some("PLA"),
                        active: i == 0 && slot_idx == 0,  // First slot of AMS A is active
                        empty: false,
                    }
                } else {
                    AmsSlot {
                        color: None,
                        material: None,
                        active: false,
                        empty: true,
                    }
                };
                ams.set_slot(slot_idx, slot);
            }
            ams.draw(display)?;
        }

        // Row 2: AMS D + HT slots + External slots
        let row2_y = row1_y + 170;

        // AMS D
        let mut ams_d = AmsView::new(Point::new(panel_x + spacing::SM, row2_y), 'D');
        for (slot_idx, color_opt) in ams_colors[3].iter().enumerate() {
            let slot = if let Some(rgba) = color_opt {
                AmsSlot {
                    color: Some(theme::rgba_to_rgb565(*rgba)),
                    material: Some("PLA"),
                    active: false,
                    empty: false,
                }
            } else {
                AmsSlot {
                    color: None,
                    material: None,
                    active: false,
                    empty: true,
                }
            };
            ams_d.set_slot(slot_idx, slot);
        }
        ams_d.draw(display)?;

        // Single slot units (HT-A, HT-B, Ext1, Ext2)
        let single_slot_y = row2_y;
        let single_slot_width = 70u32;
        let single_slot_height = 80u32;
        let single_start_x = panel_x + spacing::SM + ams_d.size().width as i32 + spacing::MD;

        let single_units = [
            ("HT-A", Some(0x673AB7FF)),
            ("HT-B", Some(0xECEFF1FF)),
            ("Ext 1", Some(0x607D8BFF)),
            ("Ext 2", Some(0x8BC34AFF)),
        ];

        for (i, (label, color_opt)) in single_units.iter().enumerate() {
            let x = single_start_x + (i as i32 * (single_slot_width as i32 + spacing::SM));

            // Card background
            RoundedRectangle::with_equal_corners(
                Rectangle::new(
                    Point::new(x, single_slot_y),
                    Size::new(single_slot_width, single_slot_height),
                ),
                Size::new(radius::SM, radius::SM),
            )
            .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0x02, 0x04, 0x02)))
            .draw(display)?;

            // Color swatch
            if let Some(rgba) = color_opt {
                let color = theme::rgba_to_rgb565(*rgba);
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(
                        Point::new(x + 8, single_slot_y + 8),
                        Size::new(single_slot_width - 16, 40),
                    ),
                    Size::new(radius::SM, radius::SM),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display)?;
            }

            // Label
            let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
            Text::new(
                label,
                Point::new(x + 8, single_slot_y + single_slot_height as i32 - 8),
                label_style,
            )
            .draw(display)?;
        }

        // Right sidebar - Action buttons (2x2 grid)
        let btn_x = sidebar_x;
        let btn_y = content_y;
        let btn_size = 82u32;
        let btn_gap = spacing::SM;

        // Button helper
        let draw_action_button = |display: &mut D, x: i32, y: i32, label: &str, icon: Icon| -> Result<(), D::Error> {
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(x, y), Size::new(btn_size, btn_size)),
                Size::new(radius::MD, radius::MD),
            )
            .into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

            icon.draw(display, Point::new(x + 26, y + 16), 30, theme.primary)?;

            let label_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
            Text::new(label, Point::new(x + 8, y + btn_size as i32 - 12), label_style)
                .draw(display)?;

            Ok(())
        };

        draw_action_button(display, btn_x, btn_y, "Scan", Icon::Nfc)?;
        draw_action_button(display, btn_x + btn_size as i32 + btn_gap, btn_y, "Catalog", Icon::Weight)?;
        draw_action_button(display, btn_x, btn_y + btn_size as i32 + btn_gap, "Calibrate", Icon::Weight)?;
        draw_action_button(display, btn_x + btn_size as i32 + btn_gap, btn_y + btn_size as i32 + btn_gap, "Settings", Icon::Setting)?;

        // Bottom status bar
        let bar_y = DISPLAY_HEIGHT as i32 - 44;

        Rectangle::new(
            Point::new(0, bar_y - 1),
            Size::new(DISPLAY_WIDTH, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(theme.border))
        .draw(display)?;

        Rectangle::new(
            Point::new(0, bar_y),
            Size::new(DISPLAY_WIDTH, 44),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0x03, 0x03, 0x03)))
        .draw(display)?;

        // Connection status
        let conn_color = if state.server_connected { theme.success } else { theme.disabled };
        Circle::new(Point::new(20, bar_y + 17), 10)
            .into_styled(PrimitiveStyle::with_fill(conn_color))
            .draw(display)?;

        let conn_text = if state.server_connected { "Connected" } else { "Disconnected" };
        Text::new(
            conn_text,
            Point::new(36, bar_y + 26),
            MonoTextStyle::new(&FONT_6X10, theme.text_primary),
        )
        .draw(display)?;

        // Print status (center)
        Text::new(
            "Ready",
            Point::new(DISPLAY_WIDTH as i32 / 2 - 20, bar_y + 26),
            MonoTextStyle::new(&FONT_6X10, theme.success),
        )
        .draw(display)?;

        Ok(())
    }
}

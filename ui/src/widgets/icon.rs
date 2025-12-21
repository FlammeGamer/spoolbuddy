//! Simple icon rendering using basic shapes.
//!
//! Since we don't have a font with icons, we draw simple icons using
//! geometric primitives.

use crate::theme;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle},
};

/// Icon type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// WiFi signal icon
    Wifi,
    /// Server/cloud icon
    Server,
    /// Settings gear icon
    Settings,
    /// Back arrow
    Back,
    /// Checkmark
    Check,
    /// X/close icon
    Close,
    /// Plus icon
    Plus,
    /// Minus icon
    Minus,
    /// NFC/contactless icon
    Nfc,
    /// Scale/weight icon
    Scale,
    /// Refresh/sync icon
    Refresh,
    /// Edit/pencil icon
    Edit,
    /// Trash/delete icon
    Trash,
    /// Warning triangle
    Warning,
    /// Info circle
    Info,
}

impl Icon {
    /// Draw the icon at the specified position with given color and size
    pub fn draw<D>(
        &self,
        display: &mut D,
        position: Point,
        size: u32,
        color: Rgb565,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let style = PrimitiveStyle::with_stroke(color, 2);
        let fill_style = PrimitiveStyle::with_fill(color);

        match self {
            Icon::Wifi => {
                // Draw WiFi arcs (simplified as lines)
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32;

                for i in 0..3 {
                    let r = (i + 1) * (size as i32 / 4);
                    // Simplified arc as angled lines
                    Line::new(
                        Point::new(cx - r, cy - r),
                        Point::new(cx, cy - r * 3 / 2),
                    )
                    .into_styled(style)
                    .draw(display)?;
                    Line::new(
                        Point::new(cx, cy - r * 3 / 2),
                        Point::new(cx + r, cy - r),
                    )
                    .into_styled(style)
                    .draw(display)?;
                }
                // Dot at bottom
                Circle::new(Point::new(cx - 2, cy - 4), 4)
                    .into_styled(fill_style)
                    .draw(display)?;
            }

            Icon::Server => {
                // Simple server/cloud icon
                let w = size;
                let h = size * 2 / 3;
                let p = position;

                // Two stacked rectangles
                Rectangle::new(p, Size::new(w, h))
                    .into_styled(style)
                    .draw(display)?;
                Rectangle::new(Point::new(p.x, p.y + h as i32 + 2), Size::new(w, h))
                    .into_styled(style)
                    .draw(display)?;

                // Indicator dots
                Circle::new(Point::new(p.x + 3, p.y + (h as i32 / 2) - 2), 4)
                    .into_styled(fill_style)
                    .draw(display)?;
            }

            Icon::Settings => {
                // Gear icon (simplified as circle with lines)
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let r = size as i32 / 3;

                Circle::new(Point::new(cx - r, cy - r), (r * 2) as u32)
                    .into_styled(style)
                    .draw(display)?;

                // Gear teeth (4 lines)
                let outer = size as i32 / 2;
                Line::new(Point::new(cx, cy - outer), Point::new(cx, cy + outer))
                    .into_styled(style)
                    .draw(display)?;
                Line::new(Point::new(cx - outer, cy), Point::new(cx + outer, cy))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Back => {
                // Left arrow
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let s = size as i32 / 3;

                Line::new(Point::new(cx + s, cy - s), Point::new(cx - s, cy))
                    .into_styled(style)
                    .draw(display)?;
                Line::new(Point::new(cx - s, cy), Point::new(cx + s, cy + s))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Check => {
                // Checkmark
                let p = position;
                let s = size as i32;

                Line::new(
                    Point::new(p.x, p.y + s * 2 / 3),
                    Point::new(p.x + s / 3, p.y + s),
                )
                .into_styled(style)
                .draw(display)?;
                Line::new(
                    Point::new(p.x + s / 3, p.y + s),
                    Point::new(p.x + s, p.y),
                )
                .into_styled(style)
                .draw(display)?;
            }

            Icon::Close => {
                // X icon
                let p = position;
                let s = size as i32;

                Line::new(p, Point::new(p.x + s, p.y + s))
                    .into_styled(style)
                    .draw(display)?;
                Line::new(Point::new(p.x + s, p.y), Point::new(p.x, p.y + s))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Plus => {
                // Plus icon
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let s = size as i32 / 3;

                Line::new(Point::new(cx, cy - s), Point::new(cx, cy + s))
                    .into_styled(style)
                    .draw(display)?;
                Line::new(Point::new(cx - s, cy), Point::new(cx + s, cy))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Minus => {
                // Minus icon
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let s = size as i32 / 3;

                Line::new(Point::new(cx - s, cy), Point::new(cx + s, cy))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Nfc => {
                // NFC/contactless icon (simplified waves)
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;

                // Inner circle/card
                Rectangle::new(
                    Point::new(cx - size as i32 / 4, cy - size as i32 / 6),
                    Size::new(size / 2, size / 3),
                )
                .into_styled(style)
                .draw(display)?;

                // Waves (simplified as arcs/lines)
                for i in 1..3 {
                    let r = i * (size as i32 / 4);
                    Line::new(
                        Point::new(cx + size as i32 / 4 + r / 2, cy - r),
                        Point::new(cx + size as i32 / 4 + r, cy),
                    )
                    .into_styled(style)
                    .draw(display)?;
                    Line::new(
                        Point::new(cx + size as i32 / 4 + r, cy),
                        Point::new(cx + size as i32 / 4 + r / 2, cy + r),
                    )
                    .into_styled(style)
                    .draw(display)?;
                }
            }

            Icon::Scale => {
                // Scale/weight icon
                let p = position;
                let w = size;
                let h = size * 2 / 3;

                // Platform
                Rectangle::new(Point::new(p.x, p.y + h as i32), Size::new(w, 4))
                    .into_styled(fill_style)
                    .draw(display)?;

                // Display
                Rectangle::new(Point::new(p.x + 4, p.y), Size::new(w - 8, h - 4))
                    .into_styled(style)
                    .draw(display)?;
            }

            Icon::Refresh => {
                // Circular arrows (simplified)
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let r = size as i32 / 3;

                // Circular path (quarter arcs)
                Circle::new(Point::new(cx - r, cy - r), (r * 2) as u32)
                    .into_styled(style)
                    .draw(display)?;

                // Arrow heads
                Triangle::new(
                    Point::new(cx + r, cy - 4),
                    Point::new(cx + r + 4, cy),
                    Point::new(cx + r - 4, cy),
                )
                .into_styled(fill_style)
                .draw(display)?;
            }

            Icon::Edit => {
                // Pencil icon
                let p = position;
                let s = size as i32;

                // Pencil body (diagonal line)
                Line::new(Point::new(p.x + s, p.y), Point::new(p.x, p.y + s))
                    .into_styled(PrimitiveStyle::with_stroke(color, 3))
                    .draw(display)?;

                // Tip
                Triangle::new(
                    Point::new(p.x, p.y + s),
                    Point::new(p.x + 4, p.y + s - 8),
                    Point::new(p.x + 8, p.y + s - 4),
                )
                .into_styled(fill_style)
                .draw(display)?;
            }

            Icon::Trash => {
                // Trash can icon
                let p = position;
                let w = size;
                let h = size;

                // Can body
                Rectangle::new(
                    Point::new(p.x + 2, p.y + (h as i32) / 4),
                    Size::new(w - 4, h * 3 / 4),
                )
                .into_styled(style)
                .draw(display)?;

                // Lid
                Rectangle::new(Point::new(p.x, p.y + 2), Size::new(w, 4))
                    .into_styled(fill_style)
                    .draw(display)?;

                // Handle
                Rectangle::new(Point::new(p.x + w as i32 / 3, p.y), Size::new(w / 3, 4))
                    .into_styled(fill_style)
                    .draw(display)?;
            }

            Icon::Warning => {
                // Warning triangle
                let p = position;
                let s = size as i32;

                Triangle::new(
                    Point::new(p.x + s / 2, p.y),
                    Point::new(p.x, p.y + s),
                    Point::new(p.x + s, p.y + s),
                )
                .into_styled(style)
                .draw(display)?;

                // Exclamation mark
                Line::new(
                    Point::new(p.x + s / 2, p.y + s / 3),
                    Point::new(p.x + s / 2, p.y + s * 2 / 3),
                )
                .into_styled(PrimitiveStyle::with_stroke(color, 2))
                .draw(display)?;
                Circle::new(Point::new(p.x + s / 2 - 2, p.y + s * 3 / 4), 4)
                    .into_styled(fill_style)
                    .draw(display)?;
            }

            Icon::Info => {
                // Info circle
                let cx = position.x + size as i32 / 2;
                let cy = position.y + size as i32 / 2;
                let r = size as i32 / 2;

                Circle::new(Point::new(cx - r, cy - r), (r * 2) as u32)
                    .into_styled(style)
                    .draw(display)?;

                // "i" letter
                Circle::new(Point::new(cx - 2, cy - r / 2), 4)
                    .into_styled(fill_style)
                    .draw(display)?;
                Line::new(
                    Point::new(cx, cy - r / 4),
                    Point::new(cx, cy + r / 2),
                )
                .into_styled(PrimitiveStyle::with_stroke(color, 2))
                .draw(display)?;
            }
        }

        Ok(())
    }
}

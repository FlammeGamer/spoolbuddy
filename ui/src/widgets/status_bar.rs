//! Status bar widget for the top of the screen.

use crate::theme::{self, spacing};
use crate::{UiState, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

/// Height of the status bar in pixels
pub const STATUS_BAR_HEIGHT: u32 = 40;

/// Status bar widget showing title, WiFi, server status, and time
pub struct StatusBar<'a> {
    /// Title text
    pub title: &'a str,
    /// Whether WiFi is connected
    pub wifi_connected: bool,
    /// WiFi signal strength (RSSI)
    pub wifi_rssi: i8,
    /// Whether server is connected
    pub server_connected: bool,
    /// Current time string (optional)
    pub time: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    /// Create a new status bar
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            wifi_connected: false,
            wifi_rssi: -100,
            server_connected: false,
            time: None,
        }
    }

    /// Create from UI state
    pub fn from_state(title: &'a str, state: &UiState) -> Self {
        Self {
            title,
            wifi_connected: state.wifi_connected,
            wifi_rssi: -60, // Default, would come from WiFi driver
            server_connected: state.server_connected,
            time: None,
        }
    }

    /// Set WiFi status
    pub fn set_wifi(&mut self, connected: bool, rssi: i8) {
        self.wifi_connected = connected;
        self.wifi_rssi = rssi;
    }

    /// Set server status
    pub fn set_server(&mut self, connected: bool) {
        self.server_connected = connected;
    }

    /// Set time string
    pub fn set_time(&mut self, time: &'a str) {
        self.time = Some(time);
    }

    /// Draw the status bar
    pub fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();

        // Background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, STATUS_BAR_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.status_bar_bg))
            .draw(display)?;

        // Title (left side)
        let title_style = MonoTextStyle::new(&FONT_6X10, theme.text_primary);
        Text::new(
            self.title,
            Point::new(spacing::MD, (STATUS_BAR_HEIGHT as i32) / 2 + 4),
            title_style,
        )
        .draw(display)?;

        // Right side indicators
        let mut x = DISPLAY_WIDTH as i32 - spacing::MD;

        // Time (if available)
        if let Some(time) = self.time {
            let time_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
            x -= (time.len() as i32) * 6 + spacing::SM;
            Text::new(
                time,
                Point::new(x, (STATUS_BAR_HEIGHT as i32) / 2 + 4),
                time_style,
            )
            .draw(display)?;
        }

        // Server indicator
        x -= 16 + spacing::SM;
        let server_color = if self.server_connected {
            theme.success
        } else {
            theme.error
        };
        Circle::new(
            Point::new(x, (STATUS_BAR_HEIGHT as i32) / 2 - 6),
            12,
        )
        .into_styled(PrimitiveStyle::with_fill(server_color))
        .draw(display)?;

        // WiFi indicator
        x -= 20 + spacing::SM;
        self.draw_wifi_icon(display, Point::new(x, (STATUS_BAR_HEIGHT as i32) / 2 - 8))?;

        Ok(())
    }

    /// Draw WiFi icon with signal strength
    fn draw_wifi_icon<D>(&self, display: &mut D, pos: Point) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let bars = if self.wifi_connected {
            theme::wifi_signal_bars(self.wifi_rssi)
        } else {
            0
        };

        let active_color = if self.wifi_connected {
            theme.primary
        } else {
            theme.error
        };
        let inactive_color = theme.disabled;

        // Draw 4 bars of increasing height
        for i in 0..4 {
            let bar_height = 4 + i * 3;
            let bar_x = pos.x + (i as i32) * 5;
            let bar_y = pos.y + 16 - bar_height as i32;

            let color = if i < bars { active_color } else { inactive_color };

            Rectangle::new(
                Point::new(bar_x, bar_y),
                Size::new(4, bar_height as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        }

        Ok(())
    }
}

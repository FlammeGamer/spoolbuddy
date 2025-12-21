//! Scale calibration screen with multi-step wizard.
//!
//! Step 1: Empty Scale
//! ┌──────────────────────────┐
//! │ Scale Calibration (1/2)  │
//! ├──────────────────────────┤
//! │                          │
//! │  Remove everything from  │
//! │  the scale               │
//! │                          │
//! │  Current: 0.0g           │
//! │                          │
//! │  [NEXT]        [CANCEL]  │
//! └──────────────────────────┘
//!
//! Step 2: Place Weight
//! ┌──────────────────────────┐
//! │ Scale Calibration (2/2)  │
//! ├──────────────────────────┤
//! │                          │
//! │  Place 500g calibration  │
//! │  weight on scale         │
//! │                          │
//! │  Current: 498.2g         │
//! │  Target:  500.0g         │
//! │                          │
//! │  [CALIBRATE]   [CANCEL]  │
//! └──────────────────────────┘

use crate::theme::{self, spacing};
use crate::widgets::Button;
use crate::widgets::button::ButtonStyle;
use crate::widgets::icon::Icon;
use crate::{UiState, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Text},
};

/// Calibration step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationStep {
    #[default]
    EmptyScale,
    PlaceWeight,
    Complete,
}

/// Calibration state
#[derive(Clone, Default)]
pub struct CalibrationState {
    pub step: CalibrationStep,
    pub target_weight: f32,
    pub zero_offset: i32,
    pub ready_to_advance: bool,
}

impl CalibrationState {
    pub fn new() -> Self {
        Self {
            step: CalibrationStep::EmptyScale,
            target_weight: 500.0, // Default 500g calibration weight
            zero_offset: 0,
            ready_to_advance: false,
        }
    }
}

/// Calibration screen renderer
pub struct CalibrationScreen;

// Global calibration state (for maintaining state across renders)
static mut CALIBRATION_STATE: Option<CalibrationState> = None;

impl CalibrationScreen {
    /// Get or create calibration state
    pub fn get_state() -> &'static mut CalibrationState {
        unsafe {
            if CALIBRATION_STATE.is_none() {
                CALIBRATION_STATE = Some(CalibrationState::new());
            }
            CALIBRATION_STATE.as_mut().unwrap()
        }
    }

    /// Reset calibration state
    pub fn reset() {
        unsafe {
            CALIBRATION_STATE = Some(CalibrationState::new());
        }
    }

    /// Render the calibration screen
    pub fn render<D>(display: &mut D, state: &UiState) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let cal_state = Self::get_state();

        // Clear background
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
            .into_styled(PrimitiveStyle::with_fill(theme.bg))
            .draw(display)?;

        // Header
        let header_height = 50;
        Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, header_height))
            .into_styled(PrimitiveStyle::with_fill(theme.status_bar_bg))
            .draw(display)?;

        // Title with step number
        let title = match cal_state.step {
            CalibrationStep::EmptyScale => "Scale Calibration (1/2)",
            CalibrationStep::PlaceWeight => "Scale Calibration (2/2)",
            CalibrationStep::Complete => "Calibration Complete",
        };

        let title_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        Text::new(title, Point::new(spacing::MD, 32), title_style).draw(display)?;

        // Main content card
        let card_margin = 40;
        let card_width = DISPLAY_WIDTH - (card_margin as u32 * 2);
        let card_height = 280;
        let card_x = card_margin;
        let card_y = header_height as i32 + spacing::LG;

        let card = RoundedRectangle::with_equal_corners(
            Rectangle::new(
                Point::new(card_x, card_y),
                Size::new(card_width, card_height),
            ),
            Size::new(theme::radius::LG, theme::radius::LG),
        );
        card.into_styled(PrimitiveStyle::with_fill(theme.card_bg))
            .draw(display)?;

        let content_x = card_x + spacing::LG;
        let content_y = card_y + spacing::LG;

        match cal_state.step {
            CalibrationStep::EmptyScale => {
                Self::render_empty_scale_step(display, state, content_x, content_y)?;
            }
            CalibrationStep::PlaceWeight => {
                Self::render_place_weight_step(display, state, cal_state, content_x, content_y)?;
            }
            CalibrationStep::Complete => {
                Self::render_complete_step(display, content_x, content_y)?;
            }
        }

        // Bottom buttons
        let button_y = DISPLAY_HEIGHT as i32 - 70;
        let button_height = 48;
        let button_width = 140;

        match cal_state.step {
            CalibrationStep::EmptyScale => {
                // Next button
                let next_button = Button::new(
                    Point::new(card_x + spacing::MD, button_y),
                    Size::new(button_width, button_height),
                    "NEXT",
                )
                .with_style(ButtonStyle::Primary)
                .with_large_font();
                next_button.draw(display)?;

                // Cancel button
                let cancel_button = Button::new(
                    Point::new(
                        card_x + card_width as i32 - spacing::MD - button_width as i32,
                        button_y,
                    ),
                    Size::new(button_width, button_height),
                    "CANCEL",
                )
                .with_style(ButtonStyle::Secondary)
                .with_large_font();
                cancel_button.draw(display)?;
            }
            CalibrationStep::PlaceWeight => {
                // Calibrate button
                let cal_button = Button::new(
                    Point::new(card_x + spacing::MD, button_y),
                    Size::new(button_width, button_height),
                    "CALIBRATE",
                )
                .with_style(ButtonStyle::Primary)
                .with_large_font();
                cal_button.draw(display)?;

                // Cancel button
                let cancel_button = Button::new(
                    Point::new(
                        card_x + card_width as i32 - spacing::MD - button_width as i32,
                        button_y,
                    ),
                    Size::new(button_width, button_height),
                    "CANCEL",
                )
                .with_style(ButtonStyle::Secondary)
                .with_large_font();
                cancel_button.draw(display)?;
            }
            CalibrationStep::Complete => {
                // Done button
                let done_button = Button::new(
                    Point::new((DISPLAY_WIDTH as i32 - button_width as i32) / 2, button_y),
                    Size::new(button_width, button_height),
                    "DONE",
                )
                .with_style(ButtonStyle::Primary)
                .with_large_font();
                done_button.draw(display)?;
            }
        }

        Ok(())
    }

    /// Render step 1: Empty scale
    fn render_empty_scale_step<D>(
        display: &mut D,
        state: &UiState,
        x: i32,
        y: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let instruction_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let detail_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let value_style = MonoTextStyle::new(&FONT_10X20, theme.primary);

        // Icon
        Icon::Scale.draw(display, Point::new(x + 100, y), 48, theme.primary)?;

        // Instructions
        Text::new(
            "Remove everything from",
            Point::new(x, y + 80),
            instruction_style,
        )
        .draw(display)?;
        Text::new("the scale", Point::new(x, y + 104), instruction_style).draw(display)?;

        // Current reading
        Text::new("Current reading:", Point::new(x, y + 150), detail_style).draw(display)?;

        let weight_text = theme::format_weight(state.weight);
        Text::new(&weight_text, Point::new(x, y + 180), value_style).draw(display)?;

        // Stability indicator
        if state.weight_stable {
            let stable_style = MonoTextStyle::new(&FONT_6X10, theme.success);
            Text::new("(stable)", Point::new(x + 120, y + 180), stable_style).draw(display)?;
        }

        Ok(())
    }

    /// Render step 2: Place calibration weight
    fn render_place_weight_step<D>(
        display: &mut D,
        state: &UiState,
        cal_state: &CalibrationState,
        x: i32,
        y: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let instruction_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);
        let detail_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        let value_style = MonoTextStyle::new(&FONT_10X20, theme.primary);

        // Instructions
        let mut target_text: heapless::String<32> = heapless::String::new();
        let _ = core::fmt::write(
            &mut target_text,
            format_args!("Place {:.0}g calibration", cal_state.target_weight),
        );
        Text::new(&target_text, Point::new(x, y + 40), instruction_style).draw(display)?;
        Text::new("weight on scale", Point::new(x, y + 64), instruction_style).draw(display)?;

        // Current reading
        Text::new("Current:", Point::new(x, y + 120), detail_style).draw(display)?;
        let weight_text = theme::format_weight(state.weight);
        Text::new(&weight_text, Point::new(x + 80, y + 120), value_style).draw(display)?;

        // Target
        Text::new("Target:", Point::new(x, y + 150), detail_style).draw(display)?;
        let target_weight_text = theme::format_weight(cal_state.target_weight);
        Text::new(&target_weight_text, Point::new(x + 80, y + 150), value_style).draw(display)?;

        // Difference
        let diff = state.weight - cal_state.target_weight;
        let diff_color = if diff.abs() < 5.0 {
            theme.success
        } else {
            theme.warning
        };
        let diff_style = MonoTextStyle::new(&FONT_6X10, diff_color);

        let mut diff_text: heapless::String<32> = heapless::String::new();
        let _ = core::fmt::write(&mut diff_text, format_args!("Difference: {:.1}g", diff));
        Text::new(&diff_text, Point::new(x, y + 190), diff_style).draw(display)?;

        Ok(())
    }

    /// Render completion step
    fn render_complete_step<D>(display: &mut D, x: i32, y: i32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let theme = theme::theme();
        let instruction_style = MonoTextStyle::new(&FONT_10X20, theme.text_primary);

        // Success icon
        Icon::Check.draw(display, Point::new(x + 100, y + 20), 64, theme.success)?;

        // Message
        Text::new("Calibration complete!", Point::new(x, y + 120), instruction_style)
            .draw(display)?;

        let detail_style = MonoTextStyle::new(&FONT_6X10, theme.text_secondary);
        Text::new(
            "Your scale is now calibrated.",
            Point::new(x, y + 150),
            detail_style,
        )
        .draw(display)?;

        Ok(())
    }

    /// Advance to next step
    pub fn next_step() {
        let state = Self::get_state();
        state.step = match state.step {
            CalibrationStep::EmptyScale => CalibrationStep::PlaceWeight,
            CalibrationStep::PlaceWeight => CalibrationStep::Complete,
            CalibrationStep::Complete => CalibrationStep::Complete,
        };
    }
}

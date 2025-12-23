//! Reusable UI widgets for SpoolBuddy.
//!
//! These widgets use embedded-graphics to render professional-looking
//! UI elements that work with both light and dark themes.

pub mod ams_view;
pub mod button;
pub mod catalog_card;
pub mod filter_pill;
pub mod icon;
pub mod info_row;
pub mod progress_bar;
pub mod settings_row;
pub mod slider;
pub mod spool_card;
pub mod status_bar;
pub mod tab_bar;
pub mod toggle;
pub mod weight_display;

pub use ams_view::{AmsSlot, AmsView};
pub use button::Button;
pub use catalog_card::CatalogCard;
pub use filter_pill::{FilterPill, FilterPillRow};
pub use info_row::InfoRow;
pub use progress_bar::ProgressBar;
pub use settings_row::{SettingsRow, StatusDot};
pub use slider::Slider;
pub use spool_card::SpoolCard;
pub use status_bar::StatusBar;
pub use tab_bar::TabBar;
pub use toggle::Toggle;
pub use weight_display::WeightDisplay;

//! NFC module for PN5180 NFC reader.
//!
//! The PN5180 is a high-performance NFC frontend supporting:
//! - ISO14443A/B (MIFARE, NFC tags)
//! - ISO15693 (ICODE, vicinity cards - longer range)
//!
//! Interface: SPI (up to 7 MHz) + BUSY + RST pins
//!
//! Hardware connection via CrowPanel Advance 7.0" headers:
//! NOTE: J9 pins (IO4/5/6) are used by the RGB LCD display!
//! We use UART0-OUT header for SPI instead.
//!
//! - IO43 (UART0-OUT) -> SPI SCK
//! - IO44 (UART0-OUT) -> SPI MISO
//! - IO16 (J11 Pin 2) -> SPI MOSI
//! - IO8  (J11 Pin 6) -> NSS chip select
//! - IO2  (J11 Pin 5) -> BUSY signal
//! - IO15 (J11 Pin 3) -> RST reset

#[allow(dead_code)]
pub mod pn5180;

// Re-exports will be used when NFC functionality is integrated
#[allow(unused_imports)]
pub use pn5180::{Pn5180State, Pn5180Error, Iso14443aCard, MifareKeyType, BAMBULAB_KEY};
#[allow(unused_imports)]
pub use pn5180::{init_stub, detect_tag_stub, rf_field_on_stub, rf_field_off_stub};

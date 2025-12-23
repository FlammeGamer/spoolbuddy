//! NFC module for PN5180 NFC reader.
//!
//! The PN5180 is a high-performance NFC frontend supporting:
//! - ISO14443A/B (MIFARE, NFC tags)
//! - ISO15693 (ICODE, vicinity cards - longer range)
//!
//! Interface: SPI (up to 7 MHz) + BUSY + RST pins
//!
//! Hardware connection via CrowPanel Advance 7.0" Wireless Module Headers:
//! (DIP switch S1=0, S0=1 for Wireless Module mode)
//!
//! - IO5  (J9 Pin 2)  -> SPI SCK
//! - IO4  (J9 Pin 3)  -> SPI MISO
//! - IO6  (J9 Pin 4)  -> SPI MOSI
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

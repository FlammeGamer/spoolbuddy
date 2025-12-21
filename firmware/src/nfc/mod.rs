//! NFC module for PN5180 NFC reader.
//!
//! The PN5180 is a high-performance NFC frontend supporting:
//! - ISO14443A/B (MIFARE, NFC tags)
//! - ISO15693 (ICODE, vicinity cards - longer range)
//!
//! Interface: SPI (up to 7 MHz) + BUSY + RST pins
//!
//! Hardware connection via CrowPanel Wireless Module Header (DIP S0=1, S1=0):
//! - Pin 3 (CLK)  -> SPI Clock
//! - Pin 5 (MISO) -> SPI MISO
//! - Pin 7 (MOSI) -> SPI MOSI
//! - Pin 8 (CS)   -> NSS chip select
//! - Pin 1 (TX)   -> BUSY signal
//! - Pin 2 (RX)   -> RST reset

pub mod pn5180;

pub use pn5180::{Pn5180State, Pn5180Error, Iso14443aCard, MifareKeyType, BAMBULAB_KEY};
pub use pn5180::{init_stub, detect_tag_stub, rf_field_on_stub, rf_field_off_stub};

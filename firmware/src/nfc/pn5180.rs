//! PN5180 NFC controller driver.
//!
//! The PN5180 communicates via SPI with the following pins:
//! - MOSI, MISO, SCLK - Standard SPI (directly controlled, directly active low)
//! - NSS - Chip select (directly controlled, active low)
//! - BUSY - Indicates when chip is processing (active high)
//! - RST - Hardware reset (active low)
//!
//! CrowPanel Advance 7.0" Wireless Module Header pinout:
//! - Pin 3 (CLK)  -> SPI Clock
//! - Pin 5 (MISO) -> SPI MISO
//! - Pin 7 (MOSI) -> SPI MOSI
//! - Pin 8 (CS)   -> NSS (directly controlled)
//! - Pin 1 (TX)   -> BUSY
//! - Pin 2 (RX)   -> RST
//!
//! GPIO assignments (when DIP switch S0=1, S1=0 for Wireless Module mode):
//! - These GPIOs are shared with SD card, so only one can be active at a time
//!
//! Commands are sent as:
//! [CMD_BYTE] [PAYLOAD...]
//!
//! Responses are read after BUSY goes low.

use log::info;

/// PN5180 command codes
#[allow(dead_code)]
pub mod commands {
    pub const WRITE_REGISTER: u8 = 0x00;
    pub const WRITE_REGISTER_OR_MASK: u8 = 0x01;
    pub const WRITE_REGISTER_AND_MASK: u8 = 0x02;
    pub const READ_REGISTER: u8 = 0x04;
    pub const WRITE_EEPROM: u8 = 0x06;
    pub const READ_EEPROM: u8 = 0x07;
    pub const SEND_DATA: u8 = 0x09;
    pub const READ_DATA: u8 = 0x0A;
    pub const SWITCH_MODE: u8 = 0x0B;
    pub const MIFARE_AUTHENTICATE: u8 = 0x0C;
    pub const EPC_INVENTORY: u8 = 0x0D;
    pub const EPC_RESUME_INVENTORY: u8 = 0x0E;
    pub const EPC_RETRIEVE_INVENTORY_RESULT_SIZE: u8 = 0x0F;
    pub const EPC_RETRIEVE_INVENTORY_RESULT: u8 = 0x10;
    pub const LOAD_RF_CONFIG: u8 = 0x11;
    pub const UPDATE_RF_CONFIG: u8 = 0x12;
    pub const RETRIEVE_RF_CONFIG_SIZE: u8 = 0x13;
    pub const RETRIEVE_RF_CONFIG: u8 = 0x14;
    pub const RF_ON: u8 = 0x16;
    pub const RF_OFF: u8 = 0x17;
}

/// PN5180 register addresses
#[allow(dead_code)]
pub mod registers {
    pub const SYSTEM_CONFIG: u8 = 0x00;
    pub const IRQ_ENABLE: u8 = 0x01;
    pub const IRQ_STATUS: u8 = 0x02;
    pub const IRQ_CLEAR: u8 = 0x03;
    pub const TRANSCEIVE_CONTROL: u8 = 0x04;
    pub const TIMER1_CONFIG: u8 = 0x0F;
    pub const TIMER1_RELOAD: u8 = 0x10;
    pub const TIMER1_VALUE: u8 = 0x11;
    pub const TX_DATA_NUM: u8 = 0x14;
    pub const RX_STATUS: u8 = 0x15;
    pub const RF_STATUS: u8 = 0x1D;
}

/// RF configuration protocols
#[allow(dead_code)]
pub mod rf_config {
    pub const ISO_14443A_106_TX: u8 = 0x00;
    pub const ISO_14443A_106_RX: u8 = 0x80;
    pub const ISO_14443A_212_TX: u8 = 0x01;
    pub const ISO_14443A_212_RX: u8 = 0x81;
    pub const ISO_14443A_424_TX: u8 = 0x02;
    pub const ISO_14443A_424_RX: u8 = 0x82;
    pub const ISO_14443A_848_TX: u8 = 0x03;
    pub const ISO_14443A_848_RX: u8 = 0x83;
}

/// MIFARE authentication key type
#[derive(Debug, Clone, Copy)]
pub enum MifareKeyType {
    KeyA,
    KeyB,
}

/// Bambu Lab MIFARE key (Crypto-1)
/// Note: This is the known key for reading Bambu Lab tags
pub const BAMBULAB_KEY: [u8; 6] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]; // Placeholder - actual key needed

/// PN5180 driver state (without hardware - for init tracking)
pub struct Pn5180State {
    /// Whether the PN5180 has been initialized
    pub initialized: bool,
    /// Firmware version (major, minor, patch)
    pub firmware_version: (u8, u8, u8),
    /// Last detected card UID (up to 10 bytes)
    pub last_uid: Option<[u8; 10]>,
    /// Length of last UID
    pub last_uid_len: u8,
    /// RF field is on
    pub rf_on: bool,
}

impl Pn5180State {
    pub fn new() -> Self {
        Self {
            initialized: false,
            firmware_version: (0, 0, 0),
            last_uid: None,
            last_uid_len: 0,
            rf_on: false,
        }
    }
}

impl Default for Pn5180State {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// STUB IMPLEMENTATION - Hardware not connected yet
// =============================================================================
// The functions below are stubs that will be implemented when the PN5180
// hardware is connected via the wireless module header.
//
// GPIO assignments (DIP switch S0=1, S1=0 for Wireless Module mode):
// - CLK (Pin 3)  -> SPI Clock
// - MISO (Pin 5) -> SPI MISO
// - MOSI (Pin 7) -> SPI MOSI
// - CS (Pin 8)   -> NSS chip select
// - TX (Pin 1)   -> BUSY signal
// - RX (Pin 2)   -> RST reset
// =============================================================================

/// Initialize the PN5180 NFC reader (STUB)
pub fn init_stub(state: &mut Pn5180State) -> Result<(), Pn5180Error> {
    info!("PN5180 NFC reader init (STUB - hardware not connected)");

    // Simulate successful initialization
    state.firmware_version = (0, 0, 0);
    state.initialized = false; // Keep false until real hardware

    Ok(())
}

/// Check if a tag is present (STUB)
pub fn detect_tag_stub(_state: &Pn5180State) -> Result<Option<Iso14443aCard>, Pn5180Error> {
    // No tag detected (stub)
    Ok(None)
}

/// Turn RF field on (STUB)
pub fn rf_field_on_stub(state: &mut Pn5180State) -> Result<(), Pn5180Error> {
    info!("RF field on (STUB)");
    state.rf_on = true;
    Ok(())
}

/// Turn RF field off (STUB)
pub fn rf_field_off_stub(state: &mut Pn5180State) -> Result<(), Pn5180Error> {
    info!("RF field off (STUB)");
    state.rf_on = false;
    Ok(())
}

/// ISO 14443A card info
#[derive(Debug, Clone)]
pub struct Iso14443aCard {
    /// UID (4, 7, or 10 bytes)
    pub uid: [u8; 10],
    /// UID length (4, 7, or 10)
    pub uid_len: u8,
    /// ATQA (2 bytes)
    pub atqa: [u8; 2],
    /// SAK byte
    pub sak: u8,
}

impl Iso14443aCard {
    /// Check if this is an NTAG (based on SAK)
    pub fn is_ntag(&self) -> bool {
        self.sak == 0x00
    }

    /// Check if this is a MIFARE Classic 1K (based on SAK)
    pub fn is_mifare_classic_1k(&self) -> bool {
        self.sak == 0x08
    }

    /// Check if this is a MIFARE Classic 4K (based on SAK)
    pub fn is_mifare_classic_4k(&self) -> bool {
        self.sak == 0x18
    }
}

/// PN5180 errors
#[derive(Debug, Clone, Copy)]
pub enum Pn5180Error {
    SpiError,
    GpioError,
    Timeout,
    NoCard,
    AuthFailed,
    ReadFailed,
    WriteFailed,
    InvalidResponse,
}

//! NAU7802 24-bit ADC Scale Driver
//!
//! SparkFun Qwiic Scale uses the NAU7802 chip for high-precision weight measurement.
//!
//! I2C Address: 0x2A
//!
//! Pinout (SparkFun Qwiic Scale):
//!   MCU Side:
//!     - GND: Ground
//!     - 3V3: 3.3V power
//!     - SDA: I2C Data
//!     - SCL: I2C Clock
//!     - INT: Interrupt (optional)
//!     - AVDD: Analog VDD (connect to 3V3)
//!
//!   Load Cell Side (green terminal):
//!     - RED: Excitation+ (E+)
//!     - BLK: Excitation- (E-)
//!     - WHT: Signal- (A-)
//!     - GRN: Signal+ (A+)

use esp_idf_hal::i2c::I2cDriver;
use log::{info, warn};

/// NAU7802 I2C address
pub const NAU7802_ADDR: u8 = 0x2A;

/// NAU7802 Register addresses
#[allow(dead_code)]
mod reg {
    pub const PU_CTRL: u8 = 0x00;      // Power-up control
    pub const CTRL1: u8 = 0x01;        // Control 1
    pub const CTRL2: u8 = 0x02;        // Control 2
    pub const OCAL1_B2: u8 = 0x03;     // Offset calibration
    pub const OCAL1_B1: u8 = 0x04;
    pub const OCAL1_B0: u8 = 0x05;
    pub const GCAL1_B3: u8 = 0x06;     // Gain calibration
    pub const GCAL1_B2: u8 = 0x07;
    pub const GCAL1_B1: u8 = 0x08;
    pub const GCAL1_B0: u8 = 0x09;
    pub const I2C_CTRL: u8 = 0x11;     // I2C control
    pub const ADCO_B2: u8 = 0x12;      // ADC output (MSB)
    pub const ADCO_B1: u8 = 0x13;      // ADC output
    pub const ADCO_B0: u8 = 0x14;      // ADC output (LSB)
    pub const ADC: u8 = 0x15;          // ADC control
    pub const PGA: u8 = 0x1B;          // PGA control
    pub const PWR_CTRL: u8 = 0x1C;     // Power control
    pub const REVISION: u8 = 0x1F;     // Revision ID
}

/// PU_CTRL register bits
#[allow(dead_code)]
mod pu_ctrl {
    pub const RR: u8 = 0x01;           // Register reset
    pub const PUD: u8 = 0x02;          // Power up digital
    pub const PUA: u8 = 0x04;          // Power up analog
    pub const PUR: u8 = 0x08;          // Power up ready (read-only)
    pub const CS: u8 = 0x10;           // Cycle start
    pub const CR: u8 = 0x20;           // Cycle ready (read-only)
    pub const OSCS: u8 = 0x40;         // Oscillator select
    pub const AVDDS: u8 = 0x80;        // AVDD source select
}

/// Sample rates
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum SampleRate {
    Sps10 = 0,
    Sps20 = 1,
    Sps40 = 2,
    Sps80 = 3,
    Sps320 = 7,
}

/// PGA Gain settings
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Gain {
    X1 = 0,
    X2 = 1,
    X4 = 2,
    X8 = 3,
    X16 = 4,
    X32 = 5,
    X64 = 6,
    X128 = 7,
}

/// LDO Voltage settings
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum LdoVoltage {
    V2_4 = 0b111,
    V2_7 = 0b110,
    V3_0 = 0b101,
    V3_3 = 0b100,
    V3_6 = 0b011,
    V3_9 = 0b010,
    V4_2 = 0b001,
    V4_5 = 0b000,
}

/// Scale calibration data
#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    /// Zero offset (tare)
    pub zero_offset: i32,
    /// Calibration factor (raw units per gram)
    pub cal_factor: f32,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            zero_offset: 0,
            // Default calibration factor - needs actual calibration
            cal_factor: 1000.0,
        }
    }
}

/// NAU7802 Scale driver state
pub struct Nau7802State {
    /// Calibration data
    pub calibration: Calibration,
    /// Whether the scale has been initialized
    pub initialized: bool,
    /// Last raw reading
    pub last_raw: i32,
    /// Filtered weight in grams
    pub weight_grams: f32,
    /// Filter alpha (0-1, higher = less filtering)
    pub filter_alpha: f32,
    /// Weight stability flag
    pub stable: bool,
    /// Consecutive stable readings counter
    pub stable_count: u8,
}

impl Nau7802State {
    /// Create a new scale state
    pub fn new() -> Self {
        Self {
            calibration: Calibration::default(),
            initialized: false,
            last_raw: 0,
            weight_grams: 0.0,
            filter_alpha: 0.1, // Smooth filtering
            stable: false,
            stable_count: 0,
        }
    }
}

impl Default for Nau7802State {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the NAU7802
pub fn init(i2c: &mut I2cDriver<'_>, state: &mut Nau7802State) -> Result<(), Nau7802Error> {
    info!("Initializing NAU7802 scale at 0x{:02X}", NAU7802_ADDR);

    // Check if device is present
    let revision = read_reg(i2c, reg::REVISION)?;
    info!("  NAU7802 revision: 0x{:02X}", revision);

    // Reset the device
    write_reg(i2c, reg::PU_CTRL, pu_ctrl::RR)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    write_reg(i2c, reg::PU_CTRL, 0x00)?;

    // Power up digital and analog
    write_reg(i2c, reg::PU_CTRL, pu_ctrl::PUD | pu_ctrl::PUA)?;

    // Wait for power-up ready
    let mut timeout = 100;
    loop {
        let status = read_reg(i2c, reg::PU_CTRL)?;
        if (status & pu_ctrl::PUR) != 0 {
            info!("  NAU7802 powered up");
            break;
        }
        timeout -= 1;
        if timeout == 0 {
            warn!("  NAU7802 power-up timeout");
            return Err(Nau7802Error::Timeout);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    // Configure sample rate (80 SPS for responsive readings)
    set_sample_rate(i2c, SampleRate::Sps80)?;

    // Configure gain (128x for load cells)
    set_gain(i2c, Gain::X128)?;

    // Configure LDO (3.3V)
    set_ldo(i2c, LdoVoltage::V3_3)?;

    // Enable internal LDO
    let ctrl1 = read_reg(i2c, reg::CTRL1)?;
    write_reg(i2c, reg::CTRL1, ctrl1 | 0x80)?; // VLDO bit

    // Start conversion cycle
    let pu_ctrl_val = read_reg(i2c, reg::PU_CTRL)?;
    write_reg(i2c, reg::PU_CTRL, pu_ctrl_val | pu_ctrl::CS)?;

    state.initialized = true;
    info!("  NAU7802 initialization complete");

    Ok(())
}

/// Set sample rate
pub fn set_sample_rate(i2c: &mut I2cDriver<'_>, rate: SampleRate) -> Result<(), Nau7802Error> {
    let ctrl2 = read_reg(i2c, reg::CTRL2)?;
    let new_ctrl2 = (ctrl2 & 0x8F) | ((rate as u8) << 4);
    write_reg(i2c, reg::CTRL2, new_ctrl2)
}

/// Set PGA gain
pub fn set_gain(i2c: &mut I2cDriver<'_>, gain: Gain) -> Result<(), Nau7802Error> {
    let ctrl1 = read_reg(i2c, reg::CTRL1)?;
    let new_ctrl1 = (ctrl1 & 0xF8) | (gain as u8);
    write_reg(i2c, reg::CTRL1, new_ctrl1)
}

/// Set LDO voltage
pub fn set_ldo(i2c: &mut I2cDriver<'_>, voltage: LdoVoltage) -> Result<(), Nau7802Error> {
    let ctrl1 = read_reg(i2c, reg::CTRL1)?;
    let new_ctrl1 = (ctrl1 & 0xC7) | ((voltage as u8) << 3);
    write_reg(i2c, reg::CTRL1, new_ctrl1)
}

/// Check if data is ready
pub fn data_ready(i2c: &mut I2cDriver<'_>) -> Result<bool, Nau7802Error> {
    let status = read_reg(i2c, reg::PU_CTRL)?;
    Ok((status & pu_ctrl::CR) != 0)
}

/// Read raw ADC value (24-bit signed)
pub fn read_raw(i2c: &mut I2cDriver<'_>, state: &mut Nau7802State) -> Result<i32, Nau7802Error> {
    // Read 3 bytes of ADC data
    let b2 = read_reg(i2c, reg::ADCO_B2)? as i32;
    let b1 = read_reg(i2c, reg::ADCO_B1)? as i32;
    let b0 = read_reg(i2c, reg::ADCO_B0)? as i32;

    // Combine into 24-bit value
    let mut raw = (b2 << 16) | (b1 << 8) | b0;

    // Sign extend from 24-bit to 32-bit
    if (raw & 0x800000) != 0 {
        raw |= 0xFF000000u32 as i32;
    }

    state.last_raw = raw;
    Ok(raw)
}

/// Read weight in grams (with filtering and stability detection)
pub fn read_weight(i2c: &mut I2cDriver<'_>, state: &mut Nau7802State) -> Result<f32, Nau7802Error> {
    if !state.initialized {
        return Err(Nau7802Error::NotInitialized);
    }

    // Check if data is ready
    if !data_ready(i2c)? {
        return Ok(state.weight_grams); // Return last value
    }

    let raw = read_raw(i2c, state)?;

    // Convert to grams using calibration
    let weight = (raw - state.calibration.zero_offset) as f32 / state.calibration.cal_factor;

    // Store previous weight for stability check
    let prev_weight = state.weight_grams;

    // Apply exponential moving average filter
    state.weight_grams = state.weight_grams * (1.0 - state.filter_alpha) + weight * state.filter_alpha;

    // Check stability (within 0.5g of previous reading)
    let diff = (state.weight_grams - prev_weight).abs();
    if diff < 0.5 {
        state.stable_count = state.stable_count.saturating_add(1);
        if state.stable_count >= 5 {
            state.stable = true;
        }
    } else {
        state.stable_count = 0;
        state.stable = false;
    }

    Ok(state.weight_grams)
}

/// Tare the scale (set current weight as zero)
pub fn tare(i2c: &mut I2cDriver<'_>, state: &mut Nau7802State) -> Result<(), Nau7802Error> {
    info!("Taring scale...");

    // Take average of multiple readings
    let mut sum: i64 = 0;
    let samples = 10;

    for _ in 0..samples {
        // Wait for data ready
        while !data_ready(i2c)? {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        sum += read_raw(i2c, state)? as i64;
    }

    state.calibration.zero_offset = (sum / samples as i64) as i32;
    state.weight_grams = 0.0;

    info!("  Tare complete, zero offset: {}", state.calibration.zero_offset);
    Ok(())
}

/// Calibrate with a known weight
pub fn calibrate(i2c: &mut I2cDriver<'_>, state: &mut Nau7802State, known_weight_grams: f32) -> Result<(), Nau7802Error> {
    info!("Calibrating scale with {} grams...", known_weight_grams);

    // Take average of multiple readings
    let mut sum: i64 = 0;
    let samples = 10;

    for _ in 0..samples {
        while !data_ready(i2c)? {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        sum += read_raw(i2c, state)? as i64;
    }

    let avg_raw = (sum / samples as i64) as i32;
    let delta = avg_raw - state.calibration.zero_offset;

    if delta.abs() < 100 {
        warn!("  Calibration failed: weight too close to zero");
        return Err(Nau7802Error::CalibrationFailed);
    }

    state.calibration.cal_factor = delta as f32 / known_weight_grams;

    info!("  Calibration complete, factor: {}", state.calibration.cal_factor);
    Ok(())
}

// --- Private helpers ---

fn read_reg(i2c: &mut I2cDriver<'_>, reg: u8) -> Result<u8, Nau7802Error> {
    let mut buf = [0u8; 1];
    i2c.write_read(NAU7802_ADDR, &[reg], &mut buf, 100)
        .map_err(|_| Nau7802Error::I2cError)?;
    Ok(buf[0])
}

fn write_reg(i2c: &mut I2cDriver<'_>, reg: u8, value: u8) -> Result<(), Nau7802Error> {
    i2c.write(NAU7802_ADDR, &[reg, value], 100)
        .map_err(|_| Nau7802Error::I2cError)?;
    Ok(())
}

/// NAU7802 error types
#[derive(Debug)]
pub enum Nau7802Error {
    I2cError,
    NotInitialized,
    Timeout,
    CalibrationFailed,
}

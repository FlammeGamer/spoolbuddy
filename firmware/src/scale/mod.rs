//! Scale module for load cell amplifiers.
//!
//! Supports:
//! - NAU7802 (SparkFun Qwiic Scale) - I2C interface, recommended
//!
//! The NAU7802 is a 24-bit ADC with I2C interface at address 0x2A.

pub mod nau7802;

pub use nau7802::{Nau7802State, Nau7802Error, NAU7802_ADDR};

#![no_std]
#![deny(missing_docs, unsafe_code)]

//! ESP PSRAM SPI Driver Crate
/// ESP-PSRAM64 and ESP-PSRAM64H are 64 Mbit serial pseudo SRAM devices that are organized in 8Mx8 bits.
/// They are fabricated using the high-performance and high-reliability CMOS technology.
///
/// ESP-PSRAM64 operates at 1.8V and can offer high data bandwidth at 144 MHz clock rate, while ESP-PSRAM64H operates at 3.3V and can support up to 133 MHz clock rate.
///
/// Note, however, that burst operations which cross page boundaries have a lower max input clock frequency at 84 MHz.
/// Both of the PSRAM devices can be accessed via the Serial Peripheral Interface (SPI).
///
/// Additionally, a Quad Peripheral Interface (QPI) is supported by the device if the application needs faster data rates. (Not yet implemented in this driver)
///
/// The devices also support unlimited reads and writes to the memory array.
///
extern crate embedded_hal as hal;
mod error;
/// Implements the driver and the storage traits
pub mod psram;

pub use crate::error::Error;

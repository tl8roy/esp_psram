
#![cfg_attr(not(test), no_std)]
#![deny(missing_docs, unsafe_code)]
#![no_std]

extern crate embedded_hal as hal;
mod error;
pub mod psram;

pub use crate::error::Error;

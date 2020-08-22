use core::fmt::{self, Debug, Display};
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::OutputPin;

mod private {
    #[derive(Debug)]
    pub enum Private {}
}

/// The error type used by this library.
///
/// This can encapsulate an SPI or GPIO error, and adds its own protocol errors
/// on top of that.
pub enum Error<SPI: Transfer<u8>, GPIO: OutputPin> {
    /// An SPI transfer failed.
    Spi(SPI::Error),

    /// A GPIO could not be set.
    Gpio(GPIO::Error),

    /// Device is not the correct type.
    InvalidDevice,

    /// Device does not support the mode of operation selected
    InvalidMode,

    #[doc(hidden)]
    __NonExhaustive(private::Private),
}

impl<SPI: Transfer<u8>, GPIO: OutputPin> Debug for Error<SPI, GPIO>
where
    SPI::Error: Debug,
    GPIO::Error: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spi(spi) => write!(f, "Error::Spi({:?})", spi),
            Error::Gpio(gpio) => write!(f, "Error::Gpio({:?})", gpio),
            Error::InvalidDevice => f.write_str("Error::InvalidDevice"),
            Error::InvalidMode => f.write_str("Error::InvalidMode"),
            Error::__NonExhaustive(_) => unreachable!(),
        }
    }
}

impl<SPI: Transfer<u8>, GPIO: OutputPin> Display for Error<SPI, GPIO>
where
    SPI::Error: Display,
    GPIO::Error: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Spi(spi) => write!(f, "SPI error: {}", spi),
            Error::Gpio(gpio) => write!(f, "GPIO error: {}", gpio),
            Error::InvalidDevice => {
                f.write_str("This is not the correct device for the driver or it is faulty")
            }
            Error::InvalidMode => f.write_str("The driver or device is not in the correct mode"),
            Error::__NonExhaustive(_) => unreachable!(),
        }
    }
}

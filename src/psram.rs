use crate::Error;
//pub mod prelude;

use hal::storage::{
    Address, AddressOffset, MultiRead, MultiWrite, SingleRead, SingleWrite, StorageSize,
};

use core::convert::TryInto;
//use core::fmt;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::OutputPin;

/// Device identification and known good flag.
pub struct Identification {
    /// 48 Bit EID of the device
    pub eid: u64,

    /// True only after all tests are passed
    pub known_good_device: bool,
}

#[allow(unused)]
enum KGD {
    Good = 0b0101_1101,
    Bad = 0b0101_0101,
}

impl Identification {
    /// Build an Identification from Read ID bytes.
    pub fn from_bytes<SPI: Transfer<u8>, CS: OutputPin>(
        buf: &[u8],
    ) -> Result<Self, Error<SPI, CS>> {
        if buf.len() < 10 {
            return Err(Error::InvalidDevice);
        }

        if buf[0] != 0x0D {
            return Err(Error::InvalidDevice);
        }

        let known_good = buf[1] == KGD::Good as u8;

        let mut bytes = [0; 8];

        for (index, i) in buf[2..].iter().enumerate() {
            bytes[index + 2] = *i;
        }

        let eid = u64::from_be_bytes(bytes);

        Ok(Self {
            eid: eid,
            known_good_device: known_good,
        })
    }
}

#[allow(unused)] // TODO support more features
enum Opcode {
    /// Slow read at 33MHz
    Read = 0x03,
    /// Faster Read speed
    FastRead = 0x0B,
    /// Really fast read using QuadSPI. Not supported yet.
    FastReadQuad = 0xEB,
    /// Slow write at 33MHz
    Write = 0x02,
    /// Really fast write using QuadSPI. Not supported yet.
    QuadWrite = 0x38,
    /// Enter QuadSPI Mode. Not supported yet.
    EnterQuadMode = 0x35,
    /// Exit QuadSPI Mode. Not supported yet.
    ExitQuadMode = 0xF5,
    /// Enable the device to be reset
    ResetEnable = 0x66,
    /// Reset the device
    Reset = 0x99,
    /// Set the burst length
    SetBurstLength = 0xC0,
    /// Read 16-bit manufacturer ID and 48-bit device ID.
    ReadID = 0x9F,
}

/// Frequency is used to enforce the page bountry limitations and burst length at runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Freq {
    /// 33MHz. Can cross page boundries. No length limit
    ThreeThree,
    /// 84MHz. Can cross page boundries.
    EightyFour,
    /// Maximum 104MHz (3V3). Can't cross page boundries.
    OneZeroFour,
    /// Maximum 133MHz (3V3). Can't cross page boundries.
    OneThreeThree,
    /// Maximum 144MHz (1V8). Can't cross page boundries.
    OneFourFour,
}

/// Burst Length is used to enforce the various operations at runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BurstLength {
    /// No Limit under 33MHz. Can cross page boundries.
    None,
    /// 32B payload. Can't cross page boundries.
    ThirtyTwoByte,
    /// 1KB payload. Can cross page boundries.
    OneKByte,
}

/// Driver for ESP SPI Psuedo SRAM chips.
///
/// # Type Parameters
///
/// * **`SPI`**: The SPI master to which the flash chip is attached.
/// * **`CS`**: The **C**hip-**S**elect line attached to the `\CS`/`\CE` pin of
///   the flash chip.
/// * **`Frequency`**: The maximum frequency that the deivce is running at. Important for cross page access
/// * **`BurstLength`**: The maximum payload size.
#[derive(Debug)]
pub struct PSRAM<SPI: Transfer<u8>, CS: OutputPin> {
    spi: SPI,
    cs: CS,
    freq: Freq,
    burst_length: BurstLength,
}

impl<SPI: Transfer<u8>, CS: OutputPin> PSRAM<SPI, CS> {
    /// Creates a new PSRAM driver.
    ///
    /// # Parameters
    ///
    /// * **`spi`**: An SPI master. Must be configured to operate in the correct
    ///   mode for the device.
    /// * **`cs`**: The **C**hip-**S**elect Pin connected to the `\CS`/`\CE` pin
    ///   of the flash chip. Will be driven low when accessing the device.
    /// * **`freq`**: The maximum frequency that the deivce is running at. Important for cross page access
    /// * **`burst_length`**: The maximum payload size.
    pub fn init(
        spi: SPI,
        cs: CS,
        freq: Freq,
        burst_length: BurstLength,
    ) -> Result<Self, Error<SPI, CS>> {
        let mut this = Self {
            spi,
            cs,
            freq,
            burst_length,
        };

        if freq != Freq::ThreeThree {
            return Err(Error::InvalidMode);
        }

        //Set the burst_length now
        if burst_length == BurstLength::ThirtyTwoByte {
            //Send the command to the device
            let mut cmd_buf = [Opcode::SetBurstLength as u8];
            this.cs.try_set_low().map_err(Error::Gpio)?;
            let spi_result = this.spi.try_transfer(&mut cmd_buf);
            spi_result.map(|_| ()).map_err(Error::Spi)?;
            this.cs.try_set_high().map_err(Error::Gpio)?;
        }

        Ok(this)
    }

    fn command(&mut self, bytes: &mut [u8]) -> Result<(), Error<SPI, CS>> {
        // If the SPI transfer fails, make sure to disable CS anyways
        self.cs.try_set_low().map_err(Error::Gpio)?;
        let spi_result = self.spi.try_transfer(bytes).map_err(Error::Spi);
        self.cs.try_set_high().map_err(Error::Gpio)?;
        spi_result?;
        Ok(())
    }

    /// Reads the manufacturer/device identification.
    pub fn read_id(&mut self) -> Result<Identification, Error<SPI, CS>> {
        // Optimistically read 12 bytes, even though some identifiers will be shorter
        let mut buf: [u8; 14] = [0; 14];
        buf[0] = Opcode::ReadID as u8;
        self.command(&mut buf)?;

        // Skip buf[0..3] (SPI read response byte)
        Identification::from_bytes(&buf[4..])
    }

    /// Reset the Device
    fn reset(&mut self) -> Result<(), Error<SPI, CS>> {
        //Enable the Reset
        let mut cmd_buf = [Opcode::ResetEnable as u8];
        self.cs.try_set_low().map_err(Error::Gpio)?;
        let spi_result = self.spi.try_transfer(&mut cmd_buf);
        spi_result.map(|_| ()).map_err(Error::Spi)?;
        self.cs.try_set_high().map_err(Error::Gpio)?;

        //Trigger the reset
        let mut cmd_buf = [Opcode::Reset as u8];
        self.cs.try_set_low().map_err(Error::Gpio)?;
        let spi_result = self.spi.try_transfer(&mut cmd_buf);
        self.cs.try_set_high().map_err(Error::Gpio)?;
        spi_result.map(|_| ()).map_err(Error::Spi)
    }

    fn set_burst(&mut self, burst: BurstLength) -> Result<(), Error<SPI, CS>> {
        //Send the burst command if the new state is 32 and the old state is not 32
        // or the new state is not 32 and the old state is 32
        if (burst == BurstLength::ThirtyTwoByte && self.burst_length != BurstLength::ThirtyTwoByte)
            || (burst != BurstLength::ThirtyTwoByte
                && self.burst_length == BurstLength::ThirtyTwoByte)
        {
            //Send the command to the device
            let mut cmd_buf = [Opcode::SetBurstLength as u8];
            self.cs.try_set_low().map_err(Error::Gpio)?;
            let spi_result = self.spi.try_transfer(&mut cmd_buf);
            spi_result.map(|_| ()).map_err(Error::Spi)?;
            self.cs.try_set_high().map_err(Error::Gpio)?;
        }

        self.burst_length = burst;

        Ok(())
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> SingleWrite<u8, u32> for PSRAM<SPI, CS> {
    type Error = Error<SPI, CS>;
    fn try_write(&mut self, address: Address<u32>, word: u8) -> nb::Result<(), Self::Error> {
        self.try_write_slice(address, &mut [word])
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> MultiWrite<u8, u32> for PSRAM<SPI, CS> {
    type Error = Error<SPI, CS>;
    fn try_write_slice(
        &mut self,
        address: Address<u32>,
        buf: &mut [u8],
    ) -> nb::Result<(), Self::Error> {
        for (c, chunk) in buf.chunks_mut(256).enumerate() {
            let current_addr: u32 = (address.0 as usize + c * 256).try_into().unwrap();
            let mut cmd_buf = [
                Opcode::Write as u8,
                (current_addr >> 16) as u8,
                (current_addr >> 8) as u8,
                current_addr as u8,
            ];

            self.cs.try_set_low().map_err(Error::Gpio)?;
            let mut spi_result = self.spi.try_transfer(&mut cmd_buf);
            if spi_result.is_ok() {
                spi_result = self.spi.try_transfer(chunk);
            }
            self.cs.try_set_high().map_err(Error::Gpio)?;
            spi_result.map(|_| ()).map_err(Error::Spi)?;
        }
        Ok(())
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> SingleRead<u8, u32> for PSRAM<SPI, CS> {
    type Error = Error<SPI, CS>;
    fn try_read(&mut self, address: Address<u32>) -> nb::Result<u8, Self::Error> {
        let mut buf = [0];
        self.try_read_slice(address, &mut buf)?;
        Ok(buf[0])
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> MultiRead<u8, u32> for PSRAM<SPI, CS> {
    type Error = Error<SPI, CS>;
    fn try_read_slice(
        &mut self,
        address: Address<u32>,
        buf: &mut [u8],
    ) -> nb::Result<(), Self::Error> {
        let mut cmd_buf = [
            Opcode::Read as u8,
            (address.0 >> 16) as u8,
            (address.0 >> 8) as u8,
            address.0 as u8,
        ];

        self.cs.try_set_low().map_err(Error::Gpio)?;
        let mut spi_result = self.spi.try_transfer(&mut cmd_buf);
        if spi_result.is_ok() {
            spi_result = self.spi.try_transfer(buf);
        }
        self.cs.try_set_high().map_err(Error::Gpio)?;
        //use nb;
        spi_result.map(|_| ()).map_err(Error::Spi)?;
        Ok(())
    }
}

impl<SPI: Transfer<u8>, CS: OutputPin> StorageSize<u8, u32> for PSRAM<SPI, CS> {
    type Error = Error<SPI, CS>;

    fn try_start_address(&mut self) -> nb::Result<Address<u32>, Self::Error> {
        Ok(Address(0))
    }

    /// 64MB
    fn try_total_size(&mut self) -> nb::Result<AddressOffset<u32>, Self::Error> {
        Ok(AddressOffset(8388608))
    }

    /// 1KB
    fn try_page_size(&mut self) -> nb::Result<AddressOffset<u32>, Self::Error> {
        Ok(AddressOffset(1024))
    }
}

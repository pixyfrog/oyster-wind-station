use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};

pub const REG_VERSION: u8 = 0x42;
pub const EXPECTED_VERSION: u8 = 0x12;

#[derive(Debug)]
pub enum RadioError {
    InvalidVersion,
    SpiError(std::io::Error),
}

impl From<std::io::Error> for RadioError {
    fn from(e: std::io::Error) -> Self {
        RadioError::SpiError(e)
    }
}

pub fn get_version() -> Result<u8, RadioError> {
    let version = read_register(REG_VERSION)?;
    if version == EXPECTED_VERSION {
        Ok(version)
    } else {
        Err(RadioError::InvalidVersion)
    }
}

fn read_register(address: u8) -> Result<u8, RadioError> {
    // Open the SPI device (the "doorway" file to the hardware)
    let mut spi = Spidev::open("/dev/spidev1.0")?;

    // Configure: 8-bit words, 1 MHz clock, SPI mode 0 (what the SX1276 speaks)
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    // The transfer: send [address, dummy], receive [junk, answer]
    let tx = [address & 0x7F, 0x00];
    let mut rx = [0u8; 2];
    let mut transfer = SpidevTransfer::read_write(&tx, &mut rx);
    spi.transfer (&mut transfer)?;

    Ok(rx[1])
}



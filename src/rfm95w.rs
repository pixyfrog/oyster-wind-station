use spidev::{Spidev, SpidevOptions, SpidevTransfer, SpiModeFlags};
use std::time::{Duration, Instant};

pub const REG_FIFO: u8 = 0x00;
pub const REG_OP_MODE: u8 = 0x01;
pub const REG_FR_MSB: u8 = 0x06;
pub const REG_FR_MID: u8 = 0x07;
pub const REG_FR_LSB: u8 = 0x08;
pub const REG_FIFO_ADDR_PTR: u8 = 0x0D;
pub const REG_FIFO_TX_BASE_ADDR: u8 = 0x0E;
pub const REG_FIFO_RX_BASE_ADDR: u8 = 0x0F;
pub const REG_FIFO_RX_CURRENT_ADDR: u8 = 0x10;
pub const REG_IRQ_FLAGS_MASK: u8 = 0x11;
pub const REG_IRQ_FLAGS: u8 = 0x12;
pub const REG_RX_NB_BYTES: u8 = 0x13;
pub const REG_MODEM_CONFIG1: u8 = 0x1D;
pub const REG_MODEM_CONFIG2: u8 = 0x1E;
pub const REG_PREAMBLE_MSB: u8 = 0x20;
pub const REG_PREAMBLE_LSB: u8 = 0x21;
pub const REG_MODEM_CONFIG3: u8 = 0x26;
pub const REG_SYNC_WORD: u8 = 0x39;

pub const IRQ_RX_TIMEOUT: u8 = 0x80;
pub const IRQ_RX_DONE: u8 = 0x40;
pub const IRQ_PAYLOAD_CRC_ERROR: u8 = 0x20;
pub const IRQ_VALID_HEADER: u8 = 0x10;
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

pub struct Radio {
    spi: Spidev,
}

impl Radio {
    fn open() -> Result<Self, RadioError> {
        let mut spi = Spidev::open("/dev/spidev1.0")?;

        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();

        spi.configure(&options)?;

        Ok(Self {spi})
    }

    pub fn new_receiver() -> Result<Self, RadioError> {
        let mut radio = Self::open()?;

        let version = radio.read_register(REG_VERSION)?;
        if version != EXPECTED_VERSION {
            return Err(RadioError::InvalidVersion);
        }
        radio.init_lora_868_rx()?;
        Ok(radio)
    }
    pub fn read_register(&mut self, address: u8) -> Result<u8, RadioError> {
        let tx = [address & 0x7F, 0x00];
        let mut rx = [0u8; 2];
        let mut transfer = SpidevTransfer::read_write(&tx, &mut rx);
        self.spi.transfer(&mut transfer)?;
        Ok(rx[1])
    }


    fn write_register(&mut self, address: u8, value: u8) -> Result<(), RadioError> {
        let tx = [address | 0x80, value];
        let mut rx = [0u8; 2];
        let mut transfer = SpidevTransfer::read_write(&tx, &mut rx);
        self.spi.transfer(&mut transfer)?;
        Ok(())
    }
        fn read_fifo(&mut self, len: usize) -> Result<Vec<u8>, RadioError> {
        let tx = vec![0u8; len +1];
        let mut rx = vec![0u8; len +1];
        let mut transfer = SpidevTransfer::read_write(&tx, &mut rx);
        self.spi.transfer(&mut transfer)?;
        Ok(rx[1..].to_vec())
    }
    pub fn poll_receive(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, RadioError> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            let irq = self.read_register(REG_IRQ_FLAGS)?;

            if irq & IRQ_RX_DONE != 0 {
                let crc_error = (irq & IRQ_PAYLOAD_CRC_ERROR) != 0;
                let valid_header = (irq & IRQ_VALID_HEADER) != 0;
                if crc_error {
                println!("Dropping CRC-error packet irq=0x{:02X}", irq);
                self.write_register(REG_IRQ_FLAGS, 0xFF)?;
                continue;
                }
                let len = self.read_register(REG_RX_NB_BYTES)? as usize;
                let current_addr = self.read_register(REG_FIFO_RX_CURRENT_ADDR)?;
                self.write_register(REG_FIFO_ADDR_PTR, current_addr)?;
                let payload = self.read_fifo(len)?;

                println!(
                    "RX irq=0x{:02X} valid_header={} crc_error={} len={}", 
                    irq, valid_header, crc_error,len
                );
                self.write_register(REG_IRQ_FLAGS, 0xFF)?;
                return Ok(Some(payload));
            }
            if irq & IRQ_RX_TIMEOUT !=0 {
                println!("RX timeout irq=0x{:02X}", irq);
                self.write_register(REG_IRQ_FLAGS, 0xFF)?;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Ok(None)
    }
        fn set_frequency_868(&mut self) -> Result<(), RadioError> {
        let freq_hz: u64 = 868_000_000;
        let frf = (freq_hz * 524_288) / 32_000_000;

        self.write_register(REG_FR_MSB, ((frf >> 16) & 0xFF) as u8)?;
        self.write_register(REG_FR_MID, ((frf >> 8) & 0xFF) as u8)?;
        self.write_register(REG_FR_LSB, (frf & 0xFF) as u8)?;

        Ok(())
    }

    fn init_lora_868_rx(&mut self) -> Result<(), RadioError> {
        // Sleep, then switch from FSK mode to LoRa mode.
        self.write_register(REG_OP_MODE, 0x00)?;
        std::thread::sleep(Duration::from_millis(10));
        self.write_register(REG_OP_MODE, 0x80)?;
        std::thread::sleep(Duration::from_millis(10));

        // Standby while configuring.
        self.write_register(REG_OP_MODE, 0x81)?;

        self.set_frequency_868()?;

        // Match the T-Beam: BW 125 kHz, CR 4/5, explicit header.
        self.write_register(REG_MODEM_CONFIG1, 0x72)?;

        // SF7, normal packet mode, payload CRC on.
        self.write_register(REG_MODEM_CONFIG2, 0x74)?;

        // AGC auto on; low-data-rate optimize off for SF7/BW125.
        self.write_register(REG_MODEM_CONFIG3, 0x04)?;

        // Preamble length 8.
        self.write_register(REG_PREAMBLE_MSB, 0x00)?;
        self.write_register(REG_PREAMBLE_LSB, 0x08)?;

        // Match the T-Beam sync word.
        self.write_register(REG_SYNC_WORD, 0x12)?;

        // FIFO layout: RX base at 0x00, TX base unused here.
        self.write_register(REG_FIFO_TX_BASE_ADDR, 0x80)?;
        self.write_register(REG_FIFO_RX_BASE_ADDR, 0x00)?;

        // We poll IRQ flags; no DIO0 wiring yet.
        self.write_register(REG_IRQ_FLAGS_MASK, 0x00)?;
        self.write_register(REG_IRQ_FLAGS, 0xFF)?;

        // LoRa RX continuous mode.
        self.write_register(REG_OP_MODE, 0x85)?;

        Ok(())
    }
}

pub fn get_version() -> Result<u8, RadioError> {
    let mut radio = Radio::open()?;
    let version = radio.read_register(REG_VERSION)?;

    if version == EXPECTED_VERSION {
        Ok(version)
    } else {
        Err(RadioError::InvalidVersion)
    }

}





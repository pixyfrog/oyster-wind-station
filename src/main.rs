use spidev::{Spidev, SpidevOptions, SpidevTransfer};
use std::io;

fn main() -> io::Result<()> {
    let mut spi = Spidev::open("/dev/spidev1.0")?;
    
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(spidev::SpiModeFlags::SPI_MODE_0)
        .build();
    
    spi.configure(&options)?;
    
    // Read RFM95W version register (0x42) at address 0x42
    let tx_buf = [0x42, 0x00];
    let mut rx_buf = [0; 2];
    let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);
    spi.transfer(&mut transfer)?;
    
    println!("Version register: 0x{:02X}", rx_buf[1]);
    
    Ok(())
}

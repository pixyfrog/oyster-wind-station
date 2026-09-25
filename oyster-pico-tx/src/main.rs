#![no_std]
#![no_main]

use bsp::entry;
use panic_halt as _;
use rp_pico as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    fugit::RateExtU32,
    gpio::{FunctionSpi, Pins},
    pac,
    sio::Sio,
    spi::Spi,
    watchdog::Watchdog,
};

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

// ---------------- SX1276 registers we use ---------------
const PACKET_VERSION: u8 = 0x02;
const SUB_SAMPLES_PER_WINDOW: u32 = 6; // — 6 for bench, 200 for real (600 s / 3 s)
const WINDOW_S: u16 = (SUB_SAMPLES_PER_WINDOW * 3) as u16; // 3s per sub_sample
const K_CM_PER_PULSE: u32 = 240; // placeholder - calibrate on site
const REG_OP_MODE: u8 = 0x01;
const REG_FRF_MSB: u8 = 0x06;
const REG_FRF_MID: u8 = 0x07;
const REG_FRF_LSB: u8 = 0x08;
const REG_PA_CONFIG: u8 = 0x09;
const REG_FIFO_ADDR_PTR: u8 = 0x0D;
const REG_FIFO_TX_BASE: u8 = 0x0E;
const REG_IRQ_FLAGS: u8 = 0x12;
const REG_MODEM_CONFIG1: u8 = 0x1D;
const REG_MODEM_CONFIG2: u8 = 0x1E;
const REG_PREAMBLE_MSB: u8 = 0x20;
const REG_PREAMBLE_LSB: u8 = 0x21;
const REG_PAYLOAD_LENGTH: u8 = 0x22;
const REG_MODEM_CONFIG3: u8 = 0x26;
const REG_SYNC_WORD: u8 = 0x39;

// RegOpMode values: 0x80 = LoRa mode + sleep, then +standby / +TX
const MODE_SLEEP: u8 = 0x80;
const MODE_STANDBY: u8 = 0x81;
const MODE_TX: u8 = 0x83;
const IRQ_TX_DONE: u8 = 0x08;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // ---- SPI0 to the radio (same wiring as the version-check step) ----
    let miso = pins.gpio16.into_function::<FunctionSpi>();
    let sck = pins.gpio18.into_function::<FunctionSpi>();
    let mosi = pins.gpio19.into_function::<FunctionSpi>();

    let spi = Spi::<_, _, _, 8>::new(pac.SPI0, (mosi, miso, sck));
    let mut spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        1_000_000u32.Hz(),
        embedded_hal::spi::MODE_0,
    );

    let mut cs = pins.gpio17.into_push_pull_output();
    let mut rst = pins.gpio20.into_push_pull_output();
    // ---- anemometer pulse input ----
    let mut anemometer = pins.gpio22.into_pull_up_input();

    // ---- USB serial ----
    let usb_bus = UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .device_class(USB_CLASS_CDC)
        .build();

    // Reset pulse, then configure the radio for LoRa TX at 868 MHz.
    cs.set_high().unwrap();
    rst.set_low().unwrap();
    delay.delay_ms(1);
    rst.set_high().unwrap();
    delay.delay_ms(10);
    radio_init_tx(&mut spi, &mut cs);

    let mut sequence: u16 = 0;
    let mut window = Window::new();
    let mut sub_samples: u32 = 0;
    loop {
        usb_dev.poll(&mut [&mut serial]);

        // Measure for 3 seconds, then report.
        let mut pulses: u32 =0;
        for _ in 0..300{
            usb_dev.poll(&mut [&mut serial]);
            pulses += count_pulses_for(&mut anemometer, &mut delay, 10);
        }
        print_u16(&mut serial, b"pulses=", pulses as u16);
        
        let speed = speed_cms(pulses, 3000);
        window.push(speed);
        sub_samples += 1;
        print_u16(&mut serial,b"speed_cms=", speed as u16);
        if sub_samples >= SUB_SAMPLES_PER_WINDOW {
            let (avg, gust, lull) = window.finish();         
            print_u16(&mut serial, b"avg=", avg as u16);
            print_u16(&mut serial, b"gust=", gust as u16);
            print_u16(&mut serial, b"lull=", lull as u16);
            print_u16(&mut serial, b"speed_cms=", speed as u16);
        
            let packet = build_packet(
                (avg / 10) as u16,
                (gust / 10) as u16,
                (lull / 10) as u16,
                3700,
                sequence,
            );
            radio_send(&mut spi, &mut cs, &mut delay, &packet);
            print_u16(&mut serial, b"TX seq=", sequence);
            sequence = sequence.wrapping_add(1);

            window = Window::new();
            sub_samples = 0;
        }
    }

}

// ---------------- radio drivers ----------------
// `impl SpiBus<u8>` = "any type that fulfils the SpiBus contract".
// You know traits as shared contracts — this just uses one as a
// parameter type so we don't have to spell out the HAL's long type name.

fn write_register(spi: &mut impl SpiBus<u8>, cs: &mut impl OutputPin, address: u8, value: u8) {
    let mut buf = [address | 0x80, value]; // MSB set = write
    cs.set_low().unwrap();
    let _ = spi.transfer_in_place(&mut buf);
    cs.set_high().unwrap();
}
fn pulse_level(pin: &mut impl InputPin) -> u16 {
    if pin.is_high().unwrap() {1} else {0} 
}

fn read_register(spi: &mut impl SpiBus<u8>, cs: &mut impl OutputPin, address: u8) -> u8 {
    let mut buf = [address & 0x7F, 0x00]; // MSB clear = read
    cs.set_low().unwrap();
    let _ = spi.transfer_in_place(&mut buf);
    cs.set_high().unwrap();
    buf[1]
}
// whatch teh pulse line for 'gate_ms' millseconds and count falling edges.
 // The line stays HIGH through the pull-up; each pulse pulls it LOW; so a 
 // HIGH -> LOW change is one pulse.
fn count_pulses_for(
    pin: &mut impl InputPin,
    delay: &mut cortex_m::delay::Delay,
    gate_ms: u32,
) -> u32 {
    let mut previous = pin.is_high().unwrap();
    let mut pulses: u32 = 0;

    for _ in 0..gate_ms {
        delay.delay_ms(1);
        let now = pin.is_high().unwrap();
        if previous && !now {
            pulses += 1;
        }
        previous = now;
    }

    pulses
}
fn speed_cms(pulses: u32, window_ms: u32) -> u32 {
    let window_s = window_ms / 1000;
    if window_s == 0 {
        return 0;
    }
    (pulses * K_CM_PER_PULSE) / window_s
}
//One measurement window, hold the numbers has subsample arrives
struct Window {
    sum_cms: u32,
    sample: u32 ,
    gust_cms: Option<u32>,
    lull_cms: Option<u32>,
}

impl Window {
    fn new() -> Self {
        Window {
        sum_cms : 0,
        sample : 0,
        gust_cms : None,
        lull_cms : None,
    }
    }
    fn push(&mut self, speed_cms: u32) {
        self.sum_cms += speed_cms;
        self.sample += 1;

        if self.gust_cms.is_none() || speed_cms > self.gust_cms.unwrap() {
            self.gust_cms = Some(speed_cms);
        }
        if self.lull_cms.is_none() || speed_cms < self.lull_cms.unwrap() {
            self.lull_cms = Some(speed_cms);
        }
    } 

    // return average, gust, lull, all in cms 
    fn finish(&self) -> (u32, u32, u32) {
        (
            if self.sample == 0 {0} else {self.sum_cms/self.sample},
            self.gust_cms.unwrap_or(0),
            self.lull_cms.unwrap_or(0),
            )
    }
}

// Burst write to the FIFO: CS stays low across address byte + all data bytes.
fn write_fifo(spi: &mut impl SpiBus<u8>, cs: &mut impl OutputPin, data: &[u8]) {
    cs.set_low().unwrap();
    let _ = spi.write(&[0x80]); // register 0x00 (FIFO), write mode
    let _ = spi.write(data);
    cs.set_high().unwrap();
}

fn radio_init_tx(spi: &mut impl SpiBus<u8>, cs: &mut impl OutputPin) {
    write_register(spi, cs, REG_OP_MODE, MODE_SLEEP); // LoRa mode can only be set in sleep
    // 868 MHz: frf = 868e6 * 2^19 / 32e6 = 0xD90000 (same math as the Orange Pi)
    write_register(spi, cs, REG_FRF_MSB, 0xD9);
    write_register(spi, cs, REG_FRF_MID, 0x00);
    write_register(spi, cs, REG_FRF_LSB, 0x00);

    write_register(spi, cs, REG_MODEM_CONFIG1, 0x72); // BW 125 kHz, CR 4/5, explicit header
    write_register(spi, cs, REG_MODEM_CONFIG2, 0x74); // SF7, payload CRC on
    write_register(spi, cs, REG_MODEM_CONFIG3, 0x04); // AGC auto
    write_register(spi, cs, REG_PREAMBLE_MSB, 0x00);
    write_register(spi, cs, REG_PREAMBLE_LSB, 0x08);  // preamble = 8 symbols
    write_register(spi, cs, REG_SYNC_WORD, 0x12);
    write_register(spi, cs, REG_FIFO_TX_BASE, 0x00);
    write_register(spi, cs, REG_PA_CONFIG, 0x8F);     // PA_BOOST, modest ~11 dBm for bench tests

    write_register(spi, cs, REG_OP_MODE, MODE_STANDBY);
}

fn radio_send(
    spi: &mut impl SpiBus<u8>,
    cs: &mut impl OutputPin,
    delay: &mut cortex_m::delay::Delay,
    payload: &[u8],
) {
    write_register(spi, cs, REG_OP_MODE, MODE_STANDBY);
    write_register(spi, cs, REG_FIFO_ADDR_PTR, 0x00); // write at TX base address
    write_register(spi, cs, REG_PAYLOAD_LENGTH, payload.len() as u8);
    write_fifo(spi, cs, payload);
    write_register(spi, cs, REG_OP_MODE, MODE_TX);    // airtime at SF7/125kHz ≈ 40 ms
    for _ in 0..100 {
        if read_register(spi, cs, REG_IRQ_FLAGS) & IRQ_TX_DONE != 0 {
            break;
        }
        delay.delay_ms(1);
    }

    write_register(spi, cs, REG_IRQ_FLAGS, 0xFF);     // clear all IRQ flags
    write_register(spi, cs, REG_OP_MODE, MODE_SLEEP); // radio naps between packets
}

// ---------------- packet (same format as packet.rs) ----------------

fn crc8_sum(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for b in data {
        crc = crc.wrapping_add(*b);
    }
    crc
}
fn build_packet(
    avg_01: u16,
    gust_01: u16,
    lull_01: u16,
    battery_mv: u16,
    sequence: u16,
    ) -> [u8; 18]{
    let mut p = [0u8; 18];
    p[0] = 0xAA;            // magic
    p[1] = PACKET_VERSION;  // 0x02
    p[2] = 1;               // node id
    p[3] = 0;               // flags (unused for now)
    p[3] = (avg_01 >> 8) as u8; // wind_avg, big-endian
    p[4] = (avg_01 & 0xFF) as u8;
    p[5] = (gust_01 >> 8) as u8;
    p[6] = (gust_01 >> 8) as u8;
    p[7] = (gust_01 & 0xFF) as u8;
    p[8] = (lull_01 >> 8) as u8;
    p[9] = (lull_01 & 0xFF) as u8;
    p[10] = (battery_mv >> 8) as u8;
    p[11] = (battery_mv & 0xFF) as u8;
    p[12] = (sequence >> 8) as u8;
    p[13] = (sequence & 0xFF) as u8;
    p[14] = (WINDOW_S >> 8) as u8;
    p[15] = (WINDOW_S & 0xFF) as u8;
    p[16] = SUB_SAMPLES_PER_WINDOW as u8;
    p[17] = crc8_sum(&p[0..17]);
    p
}


// ---------------- serial printing helpers ----------------

fn write_all(serial: &mut SerialPort<'_, bsp::hal::usb::UsbBus>, mut data: &[u8]) {
    while !data.is_empty() {
        match serial.write(data) {
            Ok(n) if n > 0 => data = &data[n..],
            _ => break,
        }
    }
}

// Decimal print without any formatting library: peel digits off the end
// (v % 10) into a small array, then send the filled slice.
fn print_u16(serial: &mut SerialPort<'_, bsp::hal::usb::UsbBus>, label: &[u8], mut v: u16) {
    let mut digits = [b'0'; 5];
    let mut i = 5;
    if v == 0 {
        i = 4;
    }
    while v > 0 {
        i -= 1;
        digits[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    write_all(serial, label);
    write_all(serial, &digits[i..]);
    write_all(serial, b"\r\n");
}

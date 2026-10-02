#![no_std]
#![no_main]

use bsp::entry;
use panic_halt as _;
use rp_pico as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock, ClockGate, ClockSource, ClocksManager, StoppableClock},
    fugit::RateExtU32,
    gpio::{FunctionSpi, Pins},
    pac,
    rosc::RingOscillator,
    sio::Sio,
    spi::Spi,
    watchdog::Watchdog,
    xosc::setup_xosc_blocking,
};

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;
use bsp::hal::fugit::ExtU32;
use bsp::hal::timer::{Alarm, Alarm0, Timer};

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use core::cell::RefCell;
use portable_atomic:: {AtomicBool, AtomicU32, Ordering};
use bsp::hal::gpio::{FunctionSio, Interrupt, Pin, PullUp, SioInput};
use bsp::hal::gpio::bank0::Gpio22;
use bsp::hal::pac::interrupt;
use critical_section::Mutex;

// ---------------- SX1276 regists we use ---------------
const REG_PA_DAC: u8 = 0x4D;
const PACKET_VERSION: u8 = 0x02;
// true  = bench build: USB serviced. Requires USE_PLL_CLOCK, which is what
//         makes clk_usb 48 MHz so the USB controller can be touched at all.
// false = field build: no USB setup at all. The USB controller is left in
//         reset, because with clk_usb stopped any access to its registers
//         stalls the AHB and hangs the core before the radio is driven.
const DEBUG_USB: bool = false;
// true  = full clock tree (PLL_SYS 125 MHz + PLL_USB 48 MHz).
// false = crystal only, no PLLs — they are the dominant current cost.
const USE_PLL_CLOCK: bool = false;
const SUB_SAMPLES_PER_WINDOW: u32 = 6; // — 6 for bench, 200 for real (600 s / 3 s)
const WINDOW_S: u16 = (SUB_SAMPLES_PER_WINDOW * 3) as u16; // 3s per sub_sample
const K_CM_PER_PULSE: u32 = 5; // calibrate on site
const LORA_SF: u8 = 9; // frozen range is 7 .. 9
const LORA_MODEM_CONFIG2: u8 = (LORA_SF << 4 ) | 0x04;  //SF in bit 7..4, paylod CRC on
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

//The ISR adds to this; the mian loop drais it with swap(0).
static PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

//The pin must be reachable from the ISR to clear the edge, so it lives
// in a critical-section-guarded slot rather than a local Variable.
type AnemometerPin = Pin<Gpio22, FunctionSio<SioInput>, PullUp>;
static ANEMOMETER: Mutex<RefCell<Option<AnemometerPin>>> = Mutex::new(RefCell::new(None));
static ALARM: Mutex<RefCell<Option<Alarm0>>> = Mutex::new(RefCell::new(None));
static ALARM_FIRED: AtomicBool = AtomicBool::new(false);

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    // Bench build: full clock tree, because USB needs PLL_USB at 48 MHz.
    // Field build: crystal only, no PLLs — they are the dominant current cost
    // (PLL_SYS measured ≈ 18 mA, PLL_USB ≈ 3 mA, the crystal ≈ 0.7 mA).
    let mut clocks = if USE_PLL_CLOCK { 
        init_clocks_and_plls(
            rp_pico::XOSC_CRYSTAL_FREQ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap()
        } else {
            // clk_ref MUST stay on the crystal: the watchdog tick, and so the TIMER's
            // 1 µs tick, derives from it. The ROSC is too imprecise to keep time.
            let xosc = setup_xosc_blocking(pac.XOSC, rp_pico::XOSC_CRYSTAL_FREQ.Hz()).unwrap();
            watchdog.enable_tick_generation((rp_pico::XOSC_CRYSTAL_FREQ / 1_000_000) as u8);
            let mut clocks = ClocksManager::new(pac.CLOCKS);
            clocks.reference_clock.configure_clock(&xosc, xosc.get_freq()).unwrap();
            // clk_ref stays on the crystal at 12 MHz for accurate timing (the watchdog
            // tick and the TIMER's 1 µs tick). clk_sys runs at half that: the domains
            // that must stay clocked through wfi (sys_io, sys_timer) then burn half the
            // dynamic power, while SPI (1 MHz) and the systick Delay stay valid.
            clocks.system_clock.configure_clock(&xosc, (rp_pico::XOSC_CRYSTAL_FREQ / 2).Hz()).unwrap();
            clocks.peripheral_clock.configure_clock(&clocks.system_clock, clocks.system_clock.freq()).unwrap();

            // Nothing but the crystal may be left running. This mirrors pico-sdk
            // sleep_run_from_dormant_source: stop the USB and ADC clocks, power down
            // both PLLs, and stop the ring oscillator. A BOOTSEL flash can leave the
            // ROM's PLL_USB alive across the jump into the app, and a live PLL costs
            // its full current even when no clock is routed from it. The PLL power
            // register 0x2d is PD | DSMPD | POSTDIVPD | VCOPD from the SDK's pll_deinit.
            clocks.usb_clock.disable();
            clocks.adc_clock.disable();
            pac.PLL_SYS.pwr().write(|w| unsafe { w.bits(0x2d) });
            pac.PLL_USB.pwr().write(|w| unsafe { w.bits(0x2d) });
            RingOscillator::new(pac.ROSC).initialize().disable();
            clocks
        };

    // In the field build, gate every peripheral clock while the core is in wfi,
    // keeping only the two wake sources: the TIMER alarm (sys_timer) and the GP22
    // anemometer edge (sys_io + sys_pads). pico-sdk's sleep_goto_sleep_for keeps
    // only the system timer; we keep IO as well because pulses are counted in wfi.
    // wake_en (all ones) restores every clock on wake, so SPI/radio still work.
    if !DEBUG_USB {
        let mut gate = ClockGate::default();
        gate.set_sys_timer(true);
        gate.set_sys_io(true);
        gate.set_sys_pads(true);
        clocks.configure_sleep_enable(gate);
    }

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
    anemometer.set_interrupt_enabled(Interrupt::EdgeLow, true);
    critical_section::with(|cs| ANEMOMETER.borrow(cs).replace(Some(anemometer)));
    unsafe { pac::NVIC::unmask(pac::Interrupt::IO_IRQ_BANK0) };

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut alarm = timer.alarm_0().unwrap();
    alarm.enable_interrupt();
    critical_section::with(|cs| ALARM.borrow(cs).replace(Some(alarm)));
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER_IRQ_0) };
    // ---- USB serial (bench build only) ----
    // Skipping this entirely when DEBUG_USB = false is not just a current saving:
    // the field clock tree never starts PLL_USB, so clk_usb is stopped, and the
    // register/DPRAM writes below would stall the bus and hang the core.
    let usb_bus = if DEBUG_USB {
        Some(UsbBusAllocator::new(bsp::hal::usb::UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        )))
    } else {
        None
    };
    let mut serial = usb_bus.as_ref().map(SerialPort::new);
    let mut usb_dev = usb_bus.as_ref().map(|bus| {
        UsbDeviceBuilder::new(bus, UsbVidPid(0x16c0, 0x27dd))
            .device_class(USB_CLASS_CDC)
            .build()
    });

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
        if DEBUG_USB {
            if let (Some(dev), Some(ser)) = (usb_dev.as_mut(), serial.as_mut()) {
                dev.poll(&mut [ser]);
            }
        }

        // Measure for 3 seconds, then report.
        critical_section::with (|cs| {
            let mut slot = ALARM.borrow(cs).borrow_mut();
            if let Some(a) = slot.as_mut() {
                a.schedule(3_000_000u32.micros()).unwrap();
            }
        });
        
        while !ALARM_FIRED.load(Ordering::Relaxed) {
            if DEBUG_USB {
                if let (Some(dev), Some(ser)) = (usb_dev.as_mut(), serial.as_mut()) {
                    dev.poll(&mut [ser]);
                }
                delay.delay_ms(1);
            } else {
                cortex_m::asm::wfi();
            }
        }
        ALARM_FIRED.store(false, Ordering::Relaxed);
        let pulses = PULSE_COUNT.swap(0, Ordering::Relaxed);
        if DEBUG_USB {
            if let Some(ser) = serial.as_mut() {
                print_u16(ser, b"pulses=", pulses as u16);
            }
        }
        
        let speed = speed_cms(pulses, 3000);
        window.push(speed);
        sub_samples += 1;
        if DEBUG_USB {
            if let Some(ser) = serial.as_mut() {
                print_u16(ser, b"speed_cms=", speed as u16);
            }
        }

        if sub_samples >= SUB_SAMPLES_PER_WINDOW {
            let (avg, gust, lull) = window.finish();

            if DEBUG_USB {
                if let Some(ser) = serial.as_mut() {
                    print_u16(ser, b"avg=", avg as u16);
                    print_u16(ser, b"gust=", gust as u16);
                    print_u16(ser, b"lull=", lull as u16);
                    print_u16(ser, b"speed_cms=", speed as u16);
                }
            }

            let packet = build_packet(
                (avg / 10) as u16,
                (gust / 10) as u16,
                (lull / 10) as u16,
                3700,
                sequence,
            );
            radio_send(&mut spi, &mut cs, &mut delay, &packet);

            if DEBUG_USB {
                if let Some(ser) = serial.as_mut() {
                    print_u16(ser, b"TX seq=", sequence);
                }
            }
            sequence = sequence.wrapping_add(1);

            window = Window::new();
            sub_samples = 0;
        }
    }
}
#[pac::interrupt]
fn IO_IRQ_BANK0() {
    // Clearl the edge that fired. Miss this and the interrupt re-frires forever.
    critical_section::with(|cs| {
        let mut slot = ANEMOMETER.borrow(cs).borrow_mut();
            if let Some(pin) = slot.as_mut() {
            pin.clear_interrupt(Interrupt::EdgeLow);
        }
    });
    PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
}
#[interrupt]
fn TIMER_IRQ_0() {
    critical_section::with(|cs| {
        let mut slot = ALARM.borrow(cs).borrow_mut();
        if let Some(a) = slot.as_mut() {
            a.clear_interrupt();
        }
    });
    ALARM_FIRED.store(true, Ordering::Relaxed);
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
    write_register(spi, cs, REG_MODEM_CONFIG2, LORA_MODEM_CONFIG2); // SF9, payload CRC on
    write_register(spi, cs, REG_MODEM_CONFIG3, 0x04); // AGC auto
    write_register(spi, cs, REG_PREAMBLE_MSB, 0x00);
    write_register(spi, cs, REG_PREAMBLE_LSB, 0x08);  // preamble = 8 symbols
    write_register(spi, cs, REG_SYNC_WORD, 0x12);
    write_register(spi, cs, REG_FIFO_TX_BASE, 0x00);
    write_register(spi, cs, REG_PA_DAC, 0x87); // enable teh +20 dBfm PAG_BOOST mode
    write_register(spi, cs, REG_PA_CONFIG, 0xFF);  // PA_BOOST, ceiling 7, trim 15
    // Rest in SLEEP, not STANDBY: between init and the first radio_send (one whole
    // measurement window) the module would otherwise idle in standby at ~1.5 mA.
    write_register(spi, cs, REG_OP_MODE, MODE_SLEEP);
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
    write_register(spi, cs, REG_OP_MODE, MODE_TX);  
    for _ in 0..500 {
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
    p[4] = (avg_01 >> 8) as u8; // wind_avg, big-endian
    p[5] = (avg_01 & 0xFF) as u8;
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

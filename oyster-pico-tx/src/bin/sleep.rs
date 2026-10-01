#![no_std]
#![no_main]

use bsp::entry;
use panic_halt as _;
use rp_pico as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock, ClockSource, ClocksManager},
    fugit::RateExtU32,
    gpio::{FunctionSPI, Pins},
    pac,
    sio::Sio,
    spi::Spi,
    watchdog::Watchdog,
    xosc::setup_xosc_blocking,
};
use embedded_hal::digital::OutputPin;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    pac.CLOCKS.sleep_en0().write(|w| unsafe { w.bits(0)});
    pac.CLOCKS.sleep_en1().write(|w| unsafe { w.bits(0)});
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // --- low-sleep: clk_ref/clk_sys on the ROSC, no PLLs, XOSC stopped ---
    let xosc = setup_xosc_blocking(pac.XOSC, rp_pico::XOSC_CRYSTAL_FREQ.Hz()).unwrap();
    watchdog.enable_tick_generation((rp_pico::XOSC_CRYSTAL_FREQ / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let rosc = RingOscillator::new(pac.ROSC).initialize();
    clocks.reference_clock.configure_clock(&rosc, rosc.operating_frequency()).unwrap();
    clocks.system_clock.configure_clock(&rosc, rosc.operating_frequency()).unwrap();
    clocks.peripheral_clock.configure_clock(&clocks.system_clock,
                                            clocks.system_clock.freq()).unwrap();

    // clk_ref/clk_sys now run from the ROSC, so stop the crystal.
    let _ = xosc.disable();

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // GP15: 3.3 V here means the loop below is halted.
    let mut probe = pins.gpio15.into_push_pull_output();

    loop {
        probe.set_high().unwrap();
        cortex_m::asm::wfi();
        probe.set_low().unwrap();
    }
}





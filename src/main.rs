#![no_std]
#![no_main]

// Hardware Abstraction Layer
use rp_pico::hal;
// periheral access crate
use hal::pac;

// on panic, just halt
use panic_halt as _;

// traits
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};

// Entry point to our bare-metal application.
// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
// as soon as all global variables are initialised.
#[hal::entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    // The single-cycle I/O block controls our GPIO pins
    let sio = hal::Sio::new(pac.SIO);

    // Set the pins to their default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output();

    loop {
        let _ = led_pin.set_high();
    }
}

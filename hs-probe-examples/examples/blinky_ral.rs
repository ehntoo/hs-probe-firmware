#![no_std]
#![no_main]

use cortex_m_rt::entry;
use hs_probe_bsp::gpio::GPIO;
use hs_probe_bsp::rcc::{CoreFrequency, RCC};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let rcc = RCC::new(stm32ral::rcc::RCC::take().unwrap());
    unsafe {
        rcc.setup(CoreFrequency::F48MHz);
    }

    let gpioc = GPIO::new(stm32ral::gpio::GPIOC::take().unwrap());
    let led = gpioc.pin(10);
    // Open-drain output to LED (active low).
    led.set_high()
        .set_otype_opendrain()
        .set_ospeed_low()
        .set_mode_output();

    rprintln!("Starting blinky...");

    loop {
        cortex_m::asm::delay(16_000_000);
        led.set_low();

        cortex_m::asm::delay(16_000_000);
        led.set_high();
    }
}

#![no_std]
#![no_main]

use cortex_m_rt::{entry, pre_init};
use git_version::git_version;
pub use hs_probe_bsp as bsp;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use stm32_device_signature::device_id_hex;

const GIT_VERSION: &str = git_version!();

mod app;
mod dap;
mod swd;
mod usb;

#[pre_init]
unsafe fn pre_init() {
    // Check if we should jump to system bootloader.
    //
    // When we receive the BOOTLOAD command over USB,
    // we write a flag to a static and reset the chip,
    // and `bootload::check()` will jump to the system
    // memory bootloader if the flag is present.
    //
    // It must be called from pre_init as otherwise the
    // flag is overwritten when statics are initialised.
    // bsp::bootload::check();
}

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let rcc = bsp::rcc::RCC::new(stm32ral::rcc::RCC::take().unwrap());

    let usb_phy = stm32ral::usbphyc::USBPHYC::take().unwrap();
    let usb_global = stm32ral::otg_hs_global::OTG_HS_GLOBAL::take().unwrap();
    let usb_device = stm32ral::otg_hs_device::OTG_HS_DEVICE::take().unwrap();
    let usb_pwrclk = stm32ral::otg_hs_pwrclk::OTG_HS_PWRCLK::take().unwrap();
    let mut usb = crate::usb::USB::new(usb_phy, usb_global, usb_device, usb_pwrclk);

    let dma = bsp::dma::DMA::new(
        stm32ral::dma::DMA1::take().unwrap(),
        stm32ral::dma::DMA2::take().unwrap(),
    );
    let spi5 = bsp::spi::SPI::new(stm32ral::spi::SPI5::take().unwrap());
    let mut uart5 = bsp::uart::UART::new(stm32ral::usart::UART5::take().unwrap(), &dma);

    let gpioa = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOA::take().unwrap());
    let gpiob = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOB::take().unwrap());
    let gpioc = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOC::take().unwrap());
    let gpiod = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOD::take().unwrap());
    let gpioe = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOE::take().unwrap());
    let gpiof = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOF::take().unwrap());
    let gpiog = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOG::take().unwrap());
    let gpioh = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOH::take().unwrap());
    let gpioi = bsp::gpio::GPIO::new(stm32ral::gpio::GPIOI::take().unwrap());

    // spi1 -> spi5
    // usart1 -> uart5

    let pins = bsp::gpio::Pins {
        led_red: gpioa.pin(10),
        led_green: gpiob.pin(0),
        led_blue: gpioe.pin(0),
        tvcc_en: gpioe.pin(2),
        reset: gpioa.pin(6),
        gnd_detect: gpiog.pin(5),
        usart5_rx: gpiod.pin(2),

        usart6_rx: gpiog.pin(9),
        usart6_tx: gpiog.pin(14),


        // pf9 tms
        // pb2 tck

        // pd2 swo uart5 rx
        // pb5 spi1 mosi, spi3 mosi
        // pc7 usar6 rx

        // pb2, ph6 clk

        // pa1
        // pf8 spi5_miso, QUADSPI_BK1_IO0

        spi5_clk: gpioh.pin(6),
        spi5_miso: gpioh.pin(7),
        spi5_mosi: gpiof.pin(9),

        spi2_clk: gpioi.pin(1),
        spi2_miso: gpioi.pin(2),
        spi2_mosi: gpioi.pin(3),

        usb_dm: gpiob.pin(14),
        usb_dp: gpiob.pin(15),
        usb_sel: gpiob.pin(10),
    };

    let swd = swd::SWD::new(&spi5, &pins);
    let mut dap = dap::DAP::new(swd, &mut uart5, &pins);

    // Create App instance with the HAL instances
    let mut app = app::App::new(&rcc, &dma, &pins, &spi5, &mut usb, &mut dap);

    rprintln!("Starting...");

    // Initialise application, including system peripherals
    unsafe {
        app.setup(device_id_hex())
    };

    loop {
        // Process events
        app.poll();
    }
}

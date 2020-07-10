// Copyright 2019-2020 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use crate::{dap, hal};
use num_enum::TryFromPrimitive;

pub struct App<'a> {
    flash: &'a bsp::flash::Flash,
    rcc: &'a bsp::rcc::RCC,
    nvic: &'a bsp::nvic::NVIC,
    dma: &'a bsp::dma::DMA,
    pins: &'a bsp::gpio::Pins<'a>,
    spi: &'a bsp::spi::SPI,
    usb: &'a mut bsp::usb::USB,
    dap: &'a mut dap::DAP<'a>,
}

impl<'a> App<'a> {
    pub fn new(
        flash: &'a bsp::flash::Flash,
        rcc: &'a bsp::rcc::RCC,
        nvic: &'a bsp::nvic::NVIC,
        dma: &'a bsp::dma::DMA,
        pins: &'a bsp::gpio::Pins<'a>,
        spi: &'a bsp::spi::SPI,
        usb: &'a mut bsp::usb::USB,
        dap: &'a mut dap::DAP<'a>,
    ) -> Self {
        App {
            flash,
            rcc,
            nvic,
            dma,
            pins,
            spi,
            usb,
            dap,
        }
    }

    pub fn setup(&mut self) {
        // Configure flash latency to 1 wait state with prefetch
        self.flash.setup();
        // Configure system clock to HSI48 and enable CRS and peripheral clocks
        self.rcc.setup();
        // Enable SEVONPEND
        self.nvic.setup();
        // Configure DMA for SPI1 and USART2 transfers
        self.dma.setup();
        // Configure GPIOs
        self.pins.setup();
        // Configure USB peripheral and connect to host
        self.usb.setup();
    }

    pub fn poll(&mut self) {
        if self.nvic.usb_pending() {
            // Handle USB interrupts
            if let Some(req) = self.usb.interrupt() {
                self.process_request(req);
            }
            self.nvic.unpend_usb();
        } else if self.dap.is_swo_streaming() && !self.usb.dap2_swo_is_busy() {
            // Poll for new UART data when streaming is enabled and
            // the SWO endpoint is ready to transmit more data.
            if let Some(data) = self.dap.poll_swo() {
                self.usb.dap2_stream_swo(data);
            }
        } else {
            // Sleep until an interrupt occurs
            cortex_m::asm::wfe();
        }
    }

    fn process_request(&mut self, req: Request) {
        match req {
            Request::DAP1Command((report, n)) => {
                let response = self.dap.process_command(&report[..n]);
                if let Some(data) = response {
                    self.usb.dap1_reply(data);
                }
            }
            Request::DAP2Command((report, n)) => {
                let response = self.dap.process_command(&report[..n]);
                if let Some(data) = response {
                    self.usb.dap2_reply(data);
                }
            }
        };
    }
}

use stm32ral::{rcc, flash};
use stm32ral::{read_reg, modify_reg, reset_reg};

pub struct RCC {
    rcc: rcc::Instance,
}

impl RCC {
    pub fn new(rcc: rcc::Instance) -> Self {
        RCC { rcc }
    }

    /// Set up the device, enabling all required clocks
    ///
    /// Unsafety: this function should be called from the main context.
    /// No other contexts should be active at the same time.
    pub unsafe fn setup(&self, frequency: CoreFrequency) -> Clocks {
        // Turn on HSI
        modify_reg!(rcc, self.rcc, CR, HSION: On);
        // Wait for HSI to be ready
        while read_reg!(rcc, self.rcc, CR, HSIRDY == NotReady) {}
        // Swap system clock to HSI
        modify_reg!(rcc, self.rcc, CFGR, SW: HSI);
        // Wait for system clock to be HSI
        while read_reg!(rcc, self.rcc, CFGR, SWS != HSI) {}

        // Disable everything
        modify_reg!(rcc, self.rcc, CR,
            HSEON: Off,
            CSSON: Off,
            PLLON: Off,
            PLLI2SON: Off,
            PLLSAION: Off
        );
        reset_reg!(rcc, self.rcc, RCC, AHB1ENR);
        reset_reg!(rcc, self.rcc, RCC, AHB2ENR);
        reset_reg!(rcc, self.rcc, RCC, AHB3ENR);
        reset_reg!(rcc, self.rcc, RCC, APB1ENR);
        reset_reg!(rcc, self.rcc, RCC, APB2ENR);

        // Configure HSE in bypass mode
        modify_reg!(rcc, self.rcc, CR, HSEBYP: Bypassed);
        // Start HSE
        modify_reg!(rcc, self.rcc, CR, HSEON: On);
        // Wait for HSE to be ready
        while read_reg!(rcc, self.rcc, CR, HSERDY == NotReady) {}

        // Calculate prescalers
        let ppre1;
        let ppre2;
        match frequency {
            CoreFrequency::F48MHz => {
                ppre1 = 0b000; // AHB clock not divided
                ppre2 = 0b000; // AHB clock not divided
            }
            CoreFrequency::F72MHz => {
                ppre1 = 0b100; // AHB clock divided by 2
                ppre2 = 0b000; // AHB clock not divided
            }
            CoreFrequency::F216MHz => {
                ppre1 = 0b101; // AHB clock divided by 4
                ppre2 = 0b100; // AHB clock divided by 2
            }
        }
        // Set prescalers
        modify_reg!(rcc, self.rcc, CFGR,
            HPRE: Div1,
            PPRE1: ppre1,
            PPRE2: ppre2
        );

        // Calculate PLL parameters and flash latency
        let pllm = 25;
        let plln;
        let pllp;
        let pllq;
        let flash_latency;
        let sysclk;
        match frequency {
            CoreFrequency::F48MHz => {
                plln = 192;
                pllp = 0b01; // /4
                pllq = 4;
                flash_latency = 0b0001;
                sysclk = 48_000_000;
            }
            CoreFrequency::F72MHz => {
                plln = 288;
                pllp = 0b01; // /4
                pllq = 6;
                flash_latency = 0b0010;
                sysclk = 72_000_000;
            }
            CoreFrequency::F216MHz => {
                plln = 432;
                pllp = 0b00; // /2
                pllq = 9;
                flash_latency = 0b0111;
                sysclk = 216_000_000;
            }
        }

        // Configure PLL from HSE
        modify_reg!(rcc, self.rcc, PLLCFGR,
            PLLSRC: HSE,
            PLLM: pllm,
            PLLN: plln,
            PLLP: pllp,
            PLLQ: pllq
        );

        // Turn on PLL
        modify_reg!(rcc, self.rcc, CR, PLLON: On);
        // Wait for PLL to be ready
        while read_reg!(rcc, self.rcc, CR, PLLRDY == NotReady) {}

        // Adjust flash wait states
        modify_reg!(flash, &*flash::FLASH, ACR, LATENCY: flash_latency);

        // Swap system clock to PLL
        modify_reg!(rcc, self.rcc, CFGR, SW: PLL);
        // Wait for system clock to be PLL
        while read_reg!(rcc, self.rcc, CFGR, SWS != PLL) {}

        // Configure HCLK, PCLK1, PCLK2
        modify_reg!(rcc, self.rcc, CFGR, PPRE1: Div1, PPRE2: Div1, HPRE: Div1);

        // Enable peripheral clocks
        modify_reg!(rcc, self.rcc, AHB1ENR,
            GPIOAEN: Enabled,
            GPIOBEN: Enabled,
            GPIOCEN: Enabled,
            GPIODEN: Enabled,
            GPIOEEN: Enabled,
            GPIOFEN: Enabled,
            GPIOGEN: Enabled,
            GPIOHEN: Enabled,
            GPIOIEN: Enabled,
            DMA1EN: Enabled,
            DMA2EN: Enabled
        );
        modify_reg!(rcc, self.rcc, APB1ENR,
            // SPI2EN: Enabled,
            // USART2EN: Enabled
            UART5EN: Enabled
        );
        modify_reg!(rcc, self.rcc, APB2ENR,
            SPI5EN: Enabled,
            USART6EN: Enabled
            // SPI1EN: Enabled,
            // USART1EN: Enabled
        );

        Clocks {
            sysclk
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum CoreFrequency {
    F48MHz,
    F72MHz,
    F216MHz,
}

pub struct Clocks {
    sysclk: u32,
}

impl Clocks {
    pub fn hclk(&self) -> u32 {
        let rcc = unsafe { &*rcc::RCC };
        let hpre = read_reg!(rcc, rcc, CFGR, HPRE);
        match hpre {
            0b1000 => self.sysclk / 2,
            0b1001 => self.sysclk / 4,
            0b1010 => self.sysclk / 8,
            0b1011 => self.sysclk / 16,
            0b1100 => self.sysclk / 64,
            0b1101 => self.sysclk / 128,
            0b1110 => self.sysclk / 256,
            0b1111 => self.sysclk / 512,
            _ => self.sysclk,
        }
    }

    pub fn pclk1(&self) -> u32 {
        let hclk = self.hclk();

        let rcc = unsafe { &*rcc::RCC };
        let ppre = read_reg!(rcc, rcc, CFGR, PPRE1);
        match ppre {
            0b100 => hclk / 2,
            0b101 => hclk / 4,
            0b110 => hclk / 8,
            0b111 => hclk / 16,
            _ => hclk,
        }
    }

    pub fn pclk2(&self) -> u32 {
        let hclk = self.hclk();

        let rcc = unsafe { &*rcc::RCC };
        let ppre = read_reg!(rcc, rcc, CFGR, PPRE2);
        match ppre {
            0b100 => hclk / 2,
            0b101 => hclk / 4,
            0b110 => hclk / 8,
            0b111 => hclk / 16,
            _ => hclk,
        }
    }
}

// Copyright 2019 Adam Greig
// Dual licensed under the Apache 2.0 and MIT licenses.

use stm32ral::dma;
use stm32ral::{modify_reg, read_reg, write_reg};

/*
spi1 -> SPI5_RX: DMA2, stream 3, channel 2
spi1 -> SPI5_TX: DMA2, stream 4, channel 2
usart1 -> UART5_RX: DMA1, stream 0, channel 4
usart2 -> USART6_RX: DMA2, stream 1, channel 5
usart2 -> USART6_TX: DMA2, stream 6, channel 5
*/

const SPI_DR_OFFSET: u32 = 0x0C;
const UART_DR_OFFSET: u32 = 0x24;

pub struct DMA {
    dma1: dma::Instance,
    dma2: dma::Instance,
}

impl DMA {
    pub fn new(dma1: dma::Instance, dma2: dma::Instance) -> Self {
        DMA { dma1, dma2 }
    }

    pub fn setup(&self) {
        // Set up DMA2 stream 2, channel 3 for SPI1_RX
        // Set up DMA2 stream 3, channel 2 for SPI5_RX
        write_reg!(
            dma,
            self.dma2,
            CR3,
            CHSEL: 2,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR3,
            stm32ral::spi::SPI5 as u32 + SPI_DR_OFFSET
        );

        // Set up DMA2 stream 3, channel 3 for SPI1_TX
        // Set up DMA2 stream 4, channel 2 for SPI5_TX
        write_reg!(
            dma,
            self.dma2,
            CR4,
            CHSEL: 2,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: MemoryToPeripheral,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR4,
            stm32ral::spi::SPI5 as u32 + SPI_DR_OFFSET
        );

        // // Set up DMA1 stream 3, channel 0 for SPI2_RX
        // write_reg!(
        //     dma,
        //     self.dma1,
        //     CR3,
        //     CHSEL: 0,
        //     PL: High,
        //     MSIZE: Bits8,
        //     PSIZE: Bits8,
        //     MINC: Incremented,
        //     PINC: Fixed,
        //     CIRC: Disabled,
        //     DIR: PeripheralToMemory,
        //     EN: Disabled
        // );
        // write_reg!(
        //     dma,
        //     self.dma1,
        //     PAR3,
        //     stm32ral::spi::SPI2 as u32 + SPI_DR_OFFSET
        // );
        //
        // // Set up DMA1 stream 4, channel 0 for SPI2_TX
        // write_reg!(
        //     dma,
        //     self.dma1,
        //     CR4,
        //     CHSEL: 0,
        //     PL: High,
        //     MSIZE: Bits8,
        //     PSIZE: Bits8,
        //     MINC: Incremented,
        //     PINC: Fixed,
        //     CIRC: Disabled,
        //     DIR: MemoryToPeripheral,
        //     EN: Disabled
        // );
        // write_reg!(
        //     dma,
        //     self.dma1,
        //     PAR4,
        //     stm32ral::spi::SPI2 as u32 + SPI_DR_OFFSET
        // );

        // Set up DMA2 stream 5, channel 4 for USART1_RX
        // Set up DMA1 stream 0, channel 4 for UART5_RX
        write_reg!(
            dma,
            self.dma1,
            CR0,
            CHSEL: 4,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Enabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma1,
            PAR0,
            stm32ral::usart::UART5 as u32 + UART_DR_OFFSET
        );

        // Set up DMA1 stream 5, channel 4 for USART2_RX
        // Set up DMA2 stream 1, channel 5 for USART6_RX
        write_reg!(
            dma,
            self.dma2,
            CR1,
            CHSEL: 5,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Enabled,
            DIR: PeripheralToMemory,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR1,
            stm32ral::usart::USART6 as u32 + UART_DR_OFFSET
        );

        // Set up DMA1 stream 6, channel 4 for USART2_TX
        // Set up DMA2 stream 6, channel 5 for USART6_TX
        write_reg!(
            dma,
            self.dma2,
            CR6,
            CHSEL: 5,
            PL: High,
            MSIZE: Bits8,
            PSIZE: Bits8,
            MINC: Incremented,
            PINC: Fixed,
            CIRC: Disabled,
            DIR: MemoryToPeripheral,
            EN: Disabled
        );
        write_reg!(
            dma,
            self.dma2,
            PAR1,
            stm32ral::usart::USART6 as u32 + UART_DR_OFFSET
        );
    }

    /// Sets up and enables a DMA transmit/receive for SPI1 (streams 2 and 3, channel 3)
    /// Sets up and enables a DMA transmit/receive for SPI5 (streams 3 and 4, channel 2)
    pub fn spi5_enable(&self, tx: &[u8], rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma2,
            LIFCR,
            CTCIF3: Clear,
            CHTIF3: Clear,
            CTEIF3: Clear,
            CDMEIF3: Clear,
            CFEIF3: Clear
        );
        write_reg!(
            dma,
            self.dma2,
            HIFCR,
            CTCIF4: Clear,
            CHTIF4: Clear,
            CTEIF4: Clear,
            CDMEIF4: Clear,
            CFEIF4: Clear
        );
        write_reg!(dma, self.dma2, NDTR3, rx.len() as u32);
        write_reg!(dma, self.dma2, NDTR4, tx.len() as u32);
        write_reg!(dma, self.dma2, M0AR3, rx.as_mut_ptr() as u32);
        write_reg!(dma, self.dma2, M0AR4, tx.as_ptr() as u32);
        modify_reg!(dma, self.dma2, CR3, EN: Enabled);
        modify_reg!(dma, self.dma2, CR4, EN: Enabled);
    }

    /// Check if SPI1 transaction is still ongoing
    /// Check if SPI5 transaction is still ongoing
    pub fn spi5_busy(&self) -> bool {
        read_reg!(dma, self.dma2, LISR, TCIF3 == NotComplete)
    }

    /// Stop SPI1 DMA
    /// Stop SPI5 DMA
    pub fn spi5_disable(&self) {
        modify_reg!(dma, self.dma2, CR3, EN: Disabled);
        modify_reg!(dma, self.dma2, CR4, EN: Disabled);
    }

    /// Start USART1 reception into provided buffer
    /// Start UART5 reception into provided buffer
    pub fn uart5_start(&self, rx: &mut [u8]) {
        write_reg!(
            dma,
            self.dma1,
            LIFCR,
            CTCIF0: Clear,
            CHTIF0: Clear,
            CTEIF0: Clear,
            CDMEIF0: Clear,
            CFEIF0: Clear
        );
        write_reg!(dma, self.dma1, NDTR0, rx.len() as u32);
        write_reg!(dma, self.dma1, M0AR0, rx.as_mut_ptr() as u32);
        modify_reg!(dma, self.dma1, CR0, EN: Enabled);
    }

    /// Return how many bytes are left to transfer for USART1
    pub fn uart5_ndtr(&self) -> usize {
        read_reg!(dma, self.dma1, NDTR0) as usize
    }

    /// Stop USART1 DMA
    pub fn uart5_stop(&self) {
        modify_reg!(dma, self.dma1, CR0, EN: Disabled);
    }
}

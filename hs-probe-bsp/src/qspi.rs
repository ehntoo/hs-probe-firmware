use core::sync::atomic::{AtomicU32, Ordering};
use stm32ral::quadspi;
use stm32ral::{modify_reg, read_reg, write_reg};

use crate::rcc::Clocks;

pub struct QSPI {
    qspi: quadspi::Instance,
    base_clock: AtomicU32,
}

impl QSPI {
    pub fn new(qspi: quadspi::Instance) -> Self {
        QSPI {
            qspi,
            base_clock: AtomicU32::new(0),
        }
    }

    pub fn set_base_clock(&self, clocks: &Clocks) {
        self.base_clock.store(clocks.hclk(), Ordering::SeqCst);
    }

    pub fn setup_swd(&self) {
    }

    pub fn calculate_prescaler(&self, max_frequency: u32) -> Option<u32> {
        let base_clock = self.base_clock.load(Ordering::SeqCst);
        if base_clock == 0 || ( base_clock / 256 ) >= max_frequency {
            return None;
        }

        Some(base_clock / max_frequency)
    }

    /// Change QSPI clock rate
    pub fn set_prescaler(&self, prescaler: u32) {
        modify_reg!(quadspi, self.qspi, CR, PRESCALER: prescaler);
    }

    /// Wait for any pending operation then disable QSPI
    pub fn disable(&self) {
        self.wait_busy();
        modify_reg!(quadspi, self.qspi, CR, EN: 0);
    }

    /// Wait for current QSPI operation to complete
    #[inline(always)]
    pub fn wait_busy(&self) {
        while read_reg!(quadspi, self.qspi, SR, BUSY != 0) {}
    }

    pub fn swd_dummy_bytes(&self) {
        // write dummy cycles
        write_reg!(
            quadspi,
            self.qspi,
            CCR,
            FMODE: 1,
            ADMODE: 2,
            ADSIZE: 1
        );
        write_reg!(quadspi, self.qspi, AR, ADDRESS: 0);
    }

    // ========================
    // SWD Read Request:
    // 8 bit request + trn (1 by default) + 3 bits ack + 32 bits data + 1 bit parity + 1 bit trn
    // 8+1+3+32+1+1 = 46
    //
    // first phase just to check ack status, second phase read if ack good, third phase padding
    // // write command, get ack
    // QUADSPI_DLR->1
    // QUADSPI_CCR->DDRM=1,DHHC=?,FMODE=01(indirect read),DMODE=1,DCYC=0,ADSIZE=3,ADMODE=2,IMODE=0
    // QUADSPI_AR->request, padded to 32 bits
    // write to address will trigger read op
    //
    // // read data, 32b + parity + trn = 34b
    // QUADSPI_DLR->17
    // QUADSPI_CCR->DDRM=1,DHHC=?,FMODE=01(indirect read),DMODE=2,DCYC=0,ADSIZE=0,ADMODE=0,IMODE=0
    //
    // QUADSPI_DLR->2
    // QUADSPI_CCR->DRM=0,FMODE=00(indirect write),DMODE=2,DCYC=0,ADMODE=0,IMODE=0
    // QUADSPI_DR->0 (as uint_16)
    pub fn swd_read_req(&self, request: u8) -> u8 {
        // 32 bits ddr-mode dual-spi address, no instruction, 0 dummy cycles,
        // read 1 byte ddr-mode single-spi (4 bits, trn+ack)
        let expanded_req =
            (request as u32 & 0x80) >> 7 << 29 |
            (request as u32 & 0x40) >> 6 << 25 |
            (request as u32 & 0x20) >> 5 << 21 |
            (request as u32 & 0x10) >> 4 << 17 |
            (request as u32 & 0x08) >> 3 << 13 |
            (request as u32 & 0x04) >> 2 <<  9 |
            (request as u32 & 0x02) >> 1 <<  5 |
            (request as u32 & 0x01) >> 0 <<  1;
            // (request as u32 & 0x80) >> 7 <<  5 |
            // (request as u32 & 0x40) >> 6 <<  1 |
            // (request as u32 & 0x20) >> 5 << 13 |
            // (request as u32 & 0x10) >> 4 <<  9 |
            // (request as u32 & 0x08) >> 3 << 21 |
            // (request as u32 & 0x04) >> 2 << 17 |
            // (request as u32 & 0x02) >> 1 << 29 |
            // (request as u32 & 0x01) >> 0 << 25;
        let expanded_req = expanded_req | expanded_req << 2;
        write_reg!(quadspi, self.qspi, DLR, DL: 1);
        write_reg!(
            quadspi,
            self.qspi,
            CCR,
            DDRM: 1,
            FMODE: 1,
            DMODE: 1,
            DCYC: 0,
            ADSIZE: 3,
            ADMODE: 2
        );
        write_reg!(quadspi, self.qspi, AR, ADDRESS: expanded_req as u32);
        // no need to wait_busy, as the access to the data register will stall
        // until operation complete
        let ack = self.read_dr_u8();
        ack & 0x1 | ack & 0x4 >> 1 | ack & 0x10 >> 2
    }

    pub fn swd_read_data(&self) -> (u32, u8) {
        // read 34 bits in ddr mode, dual spi (34 * 2 * 2 = 136, 136/8=17)
        write_reg!(quadspi, self.qspi, DLR, DL: 17);
        write_reg!(
            quadspi,
            self.qspi,
            CCR,
            DDRM: 1,
            FMODE: 1,
            DMODE: 2,
            ADMODE: 0
        );
        let mut res = 0;
        // TODO: Rework as 32-bit access instead
        for _ in 0..16 {
            let dat = self.read_dr_u8();
            res =
                (res << 2) |
                (dat & 0x20) as u32 >> 4 |
                (dat & 0x02) as u32 >> 1;
        }
        let parity = self.read_dr_u8();
        let parity = (parity >> 5) & 1;

        self.swd_dummy_bytes();
        (res, parity)
    }

    pub fn swd_write_req(&self, request: u8) -> u8 {
        // 32 bits ddr-mode dual-spi address, no instruction, 1 dummy cycle,
        // read 1 byte ddr-mode single-spi (4 bits)
        let expanded_req =
            (request as u32 & 0x80) >> 7 << 29 |
            (request as u32 & 0x40) >> 6 << 25 |
            (request as u32 & 0x20) >> 5 << 21 |
            (request as u32 & 0x10) >> 4 << 17 |
            (request as u32 & 0x08) >> 3 << 13 |
            (request as u32 & 0x04) >> 2 <<  9 |
            (request as u32 & 0x02) >> 1 <<  5 |
            (request as u32 & 0x01) >> 0 <<  1;
        let expanded_req = expanded_req | expanded_req << 2;
        write_reg!(quadspi, self.qspi, DLR, DL: 1);
        write_reg!(
            quadspi,
            self.qspi,
            CCR,
            DDRM: 1,
            FMODE: 1,
            DMODE: 1,
            DCYC: 1,
            ADSIZE: 3,
            ADMODE: 2
        );
        write_reg!(quadspi, self.qspi, AR, ADDRESS: expanded_req as u32);
        // no need to wait_busy, as the access to the data register will stall
        // until operation complete
        let ack = self.read_dr_u8();
        ack & 0x4 >> 2 | ack & 0x10 >> 3 | ack & 0x40 >> 4
    }

    // ========================
    // SWD Write Request:
    // 8 bit request + trn (1 by default) + 3 bits ack + trn + 32 bits data + 1 bit parity
    // 8+1+3+1+32+1 = 46
    //
    // disabled command
    // 8 bits for request, sent as 16 bits of dual-spi mode address (32 bits in ddr mode?)
    // disabled alternate bytes
    // 1 dummy cycles
    // 4 bits to read - 3b-ack + trn -> 1 bytes in ddr single-spi mode.
    // -- reconfigure -- (check ack status)
    // disabled command, disable address, disable alternate bytes, 0 dummy cycles
    // 33 bits to write - data + parity, but we can pad with additional idle 0 cycles
    //                    write 11 bytes in sdr dual-spi mode: data + 11 idle cycles
    //
    // configuration for first phase:
    // QUADSPI_DLR->1
    // QUADSPI_CCR->DDRM=1,DHHC=?,FMODE=01(indirect read),DMODE=1,DCYC=1,ADSIZE=3,ADMODE=2,IMODE=0
    // QUADSPI_AR->request, padded to 32 bits
    // write to address will trigger read op
    //
    // configuration for second phase:
    // QUADSPI_DLR->17
    // QUADSPI_CCR->DDRM=0,FMODE=00(indirect write),DMODE=2,DCYC=0,ADSIZE=3,ADMODE=2,IMODE=0
    // write 11 bytes of data to DR
    pub fn swd_write_data(&self, data: u32, parity: u8) {
        write_reg!(quadspi, self.qspi, DLR, DL: 11);
        write_reg!(
            quadspi,
            self.qspi,
            CCR,
            FMODE: 0,
            DMODE: 2,
            ADSIZE: 3,
            ADMODE: 2
        );
        let d1 =
            (data & 0x80000000) >> 31 << 31 |
            (data & 0x40000000) >> 30 << 29 |
            (data & 0x20000000) >> 29 << 27 |
            (data & 0x10000000) >> 28 << 25 |
            (data & 0x08000000) >> 27 << 23 |
            (data & 0x04000000) >> 26 << 21 |
            (data & 0x02000000) >> 25 << 19 |
            (data & 0x01000000) >> 24 << 17 |
            (data & 0x00800000) >> 23 << 15 |
            (data & 0x00400000) >> 22 << 13 |
            (data & 0x00200000) >> 21 << 11 |
            (data & 0x00100000) >> 20 <<  9 |
            (data & 0x00080000) >> 19 <<  7 |
            (data & 0x00040000) >> 18 <<  5 |
            (data & 0x00020000) >> 17 <<  3 |
            (data & 0x00010000) >> 16 <<  1 ;
        let d2 =
            (data & 0x00008000) >> 15 << 31 |
            (data & 0x00004000) >> 14 << 29 |
            (data & 0x00002000) >> 13 << 27 |
            (data & 0x00001000) >> 12 << 25 |
            (data & 0x00000800) >> 11 << 23 |
            (data & 0x00000400) >> 10 << 21 |
            (data & 0x00000200) >>  9 << 19 |
            (data & 0x00000100) >>  8 << 17 |
            (data & 0x00000080) >>  7 << 15 |
            (data & 0x00000040) >>  6 << 13 |
            (data & 0x00000020) >>  5 << 11 |
            (data & 0x00000010) >>  4 <<  9 |
            (data & 0x00000008) >>  3 <<  7 |
            (data & 0x00000004) >>  2 <<  5 |
            (data & 0x00000002) >>  1 <<  3 |
            (data & 0x00000001) >>  0 <<  1 ;
        let parity = parity << 7;
        write_reg!(quadspi, self.qspi, DR, DATA: d1);
        write_reg!(quadspi, self.qspi, DR, DATA: d2);
        self.write_dr_u8(parity);
        self.write_dr_u16(0);
    }

    /// Perform a 16-bit write to DR
    ///
    /// Note that this enqueues two transmissions.
    #[inline(always)]
    fn write_dr_u16(&self, data: u16) {
        unsafe { core::ptr::write_volatile(&self.qspi.DR as *const _ as *mut u16, data) };
    }

    /// Perform a 8-bit write to DR
    #[inline(always)]
    fn write_dr_u8(&self, data: u8) {
        unsafe { core::ptr::write_volatile(&self.qspi.DR as *const _ as *mut u8, data) };
    }

    /// Perform an 8-bit read from DR
    #[inline(always)]
    fn read_dr_u8(&self) -> u8 {
        unsafe { core::ptr::read_volatile(&self.qspi.DR as *const _ as *const u8) }
    }
}

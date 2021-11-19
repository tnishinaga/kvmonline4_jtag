use crate::jtag::dap::*;
use bitfield::{bitfield, bitfield_bitrange, bitfield_fields};
use log::{debug, error, info, warn};
use spin::mutex::{Mutex, MutexGuard};

pub enum Armv8DebugRegisterOffset {
    EDESR = 0x020,
    EDECR = 0x024,
    EDWARlo = 0x030,
    EDWARhi = 0x034,
    DBGDTRRX_EL0 = 0x080,
    EDITR = 0x084,
    EDSCR = 0x088,
    DBGDTRTX_EL0 = 0x08C,
    EDRCR = 0x090,
    EDACR = 0x094,
    EDECCR = 0x098,
    EDPCSRlo = 0x0A0,
    EDCIDSR = 0x0A4,
    EDVIDSR = 0x0A8,
    EDPCSRhi = 0x0AC,
    OSLAR_EL1 = 0x0300,
    EDPRCR = 0x0310,
    EDPRSR = 0x0314,
    DBGBVR_BASE_EL1 = 0x0400,
    DBGBCR_BASE_EL1 = 0x0408,
    DBGWVR_BASE_EL1 = 0x800,
    DBGWCR_BASE_EL1 = 0x808,
    MIDR_EL1 = 0xD00,
    EDPFR = 0xD20,
    EDDFR = 0xD28,
    EDPIDR0 = 0xFE0,
    EDPIDR1 = 0xFE4,
    EDPIDR2 = 0xFE8,
    EDPIDR4 = 0xFEC,
    EDDEVTYPE = 0xFCC,
}

enum CtiOffset {
    CTICONTROL = 0x000,
    CTIINTACK = 0x010,
    CTIAPPSET = 0x014,
    CTIAPPCLEAR = 0x018,
    CTIAPPPULSE = 0x01C,
    CTIINENn = 0x020,
    CTIOUTENn = 0x0A0,
    CTITRIGINSTATUS = 0x130,
    CTITRIGOUTSTATUS = 0x134,
    CTICHINSTATUS = 0x138,
    CTICHOUTSTATUS = 0x13C,
    CTIGATE = 0x140,
    CTIDEVID2 = 0xFC0,
    CTIDEVID1 = 0xFC4,
    CTIDEVID = 0xFC8,
}

bitfield! {
    pub struct EDSCR(u32);
    impl Debug;
    pub TFO, _: 31, 31;
    pub RXfull, _: 30, 30;
    pub TXfull, _: 29, 29;
    pub ITO, _: 28, 28;
    pub RXO, _: 27, 27;
    pub TXU, _: 26, 26;
    pub PipeAdv, _: 25, 25;
    pub ITE, _: 24, 24;
    pub INTdis, _: 23, 22;
    pub TDA, _: 21, 21;
    pub MA, _: 20, 20;
    pub SC2, _: 19, 19;
    pub NS, _: 18, 18;
    reserved0, _: 17,17;
    pub SDD, _: 16, 16;
    reserved1, _: 15,15;
    pub HDE, set_hde: 14, 14;
    pub RW, _: 13, 10;
    pub EL, _: 9, 8;
    pub A, _: 7, 7;
    pub ERR, _: 6, 6;
    pub STATUS, _: 5, 0;
}

bitfield! {
    pub struct EDRCR(u32);
    impl Debug;
    reserved, _: 31,5;
    pub CBRRQ, set_CBRRQ: 4, 4;
    pub CSPA, _: 3, 3;
    pub CSE, set_CSE: 2, 2;
    reserved0, _: 1,0;
    pub SDD, _: 16, 16;
}

bitfield! {
    pub struct EDPRSR(u32);
    impl Debug;
    reserved, _: 31,12;
    pub SDR, _: 11, 11;
    pub SPMAD, _: 10, 10;
    pub EPMAD, _: 9, 9;
    pub SDAD,  _: 8, 8;
    pub EDAD,  _: 7, 7;
    pub DLK,  _: 6, 6;
    pub OSLK,  _: 5, 5;
    pub HALTED,  _: 4, 4;
    pub SR,  _: 3, 3;
    pub R,  _: 2, 2;
    pub SPD,  _: 1, 1;
    pub PU, _: 0, 0;
}

pub struct Cti<'a, T> {
    pub dap: &'a Mutex<T>,
    pub baseaddr: u64,
}

impl<'a, T: DebugPort + MemoryAccessPort> Cti<'a, T> {
    fn init(&mut self) {}

    pub fn enable(&mut self) {
        self.register_u32_write(CtiOffset::CTICONTROL as u64, 1);
    }

    pub fn disable(&mut self) {
        self.register_u32_write(CtiOffset::CTICONTROL as u64, 0);
    }

    pub fn channel_gate_enable(&mut self, channel: u8) {
        // TODO: check channel < 32
        let status = self.register_u32_read(CtiOffset::CTIGATE as u64);
        self.register_u32_write(CtiOffset::CTIGATE as u64, status | (1 << channel));
    }
    pub fn channel_gate_disable(&mut self, channel: u8) {
        // TODO: check channel < 32
        let status = self.register_u32_read(CtiOffset::CTIGATE as u64);
        let mask = !(1 << channel);
        self.register_u32_write(CtiOffset::CTIGATE as u64, status & mask);
    }
    pub fn input_trigger_enable(&mut self, trigger: u8, channel: u8) {
        let offset = CtiOffset::CTIINENn as u64 + (trigger as u64) * 0x04;
        let status = self.register_u32_read(offset);
        self.register_u32_write(offset, status | (1 << channel));
    }
    pub fn input_trigger_disable(&mut self, trigger: u8, channel: u8) {
        let offset = CtiOffset::CTIINENn as u64 + (trigger as u64) * 0x04;
        let status = self.register_u32_read(offset);
        let mask = !(1 << channel);
        self.register_u32_write(offset, status & mask);
    }
    pub fn output_trigger_enable(&mut self, trigger: u8, channel: u8) {
        let offset = CtiOffset::CTIOUTENn as u64 + (trigger as u64) * 0x04;
        let status = self.register_u32_read(offset);
        self.register_u32_write(offset, status | (1 << channel));
    }
    pub fn output_trigger_disable(&mut self, trigger: u8, channel: u8) {
        let offset = CtiOffset::CTIOUTENn as u64 + (trigger as u64) * 0x04;
        let status = self.register_u32_read(offset);
        let mask = !(1 << channel);
        self.register_u32_write(offset, status & mask);
    }
    pub fn output_trigger_ack_deactivate(&mut self, trigger: u8) {
        self.register_u32_write(CtiOffset::CTIINTACK as u64, 1 << trigger);
    }
    pub fn input_trigger_status(&mut self, trigger: u8) -> bool {
        let status = self.register_u32_read(CtiOffset::CTITRIGINSTATUS as u64);
        (status & (1 << trigger)) != 0
    }
    pub fn output_trigger_status(&mut self, trigger: u8) -> bool {
        let status = self.register_u32_read(CtiOffset::CTITRIGOUTSTATUS as u64);
        (status & (1 << trigger)) != 0
    }

    pub fn generate_pulse(&mut self, channel: u32) {
        self.register_u32_write(CtiOffset::CTIAPPPULSE as u64, 1 << channel);
    }
}

impl<'a, T: DebugPort + MemoryAccessPort> AArch64Register<T> for Cti<'a, T> {
    fn baseaddr(&self) -> u64 {
        self.baseaddr
    }
    fn dap_lock(&self) -> MutexGuard<T> {
        self.dap.lock()
    }
}

pub trait AArch64Register<T: DebugPort + MemoryAccessPort> {
    fn baseaddr(&self) -> u64;
    fn dap_lock(&self) -> MutexGuard<T>;

    fn register_u32(&mut self, offset: u64, data: u32, read: bool) -> u32 {
        let offset = offset as u64;
        let bd_base = offset & (!0x0f as u64);
        let bd_index = (offset % 0x10) / 4;

        let mut dap = self.dap_lock();
        let baseaddr = self.baseaddr();

        dap.memap_tar_u64_write(baseaddr + bd_base);
        let (ack, result) = match bd_index {
            0 => dap.memap_bd0(data, read),
            1 => dap.memap_bd1(data, read),
            2 => dap.memap_bd2(data, read),
            3 => dap.memap_bd3(data, read),
            _ => panic!("unexpected value"),
        };
        // debug!("register_u32: {:?}", ack);
        drop(dap);
        result
    }
    fn register_u32_read(&mut self, offset: u64) -> u32 {
        self.register_u32(offset, 0, true)
    }
    fn register_u32_write(&mut self, offset: u64, data: u32) {
        self.register_u32(offset, data, false);
    }

    fn register_u64(&mut self, offset: u64, data: u64, read: bool) -> u64 {
        // TODO: 0xCがoffsetの最下位にある場合にTARを再設定する
        todo!();
        let offset = offset as u64;
        let bd_base = offset & (!0x0f as u64);
        let bd_index = (offset % 0x10) / 4;
        let data_low = (data & 0xffff_ffff) as u32;
        let data_high = (data >> 32) as u32;

        let mut dap = self.dap_lock();

        dap.memap_tar_u64_write(self.baseaddr() + bd_base as u64);

        let (ack, result_low) = match bd_index {
            0 => dap.memap_bd0(data_low, read),
            1 => dap.memap_bd1(data_low, read),
            2 => dap.memap_bd2(data_low, read),
            3 => dap.memap_bd3(data_low, read),
            _ => panic!("unexpected value"),
        };

        let (ack, result_high) = match bd_index + 1 {
            0 => dap.memap_bd0(data_high, read),
            1 => dap.memap_bd1(data_high, read),
            2 => dap.memap_bd2(data_high, read),
            3 => dap.memap_bd3(data_high, read),
            _ => panic!("unexpected value"),
        };

        // debug!("register_u64 ack high: {:?}", ack);
        drop(dap);

        ((result_high as u64) << 32) | (result_low as u64)
    }

    fn register_u64_read(&mut self, offset: u64) -> u64 {
        self.register_u64(offset, 0, true)
    }
    fn register_u64_write(&mut self, offset: u64, data: u64) {
        self.register_u64(offset, data, false);
    }
}

pub struct A64Target<'a, T> {
    pub dap: &'a Mutex<T>,
    pub baseaddr: u64,
}

impl<'a, T: DebugPort + MemoryAccessPort> A64Target<'a, T> {
    pub fn edscr_read(&mut self) -> EDSCR {
        EDSCR(self.register_u32_read(Armv8DebugRegisterOffset::EDSCR as u64))
    }
    pub fn edscr_write(&mut self, data: EDSCR) {
        self.register_u32_write(Armv8DebugRegisterOffset::EDSCR as u64, data.0)
    }
    pub fn edrcr_write(&mut self, data: EDRCR) {
        self.register_u32_write(Armv8DebugRegisterOffset::EDRCR as u64, data.0)
    }
    pub fn edrcr_read(&mut self) -> EDRCR {
        EDRCR(self.register_u32_read(Armv8DebugRegisterOffset::EDRCR as u64))
    }
    pub fn oslar_write(&mut self, oslk: u32) {
        self.register_u32_write(Armv8DebugRegisterOffset::OSLAR_EL1 as u64, oslk)
    }
    pub fn edprsr_read(&mut self) -> EDPRSR {
        EDPRSR(self.register_u32_read(Armv8DebugRegisterOffset::EDPRSR as u64))
    }
}

impl<'a, T: DebugPort + MemoryAccessPort> AArch64Register<T> for A64Target<'a, T> {
    fn baseaddr(&self) -> u64 {
        self.baseaddr
    }
    fn dap_lock(&self) -> MutexGuard<T> {
        self.dap.lock()
    }
}

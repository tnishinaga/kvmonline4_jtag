use bitfield::{bitfield, bitfield_bitrange, bitfield_fields};
use bitflags::bitflags;
use log::{debug, error, info, warn};

use crate::interface::JtagInterface;
use crate::jtag::jtag::TAP;

enum Instruction {
    ABORT = 0b1000,
    DPACC = 0b1010,
    APACC = 0b1011,
    IDCODE = 0b1110,
    BYPASS = 0b1111,
}

#[derive(Debug)]
pub enum DpAddress {
    // DPv0
    PDIDR_ABORT = 0b00,
    CTRLSTAT = 0b01,
    SELECT = 0b10,
    RDBUFF = 0b11,
}

impl Into<u8> for DpAddress {
    fn into(self) -> u8 {
        self as u8
    }
}

bitfield! {
    pub struct DpSelect(u32);
    impl Debug;
    pub apsel, set_apsel: 31, 24;
    reserved, _: 23, 8;
    pub apbanksel, set_apbanksel: 7, 4;
    pub dpbanksel, set_dpbanksel: 3, 0;
}

bitfield! {
    pub struct CtrlStatus(u32);
    impl Debug;
    pub CSYSPWRUPACK, _: 31, 31;
    pub CSYSPWRUPREQ, set_CSYSPWRUPREQ: 30, 30;
    pub CDBGPWRUPACK, _: 29, 29;
    pub CDBGPWRUPREQ, set_CDBGPWRUPREQ: 28,28;
    pub CDBGRSTACK, _: 27,27;
    pub CDBGRSTREQ, set_CDBGRSTREQ: 26,26;
    reserved, _: 25,24;
    pub TRNCNT, _: 23,12;
    pub MASKLANE, _: 11,8;
    pub WDATAERR, _: 7,7;
    pub READOK, _: 6,6;
    pub STICKYERR, set_STICKYERR: 5,5;
    pub STICKYCMP, _: 4,4;
    pub TRNMODE, _: 3,2;
    pub STICKYORUN, _: 1,1;
    pub ORUNDETECT, _: 0,0;
}

bitfield! {
    pub struct CSW(u32);
    impl Debug;
    pub DbgSwEnable, set_DbgSwEnable: 31, 31;
    pub PROT, _: 30, 28;
    pub CACHE, _: 27, 24;
    pub SPDIN, _: 23,23;
    reserved1, _: 22,16;
    pub Type, _: 15,12;
    pub Mode, _: 11,8;
    pub TrInProg, _: 7,7;
    pub DeviceEn, _: 6,6;
    pub AddrInc, _: 5,4;
    reserved0, _: 3,3;
    pub SIZE, _: 2,0;
}

#[derive(Debug)]
pub enum DapAck {
    Wait = 0x01,
    OkFault = 0x02,
    InvalidAck = 0,
}

impl From<u8> for DapAck {
    fn from(ack: u8) -> Self {
        match ack {
            0x01 => DapAck::Wait,
            0x02 => DapAck::OkFault,
            _ => DapAck::InvalidAck,
        }
    }
}

bitfield! {
    pub struct PdIdr(u32);
    impl Debug;
    pub REVISION, _: 31, 28;
    pub PARTNO, _: 27, 20;
    reserved, _: 19, 17;
    pub MIN, _: 16, 16;
    pub VERSION, _: 15, 12;
    pub DESIGNER, _: 11, 1;
    pub RAO, _: 0, 0;
}

pub enum MemapAddress {
    CSW = 0x00,
    TARlo = 0x04,
    TARhi = 0x08,
    DRW = 0x0C,
    BD0 = 0x10,
    BD1 = 0x14,
    BD2 = 0x18,
    BD3 = 0x1C,
    MBT = 0x20,
    BASEhi = 0xF0,
    CFG = 0xF4,
    BASElo = 0xF8,
    IDR = 0xFC,
}

pub trait DapInterface {
    fn apacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32);
    fn dpacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32);
}

struct SWD;
impl DapInterface for SWD {
    fn apacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        todo!();
    }
    fn dpacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        todo!();
    }
}

impl<'a, T: JtagInterface> DapInterface for TAP<'a, T> {
    fn apacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        self.write_instruction(Instruction::APACC as u8);
        debug!(
            "apacc: {} {:#08x} to {:?}",
            if RnW { "Read" } else { "Write" },
            data,
            a
        );
        self.acc(data, a, RnW)
    }
    fn dpacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        self.write_instruction(Instruction::DPACC as u8);
        debug!(
            "dpacc: {} {:#08x} to {:?}",
            if RnW { "Read" } else { "Write" },
            data,
            a
        );
        self.acc(data, a as u8, RnW)
    }
}

impl<'a, T: JtagInterface> TAP<'a, T> {
    fn acc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        // create apacc_data
        let mut apacc_data = [false; 35];
        apacc_data[0] = RnW;
        apacc_data[1] = (a & 1) != 0;
        apacc_data[2] = (a & 0b10) != 0;
        let mut tmp = data;
        for i in 0..32 {
            apacc_data[3 + i] = (tmp & 1) != 0;
            tmp = tmp >> 1;
        }
        // read write DR
        self.read_write_dr(&mut apacc_data, true, false, false);
        // collect
        let ack = apacc_data[0..3].iter().fold(0, |x, y| (x << 1) | *y as u8);
        apacc_data.reverse();
        let result = apacc_data[0..32]
            .iter()
            .fold(0, |x, y| (x << 1) | *y as u32);

        debug!(
            "acc debug: data: {:#x}, a: {:#x}, RnW: {:?}, ack: {:#x}, result: {:#x}",
            data, a, RnW, ack, result
        );

        (ack, result)
    }
    pub fn abort(&mut self) {
        self.write_instruction(Instruction::ABORT as u8);
        let mut buffer = [false; 32];
        buffer[0] = true;
        self.read_write_dr(&mut buffer, true, false, true);
    }
}

pub trait DebugPort: DapInterface {
    // DPレジスタアクセス関数
    fn dp_abort_write(&mut self) -> DapAck {
        self.dpacc(0, DpAddress::PDIDR_ABORT.into(), false);
        let (ack, _) = self.dp_rdbuff_read();
        ack
    }

    fn dp_select_write(&mut self, apsel: u8, apbanksel: u8, dpbanksel: u8) -> DapAck {
        let mut select = DpSelect(0);
        select.set_apsel(apsel as u32);
        select.set_apbanksel((apbanksel & 0x0f) as u32);
        select.set_dpbanksel((dpbanksel & 0x0f) as u32);

        // SELECTを実行する
        self.dpacc(select.0, DpAddress::SELECT.into(), false);
        // TODO: error check
        let (ack, _) = self.dp_rdbuff_read();
        ack
    }

    fn dp_rdbuff_read(&mut self) -> (DapAck, u32) {
        let (ack, data) = self.dpacc(0, DpAddress::RDBUFF.into(), true);
        (DapAck::from(ack), data)
    }

    fn dp_ctrlstat(&mut self, control: CtrlStatus, read: bool) -> (DapAck, CtrlStatus) {
        // select DPBANKSEL to 0
        // self.dp_select_write(0, 0, 0);
        self.dpacc(control.0, DpAddress::CTRLSTAT.into(), read);
        let (ack, result) = self.dp_rdbuff_read();
        (ack, CtrlStatus(result))
    }
    fn dp_ctrlstat_read(&mut self) -> (DapAck, CtrlStatus) {
        self.dp_ctrlstat(CtrlStatus(0), true)
    }
    fn dp_ctrlstat_write(&mut self, control: CtrlStatus) -> DapAck {
        let (ack, _) = self.dp_ctrlstat(control, false);
        ack
    }
}

pub trait MemoryAccessPort: DapInterface + DebugPort {
    // MEM-APアクセス関数たち
    fn memap(&mut self, address: MemapAddress, data: u32, read: bool) -> (DapAck, u32);

    fn memap_idr_read(&mut self) -> (DapAck, u32) {
        self.memap(MemapAddress::IDR, 0, true)
    }

    fn memap_csw_read(&mut self) -> (DapAck, CSW) {
        let (ack, result) = self.memap(MemapAddress::CSW, 0, true);
        (ack, CSW(result))
    }
    fn memap_csw_write(&mut self, csw: CSW) -> DapAck {
        let (ack, _) = self.memap(MemapAddress::CSW, csw.0, false);
        ack
    }

    fn memap_cfg_read(&mut self) -> (DapAck, u32) {
        self.memap(MemapAddress::CFG, 0, true)
    }

    fn memap_tar_u64(&mut self, address: u64, read: bool) -> (DapAck, u64) {
        let address_low = (address & 0xffff_ffff) as u32;
        let address_high = (address >> 32) as u32;
        let mut result: u64 = 0;

        debug!("set TAR address to {:#16x}", address);

        let (ack, tmp) = self.memap(MemapAddress::TARhi, address_high, read);
        result = (tmp as u64) << 32;
        let (ack, tmp) = self.memap(MemapAddress::TARlo, address_low, read);
        result = result | (tmp as u64);

        (ack, result)
    }

    fn memap_tar_u64_read(&mut self) -> (DapAck, u64) {
        self.memap_tar_u64(0, true)
    }

    fn memap_tar_u64_write(&mut self, address: u64) -> DapAck {
        let (ack, _) = self.memap_tar_u64(address, false);
        let (_, address) = self.memap_tar_u64_read();
        debug!("verify TAR address {:#16x}", address);
        ack
    }

    fn memap_tar_u32(&mut self, address: u32, read: bool) -> (DapAck, u32) {
        let (ack, result) = self.memap(MemapAddress::TARlo, address, read);
        (ack, result)
    }

    fn memap_tar_u32_read(&mut self) -> (DapAck, u32) {
        self.memap_tar_u32(0, true)
    }

    fn memap_tar_u32_write(&mut self, address: u32) -> DapAck {
        let (ack, _) = self.memap_tar_u32(address, false);
        ack
    }

    fn memap_bd0(&mut self, data: u32, read: bool) -> (DapAck, u32) {
        self.memap(MemapAddress::BD0, data, read)
    }
    fn memap_bd0_read(&mut self) -> (DapAck, u32) {
        self.memap_bd0(0, true)
    }
    fn memap_bd0_write(&mut self, data: u32) -> DapAck {
        let (ack, _) = self.memap_bd0(data, false);
        ack
    }
    fn memap_bd1(&mut self, data: u32, read: bool) -> (DapAck, u32) {
        self.memap(MemapAddress::BD1, data, read)
    }
    fn memap_bd1_read(&mut self) -> (DapAck, u32) {
        self.memap_bd1(0, true)
    }
    fn memap_bd1_write(&mut self, data: u32) -> DapAck {
        let (ack, _) = self.memap_bd1(data, false);
        ack
    }
    fn memap_bd2(&mut self, data: u32, read: bool) -> (DapAck, u32) {
        self.memap(MemapAddress::BD2, data, read)
    }
    fn memap_bd2_read(&mut self) -> (DapAck, u32) {
        self.memap_bd2(0, true)
    }
    fn memap_bd2_write(&mut self, data: u32) -> DapAck {
        let (ack, _) = self.memap_bd2(data, false);
        ack
    }
    fn memap_bd3(&mut self, data: u32, read: bool) -> (DapAck, u32) {
        self.memap(MemapAddress::BD3, data, read)
    }
    fn memap_bd3_read(&mut self) -> (DapAck, u32) {
        self.memap_bd3(0, true)
    }
    fn memap_bd3_write(&mut self, data: u32) -> DapAck {
        let (ack, _) = self.memap_bd3(data, false);
        ack
    }

    fn memap_base_u32_read(&mut self) -> (DapAck, u32) {
        self.memap(MemapAddress::BASElo, 0, true)
    }
    fn memap_base_u64_read(&mut self) -> (DapAck, u64) {
        let (ack, lo) = self.memap(MemapAddress::BASElo, 0, true);
        let (ack, hi) = self.memap(MemapAddress::BASEhi, 0, true);
        let address: u64 = ((hi as u64) << 32) | (lo as u64);
        (ack, address)
    }
}

// TODO: dpを借用かつmutexを取れるように持つ
// DAPとAPは1対1で張り付くので、APが複数あるとDAPも複数になるため
pub struct DAP<T> {
    dp: T,
    apnum: u8,
}

impl<T: DapInterface> DAP<T> {
    pub fn new(dp: T) -> Self {
        let mut dap = DAP { dp: dp, apnum: 0 };
        dap.init();
        dap
    }

    fn init(&mut self) {
        self.dp_select_write(0, 0, 0);

        // debug reset
        // let mut ctrl = CtrlStatus(0);
        // ctrl.set_CDBGRSTREQ(1);
        // self.dp_ctrlstat_write(ctrl);
        // while {
        //     let (_, ctrl) = self.dp_ctrlstat_read();
        //     debug!("requesting reset: {:?}", ctrl);
        //     ctrl.CDBGRSTACK() != 1
        // } {}
        // let ctrl = CtrlStatus(0);
        // self.dp_ctrlstat_write(ctrl);

        let (ack, ctrl) = self.dp_ctrlstat_read();
        debug!("first ctrl: {:?}, {:?}", ack, ctrl);

        let mut ctrl = CtrlStatus(0);
        ctrl.set_CDBGPWRUPREQ(1);
        ctrl.set_CSYSPWRUPREQ(1);
        ctrl.set_STICKYERR(1);
        self.dp_ctrlstat_write(ctrl);
        let mut ctrl = CtrlStatus(0);
        ctrl.set_CDBGPWRUPREQ(1);
        ctrl.set_CSYSPWRUPREQ(1);
        ctrl.set_STICKYERR(0);
        self.dp_ctrlstat_write(ctrl);

        // power up
        while {
            let (ack, ctrl) = self.dp_ctrlstat_read();
            debug!("requesting powerup: {:?}, {:?}", ack, ctrl);
            ctrl.CDBGPWRUPACK() != 1 || ctrl.CSYSPWRUPACK() != 1
        } {}

        // CSW
        let (ack, mut data) = self.memap_csw_read();
        debug!(
            "read CSW: ack: {:?}, data: {:#08x}: {:?}",
            ack, data.0, data
        );
        // enable DbgSwEnable
        data.set_DbgSwEnable(1);
        let ack = self.memap_csw_write(data);
        debug!("write CSW: ack: {:?}", ack);
        let (ack, mut data) = self.memap_csw_read();
        debug!(
            "read CSW: ack: {:?}, data: {:#08x}: {:?}",
            ack, data.0, data
        );
    }
}

impl<T: DapInterface> DapInterface for DAP<T> {
    fn apacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        self.dp.apacc(data, a, RnW)
    }
    fn dpacc(&mut self, data: u32, a: u8, RnW: bool) -> (u8, u32) {
        self.dp.dpacc(data, a, RnW)
    }
}

impl<T: DapInterface> DebugPort for DAP<T> {}

impl<T: DapInterface> MemoryAccessPort for DAP<T> {
    fn memap(&mut self, address: MemapAddress, data: u32, read: bool) -> (DapAck, u32) {
        let address = address as u8;
        let apbanksel = (address & 0xf0) >> 4;
        let address = (address & 0x0f) >> 2;
        self.dp_select_write(self.apnum, apbanksel, 0);
        self.dp.apacc(data, address, read);
        self.dp_rdbuff_read()
    }
}

mod tests {
    use super::*;

    #[test]
    fn bitfield_test() {
        let mut select = DpSelect(0);
        assert_eq!(0, select.0);
        select.set_apsel(1);
        assert_eq!(0x0100_0000, select.0);
        select.set_apbanksel(1);
        assert_eq!(0x0100_0010, select.0);
        select.set_dpbanksel(1);
        assert_eq!(0x0100_0011, select.0);
    }
}

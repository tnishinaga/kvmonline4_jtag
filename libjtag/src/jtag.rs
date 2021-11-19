use bitflags::bitflags;

pub mod dap;
pub mod jtag;
pub mod jtag_state_machine;

pub type JtagPin = u32;

bitflags! {
    #[derive(Default)]
    pub struct JtagBit: u32 {
        const NONE = 0;
        const TMS = 1 << 1;
        const TCK = 1 << 2;
        const TDI = 1 << 3;
        const TDO = 1 << 4;
        const TRST = 1 << 5;
        const SRST = 1 << 6;
    }
}

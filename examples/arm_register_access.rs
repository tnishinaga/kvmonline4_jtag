use anyhow::{Context, Result};
use bingen::bingen;
use chrono;
use log::{debug, error, info, trace, warn};
use spin::mutex::Mutex;

extern crate libjtag;

use libjtag::interface::ftdi_bitbang::FtdiBitBang;
use libjtag::jtag::dap::*;
use libjtag::jtag::jtag::{Jtag, TAP};
use libjtag::target::arm64::*;

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .filter(|meta| {
            !meta.target().contains("jtag_state_machine")
                && !meta.target().contains("dap")
                && !meta.target().contains("ftdi_mpsse")
        })
        .apply()?;
    Ok(())
}

fn main() -> Result<()> {
    setup_logger().unwrap();

    let interface = FtdiBitBang::new(0x15ba, 0x002a, 0, 1, 2, 3, 4, 5, 7);
    let jtag = Mutex::new(Jtag::new(interface));
    let tap = TAP {
        jtag: &jtag,
        ir_len: 4,
    };
    let dap = DAP::new(tap);
    let dap = Mutex::new(dap);
    const MEMAP_DEBUG_BASE_CORE0: u64 = 0x80010000;
    const MEMAP_CTI_BASE_CORE0: u64 = 0x80018000;
    let mut target = A64Target {
        dap: &dap,
        baseaddr: MEMAP_DEBUG_BASE_CORE0,
    };
    let mut cti_core0 = Cti {
        dap: &dap,
        baseaddr: MEMAP_CTI_BASE_CORE0,
    };
    // init
    target.oslar_write(0);
    let mut edrcr = EDRCR(0);
    debug!("Enter debug state");
    debug!("Clear EDSCR.{{TXU,RXO,ERR}}");
    edrcr.set_CBRRQ(1);
    edrcr.set_CSE(1);
    target.edrcr_write(edrcr);
    let edrcr = target.edrcr_read();
    debug!("EDRCR: {:?}", edrcr);

    // halt
    debug!("enable halting debug");
    let mut edscr = target.edscr_read();
    debug!("before EDSCR.HDE = {:?}", target.edscr_read().HDE());
    edscr.set_hde(1);
    target.edscr_write(edscr);
    debug!("after : EDSCR.HDE = {:?}", target.edscr_read().HDE());

    // send halt to core0
    cti_core0.enable();
    cti_core0.channel_gate_disable(0);
    cti_core0.output_trigger_enable(0, 0);
    cti_core0.generate_pulse(0);

    debug!("Read EDSCR to check state");
    let mut edscr = target.edscr_read();
    debug!("RW bits: {:#b}", edscr.RW());
    debug!("STATUS bits: {:#b}", edscr.STATUS());

    // ITRが空になるのを待つ
    while {
        debug!("waiting ITR to empty");
        edscr = target.edscr_read();
        edscr.ITE() == 0
    } {}
    // issue instruction
    debug!("read x0 register value");
    target.register_u32_write(
        Armv8DebugRegisterOffset::EDITR as u64,
        u32::from_le_bytes(bingen!("aarch64-linux-eabi", "msr DBGDTR_EL0, x1")),
    );
    // 実行完了を待つ
    while {
        debug!("waiting ITR to empty");
        edscr = target.edscr_read();
        edscr.ITE() == 0
    } {}

    // バッファに値が来るのを待つ
    while {
        debug!("waiting DTRTX full");
        edscr = target.edscr_read();
        edscr.TXfull() == 0
    } {}
    // バッファを読む
    debug!("read DBGDTRRX_EL0");
    let DBGDTRRX_EL0 = target.register_u32_read(Armv8DebugRegisterOffset::DBGDTRRX_EL0 as u64);
    debug!("data: {:#x}", DBGDTRRX_EL0);
    debug!("read DBGDTRTX_EL0");
    let DBGDTRTX_EL0 = target.register_u32_read(Armv8DebugRegisterOffset::DBGDTRTX_EL0 as u64);
    debug!("data: {:#x}", DBGDTRTX_EL0);

    Ok(())
}

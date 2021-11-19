use anyhow::{Context, Result};
use bingen::bingen;
use chrono;
use libjtag::target::arm64::Armv8DebugRegisterOffset;
use log::{debug, error, info, trace, warn};
use spin::mutex::Mutex;

extern crate libjtag;

use libjtag::interface::ftdi_bitbang::FtdiBitBang;
use libjtag::jtag::dap::*;
use libjtag::jtag::jtag::{Jtag, TAP};

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

#[inline(never)]
fn main() -> Result<()> {
    setup_logger().unwrap();

    let interface = FtdiBitBang::new(0x15ba, 0x002a, 0, 1, 2, 3, 4, 5, 7);
    let jtag = Mutex::new(Jtag::new(interface));
    let tap = TAP {
        jtag: &jtag,
        ir_len: 4,
    };
    let mut dap = DAP::new(tap);

    const MEMAP_DEBUG_BASE_CORE0: u64 = 0x80010000;
    dap.memap_tar_u64_write(MEMAP_DEBUG_BASE_CORE0 + Armv8DebugRegisterOffset::MIDR_EL1 as u64);
    let (ack, data) = dap.memap_bd0(0, true);
    println!("MIDR ACK: {:?}", ack);
    println!("MIDR DATA: {:#x}", data);

    Ok(())
}

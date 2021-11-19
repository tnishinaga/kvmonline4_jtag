use anyhow::{Context, Result};
use bingen::bingen;
use chrono;
use log::{debug, error, info, trace, warn};
use spin::mutex::Mutex;

extern crate libjtag;

use libjtag::interface::ftdi_bitbang::FtdiBitBang;
use libjtag::interface::ftdi_mpsse::FtdiMpsse;
use libjtag::jtag::dap::*;
use libjtag::jtag::jtag::{Jtag, TAP};
use libjtag::target::arm64::*;

fn setup_logger() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
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

    Ok(())
}

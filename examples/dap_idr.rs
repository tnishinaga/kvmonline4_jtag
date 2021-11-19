use anyhow::{Context, Result};
use bingen::bingen;
use chrono;
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

    let interface = FtdiBitBang::new(0x15ba, 0x002a, 0, 1, 2, 3, 4, 5, 7);
    let jtag = Mutex::new(Jtag::new(interface));
    let mut tap = TAP {
        jtag: &jtag,
        ir_len: 4,
    };

    let mut dap = DAP::new(tap);
    let address = MemapAddress::IDR as u8;

    // set APBANK to 0xF0
    let mut select = DpSelect(0);
    select.set_apsel(0);
    select.set_apbanksel(((address & 0xF0) >> 4) as u32);
    select.set_dpbanksel(0);

    dap.dpacc(select.0, DpAddress::SELECT.into(), false);
    let (ack, _) = dap.dpacc(0, DpAddress::RDBUFF.into(), true);
    println!("DP SELECT ACK: {:?}", ack);

    // read MEM-AP IDR
    dap.apacc(0, (address & 0xF) >> 2, true);
    let (ack, data) = dap.dpacc(0, DpAddress::RDBUFF.into(), true);
    println!("MEM-AP IDR ACK: {:?}", ack);
    println!("MEM-AP IDR DATA: {:#x}", data);

    Ok(())
}

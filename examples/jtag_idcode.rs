use anyhow::{Context, Result};
use chrono;
use libjtag::interface::ftdi_mpsse::FtdiMpsse;
use log::{debug, error, info, trace, warn};

extern crate libjtag;

use libjtag::interface::ftdi_bitbang::FtdiBitBang;
use libjtag::jtag::jtag::Jtag;

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

fn main() -> Result<()> {
    setup_logger().unwrap();

    let interface = FtdiBitBang::new(0x15ba, 0x002a, 0, 1, 2, 3, 4, 5, 7);
    // let interface = FtdiMpsse::new(0x15ba, 0x002a, 4, 5);
    let mut jtag = Jtag::new(interface);

    // move to reset
    jtag.write_tms(&[true; 10]);

    // move to Shift-IR
    jtag.write_tms(&[false, true, true, false, false]);

    // set IDCODE instruction(0b1110)
    jtag.raw_write_data(&[false, true, true, true], true);

    // move to Run/Idle via Update-IR from Exit-IR
    jtag.write_tms(&[true, false]);

    // move to Shift-DR
    jtag.write_tms(&[true, false, false]);

    // read IDCODE(32bit) from DR
    let mut data = [false; 32];
    jtag.raw_read_data(&mut data, true);

    // move to Run/Idle via Update-DR from Exit-DR
    jtag.write_tms(&[true, false]);

    let idcode = data.iter().rev().fold(0, |x, y| (x << 1) | *y as u32);
    println!("IDCODE: {:#x}", idcode);

    Ok(())
}

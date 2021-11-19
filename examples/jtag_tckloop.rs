use core::time;
use std::thread;

use anyhow::{Context, Result};
use chrono;
use libjtag::interface::JtagInterface;
use libjtag::jtag::JtagBit;
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
    let mut jtag = Jtag::new(interface);

    loop {
        jtag.interface.raw_write(&[JtagBit::NONE; 10]);
        thread::sleep(time::Duration::from_millis(10));
    }

    Ok(())
}

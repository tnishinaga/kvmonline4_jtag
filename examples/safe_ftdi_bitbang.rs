use anyhow::{Context, Result};
use chrono;
use log::{debug, error, info, trace, warn};
use std::{thread, time};

use safe_ftdi;

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
    let mut device = safe_ftdi::Context::new().unwrap();
    let (vid, pid) = (0x15ba, 0x002a);
    device
        .open(vid, pid)
        .with_context(|| format!("failed to open {:#04x}:{:#04x}", vid, pid))
        .unwrap();
    device.set_baudrate(1000).unwrap();
    // set TCK(ADBUS0) to output
    let bitmask = 1 << 0;
    device
        .set_bitmode(bitmask, safe_ftdi::mpsse::MpsseMode::BITMODE_SYNCBB)
        .unwrap();

    // blink TCK
    info!("blink TCK start");
    loop {
        info!("set TCK to 1");
        device.write_data(&[0x01]).unwrap();
        thread::sleep(time::Duration::from_secs(1));
        info!("set TCK to 0");
        device.write_data(&[0x00]).unwrap();
        thread::sleep(time::Duration::from_secs(1));
    }

    Ok(())
}

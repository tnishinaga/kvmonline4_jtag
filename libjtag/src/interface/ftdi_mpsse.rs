use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use safe_ftdi;
use std::cmp;
use std::collections::HashMap;

use super::JtagInterface;
use crate::jtag::JtagBit;

const CHUNK_SIZE: usize = 512;

struct FtdiJtagPin {
    position: u8,
    input: bool,
    initial_value: bool,
}

impl FtdiJtagPin {
    pub fn to_bit(&self) -> u8 {
        1 << self.position
    }
}

pub struct FtdiMpsse {
    device: safe_ftdi::Context,
    pins: HashMap<String, FtdiJtagPin>,
}

enum MpsseOpcode {
    ClockDataBitsNoReadOutOutRising = 0x1A,
    ClockDataBitsNoReadOutOutFalling = 0x1B,
    ClockDataBitsInandOutLSBfirstInRisingOutFalling = 0x3B,
    ClockDataBitsInandOutLSBfirstInFallingOutRaising = 0x3E,
    ClockDataToTMSpinNoReadOutRising = 0x4A,
    ClockDataToTMSpinNoReadOutFalling = 0x4B,
    ClockDataToTMSpinWithReadInRisingOutRising = 0x6A,
    ClockDataToTMSpinWithReadInFallingOutRising = 0x6B,
    ClockDataToTMSpinWithReadInRisingOutFalling = 0x6E,
    ClockDataToTMSpinWithReadInFallingOutFalling = 0x6F,
    ClockForNbitsWithNoDataTransfer = 0x8E,
}

impl FtdiMpsse {
    pub fn new(vid: u16, pid: u16, srst: u8, trst: u8) -> Self {
        // pins
        let mut pins: HashMap<String, FtdiJtagPin> = HashMap::new();

        pins.insert(
            "tck".to_string(),
            FtdiJtagPin {
                position: 0,
                input: false,
                initial_value: false,
            },
        );
        pins.insert(
            "tdi".to_string(),
            FtdiJtagPin {
                position: 1,
                input: false,
                initial_value: false,
            },
        );
        pins.insert(
            "tdo".to_string(),
            FtdiJtagPin {
                position: 2,
                input: true,
                initial_value: false,
            },
        );
        pins.insert(
            "tms".to_string(),
            FtdiJtagPin {
                position: 3,
                input: false,
                initial_value: true,
            },
        );
        pins.insert(
            "srst".to_string(),
            FtdiJtagPin {
                position: srst,
                input: false,
                initial_value: false,
            },
        );
        pins.insert(
            "trst".to_string(),
            FtdiJtagPin {
                position: trst,
                input: false,
                initial_value: false,
            },
        );
        pins.insert(
            "rtck".to_string(),
            FtdiJtagPin {
                position: 7,
                input: true,
                initial_value: false,
            },
        );

        let mut device = safe_ftdi::Context::new().unwrap();
        device
            .open(vid, pid)
            .with_context(|| format!("failed to open {:#04x}:{:#04x}", vid, pid))
            .unwrap();
        device.set_baudrate(1000).unwrap();

        device
            .set_bitmode(0, safe_ftdi::mpsse::MpsseMode::BITMODE_MPSSE)
            .unwrap();

        let ftdi_mpsse = FtdiMpsse {
            device: device,
            pins: pins,
        };

        ftdi_mpsse.init_mpsse();

        ftdi_mpsse
    }

    fn sync_rxbuffer(&self) {
        // sync rx buffer
        self.device.write_data(&[0xAA]).unwrap();

        let mut tmp = [0];
        self.device.read_data(&mut tmp).unwrap();
        let mut before_data = tmp[0];
        loop {
            self.device.read_data(&mut tmp).unwrap();
            let mut next_data = tmp[0];
            if before_data == 0xFA && next_data == 0xAA {
                break;
            } else {
                before_data = next_data;
            }
        }
    }

    fn init_mpsse(&self) {
        self.sync_rxbuffer();
        // use 60MHz clock
        self.device.write_data(&[0x8A]).unwrap();
        // disable adaptive clock
        self.device.write_data(&[0x97]).unwrap();
        // disable 3 phase clock
        self.device.write_data(&[0x97]).unwrap();
        // setup direction
        let direction = !self
            .pins
            .iter()
            .filter(|x| x.1.input)
            .fold(0 as u16, |x, y| x + y.1.to_bit() as u16);
        // TODO: directionとvalueを自動設定できるようにする
        let direction = 0x0a1b;
        let direction_low = (direction & 0xff) as u8;
        let direction_high = (direction >> 8) as u8;
        let value = self
            .pins
            .iter()
            .filter(|x| x.1.initial_value)
            .fold(0 as u16, |x, y| x + y.1.to_bit() as u16);
        let value = 0x0808;
        let value_low = (value & 0xff) as u8;
        let value_high = (value >> 8) as u8;
        debug!("value: {:#4x}", value);
        debug!("direction: {:#4x}", direction);
        self.device
            .write_data(&[0x80, value_low, direction_low])
            .unwrap();
        self.device
            .write_data(&[0x82, value_high, direction_high])
            .unwrap();
        // setup clock speed(min)
        self.device.write_data(&[0x86, 0xFF, 0xFF]).unwrap();
        // disable loopback
        self.device.write_data(&[0x85]).unwrap();
    }

    // fn separate(&self, data: &[JtagBit]) -> Vec<Vec<JtagBit>>{
    //     let mut separated = vec!(vec!(data[0]));
    //     // TMSを区切りにする
    //     // separate data
    //     for i in 1..data.len() {

    //     }
    //     debug!("separated data: {:?}",separated);
    //     separated
    // }
}

impl JtagInterface for FtdiMpsse {
    fn write_tms(&self, tms: &[bool]) {
        let mut commands: Vec<u8> = Vec::new();
        // Clock Data TMS pin(no read) command can only send 6 bits at a command
        const MPSSE_TMS_BITS_MAX: usize = 7;
        for i in (0..tms.len()).step_by(MPSSE_TMS_BITS_MAX) {
            let rest = tms.len() - i;
            let length = cmp::min(rest, MPSSE_TMS_BITS_MAX) as u8;
            // debug!(
            //     "length {:?}, rest {:?}, tms.len() {:?}",
            //     length,
            //     rest,
            //     tms.len()
            // );
            let mut byte1 = 0;
            for j in 0..length {
                byte1 = byte1 | ((tms[i + j as usize] as u8) << j);
            }
            commands.push(MpsseOpcode::ClockDataToTMSpinNoReadOutFalling as u8);
            commands.push(length - 1);
            commands.push(byte1);
        }
        // debug!("write_tms tms: {:?}", tms);
        // debug!("write_tms commands: {:?}", commands);
        let res = self.device.write_data(commands.as_slice()).unwrap() as usize;
        if res != commands.len() {
            panic!("write failed at write_tms");
        }
    }

    fn write_data(&self, tdi: &[bool], exit: bool) {
        let mut commands: Vec<u8> = Vec::new();
        let tdi_length = tdi.len() - if exit { 1 } else { 0 };
        for i in (0..tdi_length).step_by(8) {
            let rest = tdi_length - i;
            let length = cmp::min(rest, 8) as u8;
            let mut byte1 = 0;
            for j in 0..length {
                byte1 = byte1 | ((tdi[i + j as usize] as u8) << j);
            }
            commands.push(MpsseOpcode::ClockDataBitsInandOutLSBfirstInRisingOutFalling as u8);
            commands.push(length - 1);
            commands.push(byte1);
        }
        if exit {
            // Append TMS bit
            let byte1 = 3 | if *tdi.last().unwrap() { 0x80 } else { 0 };
            commands.push(MpsseOpcode::ClockDataToTMSpinNoReadOutFalling as u8);
            commands.push(0);
            commands.push(byte1);
        }
        // debug!(
        //     "write_data commands({} bytes): {:?}",
        //     commands.len(),
        //     commands
        // );
        self.device.write_data(commands.as_slice()).unwrap();
    }
    fn read_data(&self, tditdo: &mut [bool], exit: bool) {
        // sync
        // TODO: improve
        let mut buffer = [0; CHUNK_SIZE];
        let res = self.device.read_data(&mut buffer).unwrap();
        debug!("read {:?} bytes", res);
        // self.sync_rxbuffer();

        // TODO: CHUNK_SIZE超えを処理する
        let mut commands: Vec<u8> = Vec::new();
        // "Clock Data Bits In and Out LSB first" command cannot send tms
        let tditdo_length = tditdo.len() - if exit { 1 } else { 0 };
        for i in (0..tditdo_length).step_by(8) {
            let rest = tditdo_length - i;
            let length = cmp::min(rest, 8) as u8;
            // debug!(
            //     "length {:?}, rest {:?}, tditdo_length {:?}, tditdo.len() {:?}",
            //     length,
            //     rest,
            //     tditdo_length,
            //     tditdo.len()
            // );
            let mut byte1 = 0;
            for j in 0..length {
                byte1 = byte1 | ((tditdo[i + j as usize] as u8) << j);
            }
            commands.push(MpsseOpcode::ClockDataBitsInandOutLSBfirstInRisingOutFalling as u8);
            commands.push(length - 1);
            commands.push(byte1);
        }
        if exit {
            // Append TMS bit
            let byte1 = 3 | if *tditdo.last().unwrap() { 0x80 } else { 0 };
            // commands.push(MpsseOpcode::ClockDataToTMSpinWithReadInFallingOutRising as u8);
            commands.push(0x6B);
            commands.push(0);
            commands.push(byte1);
        }
        // debug!("read_data tditdo({} bits): {:?}", tditdo.len(), tditdo);
        // debug!(
        //     "read_data commands({} bytes): {:?}",
        //     commands.len(),
        //     commands
        // );
        // https://gist.github.com/bjornvaktaren/d2461738ec44e3ad8b3bae4ce69445b4#file-minimal_spi-cpp-L96
        self.device.purge_usb_tx_buffer().unwrap();
        self.device.purge_usb_rx_buffer().unwrap();
        self.device.write_data(commands.as_mut_slice()).unwrap();
        let mut buffer = [0; CHUNK_SIZE];
        let buffer_idx_max = tditdo.len() / 8 + if (tditdo.len() % 8) != 0 { 1 } else { 0 };
        // let res = self.device.read_data(&mut buffer[0..buffer_idx_max]).unwrap();
        let res = self.device.read_data(&mut buffer).unwrap();

        // [u8] to [bool]
        for i in 0..tditdo.len() {
            let index = i / 8;
            let position = i % 8;
            tditdo[i] = buffer[index] & (1 << position) != 0;
        }

        debug!("read/write {:?} bits, buffer {:?} bytes", tditdo.len(), res);
        debug!("{:?}", buffer);
        let mut reversed_buffer = Vec::new();
        reversed_buffer.extend_from_slice(&buffer[0..(res as usize)]);
        reversed_buffer.reverse();
        debug!(
            "read buffer: {:?}",
            reversed_buffer
                .iter()
                .map(|x| format!("{:02x}", x))
                .collect::<String>()
        );

        // debug!("tditdo: {:?}", tditdo);
    }

    fn raw_read(&self, data: &mut [JtagBit]) {
        unimplemented!();
    }

    fn raw_write(&self, data: &[JtagBit]) {
        unimplemented!();
    }
}

use anyhow::{Context, Result};
use log::{debug, error, info, warn};
use safe_ftdi;
use std::cmp;
use std::collections::HashMap;
use std::{thread, time};

use crate::interface::JtagInterface;
use crate::jtag::JtagBit;

const CHUNK_SIZE: usize = 512;

struct FtdiJtagPin {
    position: u8,
    input: bool,
}

impl FtdiJtagPin {
    pub fn to_bit(&self) -> u8 {
        1 << self.position
    }
}

struct FtdiJtagPins {
    tck: FtdiJtagPin,
    tms: FtdiJtagPin,
    tdi: FtdiJtagPin,
    tdo: FtdiJtagPin,
    rtck: FtdiJtagPin,
    trst: FtdiJtagPin,
    srst: FtdiJtagPin,
}

pub struct FtdiBitBang {
    device: safe_ftdi::Context,
    pins: HashMap<String, FtdiJtagPin>,
}

impl FtdiBitBang {
    pub fn new(
        vid: u16,
        pid: u16,
        tck: u8,
        tdi: u8,
        tdo: u8,
        tms: u8,
        srst: u8,
        trst: u8,
        rtck: u8,
    ) -> Self {
        // pins
        let mut pins: HashMap<String, FtdiJtagPin> = HashMap::new();
        pins.insert(
            "tck".to_string(),
            FtdiJtagPin {
                position: tck,
                input: false,
            },
        );
        pins.insert(
            "tdi".to_string(),
            FtdiJtagPin {
                position: tdi,
                input: false,
            },
        );
        pins.insert(
            "tdo".to_string(),
            FtdiJtagPin {
                position: tdo,
                input: true,
            },
        );
        pins.insert(
            "tms".to_string(),
            FtdiJtagPin {
                position: tms,
                input: false,
            },
        );
        pins.insert(
            "srst".to_string(),
            FtdiJtagPin {
                position: srst,
                input: false,
            },
        );
        pins.insert(
            "trst".to_string(),
            FtdiJtagPin {
                position: trst,
                input: false,
            },
        );
        pins.insert(
            "rtck".to_string(),
            FtdiJtagPin {
                position: rtck,
                input: true,
            },
        );

        let mut device = safe_ftdi::Context::new().unwrap();
        device
            .open(vid, pid)
            .with_context(|| format!("failed to open {:#04x}:{:#04x}", vid, pid))
            .unwrap();
        device.set_baudrate(10000).unwrap();
        // set gpio in/out
        let bitmask = !pins
            .iter()
            .filter(|x| x.1.input)
            .fold(0, |x, y| x + y.1.to_bit());
        device
            .set_bitmode(bitmask, safe_ftdi::mpsse::MpsseMode::BITMODE_SYNCBB)
            .unwrap();

        // device.set_read_chunk_size(CHUNK_SIZE).unwrap();
        // device.set_write_chunk_size(CHUNK_SIZE).unwrap();

        FtdiBitBang {
            device: device,
            pins: pins,
        }
    }

    fn pins_to_u8(&self, pins: &JtagBit) -> u8 {
        let mut data = 0;
        data = data
            | if pins.contains(JtagBit::TMS) {
                1 << self.pins["tms"].position
            } else {
                0
            };
        data = data
            | if pins.contains(JtagBit::TCK) {
                1 << self.pins["tck"].position
            } else {
                0
            };
        data = data
            | if pins.contains(JtagBit::TDI) {
                1 << self.pins["tdi"].position
            } else {
                0
            };
        data = data
            | if pins.contains(JtagBit::TDO) {
                1 << self.pins["tdo"].position
            } else {
                0
            };
        data = data
            | if pins.contains(JtagBit::TRST) {
                1 << self.pins["trst"].position
            } else {
                0
            };
        data = data
            | if pins.contains(JtagBit::SRST) {
                1 << self.pins["srst"].position
            } else {
                0
            };
        data
    }

    fn u8_to_pins(&self, pins: u8) -> JtagBit {
        let jtagpin = {
            JtagBit::empty()
                | if (pins & (1 << self.pins["tms"].position)) != 0 {
                    JtagBit::TMS
                } else {
                    JtagBit::NONE
                }
                | if (pins & (1 << self.pins["tck"].position)) != 0 {
                    JtagBit::TCK
                } else {
                    JtagBit::NONE
                }
                | if (pins & (1 << self.pins["tdi"].position)) != 0 {
                    JtagBit::TDI
                } else {
                    JtagBit::NONE
                }
                | if (pins & (1 << self.pins["tdo"].position)) != 0 {
                    JtagBit::TDO
                } else {
                    JtagBit::NONE
                }
                | if (pins & (1 << self.pins["trst"].position)) != 0 {
                    JtagBit::TRST
                } else {
                    JtagBit::NONE
                }
                | if (pins & (1 << self.pins["srst"].position)) != 0 {
                    JtagBit::SRST
                } else {
                    JtagBit::NONE
                }
        };

        jtagpin
    }
}

impl JtagInterface for FtdiBitBang {
    fn raw_read(&self, data: &mut [JtagBit]) {
        // purge rx data
        // TODO: read_data実行時間が遅い原因を探る
        let mut tmp = [0; CHUNK_SIZE];
        self.device.read_data(&mut tmp).unwrap();

        // debug!("raw_read: {:?}", data);

        let mut buffer: Vec<u8> = Vec::new();

        for i in 0..data.len() {
            let pin = self.pins_to_u8(&data[i]);
            buffer.push(pin);
            buffer.push(pin | self.pins["tck"].to_bit());
        }
        self.device.write_data(buffer.as_slice()).unwrap();
        self.device.read_data(buffer.as_mut_slice()).unwrap();
        for i in 0..data.len() {
            data[i] = self.u8_to_pins(buffer[2 * i + 1]);
        }
    }
    fn raw_write(&self, data: &[JtagBit]) {
        // with clock version
        let mut vec = Vec::with_capacity(data.len() * 2);

        // debug!("raw_write: {:?}", data);

        for d in data {
            let pin_value = self.pins_to_u8(&d);
            vec.push(pin_value);
            vec.push(pin_value | (self.pins["tck"].to_bit()));
        }

        self.device.write_data(vec.as_slice()).unwrap();
    }
}

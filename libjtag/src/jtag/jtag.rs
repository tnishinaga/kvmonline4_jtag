use spin::mutex::Mutex;

use core::cmp;

use jep106;
use log::{debug, error, info, warn};
use rust_fsm::*;

use crate::interface::JtagInterface;
use crate::jtag::jtag_state_machine::{JtagState as JS, JtagStateMachine};

use super::JtagBit as JB;

const TAP_DEVICE_MAX: usize = 2;

pub struct Jtag<T> {
    pub interface: T,
    state_machine: StateMachine<JtagStateMachine>,
    idcodes: [Option<u32>; TAP_DEVICE_MAX],
}

impl<T: JtagInterface> Jtag<T> {
    pub fn new(interface: T) -> Self {
        let jtag_state_machine: StateMachine<JtagStateMachine> = StateMachine::new();

        let mut jtag = Jtag {
            interface,
            state_machine: jtag_state_machine,
            idcodes: [None; TAP_DEVICE_MAX],
        };
        jtag.scan();

        // set initial state
        jtag.change_state(JS::Reset);

        jtag
    }

    pub fn state(&self) -> JS {
        *self.state_machine.state()
    }

    pub fn write_tms(&mut self, tms: &[bool]) {
        self.interface.write_tms(tms);
        for i in 0..tms.len() {
            self.state_machine.consume(&tms[i]).unwrap();
        }
    }

    pub fn raw_write_data(&mut self, tdi: &[bool], exit: bool) {
        self.interface.write_data(tdi, exit);
        if exit {
            self.state_machine.consume(&true).unwrap();
        }
    }

    pub fn raw_read_data(&mut self, tditdo: &mut [bool], exit: bool) {
        self.interface.read_data(tditdo, exit);
        if exit {
            self.state_machine.consume(&true).unwrap();
        }
    }

    pub fn change_state(&mut self, to: JS) {
        let from = self.state_machine.state();

        if (*from == to) && to != JS::Reset {
            // do nothing
            return ();
        }

        match (from, to) {
            (_, JS::Reset) => self.write_tms(&[true; 5]),
            (JS::Reset, JS::RunIdle) => self.write_tms(&[false]),
            (JS::Reset, _) => {
                self.change_state(JS::RunIdle);
                self.change_state(to);
            }
            (JS::RunIdle, JS::RunIdle) => self.write_tms(&[false]),
            (JS::SelectDRScan | JS::SelectIRScan, JS::RunIdle) => {
                self.write_tms(&[false, true, true, false])
            }
            (JS::CaptureDR | JS::CaptureIR, JS::RunIdle) => self.write_tms(&[true, true, false]),
            (JS::ShiftDR | JS::ShiftIR, JS::RunIdle) => self.write_tms(&[true, true, false]),
            (JS::Exit1DR | JS::Exit1IR, JS::RunIdle) => self.write_tms(&[true, false]),
            (JS::PauseDR | JS::PauseIR, JS::RunIdle) => self.write_tms(&[true, true, false]),
            (JS::Exit2DR | JS::Exit2IR, JS::RunIdle) => self.write_tms(&[true, false]),
            (JS::UpdateDR | JS::UpdateIR, JS::RunIdle) => self.write_tms(&[false]),

            (JS::RunIdle, JS::CaptureDR) => self.write_tms(&[true, false]),
            (JS::RunIdle, JS::ShiftDR) => self.write_tms(&[true, false, false]),

            (JS::Exit1DR, JS::UpdateDR) => self.write_tms(&[true]),
            (JS::RunIdle, JS::CaptureIR) => self.write_tms(&[true, true, false]),
            (JS::RunIdle, JS::ShiftIR) => self.write_tms(&[true, true, false, false]),

            (JS::Exit1IR, JS::UpdateIR) => self.write_tms(&[true]),

            (_, _) => panic!("not supported from {:?} to {:?}", from, to),
        }
    }

    pub fn write_ir(&mut self, ir_bitstream: &mut [bool], exit: bool, reverse: bool) {
        // TODO: remove mut from ir_bitstream
        match self.state_machine.state() {
            JS::Reset | JS::RunIdle | JS::ShiftIR => (),
            _ => self.change_state(JS::RunIdle),
        };
        self.change_state(JS::ShiftIR);

        if reverse {
            ir_bitstream.reverse();
        }
        self.raw_write_data(ir_bitstream, exit);
        if reverse {
            ir_bitstream.reverse();
        }
        // Exit1 -> RunIdle
        self.change_state(JS::RunIdle);
    }

    pub fn read_write_dr(
        &mut self,
        data: &mut [bool],
        exit: bool,
        reverse_input: bool,
        reverse_output: bool,
    ) {
        match self.state_machine.state() {
            JS::Reset | JS::RunIdle | JS::ShiftDR => (),
            _ => self.change_state(JS::RunIdle),
        };
        self.change_state(JS::ShiftDR);

        if reverse_input {
            data.reverse();
        }

        self.raw_read_data(data, exit);

        if reverse_output {
            data.reverse();
        }
        // Exit1 -> RunIdle
        self.change_state(JS::RunIdle);
    }

    pub fn scan(&mut self) {
        // IDCODEスキャンを行う
        debug!("change state to Reset");
        self.change_state(JS::Reset);
        debug!("change state to ShiftDR");
        self.change_state(JS::ShiftDR);
        // send 0x0ff
        let mut data = [false; TAP_DEVICE_MAX * 32];
        let mut dummy_id: u32 = 0x0000_00ff;
        for i in 0..32 {
            data[i] = (dummy_id & 1) != 0;
            dummy_id >>= 1;
        }
        debug!("write dummy id");
        self.read_write_dr(&mut data, true, false, false);

        let mut i = 0;
        let mut tap_device_counter = 0;
        let end = data.len();
        while i < end {
            if data[i] {
                // 頭が1ならIDCODEの可能性あり
                // 残り31bitを調査
                let idcode = data[(i..i + 32)]
                    .iter()
                    .rev()
                    .fold(0, |x, y| (x << 1) | *y as u32);
                i += 32;
                if idcode == 0x0000_00ff {
                    break;
                }
                // Continuation code
                let cc = (idcode >> 8) & 0b1111;
                // Identity code
                let id = (idcode >> 1) & 0b0111_1111;
                info!(
                    "{} device (IDCODE:{:#08x}) found",
                    jep106::JEP106Code::new(cc as u8, id as u8)
                        .get()
                        .unwrap_or("Unknown"),
                    idcode
                );
                self.idcodes[tap_device_counter] = Some(idcode);
                tap_device_counter += 1;
            } else {
                info!("bypass device found");
                self.idcodes[tap_device_counter] = Some(0);
                tap_device_counter += 1;
                i += 1;
            }
        }
    }
}

pub struct TAP<'a, T: JtagInterface> {
    pub jtag: &'a Mutex<Jtag<T>>,
    pub ir_len: usize,
}

impl<'a, T: JtagInterface> TAP<'a, T> {
    // TODO: IRの位置をずらす機能の追加
    pub fn write_instruction(&mut self, instruction: u8) {
        let mut ir = [false; 8];
        let mut tmp = instruction;
        for i in 0..self.ir_len {
            ir[i] = (tmp & 1) != 0;
            tmp = tmp >> 1;
        }
        let mut jtag = self.jtag.lock();
        jtag.write_ir(&mut ir[0..self.ir_len], true, false);
        drop(jtag);
    }
    pub fn read_write_dr(
        &mut self,
        data: &mut [bool],
        exit: bool,
        reverse_input: bool,
        reverse_output: bool,
    ) {
        let mut jtag = self.jtag.lock();
        jtag.read_write_dr(data, exit, reverse_input, reverse_output);
        drop(jtag);
    }
}

impl<'a, T: JtagInterface> Drop for TAP<'a, T> {
    fn drop(&mut self) {
        let mut jtag = self.jtag.lock();
        jtag.change_state(JS::Reset);
    }
}

mod tests {
    use super::*;

    struct DummyInterface;
    impl JtagInterface for DummyInterface {
        fn write_tms(&self, tms: &[bool]) {
            ()
        }
        fn write_data(&self, tdi: &[bool], exit: bool) {
            ()
        }
        fn read_data(&self, tditdo: &mut [bool], exit: bool) {
            ()
        }

        fn raw_write(&self, pins: &[JB]) {
            ()
        }
        fn raw_read(&self, buffer: &mut [JB]) {
            // return 0x0000_00ff to pass IR scan
            let mut dummy = 0x0000_00ff;
            for i in 0..32 {
                buffer[i] = if (dummy & 1) != 0 {
                    JB::TDO
                } else {
                    JB::empty()
                };
                dummy >>= 1;
            }
        }
    }

    impl<T: JtagInterface> Jtag<T> {
        pub fn debug_set_state(&mut self, to: JS) {
            // change state to reset
            for i in 0..5 {
                self.state_machine.consume(&true).unwrap();
            }
            let seq: &[bool] = match to {
                JS::Reset => &[true],
                JS::RunIdle => &[false],
                JS::SelectDRScan => &[false, true],
                JS::CaptureDR => &[false, true, false],
                JS::ShiftDR => &[false, true, false, false],
                JS::Exit1DR => &[false, true, false, true],
                JS::PauseDR => &[false, true, false, true, false],
                JS::Exit2DR => &[false, true, false, true, false, true],
                JS::UpdateDR => &[false, true, false, true, true],
                JS::SelectIRScan => &[false, true, true],
                JS::CaptureIR => &[false, true, true, false],
                JS::ShiftIR => &[false, true, true, false, false],
                JS::Exit1IR => &[false, true, true, false, true],
                JS::PauseIR => &[false, true, true, false, true, false],
                JS::Exit2IR => &[false, true, true, false, true, false, true],
                JS::UpdateIR => &[false, true, true, false, true, true],
            };

            for s in seq {
                self.state_machine.consume(&s).unwrap();
            }
        }
    }

    #[test]
    fn change_state_test() {
        let interface = DummyInterface;
        let mut jtag = Jtag::new(interface);

        // to Reset
        let froms = [
            JS::Reset,
            JS::RunIdle,
            JS::SelectDRScan,
            JS::CaptureDR,
            JS::ShiftDR,
            JS::Exit1DR,
            JS::PauseDR,
            JS::Exit2DR,
            JS::UpdateDR,
            JS::SelectIRScan,
            JS::CaptureIR,
            JS::ShiftIR,
            JS::Exit1IR,
            JS::PauseIR,
            JS::Exit2IR,
            JS::UpdateIR,
        ];
        for s in froms {
            jtag.debug_set_state(s);
            assert_eq!(s, jtag.state());
            jtag.change_state(JS::Reset);
            assert_eq!(
                JS::Reset,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                s,
                JS::Reset,
                s,
                jtag.state()
            );
        }

        // to RunIdle
        let to = JS::RunIdle;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            jtag.change_state(to);
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to CaptureDR
        let froms = [JS::Reset, JS::RunIdle];
        let to = JS::CaptureDR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to ShiftDR
        let froms = [JS::Reset, JS::RunIdle];
        let to = JS::ShiftDR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to UpdateDR
        let froms = [JS::Exit1DR];
        let to = JS::UpdateDR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to CaptureIR
        let froms = [JS::Reset, JS::RunIdle];
        let to = JS::CaptureIR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to ShiftIR
        let froms = [JS::Reset, JS::RunIdle];
        let to = JS::ShiftIR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }

        // to UpdateIR
        let froms = [JS::Exit1IR];
        let to = JS::UpdateIR;
        for from in froms {
            jtag.debug_set_state(from);
            assert_eq!(from, jtag.state());
            println!("{:?}", jtag.state());
            jtag.change_state(to);
            println!("{:?}", jtag.state());
            assert_eq!(
                to,
                jtag.state(),
                "expected state {:?} -> {:?}, but actual state {:?} -> {:?}",
                from,
                to,
                from,
                jtag.state()
            );
        }
    }
}

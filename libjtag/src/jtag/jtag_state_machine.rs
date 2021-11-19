use log::{debug, error, info, warn};
use rust_fsm::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JtagState {
    Reset,
    RunIdle,
    SelectDRScan,
    CaptureDR,
    ShiftDR,
    Exit1DR,
    PauseDR,
    Exit2DR,
    UpdateDR,
    SelectIRScan,
    CaptureIR,
    ShiftIR,
    Exit1IR,
    PauseIR,
    Exit2IR,
    UpdateIR,
}

#[derive(Debug, PartialEq)]
pub struct JtagOutputHoge;

#[derive(Debug)]
pub struct JtagStateMachine;

impl StateMachineImpl for JtagStateMachine {
    type Input = bool;
    type State = JtagState;
    type Output = JtagOutputHoge;

    const INITIAL_STATE: Self::State = JtagState::Reset;

    fn transition(state: &Self::State, input: &Self::Input) -> Option<Self::State> {
        let res = match (state, input) {
            // Reset
            (JtagState::Reset, &true) => Some(JtagState::Reset),
            (JtagState::Reset, &false) => Some(JtagState::RunIdle),

            // RunIdle
            (JtagState::RunIdle, &true) => Some(JtagState::SelectDRScan),
            (JtagState::RunIdle, &false) => Some(JtagState::RunIdle),

            // DR
            // SelectDRScan
            (JtagState::SelectDRScan, &true) => Some(JtagState::SelectIRScan),
            (JtagState::SelectDRScan, &false) => Some(JtagState::CaptureDR),
            // CaptureDR
            (JtagState::CaptureDR, &true) => Some(JtagState::Exit1DR),
            (JtagState::CaptureDR, &false) => Some(JtagState::ShiftDR),
            // ShiftDR
            (JtagState::ShiftDR, &true) => Some(JtagState::Exit1DR),
            (JtagState::ShiftDR, &false) => Some(JtagState::ShiftDR),
            // Exit1DR
            (JtagState::Exit1DR, &true) => Some(JtagState::UpdateDR),
            (JtagState::Exit1DR, &false) => Some(JtagState::PauseDR),
            // PauseDR
            (JtagState::PauseDR, &true) => Some(JtagState::Exit2DR),
            (JtagState::PauseDR, &false) => Some(JtagState::PauseDR),
            // Exit2DR
            (JtagState::Exit2DR, &true) => Some(JtagState::UpdateDR),
            (JtagState::Exit2DR, &false) => Some(JtagState::ShiftDR),
            // UpdateDR
            (JtagState::UpdateDR, &true) => Some(JtagState::SelectDRScan),
            (JtagState::UpdateDR, &false) => Some(JtagState::RunIdle),

            // IR
            // SelectIRScan
            (JtagState::SelectIRScan, &true) => Some(JtagState::Reset),
            (JtagState::SelectIRScan, &false) => Some(JtagState::CaptureIR),
            // CaptureIR
            (JtagState::CaptureIR, &true) => Some(JtagState::Exit1IR),
            (JtagState::CaptureIR, &false) => Some(JtagState::ShiftIR),
            // ShiftIR
            (JtagState::ShiftIR, &true) => Some(JtagState::Exit1IR),
            (JtagState::ShiftIR, &false) => Some(JtagState::ShiftIR),
            // Exit1IR
            (JtagState::Exit1IR, &true) => Some(JtagState::UpdateIR),
            (JtagState::Exit1IR, &false) => Some(JtagState::PauseIR),
            // PauseIR
            (JtagState::PauseIR, &true) => Some(JtagState::Exit2IR),
            (JtagState::PauseIR, &false) => Some(JtagState::PauseIR),
            // Exit2IR
            (JtagState::Exit2IR, &true) => Some(JtagState::UpdateIR),
            (JtagState::Exit2IR, &false) => Some(JtagState::ShiftIR),
            // UpdateIR
            (JtagState::UpdateIR, &true) => Some(JtagState::SelectIRScan),
            (JtagState::UpdateIR, &false) => Some(JtagState::RunIdle),
        };
        debug!("jtag state change: {:?} -> {:?}", state, res.unwrap());
        res
    }
    fn output(state: &Self::State, input: &Self::Input) -> Option<Self::Output> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut machine: StateMachine<JtagStateMachine> = StateMachine::new();
        let res = machine.consume(&false).unwrap();
        println!("{:?}", machine.state());
    }

    // #[test]
    // fn coverage_test() {
    //     let jtag_states = [
    //         JtagState::Reset,
    //         JtagState::Reset,
    //         JtagState::RunIdle,
    //         JtagState::SelectDRScan,
    //         JtagState::CaptureDR,
    //         JtagState::ShiftDR,
    //         JtagState::Exit1DR,
    //         JtagState::PauseDR,
    //         JtagState::Exit2DR,
    //         JtagState::UpdateDR,
    //         JtagState::SelectIRScan,
    //         JtagState::CaptureIR,
    //         JtagState::ShiftIR,
    //         JtagState::Exit1IR,
    //         JtagState::PauseIR,
    //         JtagState::Exit2IR,
    //         JtagState::UpdateIR,
    //     ];

    //     for from in jtag_states {
    //         let mut machine: StateMachine<JtagStateMachine> = StateMachine::new();
    //         for x in route!(JtagState::Reset, from) {
    //             machine.consume(x).unwrap();
    //         }

    //         for to in jtag_states {
    //             let mut machine2 = machine.clone();
    //             for x in route!(JtagState::Reset, JtagState::SelectDRScan) {
    //                 machine.consume(x).unwrap();
    //             }
    //             assert_eq!(&to, machine.state());
    //         }
    //     }

    // }
}

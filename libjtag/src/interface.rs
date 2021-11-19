use crate::jtag::JtagBit;

#[cfg(feature = "std")]
pub mod ftdi_bitbang;
#[cfg(feature = "std")]
pub mod ftdi_mpsse;

pub trait JtagInterface {
    fn write_tms(&self, tms: &[bool]) {
        let data: Vec<_> = tms
            .iter()
            .map(|x| if *x { JtagBit::TMS } else { JtagBit::empty() })
            .collect();
        self.raw_write(data.as_slice());
    }
    fn write_data(&self, tdi: &[bool], exit: bool) {
        let mut data: Vec<_> = tdi
            .iter()
            .map(|x| if *x { JtagBit::TDI } else { JtagBit::empty() })
            .collect();
        if exit {
            let last = data.last_mut().unwrap();
            *last = *last | JtagBit::TMS;
        }
        self.raw_write(data.as_slice());
    }
    fn read_data(&self, tditdo: &mut [bool], exit: bool) {
        let mut data: Vec<_> = tditdo
            .iter()
            .map(|x| if *x { JtagBit::TDI } else { JtagBit::empty() })
            .collect();
        if exit {
            let last = data.last_mut().unwrap();
            *last = *last | JtagBit::TMS;
        }
        self.raw_read(data.as_mut_slice());
        for i in 0..tditdo.len() {
            tditdo[i] = data[i].contains(JtagBit::TDO);
        }
    }

    fn raw_write(&self, data: &[JtagBit]);
    fn raw_read(&self, data: &mut [JtagBit]);
}

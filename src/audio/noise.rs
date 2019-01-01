use super::{AudioChannelRegs, AudioChannelGen};
use super::common::*;

#[derive(Clone)]
pub struct NoiseRegs {
    length_reg:         u8,
    output_lev_reg:     u8,
    vol_envelope_reg:   u8,
    init_reg:           u8,
}

impl NoiseRegs {
    pub fn new() -> Self {
        NoiseRegs {
            length_reg:         0,
            output_lev_reg:     0,
            vol_envelope_reg:   0,
            init_reg:           0,
        }
    }
}

impl AudioChannelRegs for NoiseRegs {
    fn read_nrx1(&self) -> u8 {
        self.length_reg
    }
    fn read_nrx2(&self) -> u8 {
        self.output_lev_reg
    }
    fn read_nrx3(&self) -> u8 {
        self.vol_envelope_reg
    }
    fn read_nrx4(&self) -> u8 {
        self.init_reg
    }

    fn write_nrx1(&mut self, val: u8) {
        self.length_reg = val;
    }
    fn write_nrx2(&mut self, val: u8) {
        self.output_lev_reg = val;
    }
    fn write_nrx3(&mut self, val: u8) {
        self.vol_envelope_reg = val;
    }
    fn write_nrx4(&mut self, val: u8) {
        self.init_reg = val;
    }

    fn triggered(&mut self) -> bool {
        if (self.init_reg & 0x80) != 0 {
            self.init_reg &= 0x7F;
            return true;
        } else {
            return false;
        }
    }
}

pub struct NoiseGen {
    sample_rate: usize
}

impl NoiseGen {
    pub fn new(sample_rate: usize) -> Self {
        NoiseGen {
            sample_rate:    sample_rate,
        }
    }
}

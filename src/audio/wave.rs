use super::{AudioChannelRegs, AudioChannelGen};
use super::common::*;

#[derive(Clone)]
pub struct WaveRegs {
    on_off_reg:     u8,
    length_reg:     u8,
    output_lev_reg: u8,
    freq_lo_reg:    u8,
    freq_hi_reg:    u8,

    samples:        [u8; 16],
}

impl WaveRegs {
    pub fn new() -> Self {
        WaveRegs {
            on_off_reg:     0,
            length_reg:     0,
            output_lev_reg: 0,
            freq_lo_reg:    0,
            freq_hi_reg:    0,

            samples:        [0; 16],
        }
    }

    pub fn read_nrx0(&self) -> u8 {
        self.on_off_reg
    }

    pub fn write_nrx0(&mut self, val: u8) {
        self.on_off_reg = val;
    }

    pub fn read_wave(&self, loc: u16) -> u8 {
        self.samples[loc as usize]
    }

    pub fn write_wave(&mut self, loc: u16, val: u8) {
        self.samples[loc as usize] = val;
    }
}

impl AudioChannelRegs for WaveRegs {
    fn read_nrx1(&self) -> u8 {
        self.length_reg
    }
    fn read_nrx2(&self) -> u8 {
        self.output_lev_reg
    }
    fn read_nrx3(&self) -> u8 {
        self.freq_lo_reg
    }
    fn read_nrx4(&self) -> u8 {
        self.freq_hi_reg
    }

    fn write_nrx1(&mut self, val: u8) {
        self.length_reg = val;
    }
    fn write_nrx2(&mut self, val: u8) {
        self.output_lev_reg = val;
    }
    fn write_nrx3(&mut self, val: u8) {
        self.freq_lo_reg = val;
    }
    fn write_nrx4(&mut self, val: u8) {
        self.freq_hi_reg = val;
    }

    fn triggered(&mut self) -> bool {
        if (self.freq_hi_reg & 0x80) != 0 {
            self.freq_hi_reg &= 0x7F;
            return true;
        } else {
            return false;
        }
    }
}

pub struct WaveGen {
    sample_rate: usize
}

impl WaveGen {
    pub fn new(sample_rate: usize) -> Self {
        WaveGen {
            sample_rate:    sample_rate,
        }
    }
}

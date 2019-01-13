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

impl AudioChannelGen<NoiseRegs> for NoiseGen {
    fn init_signal(&mut self, regs: &NoiseRegs) {

    }

    fn generate_signal(&mut self, buffer: &mut [u8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            /*if self.phase > self.duty_len {
                *i = 0;
            } else {
                *i = self.amplitude;
            }
            self.phase = (self.phase + 1) % self.phase_len;

            self.amp_counter += 1;
            if self.amp_counter >= self.amp_sweep_step {
                self.amplitude = match self.amp_sweep_dir {
                    AmpDirection::Increase => ((self.amplitude as u16) + 1) as u8,
                    AmpDirection::Decrease => ((self.amplitude as i16) - 1) as u8,
                    AmpDirection::None => self.amplitude,
                };
            }*/
            *i = 0;
        }
    }
}

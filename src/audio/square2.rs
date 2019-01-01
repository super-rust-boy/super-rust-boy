use super::{AudioChannelRegs, AudioChannelGen};
use common::*;

#[derive(Clone)]
pub struct Square2Regs {
    duty_length_reg:    u8,
    vol_envelope_reg:   u8,
    freq_lo_reg:        u8,
    freq_hi_reg:        u8,
}

impl Square2Regs {
    pub fn new() -> Self {
        Square2Regs {
            duty_length_reg:    0,
            vol_envelope_reg:   0,
            freq_lo_reg:        0,
            freq_hi_reg:        0,
        }
    }
}

impl AudioChannelRegs for Square2Regs {
    fn read_nrx1(&self) -> u8 {
        self.duty_length_reg
    }
    fn read_nrx2(&self) -> u8 {
        self.vol_envelope_reg
    }
    fn read_nrx3(&self) -> u8 {
        self.freq_lo_reg
    }
    fn read_nrx4(&self) -> u8 {
        self.freq_hi_reg
    }

    fn write_nrx1(&mut self, val: u8) {
        self.duty_length_reg = val;
    }
    fn write_nrx2(&mut self, val: u8) {
        self.vol_envelope_reg = val;
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

pub struct Square2Gen {
    sample_rate: usize,

    phase:          usize,
    phase_len:      usize,
    duty_len:       usize,

    amplitude:      u8,
    amp_sweep_step: usize,
    amp_counter:    usize,
    amp_sweep_dir:  AmpDirection,
}

impl Square2Gen {
    pub fn new(sample_rate: usize) -> Self {
        Square2Gen {
            sample_rate:    sample_rate,

            phase:          0,
            phase_len:      0,
            duty_len:       0,

            amplitude:      0,
            amp_sweep_step: 0,
            amp_counter:    0,
            amp_sweep_dir:  Direction::None,
        }
    }

    pub fn init_signal(&mut self, regs: Square2Regs) {
        let freq_n = ((regs.freq_hi_reg as usize) << 8) | (regs.freq_lo_reg as usize);
        let frequency = FREQ_MAX / (FREQ_MOD - freq_n);

        self.phase_len = self.sample_rate / frequency;
        self.duty_len = match regs.duty_length_reg & 0xC0 {
            DUTY_12_5   => phase_len / 8,
            DUTY_25     => phase_len / 4,
            DUTY_50     => phase_len / 2,
            DUTY_75     => (phase_len / 4) * 3,
            _           => phase_len / 2,
        };

        self.amplitude = (regs.vol_envelope_reg & 0xF0) >> 4;
        self.amp_sweep_step = (self.sample_rate * (regs.vol_envelope_reg & 0x7)) / 64;
        self.amp_counter = 0;
        self.amp_sweep_dir = if self.amp_sweep_step == 0 {
            AmpDirection::None
        } else if (regs.vol_envelope_reg & 0x8) != 0 {
            AmpDirection::Increase
        } else {
            AmpDirection::Decrease
        };
    }
}

impl AudioChannelGen for Square2Gen {
    fn generate_signal(&mut self, buffer: &mut [u8], start: f32) {
        let skip = ((self.sample_rate as f32) * start) as usize;

        for i in buffer.iter_mut().skip(skip) {
            if self.phase > self.on_len {
                *i = 0;
            } else {
                *i = self.amplitude;
            }
            self.phase = (self.phase + 1) % self.phase_len;

            self.amp_counter += 1;
            if self.amp_counter >= self.amp_sweep_step {
                self.amplitude = match self.dir {
                    AmpDirection::Increase => ((self.amplitude as u16) + 1) as u8,
                    AmpDirection::Increase => ((self.amplitude as i16) - 1) as u8,
                    AmpDirection::None => self.amplitude,
                };
            }
        }
    }
}

use super::{AudioChannelRegs, AudioChannelGen};
use super::common::*;

#[derive(Clone)]
pub struct Square1Regs {
    sweep_reg:          u8,
    duty_length_reg:    u8,
    vol_envelope_reg:   u8,
    freq_lo_reg:        u8,
    freq_hi_reg:        u8,
}

impl Square1Regs {
    pub fn new() -> Self {
        Square1Regs {
            sweep_reg:          0,
            duty_length_reg:    0,
            vol_envelope_reg:   0,
            freq_lo_reg:        0,
            freq_hi_reg:        0,
        }
    }

    pub fn read_nrx0(&self) -> u8 {
        self.sweep_reg
    }

    pub fn write_nrx0(&mut self, val: u8) {
        self.sweep_reg = val;
    }
}

impl AudioChannelRegs for Square1Regs {
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

pub struct Square1Gen {
    sample_rate:    usize,

    frequency:      usize,

    phase:          usize,
    phase_len:      usize,
    duty_len:       usize,

    amplitude:      u8,
    amp_sweep_step: usize,
    amp_counter:    usize,
    amp_sweep_dir:  AmpDirection,
}

impl Square1Gen {
    pub fn new(sample_rate: usize) -> Self {
        Square1Gen {
            sample_rate:    sample_rate,

            frequency:      0,

            phase:          0,
            phase_len:      0,
            duty_len:       0,

            amplitude:      0,
            amp_sweep_step: 0,
            amp_counter:    0,
            amp_sweep_dir:  AmpDirection::None,
        }
    }
}

impl AudioChannelGen<Square1Regs> for Square1Gen {
    fn init_signal(&mut self, regs: &Square1Regs) {
        let freq_n = ((regs.freq_hi_reg as usize) << 8) | (regs.freq_lo_reg as usize);
        self.frequency = FREQ_MAX / (FREQ_MOD - freq_n);

        self.phase_len = self.sample_rate / self.frequency;
        self.duty_len = match regs.duty_length_reg & 0xC0 {
            DUTY_12_5   => self.phase_len / 8,
            DUTY_25     => self.phase_len / 4,
            DUTY_50     => self.phase_len / 2,
            DUTY_75     => (self.phase_len / 4) * 3,
            _           => self.phase_len / 2,
        };

        self.amplitude = (regs.vol_envelope_reg & 0xF0) >> 4;
        self.amp_sweep_step = (self.sample_rate * (regs.vol_envelope_reg & 0x7) as usize) / 64;
        self.amp_counter = 0;
        self.amp_sweep_dir = if self.amp_sweep_step == 0 {
            AmpDirection::None
        } else if (regs.vol_envelope_reg & 0x8) != 0 {
            AmpDirection::Increase
        } else {
            AmpDirection::Decrease
        };
    }

    fn generate_signal(&mut self, buffer: &mut [u8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            if self.phase > self.duty_len {
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
            }
        }
    }
}

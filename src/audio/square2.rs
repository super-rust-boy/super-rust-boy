use super::{AudioChannelRegs, AudioChannelGen};
use super::common::*;

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

    length:         Option<usize>,

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
            phase_len:      1,
            duty_len:       0,

            length:         None,

            amplitude:      0,
            amp_sweep_step: 0,
            amp_counter:    0,
            amp_sweep_dir:  AmpDirection::None,
        }
    }
}

impl AudioChannelGen<Square2Regs> for Square2Gen {
    fn init_signal(&mut self, regs: &Square2Regs) {
        let freq_n = (((regs.freq_hi_reg & 0x7) as usize) << 8) | (regs.freq_lo_reg as usize);
        let frequency = FREQ_MAX / (FREQ_MOD - freq_n);

        self.phase_len = self.sample_rate / frequency;
        self.duty_len = match regs.duty_length_reg & 0xC0 {
            DUTY_12_5   => self.phase_len / 8,
            DUTY_25     => self.phase_len / 4,
            DUTY_50     => self.phase_len / 2,
            DUTY_75     => (self.phase_len / 4) * 3,
            _           => self.phase_len / 2,
        };

        self.length = if (regs.freq_hi_reg & 0x40) != 0 {
            Some((self.sample_rate * (64 - (regs.duty_length_reg & 0x3F) as usize)) / 256)
        } else {
            None
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
            if (self.length.unwrap_or(1) > 0) && (self.phase < self.duty_len) {
                *i = self.amplitude;
            } else {
                *i = 0
            }
            self.phase = (self.phase + 1) % self.phase_len;

            match self.length {
                Some(n) if n > 0 => self.length = Some(n - 1),
                _ => {},
            }

            self.amp_counter += 1;
            if self.amp_counter >= self.amp_sweep_step {
                match self.amp_sweep_dir {
                    AmpDirection::Increase => {
                        if self.amplitude < 15 {
                            self.amplitude += 1;
                        }
                    },
                    AmpDirection::Decrease => {
                        if self.amplitude > 0 {
                            self.amplitude -= 1;
                        }
                    },
                    AmpDirection::None => {},
                }
            }
        }
    }
}

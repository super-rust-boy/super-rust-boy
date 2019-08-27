use super::{AudioChannelRegs, AudioChannelGen};
use super::common::*;

#[derive(Clone)]
pub struct Square2Regs {
    duty_length_reg:    u8,
    vol_envelope_reg:   u8,
    freq_lo_reg:        u8,
    freq_hi_reg:        u8,

    trigger:            bool,
}

impl Square2Regs {
    pub fn new() -> Self {
        Square2Regs {
            duty_length_reg:    0,
            vol_envelope_reg:   0,
            freq_lo_reg:        0,
            freq_hi_reg:        0,

            trigger:            false,
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
        self.freq_hi_reg & 0x7F
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
        self.trigger = true;
        self.freq_hi_reg = val;
    }

    fn triggered(&mut self) -> bool {
        if self.trigger {
            self.trigger = false;
            return true;
        } else {
            return false;
        }
    }
}

pub struct Square2Gen {
    enable:             bool,

    sample_rate:        usize,

    phase_int_count:    usize,
    phase_frac_count:   f32,
    phase_int_len:      usize,
    phase_frac_len:     f32,
    extra_sample:       bool,
    duty_len:           usize,

    length:         Option<usize>,

    amplitude:      i8,
    amp_sweep_step: usize,
    amp_counter:    usize,
    amp_sweep_dir:  Direction,
}

impl Square2Gen {
    pub fn new(sample_rate: usize) -> Self {
        Square2Gen {
            enable:             false,
            sample_rate:        sample_rate,

            phase_int_count:    0,
            phase_frac_count:   0.0,
            phase_int_len:      1,
            phase_frac_len:     0.0,
            extra_sample:       false,
            duty_len:           0,

            length:         None,

            amplitude:      0,
            amp_sweep_step: 0,
            amp_counter:    0,
            amp_sweep_dir:  Direction::None,
        }
    }

    fn phase_step(&mut self) {
        self.phase_int_count = (self.phase_int_count + 1).checked_rem(self.phase_int_len).unwrap_or(0);

        if self.phase_int_count == 0 {
            if !self.extra_sample {
                self.phase_frac_count += self.phase_frac_len;
                if self.phase_frac_count >= 1.0 {
                    self.phase_frac_count -= 1.0;
                    self.phase_int_count = self.phase_int_len.checked_sub(1).unwrap_or(0);
                    self.extra_sample = true;
                }
            } else {
                self.extra_sample = false;
            }
        }
    }

    fn amp_sweep(&mut self) {
        self.amp_counter += 1;
        if self.amp_counter >= self.amp_sweep_step {
            match self.amp_sweep_dir {
                Direction::Increase => {
                    if self.amplitude < 15 {
                        self.amplitude += 1;
                    }
                },
                Direction::Decrease => {
                    if self.amplitude > 0 {
                        self.amplitude -= 1;
                    }
                },
                Direction::None => {},
            }
            self.amp_counter = 0;
        }
    }
}

impl AudioChannelGen<Square2Regs> for Square2Gen {
    fn init_signal(&mut self, regs: &Square2Regs) {
        self.enable = regs.freq_hi_reg & 0x80 != 0;

        if self.enable {
            let freq_n = (((regs.freq_hi_reg & 0x7) as usize) << 8) | (regs.freq_lo_reg as usize);
            let frequency = FREQ_MAX / (FREQ_MOD - freq_n) as f32;

            let true_phase = (self.sample_rate as f32) / frequency;
            self.phase_int_len = true_phase.trunc() as usize;
            self.phase_frac_len = true_phase.fract();
            self.duty_len = match regs.duty_length_reg & 0xC0 {
                DUTY_12_5   => self.phase_int_len / 8,
                DUTY_25     => self.phase_int_len / 4,
                DUTY_50     => self.phase_int_len / 2,
                DUTY_75     => (self.phase_int_len / 4) * 3,
                _           => self.phase_int_len / 2,
            };
            self.phase_int_count = 0;
            self.phase_frac_count = 0.0;
            self.extra_sample = false;

            self.length = if (regs.freq_hi_reg & 0x40) != 0 {
                Some((self.sample_rate * (64 - (regs.duty_length_reg & 0x3F) as usize)) / 256)
            } else {
                None
            };

            self.amplitude = ((regs.vol_envelope_reg & 0xF0) >> 4) as i8;
            self.amp_sweep_step = (self.sample_rate * (regs.vol_envelope_reg & 0x7) as usize) / 64;
            self.amp_counter = 0;
            self.amp_sweep_dir = if self.amp_sweep_step == 0 {
                Direction::None
            } else if (regs.vol_envelope_reg & 0x8) != 0 {
                Direction::Increase
            } else {
                Direction::Decrease
            };
        }
    }

    fn generate_signal(&mut self, buffer: &mut [i8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            if self.enable {
                *i = if (self.length.unwrap_or(1) > 0) && (self.phase_int_count < self.duty_len) {
                    self.amplitude  // HI
                } else if self.length == Some(0) {
                    0               // OFF
                } else {
                    -self.amplitude // LO
                };
                
                self.phase_step();

                match self.length {
                    Some(n) if n > 0 => self.length = Some(n - 1),
                    _ => {},
                }

                self.amp_sweep();
            } else {
                *i = 0;
            }
        }
    }

}

use super::{AudioChannelRegs, AudioChannelGen};
use super::common::FREQ_MOD;

const FREQ_MAX: f32 = 4_194_304.0 / 2.0;

#[derive(Clone)]
pub struct WaveRegs {
    on_off_reg:     u8,
    length_reg:     u8,
    output_lev_reg: u8,
    freq_lo_reg:    u8,
    freq_hi_reg:    u8,

    samples:        [u8; 16],

    trigger:        bool,
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

            trigger:        false,
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
        self.freq_hi_reg & 0x7F
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

pub struct WaveGen {
    enable:             bool,

    sample_rate:        usize,

    phase_int_count:    usize,
    phase_frac_count:   f32,
    phase_int_len:      usize,
    phase_frac_len:     f32,

    index:              usize,
    samples:            [i8; 32],

    length:             Option<usize>,

    sound_on:           bool,
    output_shift:       Option<usize>,
}

impl WaveGen {
    pub fn new(sample_rate: usize) -> Self {
        WaveGen {
            enable:             false,

            sample_rate:        sample_rate,

            phase_int_count:    0,
            phase_frac_count:   0.0,
            phase_int_len:      1,
            phase_frac_len:     0.0,

            index:              0,
            samples:            [0; 32],

            length:             None,

            sound_on:           false,
            output_shift:       Some(0),
        }
    }
}

impl AudioChannelGen<WaveRegs> for WaveGen {
    fn init_signal(&mut self, regs: &WaveRegs) {
        self.enable = regs.freq_hi_reg & 0x80 != 0;
        self.sound_on = (regs.on_off_reg & 0x80) != 0;

        if self.enable {
            let freq_n = (((regs.freq_hi_reg & 0x7) as usize) << 8) | (regs.freq_lo_reg as usize);
            let frequency = FREQ_MAX / (FREQ_MOD - freq_n) as f32;
            let true_phase = self.sample_rate as f32 / frequency;

            // Int and frac parts must be separated (to differentiate high frequencies)
            self.phase_int_len = true_phase.trunc() as usize;
            self.phase_frac_len = true_phase.fract();
            self.phase_int_count = 0;
            self.phase_frac_count = 0.0;

            self.length = if (regs.freq_hi_reg & 0x40) != 0 {
                Some((self.sample_rate * (256 - regs.length_reg as usize)) / 256) // TODO: more precise?
            } else {
                None
            };

            self.output_shift = match (regs.output_lev_reg & 0x60) >> 5 {
                1 => Some(0),
                2 => Some(1),
                3 => Some(2),
                _ => None
            };

            self.index = 0;
            for (i, s) in regs.samples.iter().enumerate() {
                let internal_idx = i * 2;
                self.samples[internal_idx] = (((s >> 4) as i8) - 8) * 2;
                self.samples[internal_idx + 1] = (((s & 0xF) as i8) - 8) * 2;
            }
        }
    }

    fn generate_signal(&mut self, buffer: &mut [i8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            if self.enable {
                // Sample
                *i = if (self.length.unwrap_or(1) > 0) && self.sound_on {
                    if let Some(shift) = self.output_shift {
                        self.samples[self.index] >> shift
                    } else { 0 }
                } else {
                    0
                };

                self.phase_int_count += 1;
                if self.phase_int_count == self.phase_int_len {
                    self.phase_frac_count += self.phase_frac_len;
                    // If the fractional part has 'rolled over', add another sample.
                    if self.phase_frac_count >= 1.0 {
                        self.phase_frac_count -= 1.0;
                    } else {    // Otherwise move onto the next.
                        self.index = (self.index + 1) % 32;
                        self.phase_int_count = 0;
                    }
                } else if self.phase_int_count > self.phase_int_len {
                    self.index = (self.index + 1) % 32;
                    self.phase_int_count = 0;
                }

                match self.length {
                    Some(n) if n > 0 => self.length = Some(n - 1),
                    _ => {},
                }
            } else {
                *i = 0;
            }
        }
    }

}

use super::{AudioChannelRegs, AudioChannelGen};
use super::common::FREQ_MOD;

const FREQ_MAX: usize = 65_536;

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
    sample_rate:    usize,

    phase:          usize,
    phase_len:      usize,

    index:          usize,
    samples:        [u8; 32],

    length:         Option<usize>,

    enable:         bool,
    output_shift:   u8,
}

impl WaveGen {
    pub fn new(sample_rate: usize) -> Self {
        WaveGen {
            sample_rate:    sample_rate,

            phase:          0,
            phase_len:      1,

            index:          0,
            samples:        [0; 32],

            length:         None,

            enable:         false,
            output_shift:   0,
        }
    }
}

impl AudioChannelGen<WaveRegs> for WaveGen {
    fn init_signal(&mut self, regs: &WaveRegs) {
        self.enable = (regs.on_off_reg & 0x80) != 0;

        let freq_n = (((regs.freq_hi_reg & 0x7) as usize) << 8) | (regs.freq_lo_reg as usize);
        let frequency = FREQ_MAX / (FREQ_MOD - freq_n);
        self.phase = 0;
        self.phase_len = self.sample_rate / frequency;

        self.length = if (regs.freq_hi_reg & 0x40) != 0 {
            Some((self.sample_rate * (256 - regs.length_reg as usize)) / 256) // TODO: more precise?
        } else {
            None
        };

        self.output_shift = match (regs.output_lev_reg & 0x60) >> 5 {
            1 => 0,
            2 => 1,
            3 => 2,
            _ => 4
        };

        self.index = 0;
        for (i, s) in regs.samples.iter().enumerate() {
            let internal_idx = i * 2;
            self.samples[internal_idx] = (s >> 4) & 0xF_u8;
            self.samples[internal_idx + 1] = s & 0xF;
        }
    }

    fn generate_signal(&mut self, buffer: &mut [u8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            // Sample
            if (self.length.unwrap_or(1) > 0) && self.enable {
                *i = self.samples[self.index] >> self.output_shift;
            } else {
                *i = 0;
            }

            self.phase += 1;
            if self.phase >= self.phase_len {
                self.index = (self.index + 1) % 32;
                self.phase = 0;
            }

            match self.length {
                Some(n) if n > 0 => self.length = Some(n - 1),
                _ => {},
            }
        }
    }
}

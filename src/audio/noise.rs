use super::{AudioChannelRegs, AudioChannelGen};
use super::common::Direction;

const FREQ_CONST: usize = 524_288;

const DIVISOR: [usize; 8] = [
    8,
    16,
    32,
    48,
    64,
    80,
    96,
    112
];

#[derive(Clone)]
pub struct NoiseRegs {
    length_reg:         u8,
    vol_envelope_reg:   u8,
    poly_counter_reg:   u8,
    init_reg:           u8,

    trigger:            bool,
}

impl NoiseRegs {
    pub fn new() -> Self {
        NoiseRegs {
            length_reg:         0,
            vol_envelope_reg:   0,
            poly_counter_reg:   0,
            init_reg:           0,

            trigger:            false,
        }
    }
}

impl AudioChannelRegs for NoiseRegs {
    fn read_nrx1(&self) -> u8 {
        self.length_reg
    }
    fn read_nrx2(&self) -> u8 {
        self.vol_envelope_reg
    }
    fn read_nrx3(&self) -> u8 {
        self.poly_counter_reg
    }
    fn read_nrx4(&self) -> u8 {
        self.init_reg & 0x7F
    }

    fn write_nrx1(&mut self, val: u8) {
        self.length_reg = val;
    }
    fn write_nrx2(&mut self, val: u8) {
        self.vol_envelope_reg = val;
    }
    fn write_nrx3(&mut self, val: u8) {
        self.poly_counter_reg = val;
    }
    fn write_nrx4(&mut self, val: u8) {
        self.trigger = true;
        self.init_reg = val;
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

pub struct NoiseGen {
    enable:         bool,

    sample_rate:    usize,

    freq_counter:   usize,
    freq_step:      usize,
    rand_counter:   u16,
    counter_width:  bool,

    length:         Option<usize>,

    amplitude:      i8,
    amp_sweep_step: usize,
    amp_counter:    usize,
    amp_sweep_dir:  Direction,
}

impl NoiseGen {
    pub fn new(sample_rate: usize) -> Self {
        NoiseGen {
            enable:         false,

            sample_rate:    sample_rate,

            freq_counter:   0,
            freq_step:      0,

            rand_counter:   0xFFFF,
            counter_width:  false,   // true = 7 bits

            length:         None,

            amplitude:      0,
            amp_sweep_step: 0,
            amp_counter:    0,
            amp_sweep_dir:  Direction::None,
        }
    }
}

impl AudioChannelGen<NoiseRegs> for NoiseGen {
    fn init_signal(&mut self, regs: &NoiseRegs) {
        self.enable = (regs.init_reg & 0x80) != 0;

        let s = ((regs.poly_counter_reg & 0xF0) >> 4) as usize;
        let r = (regs.poly_counter_reg & 0x7) as usize;
        let divisor = DIVISOR[r] << s;

        self.freq_step = self.sample_rate.checked_div(FREQ_CONST / divisor).unwrap_or(1); // number of samples between counter switches
        self.freq_counter = 0;
        self.counter_width = (regs.poly_counter_reg & 8) == 8;

        self.length = if (regs.init_reg & 0x40) != 0 {
            Some((self.sample_rate * (64 - (regs.length_reg & 0x3F) as usize)) / 256) // TODO: more precise?
        } else {
            None
        };

        self.amplitude = ((regs.vol_envelope_reg & 0xF0) >> 4) as i8;
        self.amp_sweep_step = (self.sample_rate * (regs.vol_envelope_reg & 0x7) as usize) / 64; // TODO: more precise?
        self.amp_counter = 0;
        self.amp_sweep_dir = if self.amp_sweep_step == 0 {
            Direction::None
        } else if (regs.vol_envelope_reg & 0x8) != 0 {
            Direction::Increase
        } else {
            Direction::Decrease
        };
    }

    fn generate_signal(&mut self, buffer: &mut [i8], start: f32, end: f32) {
        let take = (buffer.len() as f32 * end) as usize;
        let skip = (buffer.len() as f32 * start) as usize;

        for i in buffer.iter_mut().take(take).skip(skip) {
            if self.enable {
                *i = if (self.length.unwrap_or(1) > 0) && (self.rand_counter & 1 == 1) {
                    self.amplitude  // HI
                } else if self.length == Some(0) {
                    0               // OFF
                } else {
                    -self.amplitude // LO
                };
                
                self.freq_counter += 1;
                if self.freq_counter >= self.freq_step {
                    let low_bit = self.rand_counter & 1;
                    self.rand_counter >>= 1;
                    let xor_bit = (self.rand_counter & 1) ^ low_bit;

                    self.rand_counter &= 0x3FFF;
                    self.rand_counter |= xor_bit << 14;
                    if self.counter_width {
                        self.rand_counter &= 0xFFBF;
                        self.rand_counter |= xor_bit << 6;
                    }

                    self.freq_counter = 0;
                }

                match self.length {
                    Some(n) if n > 0 => self.length = Some(n - 1),
                    _ => {},
                }

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
            } else {
                *i = 0
            }
        }
    }

}

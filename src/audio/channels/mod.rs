// Audio channels.
pub mod square1;
pub mod square2;
pub mod wave;
pub mod noise;

pub type Stereo<T> = [T; 2];

pub trait Channel {
    // Clock the channel and recalculate the output if necessary.
    // Call this with individual CPU cycles.
    fn sample_clock(&mut self, cycles: u32);

    // Call at 256Hz, to decrement the length counter.
    fn length_clock(&mut self);

    // Call at 64Hz, for volume envelope.
    fn envelope_clock(&mut self);

    // Get the current output sample.
    fn get_sample(&self) -> f32;

    // Reset all internat timers and buffers.
    fn reset(&mut self);
}

#[derive(Clone, Copy)]
pub enum SquareDuty {
    Lo,
    Hi
}
const DUTY_0: [SquareDuty; 8] = [SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Hi];
const DUTY_1: [SquareDuty; 8] = [SquareDuty::Hi, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Hi];
const DUTY_2: [SquareDuty; 8] = [SquareDuty::Hi, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Lo, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Hi];
const DUTY_3: [SquareDuty; 8] = [SquareDuty::Lo, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Hi, SquareDuty::Lo];

pub struct DutyCycleCounter {
    pattern:    &'static [SquareDuty; 8],
    index:      usize
}

impl DutyCycleCounter {
    pub fn new(duty: u8) -> Self {
        Self {
            pattern: match duty & 0x3 {
                0 => &DUTY_0,
                1 => &DUTY_1,
                2 => &DUTY_2,
                3 => &DUTY_3,
                _ => unreachable!()
            },
            index: 0
        }
    }

    pub fn step(&mut self) {
        self.index = (self.index + 1) % 8;
    }

    pub fn read(&self) -> SquareDuty {
        self.pattern[self.index]
    }
}

pub const MAX_VOL: u8 = 15;
pub const MIN_VOL: u8 = 0;

pub fn get_freq_modulo(hi_reg: u8, lo_reg: u8) -> u32 {
    const HI_FREQ_MASK: u8 = bits![2, 1, 0];
    let hi = hi_reg & HI_FREQ_MASK;
    make_16!(hi, lo_reg) as u32
}

// Convert from 4-bit samples to 32-bit floating point.
pub fn u4_to_f32(amplitude: u8) -> f32 {
    const MAX_AMP: f32 = 15.0;

    (amplitude as f32) / MAX_AMP
}

// Convert from 4-bit signed samples to 32-bit floating point.
pub fn i4_to_f32(amplitude: u8) -> f32 {
    const MAX_AMP: f32 = 7.5;

    ((amplitude as f32) - MAX_AMP) / MAX_AMP
}
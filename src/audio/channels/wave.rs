use super::*;

const MAX_LEN: u16 = 256;

enum ShiftAmount {
    Mute,
    Full,
    Half,
    Quarter
}

pub struct Wave {
    // Public registers
    pub playback_reg:   u8,
    pub length_reg:     u8,
    pub vol_reg:        u8,
    pub freq_lo_reg:    u8,
    pub freq_hi_reg:    u8,

    // Sample table
    pub wave_pattern:   [u8; 16],

    // Internal registers
    enabled:        bool,
    pattern_index:  usize,

    shift_amount:   ShiftAmount,

    length_counter: u16,
    length_modulo:  u16,

    freq_counter:   u32,
    freq_modulo:    u32,

}

impl Wave {
    pub fn new() -> Self {
        Self {
            playback_reg:   0,
            length_reg:     0,
            vol_reg:        0,
            freq_lo_reg:    0,
            freq_hi_reg:    0,

            wave_pattern:   [0; 16],

            enabled:            false,
            pattern_index:      0,

            shift_amount:       ShiftAmount::Mute,

            length_counter:     0,
            length_modulo:      MAX_LEN,

            freq_counter:       0,
            freq_modulo:        0,
        }
    }

    pub fn set_playback_reg(&mut self, val: u8) {
        self.playback_reg = val;
    }

    pub fn set_length_reg(&mut self, val: u8) {
        self.length_reg = val;
    }

    pub fn set_vol_reg(&mut self, val: u8) {
        self.vol_reg = val;
    }

    pub fn set_freq_lo_reg(&mut self, val: u8) {
        self.freq_lo_reg = val;
    }

    pub fn set_freq_hi_reg(&mut self, val: u8) {
        self.freq_hi_reg = val;
        // And trigger event...
        if test_bit!(val, 7) {
            self.trigger();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn write_wave(&mut self, loc: u16, val: u8) {
        self.wave_pattern[loc as usize] = val;
    }

    pub fn read_wave(&self, loc: u16) -> u8 {
        self.wave_pattern[loc as usize]
    }
}

impl Channel for Wave {
    fn sample_clock(&mut self, cycles: u32) {
        self.freq_counter += cycles;
        if self.freq_counter >= self.freq_modulo {
            self.freq_counter -= self.freq_modulo;
            self.pattern_index = (self.pattern_index + 1) % 32;
        }
    }

    fn length_clock(&mut self) {
        if self.enabled && test_bit!(self.freq_hi_reg, 6) {
            self.length_counter -= 1;
            if self.length_counter == self.length_modulo {
                self.enabled = false;
            }
        }
    }

    fn envelope_clock(&mut self) {
    }

    fn get_sample(&self) -> f32 {
        if self.enabled {
            self.read_wave_pattern()
        } else {
            0.0
        }
    }

    fn reset(&mut self) {
        self.pattern_index = 0;
        self.freq_lo_reg = 0;
        self.freq_hi_reg = 0;

        self.freq_counter = 0;
        self.length_counter = MAX_LEN;

        self.enabled = false;
    }
}

impl Wave {
    fn trigger(&mut self) {
        const SHIFT_MASK: u8 = bits![6, 5];

        self.shift_amount = match (self.vol_reg & SHIFT_MASK) >> 5 {
            0 => ShiftAmount::Mute,
            1 => ShiftAmount::Full,
            2 => ShiftAmount::Half,
            3 => ShiftAmount::Quarter,
            _ => unreachable!()
        };

        self.freq_counter = 0;
        self.freq_modulo = (2048 - get_freq_modulo(self.freq_hi_reg, self.freq_lo_reg)) * 2;

        self.length_counter = MAX_LEN;
        self.length_modulo = self.length_reg as u16;

        self.enabled = true;
    }

    fn read_wave_pattern(&self) -> f32 {
        let u8_index = self.pattern_index / 2;
        let shift = 4 * ((self.pattern_index + 1) % 2);
        let raw_sample = (self.wave_pattern[u8_index] >> shift) & 0xF;

        match self.shift_amount {
            ShiftAmount::Mute => 0.0,
            ShiftAmount::Full => i4_to_f32(raw_sample),
            ShiftAmount::Half => i4_to_f32(raw_sample) * 0.5,
            ShiftAmount::Quarter => i4_to_f32(raw_sample) * 0.25,
        }
    }
}

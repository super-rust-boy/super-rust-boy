// Common stuff for channels

// Square wave duty cycle bits.
pub const DUTY_12_5: u8 = 0b00 << 6;
pub const DUTY_25: u8   = 0b01 << 6;
pub const DUTY_50: u8   = 0b10 << 6;
pub const DUTY_75: u8   = 0b11 << 6;

// Amplitude/frequency sweep direction/setting.
pub enum Direction {
    Increase,
    Decrease,
    None,
}

// Frequency constants
pub const FREQ_MAX: f32 = 131_072.0;
pub const FREQ_MOD: usize = 2048;

// Common stuff for channels

// Square wave duty cycle bits.
pub const DUTY_12_5: u8 = 0b00 << 6;
pub const DUTY_25: u8   = 0b01 << 6;
pub const DUTY_50: u8   = 0b10 << 6;
pub const DUTY_75: u8   = 0b11 << 6;

// Amplitude sweep direction/setting.
pub enum AmpDirection {
    Increase,
    Decrease,
    None,
}

// Frequency constants
pub const FREQ_MAX: usize = 131_072;
pub const FREQ_MOD: usize = 2048;

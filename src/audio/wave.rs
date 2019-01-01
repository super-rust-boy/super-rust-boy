use super::{AudioChannelRegs, AudioChannelGen};
use common::*;

#[derive(Clone)]
pub struct WaveRegs {
    on_off_reg:         u8,
    length_reg:         u8,
    output_lev_reg:     u8,
    freq_lo_reg:        u8,
    freq_hi_reg:        u8,
}

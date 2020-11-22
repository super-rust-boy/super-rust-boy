mod channels;
mod resampler;

use bitflags::bitflags;
use crossbeam_channel::Sender;
use sample::frame::Stereo;

use crate::mem::MemDevice;

pub use resampler::Resampler;
use channels::{
    Channel,
    square1::Square1,
    square2::Square2,
    wave::Wave,
    noise::Noise
};

bitflags! {
    #[derive(Default)]
    struct VolumeControl: u8 {
        const VIN_LEFT  = bit!(7);
        const LEFT_VOL  = bits![6, 5, 4];
        const VIN_RIGHT = bit!(3);
        const RIGHT_VOL = bits![2, 1, 0];
    }
}

impl VolumeControl {
    fn vol_left(&self) -> f32 {
        let bits = (*self & VolumeControl::LEFT_VOL).bits() >> 4;
        bits as f32
    }

    fn vol_right(&self) -> f32 {
        let bits = (*self & VolumeControl::RIGHT_VOL).bits();
        bits as f32
    }
}

bitflags! {
    #[derive(Default)]
    struct ChannelEnables: u8 {
        const LEFT_4    = bit!(7);
        const LEFT_3    = bit!(6);
        const LEFT_2    = bit!(5);
        const LEFT_1    = bit!(4);
        const RIGHT_4   = bit!(3);
        const RIGHT_3   = bit!(2);
        const RIGHT_2   = bit!(1);
        const RIGHT_1   = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    struct PowerControl: u8 {
        const POWER     = bit!(7);
        const PLAYING_4 = bit!(3);
        const PLAYING_3 = bit!(2);
        const PLAYING_2 = bit!(1);
        const PLAYING_1 = bit!(0);
    }
}

impl PowerControl {
    fn is_on(&self) -> bool {
        self.contains(PowerControl::POWER)
    }
}

const SAMPLE_PACKET_SIZE: usize = 32;
const CYCLES_PER_SECOND: usize = 154 * 456 * 60;
const INPUT_SAMPLE_RATE: f64 = 131_072.0;

pub type SamplePacket = Box<[Stereo<f32>]>;

// The structure that exists in memory. Sends data to the audio thread.
pub struct AudioDevice {
    // Raw channel data
    square_1:   Square1,
    square_2:   Square2,
    wave:       Wave,
    noise:      Noise,

    // Control
    volume_control:     VolumeControl,
    channel_enables:    ChannelEnables,
    power_control:      PowerControl,

    // Managing output of samples
    sample_buffer:      Vec<Stereo<f32>>,
    sender:             Option<Sender<SamplePacket>>,
    cycle_count:        f64,
    cycles_per_sample:  f64,

    vol_left:           f32,
    vol_right:          f32,

    // Managing clocking channels
    frame_cycle_count:  u32,
    frame_count:        u8,
}

impl AudioDevice {
    pub fn new() -> Self {
        AudioDevice {
            square_1:   Square1::new(),
            square_2:   Square2::new(),
            wave:       Wave::new(),
            noise:      Noise::new(),

            volume_control:     VolumeControl::default(),
            channel_enables:    ChannelEnables::default(),
            power_control:      PowerControl::default(),

            sample_buffer:      Vec::new(),
            sender:             None,
            cycle_count:        0.0,
            cycles_per_sample:  0.0,

            vol_left:           0.0,
            vol_right:          0.0,

            frame_cycle_count:  0,
            frame_count:        0,
        }
    }

    // Call to enable audio on the appropriate thread (this should be done before any processing)
    pub fn enable_audio(&mut self, sender: Sender<SamplePacket>) {
        self.sender = Some(sender);

        let seconds_per_sample = 1.0 / INPUT_SAMPLE_RATE;
        self.cycles_per_sample = seconds_per_sample * (CYCLES_PER_SECOND as f64);
    }

    pub fn clock(&mut self, cycles: u32) {
        self.cycle_count += cycles as f64;

        // Modify channels
        self.clock_channels(cycles);
        
        if self.cycle_count >= self.cycles_per_sample {
            self.cycle_count -= self.cycles_per_sample;

            // Generate sample
            let sample = self.generate_sample();
            self.sample_buffer.push(sample);
            
            // Output to audio thread
            if self.sample_buffer.len() > SAMPLE_PACKET_SIZE {
                let sample_packet = self.sample_buffer.drain(..).collect::<SamplePacket>();
                if let Some(s) = &self.sender {
                    s.send(sample_packet).expect("Error sending!");
                }
            }
        }
    }
}

impl MemDevice for AudioDevice {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0xFF10  => self.square_1.sweep_reg,
            0xFF11  => self.square_1.duty_length_reg,
            0xFF12  => self.square_1.vol_envelope_reg,
            0xFF13  => self.square_1.freq_lo_reg,
            0xFF14  => self.square_1.freq_hi_reg,

            0xFF16  => self.square_2.duty_length_reg,
            0xFF17  => self.square_2.vol_envelope_reg,
            0xFF18  => self.square_2.freq_lo_reg,
            0xFF19  => self.square_2.freq_hi_reg,

            0xFF1A  => self.wave.playback_reg,
            0xFF1B  => self.wave.length_reg,
            0xFF1C  => self.wave.vol_reg,
            0xFF1D  => self.wave.freq_lo_reg,
            0xFF1E  => self.wave.freq_hi_reg,

            0xFF20  => self.noise.length_reg,
            0xFF21  => self.noise.vol_envelope_reg,
            0xFF22  => self.noise.poly_counter_reg,
            0xFF23  => self.noise.trigger_reg,

            0xFF24  => self.volume_control.bits(),
            0xFF25  => self.channel_enables.bits(),
            0xFF26  => {
                let mut bits = self.power_control;
                bits.set(PowerControl::PLAYING_1, self.square_1.is_enabled());
                bits.set(PowerControl::PLAYING_2, self.square_2.is_enabled());
                bits.set(PowerControl::PLAYING_3, self.wave.is_enabled());
                bits.set(PowerControl::PLAYING_4, self.noise.is_enabled());

                bits.bits()
            },

            0xFF30..=0xFF3F => self.wave.read_wave(loc - 0xFF30),

            _   => 0,
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0xFF10  => self.square_1.set_sweep_reg(val),
            0xFF11  => self.square_1.set_duty_length_reg(val),
            0xFF12  => self.square_1.set_vol_envelope_reg(val),
            0xFF13  => self.square_1.set_freq_lo_reg(val),
            0xFF14  => self.square_1.set_freq_hi_reg(val),

            0xFF16  => self.square_2.set_duty_length_reg(val),
            0xFF17  => self.square_2.set_vol_envelope_reg(val),
            0xFF18  => self.square_2.set_freq_lo_reg(val),
            0xFF19  => self.square_2.set_freq_hi_reg(val),

            0xFF1A  => self.wave.set_playback_reg(val),
            0xFF1B  => self.wave.set_length_reg(val),
            0xFF1C  => self.wave.set_vol_reg(val),
            0xFF1D  => self.wave.set_freq_lo_reg(val),
            0xFF1E  => self.wave.set_freq_hi_reg(val),

            0xFF20  => self.noise.set_length_reg(val),
            0xFF21  => self.noise.set_vol_envelope_reg(val),
            0xFF22  => self.noise.set_poly_counter_reg(val),
            0xFF23  => self.noise.set_trigger_reg(val),

            0xFF24  => {
                const REDUCTION_FACTOR: f32 = 1.0 / (4.0 * 7.0);    // 4 channels, max vol = 7
                let vol_ctrl = VolumeControl::from_bits_truncate(val);
                self.vol_left = vol_ctrl.vol_left() * REDUCTION_FACTOR;
                self.vol_right = vol_ctrl.vol_right() * REDUCTION_FACTOR;
                self.volume_control = vol_ctrl;
            },
            0xFF25  => self.channel_enables = ChannelEnables::from_bits_truncate(val),
            0xFF26  => {
                let power_on = test_bit!(val, 7);
                if !power_on {
                    self.power_control.remove(PowerControl::POWER);
                    self.reset();
                } else {
                    self.power_control.insert(PowerControl::POWER);
                }
            },

            0xFF30..=0xFF3F => self.wave.write_wave(loc - 0xFF30, val),

            _   => {},
        }
    }
}

impl AudioDevice {
    fn generate_sample(&mut self) -> Stereo<f32> {
        if self.power_control.is_on() {
            let square_1 = self.square_1.get_sample();
            let square_2 = self.square_2.get_sample();
            let wave = self.wave.get_sample();
            let noise = self.noise.get_sample();

            let left_1 = if self.channel_enables.contains(ChannelEnables::LEFT_1) {square_1} else {0.0};
            let left_2 = if self.channel_enables.contains(ChannelEnables::LEFT_2) {square_2} else {0.0};
            let left_3 = if self.channel_enables.contains(ChannelEnables::LEFT_3) {wave} else {0.0};
            let left_4 = if self.channel_enables.contains(ChannelEnables::LEFT_4) {noise} else {0.0};

            let right_1 = if self.channel_enables.contains(ChannelEnables::RIGHT_1) {square_1} else {0.0};
            let right_2 = if self.channel_enables.contains(ChannelEnables::RIGHT_2) {square_2} else {0.0};
            let right_3 = if self.channel_enables.contains(ChannelEnables::RIGHT_3) {wave} else {0.0};
            let right_4 = if self.channel_enables.contains(ChannelEnables::RIGHT_4) {noise} else {0.0};

            let left_mixed = (left_1 + left_2 + left_3 + left_4) * self.vol_left;
            let right_mixed = (right_1 + right_2 + right_3 + right_4) * self.vol_right;

            [left_mixed, right_mixed]
        } else {
            [0.0, 0.0]
        }
    }

    fn reset(&mut self) {
        self.square_1.reset();
        self.square_2.reset();
        self.wave.reset();
        self.noise.reset();

        self.volume_control = VolumeControl::default();
        self.channel_enables = ChannelEnables::default();
    }

    fn clock_channels(&mut self, cycles: u32) {
        const FRAME_MODULO: u32 = 8192; // Clock rate / 8192 = 512
        // Advance samples
        self.square_1.sample_clock(cycles);
        self.square_2.sample_clock(cycles);
        self.wave.sample_clock(cycles);
        self.noise.sample_clock(cycles);

        self.frame_cycle_count += cycles;
        // Clock length and sweeping at 512Hz
        if self.frame_cycle_count >= FRAME_MODULO {
            self.frame_cycle_count -= FRAME_MODULO;

            // Clock length at 256Hz
            if self.frame_count % 2 == 0 {
                self.square_1.length_clock();
                self.square_2.length_clock();
                self.wave.length_clock();
                self.noise.length_clock();
            }

            // Clock envelope sweep at 64Hz
            if self.frame_count == 7 {
                self.square_1.envelope_clock();
                self.square_2.envelope_clock();
                self.noise.envelope_clock();
            }
            
            // Clock frequency sweep at 128Hz
            if self.frame_count % 4 == 2 {
                self.square_1.sweep_clock();
            }

            self.frame_count = (self.frame_count + 1) % 8;
        }
    }
}
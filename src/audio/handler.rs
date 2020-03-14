use std::thread;
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex
};

use super::{AudioCommand, AudioChannelGen, AudioChannelRegs};

use super::square1::{Square1Regs, Square1Gen};
use super::square2::{Square2Regs, Square2Gen};
use super::wave::{WaveRegs, WaveGen};
use super::noise::{NoiseRegs, NoiseGen};

use crossbeam_channel::{
    Receiver,
    Sender
};
use bitflags::bitflags;

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

const DIV_4_BIT: f32 = 1.0 / 16.0;
// Convert 4-bit sample to float
macro_rules! sample {
    ( $x:expr ) => {
        {
            (($x as f32) * DIV_4_BIT)
        }
    };
}

type AudioFrame = [f32; 2];

// Receives updates from the AudioDevice, and processes and generates signals.
pub struct AudioHandler {
    receiver:   Receiver<AudioCommand>,

    // Data lists for each note
    square1_data:   VecDeque<(Square1Regs, f32)>,
    square2_data:   VecDeque<(Square2Regs, f32)>,
    wave_data:      VecDeque<(WaveRegs, f32)>,
    noise_data:     VecDeque<(NoiseRegs, f32)>,

    // Signal generators
    square1:    Square1Gen,
    square2:    Square2Gen,
    wave:       WaveGen,
    noise:      NoiseGen,

    // Control values
    sound_on:   bool,
    left_vol:   f32,
    right_vol:  f32,
    channel_enables: ChannelEnables,

    // Raw channel buffers
    buffers:    AudioBuffers,
}

impl AudioHandler {
    pub fn new(recv: Receiver<AudioCommand>, sample_rate: usize) -> Self {
        AudioHandler {
            receiver:   recv,

            square1_data:   VecDeque::new(),
            square2_data:   VecDeque::new(),
            wave_data:      VecDeque::new(),
            noise_data:     VecDeque::new(),

            square1:    Square1Gen::new(sample_rate),
            square2:    Square2Gen::new(sample_rate),
            wave:       WaveGen::new(sample_rate),
            noise:      NoiseGen::new(sample_rate),

            sound_on:   false,
            left_vol:   0.0,
            right_vol:  0.0,
            channel_enables: ChannelEnables::default(),

            buffers:    AudioBuffers::new(sample_rate / 60),
        }
    }

    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for f in buffer.chunks_exact_mut(2) {
            let frame = self.process_frame();
            for (o, i) in f.iter_mut().zip(frame.iter()) {
                *o = *i;
            }
        }
    }

    // Run the main loop (wait for commands)
    /*fn run_loop(&mut self, audio_packet: Arc<Mutex<Vec<f32>>>) {
        loop {
            let command = self.receiver.recv().unwrap();
            match command {
                AudioCommand::Control{
                    channel_control,
                    output_select,
                    on_off,
                } => {
                    let ap = audio_packet.lock().unwrap();
                    self.set_controls(channel_control, output_select, on_off);
                    self.gen_audio_packet(&mut ap);
                },
                AudioCommand::Frame => self.gen_audio_packet(&mut audio_packet.lock().unwrap()),
                AudioCommand::NR1(regs, time) => self.square1_data.push_back((regs, time)),
                AudioCommand::NR2(regs, time) => self.square2_data.push_back((regs, time)),
                AudioCommand::NR3(regs, time) => self.wave_data.push_back((regs, time)),
                AudioCommand::NR4(regs, time) => self.noise_data.push_back((regs, time)),
            }
        }
    }

    // Generate an audio packet (1/60s of audio)
    fn gen_audio_packet(&mut self, audio_packet: &mut Vec<f32>) {
        self.replier.send(()).expect("Couldn't reply from audio thread");

        process_command_buffer(&mut self.square1, &mut self.square1_data, &mut self.buffers.square1);
        process_command_buffer(&mut self.square2, &mut self.square2_data, &mut self.buffers.square2);
        process_command_buffer(&mut self.wave, &mut self.wave_data, &mut self.buffers.wave);
        process_command_buffer(&mut self.noise, &mut self.noise_data, &mut self.buffers.noise);

        // Mix first samples of new data.
        match self.buffers.get_next() {
            Some(vals) => {
                let frame = self.mix_output(vals);
            },
            None => panic!("Can't find any audio."),
        }
    }*/

    // Generator function that produces the next two samples (left & right channel)
    fn process_frame(&mut self) -> AudioFrame {
        let n = self.buffers.next();
        match n {
            Some(vals) => self.mix_output(vals),
            None => {
                // Fetch updates - keep waiting until we get control update.
                loop {
                    let command = self.receiver.recv().unwrap();
                    match command {
                        AudioCommand::Control{
                            channel_control,
                            output_select,
                            on_off,
                        } => {
                            self.set_controls(channel_control, output_select, on_off);
                            break;
                        },
                        AudioCommand::Frame => break,
                        AudioCommand::NR1(regs, time) => self.square1_data.push_back((regs, time)),
                        AudioCommand::NR2(regs, time) => self.square2_data.push_back((regs, time)),
                        AudioCommand::NR3(regs, time) => self.wave_data.push_back((regs, time)),
                        AudioCommand::NR4(regs, time) => self.noise_data.push_back((regs, time)),
                    }
                }

                // Generate signals for each buffer
                process_command_buffer(&mut self.square1, &mut self.square1_data, &mut self.buffers.square1);
                process_command_buffer(&mut self.square2, &mut self.square2_data, &mut self.buffers.square2);
                process_command_buffer(&mut self.wave, &mut self.wave_data, &mut self.buffers.wave);
                process_command_buffer(&mut self.noise, &mut self.noise_data, &mut self.buffers.noise);

                // Mix first samples of new data.
                match self.buffers.next() {
                    Some(vals) => self.mix_output(vals),
                    None => panic!("Can't find any audio."),
                }
            },
        }
    }

    #[inline]
    fn mix_output(&mut self, vals: (i8, i8, i8, i8)) -> AudioFrame {
        if self.sound_on {
            let samp_0 = sample!(vals.0);
            let samp_1 = sample!(vals.1);
            let samp_2 = sample!(vals.2);
            let samp_3 = sample!(vals.3);

            let left_1 = if self.channel_enables.contains(ChannelEnables::LEFT_1) {samp_0} else {0.0};
            let left_2 = if self.channel_enables.contains(ChannelEnables::LEFT_2) {samp_1} else {0.0};
            let left_3 = if self.channel_enables.contains(ChannelEnables::LEFT_3) {samp_2} else {0.0};
            let left_4 = if self.channel_enables.contains(ChannelEnables::LEFT_4) {samp_3} else {0.0};

            let right_1 = if self.channel_enables.contains(ChannelEnables::RIGHT_1) {samp_0} else {0.0};
            let right_2 = if self.channel_enables.contains(ChannelEnables::RIGHT_2) {samp_1} else {0.0};
            let right_3 = if self.channel_enables.contains(ChannelEnables::RIGHT_3) {samp_2} else {0.0};
            let right_4 = if self.channel_enables.contains(ChannelEnables::RIGHT_4) {samp_3} else {0.0};

            let left_finished = (left_1 + left_2 + left_3 + left_4) * self.left_vol;
            let right_finished = (right_1 + right_2 + right_3 + right_4) * self.right_vol;

            [left_finished, right_finished]
        } else {
            [0.0, 0.0]
        }
    }

    fn set_controls(&mut self, channel_control: u8, output_select: u8, on_off: u8) {
        self.sound_on = test_bit!(on_off, 7);

        self.left_vol = if test_bit!(channel_control, 7) {
            0.0
        } else {    // Divide by max value * num of channels * reduction factor
            (((channel_control & 0x70) >> 4) as f32 + 1.0) / 128.0
        };

        self.right_vol = if test_bit!(channel_control, 3) {
            0.0
        } else {
            ((channel_control & 0x7) as f32 + 1.0) / 128.0
        };

        self.channel_enables = ChannelEnables::from_bits_truncate(output_select);
    }
}

#[inline]
fn process_command_buffer<G, R>(gen: &mut G, data: &mut VecDeque<(R, f32)>, buffer: &mut [i8])
    where R: AudioChannelRegs, G: AudioChannelGen<R>
{
    // First note:
    let end_time = if data.len() > 0 {data[0].1} else {1.0};
    gen.generate_signal(buffer, 0.0, end_time);

    for i in 0..data.len() {
        gen.init_signal(&data[i].0);

        let start_time = data[i].1;
        let end_time = if i + 1 < data.len() {data[i + 1].1} else {1.0};

        gen.generate_signal(buffer, start_time, end_time);
    }

    data.clear();
}

struct AudioBuffers {
    square1:    Vec<i8>,
    square2:    Vec<i8>,
    wave:       Vec<i8>,
    noise:      Vec<i8>,

    size:       usize,
    i:          usize,
}

impl AudioBuffers {
    fn new(buffer_size: usize) -> Self {
        AudioBuffers {
            square1:    vec![0; buffer_size],
            square2:    vec![0; buffer_size],
            wave:       vec![0; buffer_size],
            noise:      vec![0; buffer_size],

            size:       buffer_size,
            i:          0,
        }
    }
}

impl Iterator for AudioBuffers {
    type Item = (i8, i8, i8, i8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.size {
            self.i = 0;
            None
        } else {
            let ret = (
                self.square1[self.i],
                self.square2[self.i],
                self.wave[self.i],
                self.noise[self.i]
            );
            self.i += 1;
            Some(ret)
        }
    }
}
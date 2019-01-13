use std::sync::mpsc::Receiver;
use std::thread;
use std::collections::VecDeque;

use super::{AudioCommand, AudioChannelGen, AudioChannelRegs};

use super::square1::{Square1Regs, Square1Gen};
use super::square2::{Square2Regs, Square2Gen};
use super::wave::{WaveRegs, WaveGen};
use super::noise::{NoiseRegs, NoiseGen};

use cpal;

const DIV_4_BIT: f32 = 1.0/15.0;//1.0 / 7.5;
// Convert 4-bit sample to float
macro_rules! sample {
    ( $x:expr ) => {
        {
            ((($x as f32) * DIV_4_BIT)/* - 1.0*/) * 0.25
        }
    };
}

// TODO: better error handling
pub fn start_audio_handler_thread(recv: Receiver<AudioCommand>) {
    thread::spawn(move || {
        let event_loop = cpal::EventLoop::new();

        let device = cpal::default_output_device().expect("no output device available.");

        let mut supported_formats_range = device.supported_output_formats()
            .expect("error while querying formats");

        let format = supported_formats_range.next()
            .expect("No supported format")
            .with_max_sample_rate();

        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();

        let sample_rate = format.sample_rate.0 as usize;
        let mut process = true;
        let mut right_sample = 0.0;

        let mut handler = AudioHandler::new(recv, sample_rate);

        event_loop.play_stream(stream_id);

        event_loop.run(move |_stream_id, stream_data| {
            use cpal::StreamData::*;
            use cpal::UnknownTypeOutputBuffer::*;

            match stream_data {
                Output { buffer: U16(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        if process {
                            let frame = handler.process_frame();
                            *elem = (frame.0 * u16::max_value() as f32) as u16;
                            right_sample = frame.1;
                            process = false;
                        } else {
                            *elem = (right_sample * u16::max_value() as f32) as u16;
                            process = true;
                        }
                    }
                },
                Output { buffer: I16(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        if process {
                            let frame = handler.process_frame();
                            *elem = (frame.0 * i16::max_value() as f32) as i16;
                            right_sample = frame.1;
                            process = false;
                        } else {
                            *elem = (right_sample * i16::max_value() as f32) as i16;
                            process = true;
                        }
                    }
                },
                Output { buffer: F32(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        if process {
                            let frame = handler.process_frame();
                            *elem = frame.0;
                            right_sample = frame.1;
                            process = false;
                        } else {
                            *elem = right_sample;
                            process = true;
                        }
                    }
                },
                _ => {},
            }
        });
    });
}


// Receives updates from the AudioDevice, and processes and generates signals.
struct AudioHandler {
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

    // Control reg buffers
    /*channel_control: Some<u8>,
    output_select:   Some<u8>,
    on_off:          Some<u8>,*/

    // Control values
    sound_on:   bool,
    left_vol:   f32,
    right_vol:  f32,
    left_1:     bool,
    left_2:     bool,
    left_3:     bool,
    left_4:     bool,
    right_1:    bool,
    right_2:    bool,
    right_3:    bool,
    right_4:    bool,

    // Raw channel buffers
    buffers:    AudioBuffers,
}

impl AudioHandler {
    fn new(recv: Receiver<AudioCommand>, sample_rate: usize) -> Self {
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
            left_1:     false,
            left_2:     false,
            left_3:     false,
            left_4:     false,
            right_1:    false,
            right_2:    false,
            right_3:    false,
            right_4:    false,

            buffers:    AudioBuffers::new(sample_rate / 60),
        }
    }

    // Generator function that produces the next two samples (left & right channel)
    fn process_frame(&mut self) -> (f32, f32) {
        let n = self.buffers.get_next();
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
                match self.buffers.get_next() {
                    Some(vals) => self.mix_output(vals),
                    None => panic!("Can't find any audio."),
                }
            },
        }
    }

    #[inline]
    fn mix_output(&mut self, vals: (u8, u8, u8, u8)) -> (f32, f32) {
        if self.sound_on {
            let left_1 = if self.left_1 {sample!(vals.0)} else {0.0};
            let left_2 = if self.left_2 {sample!(vals.1)} else {0.0};
            let left_3 = if self.left_3 {sample!(vals.2)} else {0.0};
            let left_4 = if self.left_4 {sample!(vals.3)} else {0.0};

            let right_1 = if self.right_1 {sample!(vals.0)} else {0.0};
            let right_2 = if self.right_2 {sample!(vals.1)} else {0.0};
            let right_3 = if self.right_3 {sample!(vals.2)} else {0.0};
            let right_4 = if self.right_4 {sample!(vals.3)} else {0.0};

            ((left_1 + left_2 + left_3 + left_4) * self.left_vol,
             (right_1 + right_2 + right_3 + right_4) * self.right_vol)
        } else {
            (0.0, 0.0)
        }
    }

    fn set_controls(&mut self, channel_control: u8, output_select: u8, on_off: u8) {
        /*self.channel_control = channel_control;
        self.output_select = output_select;
        self.on_off = on_off;*/

        self.sound_on = (on_off & 0x80) != 0;

        self.left_vol = if (channel_control & 0x80) != 0 {
            0.0
        } else {
            ((channel_control & 0x70) >> 4) as f32 / 7.0
        };

        self.right_vol = if (channel_control & 0x8) != 0 {
            0.0
        } else {
            (channel_control & 0x7) as f32 / 7.0
        };

        self.left_4  = (output_select & 0x80) != 0;
        self.left_3  = (output_select & 0x40) != 0;
        self.left_2  = (output_select & 0x20) != 0;
        self.left_1  = (output_select & 0x10) != 0;
        self.right_4 = (output_select & 0x08) != 0;
        self.right_3 = (output_select & 0x04) != 0;
        self.right_2 = (output_select & 0x02) != 0;
        self.right_1 = (output_select & 0x01) != 0;
    }
}

#[inline]
fn process_command_buffer<G, R>(gen: &mut G, data: &mut VecDeque<(R, f32)>, buffer: &mut [u8])
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
    square1:    Vec<u8>,
    square2:    Vec<u8>,
    wave:       Vec<u8>,
    noise:      Vec<u8>,

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

    fn get_next(&mut self) -> Option<(u8, u8, u8, u8)> {
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

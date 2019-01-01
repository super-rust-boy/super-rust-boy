use std::sync::mpsc::Receiver;
use std::thread;

use super::AudioCommand;

use square1::Square1Gen;
use square2::Square2Gen;
use wave::WaveGen;
use noise::NoiseGen;

use cpal;

const DIV_4_BIT: f32 = 1 / 7.5;
// Convert 4-bit sample to float
macro_rules! sample {
    ( $x:expr ) => {
        {
            ((($x as f32) / DIV_4_BIT) - 1.0) * 0.25
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

        let handler = AudioHandler::new();

        let sample_rate = format.sample_rate.0 as usize;
        let mut process = true;
        let mut right_sample = 0_u8;

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
                        } else {
                            *elem = (right_sample * u16::max_value() as f32) as u16;
                        }
                    }
                },
                Output { buffer: I16(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        if process {
                            let frame = handler.process_frame();
                            *elem = (frame.0 * i16::max_value() as f32) as u16;
                            right_sample = frame.1;
                        } else {
                            *elem = (right_sample * i16::max_value() as f32) as u16;
                        }
                    }
                },
                Output { buffer: F32(mut buffer) } => {
                    for elem in buffer.iter_mut() {
                        if process {
                            let frame = handler.process_frame();
                            *elem = frame.0;
                            right_sample = frame.1;
                        } else {
                            *elem = right_sample;
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

    // Signal generators
    square1:    Square1Gen,
    square2:    Square2Gen,
    wave:       WaveGen,
    noise:      NoiseGen,

    // Controls
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
    fn new(recv: Receiver<AudioCommand>, buffer_size: usize) -> Self {
        AudioHandler {
            receiver:   recv,

            square1:    Square1Gen::new(),
            square2:    Square2Gen::new(),
            wave:       WaveGen::new(),
            noise:      NoiseGen::new(),

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

            buffers:    AudioBuffers::new(buffer_size),
        }
    }

    // Generator function that produces the next two samples (left & right channel)
    fn process_frame(&mut self) -> (f32, f32) {
        match self.buffers.get_next() {
            Some(vals) => self.mix_output(vals),
            None => {
                // Fetch updates - keep waiting until we get control update
                // Generate signals for each buffer
                // get next and mix
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

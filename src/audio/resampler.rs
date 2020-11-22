// Module that resamples from 32_000 to the output sample rate.
use crossbeam_channel::Receiver;
use sample::{
    frame::{Frame, Stereo},
    interpolate::{Converter, Sinc},
    ring_buffer::Fixed,
    signal::Signal
};

pub struct Resampler {
    converter: Converter<Source, Sinc<[Stereo<f32>; 2]>>
}

impl Resampler {
    pub fn new(receiver: Receiver<super::SamplePacket>, target_sample_rate: f64) -> Self {
        let sinc = Sinc::new(Fixed::from([Stereo::equilibrium(); 2]));
        Resampler {
            converter: Source::new(receiver).from_hz_to_hz(sinc, super::INPUT_SAMPLE_RATE, target_sample_rate)
        }
    }
}

impl Iterator for Resampler {
    type Item = Stereo<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.converter.is_exhausted() {}
        Some(self.converter.next())
    }
}

// TODO: replace this with an async stream?
struct Source {
    receiver:   Receiver<super::SamplePacket>,

    current:    super::SamplePacket,
    n:          usize,
}

impl Source {
    fn new(receiver: Receiver<super::SamplePacket>) -> Self {
        Source {
            receiver:   receiver,

            current:    Box::new([]),
            n:          0,
        }
    }
}

impl Signal for Source {
    type Frame = Stereo<f32>;

    fn next(&mut self) -> Self::Frame {
        if self.n < self.current.len() {
            let out = self.current[self.n];
            self.n += 1;
            out
        } else {
            self.current = self.receiver.recv().unwrap();
            self.n = 1;
            self.current[0]
        }
    }
}
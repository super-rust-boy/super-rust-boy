// Audio handling module

mod common;
mod handler;
mod square1;
mod square2;
mod wave;
mod noise;

use square1::Square1Regs;
use square2::Square2Regs;
use wave::WaveRegs;
use noise::NoiseRegs;

use mem::MemDevice;

use std::sync::mpsc::Sender;

pub use handler::start_audio_handler_thread;


// The structure that exists in memory. Sends data to the audio thread.
pub struct AudioDevice {
    // Raw channel data
    nr1: Square1Regs,
    nr2: Square2Regs,
    nr3: WaveRegs,
    nr4: NoiseRegs,

    // Control
    channel_control: u8,
    output_select:   u8,
    on_off:          u8,

    // Managing audio handler
    update:          bool,
    sender:          Sender<AudioCommand>,
}

impl AudioDevice {
    pub fn new(sender: Sender<AudioCommand>) -> Self {
        AudioDevice {
            nr1: Square1Regs::new(),
            nr2: Square2Regs::new(),
            nr3: WaveRegs::new(),
            nr4: NoiseRegs::new(),

            channel_control: 0,
            output_select:   0,
            on_off:          0,

            update: false,
            sender: sender,
        }
    }

    // Call every instruction to send update
    pub fn send_update(&mut self, cycle_count: u32) {
        // If trigger bit was just written, send timed update
        if self.update {
            if nr1.triggered() {
                sender.send(AudioCommand::NR1(self.nr1.clone())),
            } else if nr2.triggered() {
                sender.send(AudioCommand::NR2(self.nr2.clone())),
            } else if nr3.triggered() {
                sender.send(AudioCommand::NR3(self.nr3.clone())),
            } else if nr4.triggered() {
                sender.send(AudioCommand::NR4(self.nr4.clone())),
            }
        }
    }

    // Send frame batch update
    pub fn frame_update(&self) {
        // End of last frame...
        sender.send(AudioCommand::Control{
            channel_control: self.channel_control,
            output_select:   self.output_select,
            on_off:          self.on_off,
        }).unwrap();

        // ... start of the next frame
        sender.send(AudioCommand::Start).unwrap();
    }
}

impl MemDevice for AudioDevice {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0xFF10  => self.nr1.read_nrx0(),
            0xFF11  => self.nr1.read_nrx1(),
            0xFF12  => self.nr1.read_nrx2(),
            0xFF13  => self.nr1.read_nrx3(),
            0xFF14  => self.nr1.read_nrx4(),

            0xFF16  => self.nr2.read_nrx1(),
            0xFF17  => self.nr2.read_nrx2(),
            0xFF18  => self.nr2.read_nrx3(),
            0xFF19  => self.nr2.read_nrx4(),

            0xFF1A  => self.nr3.read_nrx0(),
            0xFF1B  => self.nr3.read_nrx1(),
            0xFF1C  => self.nr3.read_nrx2(),
            0xFF1D  => self.nr3.read_nrx3(),
            0xFF1E  => self.nr3.read_nrx4(),

            0xFF20  => self.nr4.read_nrx1(),
            0xFF21  => self.nr4.read_nrx2(),
            0xFF22  => self.nr4.read_nrx3(),
            0xFF23  => self.nr4.read_nrx4(),

            0xFF24  => self.channel_control,
            0xFF25  => self.output_select,
            0xFF26  => self.on_off,

            0xFF30...0xFF3F => self.nr3.read_wave(loc - 0xFF30),

            _   => 0,
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0xFF10  => self.nr1.write_nrx0(val),
            0xFF11  => self.nr1.write_nrx1(val),
            0xFF12  => self.nr1.write_nrx2(val),
            0xFF13  => self.nr1.write_nrx3(val),
            0xFF14  => {
                self.nr1.write_nrx4(val);
                self.update = (val & 0x80) != 0;
            },

            0xFF16  => self.nr2.write_nrx1(val),
            0xFF17  => self.nr2.write_nrx2(val),
            0xFF18  => self.nr2.write_nrx3(val),
            0xFF19  => {
                self.nr2.write_nrx4(val);
                self.update = (val & 0x80) != 0;
            },

            0xFF1A  => self.nr3.write_nrx0(val),
            0xFF1B  => self.nr3.write_nrx1(val),
            0xFF1C  => self.nr3.write_nrx2(val),
            0xFF1D  => self.nr3.write_nrx3(val),
            0xFF1E  => {
                self.nr3.write_nrx4(val);
                self.update = (val & 0x80) != 0;
            },

            0xFF20  => self.nr4.write_nrx1(val),
            0xFF21  => self.nr4.write_nrx2(val),
            0xFF22  => self.nr4.write_nrx3(val),
            0xFF23  => {
                self.nr4.write_nrx4(val);
                self.update = (val & 0x80) != 0;
            },

            // If any of the below change, send an update at the end of the frame.
            0xFF24  => self.channel_control = val,
            0xFF25  => self.output_select = val,
            0xFF26  => self.on_off = val,

            0xFF30...0xFF3F => self.nr3.write_wave(loc - 0xFF30, val),

            _   => 0,
        }
    }
}

// Commands to be sent to the AudioHandler asynchronously.
enum AudioCommand {
    Control{
        channel_control: u8,
        output_select:   u8,
        on_off:          u8,
    },
    NR1(Square1Regs, u32),
    NR2(Square2Regs, u32),
    NR3(WaveRegs,    u32),
    NR4(NoiseRegs,   u32),
    Start,
}

// All 4 channels implement these traits:
// This trait is for the cpu-side raw data.
trait AudioChannelRegs {
    fn read_nrx1(&self) -> u8;
    fn read_nrx2(&self) -> u8;
    fn read_nrx3(&self) -> u8;
    fn read_nrx4(&self) -> u8;

    fn write_nrx1(&mut self, val: u8);
    fn write_nrx2(&mut self, val: u8);
    fn write_nrx3(&mut self, val: u8);
    fn write_nrx4(&mut self, val: u8);

    fn triggered(&mut self) -> bool;
}

// This trait is for the audio handler-side.
trait AudioChannelGen {
    fn generate_signal(&mut self, buffer: &mut [u8]);
}

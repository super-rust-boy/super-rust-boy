//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;
mod audio;
mod interrupt;

#[cfg(feature = "debug")]
mod debug;

use time::{Duration, PreciseTime};
use clap::{clap_app, crate_version};

use std::sync::mpsc::channel;

use cpu::CPU;
use video::VideoDevice;
use audio::{
    AudioDevice,
    start_audio_handler_thread
};
use mem::MemBus;

const FRAME_TIME: i64 = 16_666;
//const FRAME_TIME: i64 = 16_743; // 59.73 fps

fn main() {
    let app = clap_app!(rustboy =>
        (version: crate_version!())
        (author: "Simon Cooper")
        (about: "Game Boy emulator.")
        (@arg CART: "The location of the game cart to use.")
        (@arg mute: -m "Mutes the emulator.")
        (@arg green: -g "Uses classic green palette. By default, greyscale palette is used.")
        (@arg save: -s +takes_value "Save file location.")
    );

    let cmd_args = app.get_matches();

    let cart = match cmd_args.value_of("CART") {
        Some(c) => c.to_string(),
        None => panic!("Usage: rustboy [cart name]. Run with --help for more options."),
    };

    let save_file = match cmd_args.value_of("save") {
        Some(c) => c.to_string(),
        None => make_save_name(&cart),
    };

    let greyscale = !cmd_args.is_present("green");

    let (send, recv) = channel();

    let vd = VideoDevice::new(greyscale);
    let ad = AudioDevice::new(send);
    let mem = MemBus::new(&cart, &save_file, vd, ad);

    let mut state = CPU::new(mem);

    if !cmd_args.is_present("mute") {
        start_audio_handler_thread(recv);
    }
    
    if cfg!(feature = "debug") {
        #[cfg(feature = "debug")]
        debug::debug_mode(&mut state);
    } else {
        loop {
            let frame = PreciseTime::now();

            while state.step() {}   // Execute up to v-blanking

            state.frame_update();   // Draw video and read inputs

            while frame.to(PreciseTime::now()) < Duration::microseconds(FRAME_TIME) {};  // Wait until next frame.
        }
    }
}

fn make_save_name(cart_name: &str) -> String {
    match cart_name.find(".") {
        Some(pos) => cart_name[0..pos].to_string() + ".sav",
        None      => cart_name.to_string() + ".sav"
    }
}
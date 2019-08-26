//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;
mod audio;
mod interrupt;

#[cfg(feature = "debug")]
mod debug;

use clap::{clap_app, crate_version};
use chrono::Utc;

use std::sync::mpsc::channel;

use cpu::CPU;
use video::UserPalette;
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
        (about: "Game Boy and Game Boy Color emulator.")
        (@arg CART: "The path to the game cart to use.")
        (@arg debug: -d "Enter debug mode.")
        (@arg mute: -m "Mutes the emulator.")
        (@arg palette: -p +takes_value "Choose a palette. 'g' selects the classic green scheme, 'bw' forces greyscale. By default SGB colour will be used if available.")
        (@arg save: -s +takes_value "Save file path.")
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

    let palette = choose_palette(cmd_args.value_of("palette"));

    let (send, recv) = channel();

    let ad = AudioDevice::new(send);
    let mem = MemBus::new(&cart, &save_file, palette, ad);

    let mut state = CPU::new(mem);

    let mut audio_recv = if !cmd_args.is_present("mute") {
        start_audio_handler_thread(recv);
        None
    } else {
        Some(recv)
    };
    
    if cmd_args.is_present("debug") {
        #[cfg(feature = "debug")]
        debug::debug_mode(&mut state);
    } else {
        loop {
            let frame = Utc::now();

            while state.step() {}   // Execute up to v-blanking

            state.frame_update();   // Draw video and read inputs

            while (Utc::now() - frame) < chrono::Duration::microseconds(FRAME_TIME) {}  // Wait until next frame.

            if let Some(recv) = &mut audio_recv {
                while let Ok(_) = recv.try_recv() {}
            }
        }
    }
}

fn make_save_name(cart_name: &str) -> String {
    match cart_name.find(".") {
        Some(pos) => cart_name[0..pos].to_string() + ".sav",
        None      => cart_name.to_string() + ".sav"
    }
}

fn choose_palette(palette: Option<&str>) -> UserPalette {
    match palette {
        Some(s) => match s {
            "g" => UserPalette::Classic,
            "bw" => UserPalette::Greyscale,
            _ => UserPalette::Default
        },
        None => UserPalette::Default
    }
}
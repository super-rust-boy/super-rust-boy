extern crate rustboy;

mod debug;

use rustboy::{
    RustBoy,
    UserPalette
};

use clap::{clap_app, crate_version};
use chrono::Utc;

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

    let mut rustboy = RustBoy::new(&cart, &save_file, palette, cmd_args.is_present("mute"));
    
    if cmd_args.is_present("debug") {
        //#[cfg(feature = "debug")]
        debug::debug_mode(&mut rustboy);
    } else {
        loop {
            let frame = Utc::now();

            while rustboy.step() {}   // Execute up to v-blanking

            rustboy.frame();   // Draw video and read inputs

            while (Utc::now() - frame) < chrono::Duration::microseconds(FRAME_TIME) {}  // Wait until next frame.
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
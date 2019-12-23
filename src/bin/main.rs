extern crate rustboy;

mod debug;
mod avg;

use rustboy::*;

use clap::{clap_app, crate_version};
use chrono::Utc;
use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    ElementState,
    VirtualKeyCode
};

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

    let mut events_loop = EventsLoop::new();
    let renderer = VulkanRenderer::new(WindowType::Winit(&events_loop));
    let mut rustboy = RustBoy::new(&cart, &save_file, palette, cmd_args.is_present("mute"), renderer);

    //let mut averager = avg::Averager::<i64>::new(60);
    
    if cmd_args.is_present("debug") {
        debug::debug_mode(&mut rustboy);
    } else {
        loop {
            let frame = Utc::now();

            read_inputs(&mut events_loop, &mut rustboy);
            rustboy.frame();

            //averager.add((Utc::now() - frame).num_milliseconds());
            //println!("Frame t: {}ms", averager.get_avg());

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

fn read_inputs(events_loop: &mut EventsLoop, rustboy: &mut RustBoy) {
    events_loop.poll_events(|e| {
        match e {
            Event::WindowEvent {
                window_id: _,
                event: w,
            } => match w {
                WindowEvent::CloseRequested => {
                    ::std::process::exit(0);
                },
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input: k,
                } => {
                    let pressed = match k.state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };
                    match k.virtual_keycode {
                        Some(VirtualKeyCode::X)         => rustboy.set_button(Button::A, pressed),
                        Some(VirtualKeyCode::Z)         => rustboy.set_button(Button::B, pressed),
                        Some(VirtualKeyCode::Space)     => rustboy.set_button(Button::Select, pressed),
                        Some(VirtualKeyCode::Return)    => rustboy.set_button(Button::Start, pressed),
                        Some(VirtualKeyCode::Up)        => rustboy.set_button(Button::Up, pressed),
                        Some(VirtualKeyCode::Down)      => rustboy.set_button(Button::Down, pressed),
                        Some(VirtualKeyCode::Left)      => rustboy.set_button(Button::Left, pressed),
                        Some(VirtualKeyCode::Right)     => rustboy.set_button(Button::Right, pressed),
                        _ => {},
                    }
                },
                WindowEvent::Resized(_) => rustboy.on_resize(),
                _ => {}
            },
            _ => {},
        }
    });
}
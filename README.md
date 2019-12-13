# Super Rust Boy

Game Boy and Game Boy Color emulator written in Rust and Vulkan.

This project features a library which can be used for external use, and a binary that can run the emulator locally (also for debugging purposes).

### Debug Mode
The emulator library can be built in debug mode by enabling the `debug` feature at compile time: `cargo build --features debug`.

To build this for macOS or iOS, MoltenVK is required.

### Making the Binary
To make the binary, run `cargo build --features debug --bin rustboy-bin --release`. To run the binary in debug mode, pass in the argument `--debug`.

### Options
Run with `-p=g` for classic green palette, `-p=bw` for greyscale palette. `-m` to mute. `-s "SAVE_FILE_NAME"` to specify a save file. By default a save file will be created (if needed) with the name of the cart in the same directory, with `.sav` extension.

By default the emulator will try and run the game in colour if it is available. Certain original Game Boy games have unique palettes that will be selected (made by Nintendo). If you don't want these, force the palette with `-p=g` for classic green palette, `-p=bw` for greyscale palette. Running a Game Boy Color game that is compatible with the original Game Boy (such as Pokemon Gold/Silver) will run in classic mode if you set one of these palettes.

### Example:
To run a ROM in debug mode:
`cargo run --features debug --bin rustboy-bin --release -- path/to/ROM.gb --debug`

To run a ROM with a custom save file and using a greyscale palette:
`cargo run --features debug --bin rustboy-bin --release -- path/to/ROM.gb -s=path/to/save_file.sav -p=bw`

### TODO video:
* Better error checking.
* Separate video thread to enable "turbo" mode.
* Optimisations, reintroduce caching where possible.

### TODO audio:
* Noise wave high freq. higher precision.
* Allow changing duty cycle mid-sound.

### TODO other:
* Optimisations in CPU (?)
* Add ability to use preset ROM (internally - for testing)
* MBC 6,7 bank swapping systems
* Save states
* Cleanup

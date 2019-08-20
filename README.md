# Super Rust Boy

Game Boy and Game Boy Color emulator written in Rust and Vulkan.

### Debug Mode
The emulator can be built in debug mode by enabling the `debug` feature at compile time: `cargo build --features "debug"`

Run with `-p=g` for classic green palette, `-p=bw` for greyscale palette. `-m` to mute. `-s "SAVE_FILE_NAME"` to specify a save file. By default a save file will be created (if needed) with the name of the cart in the same directory, with `.sav` extension.

By default the emulator will try and run the game in colour if it is available. Certain original Game Boy games have unique palettes that will be selected (made by Nintendo). If you don't want these, force the palette with `-p=g` for classic green palette, `-p=bw` for greyscale palette. Running a Game Boy Color game that is compatible with the original Game Boy (such as Pokemon Gold/Silver) will run in classic mode if you set one of these palettes.

### TODO video:
* Better error checking.
* Minor issue of sprites appearing above window.
* Separate video thread to enable "turbo" mode.
* Some bugs in Color mode when drawing sprites below the background.

### TODO audio:
* Noise wave high freq. higher precision.
* Allow changing duty cycle mid-sound.

### TODO other:
* Use rotate instructions in CPU
* Add ability to use preset ROM (internally - for testing)
* MBC 5-7 bank swapping systems
* Add cache to bank swapper and save RAM
* Save states
* Video commands for mid-frame updates (scroll X)
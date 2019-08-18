# Super Rust Boy

Game Boy emulator written in Rust.

### Debug Mode
The emulator can be built in debug mode by enabling the `debug` feature at compile time: `cargo build --features "debug"`

Run with `-g` for classic green palette. `-m` to mute. `-s "SAVE_FILE_NAME"` to specify a save file. By default a save file will be created (if needed) with the name of the cart in the same directory, with `.sav` extension.

### TODO video:
* Better error checking.
* Minor issue of sprites appearing above window.
* Separate video thread to enable "turbo" mode.

### TODO audio:
* Square / Noise wave high freq. higher precision.

### TODO other:
* Use rotate instructions in CPU
* Add ability to use preset ROM (internally - for testing)
* MBC 5-7 bank swapping systems
* Add cache to bank swapper and save RAM
* Save states
* Video commands for mid-frame updates (scroll X)
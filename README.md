# Super Rust Boy

Game Boy and Game Boy Color emulator written in Rust.

This project features a library which can be used for external use, and a binary that can run the emulator locally (also for debugging purposes).

### Debug Mode
The emulator library can be built in debug mode by enabling the `debug` feature at compile time: `cargo build --features debug`.

### Making the Binary
To build a binary for use on Windows, macOS (with MoltenVK) and Linux, see [here](https://github.com/super-rust-boy/super-rust-boy-bin).

### TODO video:
* Separate video render calls to enable "turbo" mode.
* Better cache invalidation detection.

### TODO audio:
* Noise wave high freq. higher precision.
* Allow changing duty cycle mid-sound.

### TODO other:
* Optimisations in CPU (?)
* Add ability to use preset ROM (internally - for testing)
* MBC 6,7 bank swapping systems
* Save states
* Further cleanup
* Link cables via network

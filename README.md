# Super Rust Boy

Game Boy emulator written in Rust.

### Debug Mode
The emulator can be built in debug mode by enabling the `debug` feature at compile time: `cargo run --features "debug" -- [cart_name]`

## TODO:
* Fix save files.
* Re-ordering of modules.

### TODO video:
* Better error checking.
* Minor issue of sprites appearing above window.

### TODO audio:
* Figure out square 1 bugs (not playing until noise is generated)
* Square 2 bugs (some short sounds not playing)

### TODO later:
* Use rotate instructions in CPU
* Add ability to use preset ROM (internally - for testing)
* MB5-7 bank swapping systems
* Add cache to bank swapper
* Save states
* Video commands for mid-frame updates

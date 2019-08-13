# Super Rust Boy

Game Boy emulator written in Rust.

### Debug Mode
The emulator can be built in debug mode by enabling the `debug` feature at compile time: `cargo run --features "debug" -- [cart_name]`

## TODO:
* General cleanup

### TODO video:
* Sprites
* Tests
* Wraparound

### TODO audio:
* Test wave/noise generators
* Test time stretching on wave/noise
* Add frequency sweep to square1

### TODO later:
* Use rotate instructions in CPU
* Add ability to use preset ROM (internally - for testing)
* Test MB1-3 bank swapping systems, add MB4 & 5
* Add cache to bank swapper
* Fully test memory bus and mem systems
* Add save data (.sav) also in future add save states
* More constants
* Video commands for mid-frame updates

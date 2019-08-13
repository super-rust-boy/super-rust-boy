mod tilemem;
mod vertexgrid;
mod palette;

use vulkano::device::{
    Device,
    Queue
};

use bitflags::bitflags;

use std::sync::Arc;

use crate::mem::MemDevice;

use tilemem::*;
use vertexgrid::*;
use palette::*;

const TILE_SIZE: usize = 8;         // Width / Height of a tile in pixels.
const TILE_DATA_WIDTH: usize = 16;  // Width of the tile data in tiles.
const TILE_DATA_HEIGHT: usize = 24; // Height of the tile data in tiles.
const MAP_SIZE: usize = 32;         // Width / Height of bg/window tile maps.
const VIEW_WIDTH: usize = 20;       // Width of visible area.
const VIEW_HEIGHT: usize = 18;      // Height of visible area.

const OFFSET_FRAC_X: f32 = (MAP_SIZE as f32 / VIEW_WIDTH as f32) / 256.0;   // Mult with an offset to get the amount to offset by
const OFFSET_FRAC_Y: f32 = (MAP_SIZE as f32 / VIEW_HEIGHT as f32) / 256.0;  // Mult with an offset to get the amount to offset by

bitflags! {
    #[derive(Default)]
    struct LCDControl: u8 {
        const ENABLE                    = 0b10000000;
        const WINDOW_TILE_MAP_SELECT    = 0b01000000;
        const WINDOW_DISPLAY_ENABLE     = 0b00100000;
        const TILE_DATA_SELECT          = 0b00010000;
        const BG_TILE_MAP_SELECT        = 0b00001000;
        const OBJ_SIZE                  = 0b00000100;
        const OBJ_DISPLAY_ENABLE        = 0b00000010;
        const DISPLAY_PRIORITY          = 0b00000001;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct LCDStatusFlags: u8 {
        const COINCEDENCE_INT   = 0b01000000;
        const OAM_INT           = 0b00100000;
        const V_BLANK_INT       = 0b00010000;
        const H_BLANK_INT       = 0b00001000;
        const COINCEDENCE_FLAG  = 0b00000100;
    }
}

pub struct LCDStatus {
    flags: LCDStatusFlags,
    video_mode: super::Mode,
}

impl LCDStatus {
    fn new() -> Self {
        LCDStatus {
            flags: LCDStatusFlags::default(),
            video_mode: super::Mode::_2
        }
    }

    fn read(&self) -> u8 {
        self.flags.bits() | self.video_mode.clone() as u8
    }

    fn write(&mut self, val: u8) {
        self.flags = LCDStatusFlags::from_bits_truncate(val);
        self.video_mode = super::Mode::from(val);
    }

    pub fn read_flags(&self) -> LCDStatusFlags {
        self.flags
    }

    pub fn write_flags(&mut self, flags: LCDStatusFlags) {
        self.flags = flags;
    }

    pub fn read_mode(&self) -> super::Mode {
        self.video_mode.clone()
    }

    pub fn write_mode(&mut self, mode: super::Mode) {
        self.video_mode = mode;
    }
}

// Video memory layer
pub struct VideoMem {
    // Raw tile mem and tile maps
    tile_mem: TileAtlas,
    tile_map_0: VertexGrid,
    tile_map_1: VertexGrid,
    // sprites?

    // Flags / registers
    lcd_control: LCDControl,
    pub lcd_status: LCDStatus,
    scroll_y: u8,
    scroll_x: u8,
    lcdc_y: u8,
    ly_compare: u8,

    bg_palette: Palette,
    obj_0_palette: Palette,
    obj_1_palette: Palette,

    window_y: u8,
    window_x: u8
}

impl VideoMem {
    pub fn new(device: &Arc<Device>) -> Self {
        VideoMem {
            tile_mem: TileAtlas::new((TILE_DATA_WIDTH, TILE_DATA_HEIGHT), TILE_SIZE),
            tile_map_0: VertexGrid::new(device, (MAP_SIZE, MAP_SIZE), (VIEW_WIDTH, VIEW_HEIGHT), (TILE_DATA_WIDTH, TILE_DATA_HEIGHT)),
            tile_map_1: VertexGrid::new(device, (MAP_SIZE, MAP_SIZE), (VIEW_WIDTH, VIEW_HEIGHT), (TILE_DATA_WIDTH, TILE_DATA_HEIGHT)),

            lcd_control: LCDControl::default(),
            lcd_status: LCDStatus::new(),
            scroll_y: 0,
            scroll_x: 0,
            lcdc_y: 0,
            ly_compare: 0,

            bg_palette: Palette::new_monochrome(device),
            obj_0_palette: Palette::new_monochrome(device),
            obj_1_palette: Palette::new_monochrome(device),

            window_y: 0,
            window_x: 0
        }
    }
    
    pub fn inc_lcdc_y(&mut self) {
        self.lcdc_y += 1;
    }

    pub fn set_lcdc_y(&mut self, val: u8) {
        self.lcdc_y = val;
    }

    pub fn compare_ly_equal(&self) -> bool {
        self.lcdc_y == self.ly_compare
    }
}

// Renderer access functions.
impl VideoMem {
    // Get background vertices.
    pub fn get_background(&mut self) -> VertexBuffer {
        if !self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT) {
            self.tile_map_0.get_vertex_buffer()
        } else {
            self.tile_map_1.get_vertex_buffer()
        }
    }

    // Get window
    pub fn get_window(&mut self) -> Option<VertexBuffer> {
        if self.lcd_control.contains(LCDControl::WINDOW_DISPLAY_ENABLE) {
            Some(if !self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT) {
                self.tile_map_0.get_vertex_buffer()
            } else {
                self.tile_map_1.get_vertex_buffer()
            })
        } else {
            None
        }
    }
    // Get sprites

    // Get tile atlas
    pub fn get_tile_atlas(&mut self, queue: Arc<Queue>) -> (TileImage, TileFuture) {
        self.tile_mem.make_image(queue)
    }

    // Get palette for bg and window
    pub fn get_palette_buffer(&mut self) -> PaletteBuffer {
        self.bg_palette.get_buffer()
    }

    // Get push constants
    pub fn get_bg_scroll(&self) -> [f32; 2] {
        //[self.scroll_x as f32 * -OFFSET_FRAC_X, self.scroll_y as f32 * -OFFSET_FRAC_Y]
        [0.0, 0.0] // no scroll
    }

    pub fn get_window_position(&self) -> [f32; 2] {
        [(self.window_x as f32 - 7.0) * OFFSET_FRAC_X, self.window_y as f32 * OFFSET_FRAC_Y]
    }

    pub fn get_tile_data_offset(&self) -> i32 {
        if self.lcd_control.contains(LCDControl::TILE_DATA_SELECT) {
            0
        } else {
            256
        }
    }
}

impl MemDevice for VideoMem {
    fn read(&self, loc: u16) -> u8 {
        let val = match loc {
            // Raw tile data
            0x8000...0x97FF => {
                let base = (loc - 0x8000) as usize;

                let mut ret = 0;

                if base % 2 == 0 {  // Lower bit
                    let base_pixel = base * 4;
                    for i in 0..8 {
                        ret |= (self.tile_mem.atlas[base_pixel + i] & 0b01) << (7 - i);
                    }
                } else {    // Upper bit
                    let base_pixel = (base - 1) * 4;
                    for i in 0..8 {
                        ret |= ((self.tile_mem.atlas[base_pixel + i] & 0b10) >> 1) << (7 - i);
                    }
                }

                ret
            },
            // Background Map A
            0x9800...0x9BFF => {
                let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_0.get_tile_texture(x, y) as u8
            },
            // Background Map B
            0x9C00...0x9FFF => {
                let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_1.get_tile_texture(x, y) as u8
            },
            // Sprite data
            0xFE00...0xFE9F => {
                0
            },
            // Registers
            0xFF40 => self.lcd_control.bits(),
            0xFF41 => self.lcd_status.read(),
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.lcdc_y,
            0xFF45 => self.ly_compare,
            0xFF47 => self.bg_palette.read(),
            0xFF48 => self.obj_0_palette.read(),
            0xFF49 => self.obj_1_palette.read(),
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            _ => 0
        };
        //println!("Reading {:X} from {:X}", val, loc);
        val
    }

    fn write(&mut self, loc: u16, val: u8) {
        //println!("Writing {:X} to {:X}", val, loc);
        match loc {
            // Raw tile data
            0x8000...0x97FF => {
                let base = (loc - 0x8000) as usize;

                if base % 2 == 0 {  // Lower bit
                    let tile_x = (base % 0x100) / 0x10;
                    let tile_y = base / 0x100;

                    let pixel_row_num = (base / 2) % 8;

                    // tile_x * 8 pixels across per tile
                    // tile_y * 64 pixels per tile * 16 tiles per row
                    // pixel_row_num * 8 pixels across per tile * 16 tiles per row
                    let base_pixel = (tile_x * 8) + (tile_y * 64 * 16) + (pixel_row_num * 8 * 16);

                    for i in 0..8 {
                        let lower_bit = (val >> (7 - i)) & 1;
                        let upper_bit = self.tile_mem.atlas[base_pixel + i] & 0b10;
                        self.tile_mem.atlas[base_pixel + i] = upper_bit | lower_bit;
                    }
                } else {    // Upper bit
                    let base = base - 1;
                    let tile_x = (base % 0x100) / 0x10;
                    let tile_y = base / 0x100;

                    let pixel_row_num = (base / 2) % 8;

                    let base_pixel = (tile_x * 8) + (tile_y * 64 * 16) + (pixel_row_num * 8 * 16);

                    for i in 0..8 {
                        let upper_bit = (val >> (7 - i)) & 1;
                        let lower_bit = self.tile_mem.atlas[base_pixel + i] & 0b01;
                        self.tile_mem.atlas[base_pixel + i] = (upper_bit << 1) | lower_bit;
                    }
                }
            },
            // Background Map A
            0x9800...0x9BFF => {
                let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_0.set_tile_texture(x, y, val as i32);
            },
            // Background Map B
            0x9C00...0x9FFF => {
                let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_1.set_tile_texture(x, y, val as i32);
            },
            // Sprite data
            0xFE00...0xFE9F => {

            },
            0xFF40 => self.lcd_control = LCDControl::from_bits_truncate(val),
            0xFF41 => self.lcd_status.write(val),
            0xFF42 => self.scroll_y = val,
            0xFF43 => self.scroll_x = val,
            0xFF44 => self.lcdc_y = 0,
            0xFF45 => self.ly_compare = val,
            0xFF47 => self.bg_palette.write(val),
            0xFF48 => self.obj_0_palette.write(val),
            0xFF49 => self.obj_1_palette.write(val),
            0xFF4A => self.window_y = val,
            0xFF4B => self.window_x = val,
            _ => {}
        }
    }
}

// Writing raw tile data explained:
// We have to convert a number in the range (0x8000, 0x9800) to 8 adjacent pixels.
// We then convert that 1D array of pixels to a 2D image (the texture atlas).
// The image is 16x24 tiles, and each tile is 8x8 pixels. So the total size is 128x192 pixels.
// As explained in tilemem.rs (in hex):
    // the tile x coord is nibble xxXx
    // the tile y coord is nibble xXxx
    // the row is 2 bytes of nibble xxxX
// The first 128 bytes (pixels) of the atlas is the first row of the first 16 tiles.
// So to get the exact pixel:
    // Subtract 0x8000 (ignore the top nibble)
    // Get the tile x coord, multiply by 8 to get the x offset
    // Get the row of the tile, multiply by (8x16) to get the inner tile y offset
    // Get the tile y coord, multiply by (8x16x8) to get the tile y offset
    // And then just add these offsets together as we are working with a 1D array.
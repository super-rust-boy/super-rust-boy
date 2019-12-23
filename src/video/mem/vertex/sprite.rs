// Dealing with sprites.
use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use bitflags::bitflags;

use crate::mem::MemDevice;

use super::{
    Side, Vertex, VertexBuffer
};

use std::sync::Arc;

const SPRITE_SMALL_HEIGHT: u8 = 8;
const SPRITE_LARGE_HEIGHT: u8 = 16;

const SPRITE_WIDTH: f32 = (8.0 / 160.0) * 2.0;
const LINE_HEIGHT: f32 = (1.0 / 144.0) * 2.0;

const GB_OBJ_PALETTE_0: u32 = 2 << 12;
const GB_OBJ_PALETTE_1: u32 = 3 << 12;

bitflags! {
    #[derive(Default)]
    struct SpriteFlags: u8 {
        const PRIORITY  = bit!(7);
        const Y_FLIP    = bit!(6);
        const X_FLIP    = bit!(5);
        const PALETTE   = bit!(4);
        const VRAM_BANK = bit!(3);
        const CGB_PAL_2 = bit!(2);
        const CGB_PAL_1 = bit!(1);
        const CGB_PAL_0 = bit!(0);
    }
}

#[derive(Clone)]
struct Sprite {
    pub y:          u8,
    pub x:          u8,
    pub tile_num:   u8,
    pub flags:      SpriteFlags
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y:          0,
            x:          0,
            tile_num:   0,
            flags:      SpriteFlags::default()
        }
    }

    // Get vertices for sprites on a line. Needs to know if this is 8x8 or 8x16.
    fn make_vertices(&self, line_y: u8, large: bool, lo_priority: bool, cgb_mode: bool) -> Option<[Vertex; 6]> {
        let sprite_height = if large {SPRITE_LARGE_HEIGHT} else {SPRITE_SMALL_HEIGHT};
        let y_compare = line_y + 16;
        let in_range = (self.y + sprite_height > y_compare) && (self.y <= y_compare);

        // This sprite should be in the batch.
        if (lo_priority == self.flags.contains(SpriteFlags::PRIORITY)) && in_range {
            // Position
            let pos_x = ((self.x as f32 - 8.0) / 80.0) - 1.0;
            let pos_y = (line_y as f32 / 72.0) - 1.0;

            // Texture Y
            let base_tex_y = (y_compare - self.y) as u32;
            let tex_y = if self.flags.contains(SpriteFlags::Y_FLIP) {
                (if large {15} else {7}) - base_tex_y
            } else {
                base_tex_y
            };

            let (left, right) = if self.flags.contains(SpriteFlags::X_FLIP) {
                (Side::Right, Side::Left)
            } else {
                (Side::Left, Side::Right)
            };

            let palette_num = if cgb_mode {
                ((self.flags.bits() & 0xF) as u32) << 12
            } else {
                if self.flags.contains(SpriteFlags::PALETTE) {GB_OBJ_PALETTE_1} else {GB_OBJ_PALETTE_0}
            };

            let tile_num = self.tile_num as u32 + if tex_y >= 8 {1} else {0};

            let y = (tex_y % 8) << 9;

            Some([
                Vertex{ position: [pos_x, pos_y],                               data: palette_num | y | left as u32 | tile_num },
                Vertex{ position: [pos_x, pos_y + LINE_HEIGHT],                 data: palette_num | y | left as u32 | tile_num },
                Vertex{ position: [pos_x + SPRITE_WIDTH, pos_y],                data: palette_num | y | right as u32 | tile_num },
                Vertex{ position: [pos_x, pos_y + LINE_HEIGHT],                 data: palette_num | y | left as u32 | tile_num },
                Vertex{ position: [pos_x + SPRITE_WIDTH, pos_y],                data: palette_num | y | right as u32 | tile_num },
                Vertex{ position: [pos_x + SPRITE_WIDTH, pos_y + LINE_HEIGHT],  data: palette_num | y | right as u32 | tile_num }
            ])
        } else {
            None
        }
    }
}

pub struct ObjectMem {
    objects:        Vec<Sprite>,

    buffer:         Vec<Vertex>,
    buffer_pool:    CpuBufferPool<Vertex>,
}

impl ObjectMem {
    pub fn new(device: &Arc<Device>) -> Self {
        ObjectMem {
            objects:        vec![Sprite::new(); 40],

            buffer:         Vec::new(),
            buffer_pool:    CpuBufferPool::vertex_buffer(device.clone())
        }
    }

    // Gets vertices for a line.
    // Only retrieves the vertices that appear below the background.
    pub fn get_lo_vertex_buffer(&mut self, y: u8, large: bool, cgb_mode: bool) -> Option<VertexBuffer> {
        self.buffer.clear();

        for o in self.objects.iter().rev() {
            if let Some(v) = o.make_vertices(y, large, true, cgb_mode) {
                self.buffer.extend(v.iter());
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(self.buffer.drain(..)).unwrap())
        }
    }

    // Gets vertices for a line.
    // Only retrieves the vertices that appear above the background.
    pub fn get_hi_vertex_buffer(&mut self, y: u8, large: bool, cgb_mode: bool) -> Option<VertexBuffer> {
        self.buffer.clear();

        for o in self.objects.iter().rev() {
            if let Some(v) = o.make_vertices(y, large, false, cgb_mode) {
                self.buffer.extend(v.iter());
            }
        }

        if self.buffer.is_empty() {
            None
        } else {
            Some(self.buffer_pool.chunk(self.buffer.drain(..)).unwrap())
        }
    }
}

// Expects a loc range from 0 -> 0x9F
impl MemDevice for ObjectMem {
    fn read(&self, loc: u16) -> u8 {
        let index = (loc / 4) as usize;

        match loc % 4 {
            0 => self.objects[index].y,
            1 => self.objects[index].x,
            2 => self.objects[index].tile_num,
            _ => self.objects[index].flags.bits()
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        let index = (loc / 4) as usize;

        match loc % 4 {
            0 => self.objects[index].y = val,
            1 => self.objects[index].x = val,
            2 => self.objects[index].tile_num = val,
            _ => self.objects[index].flags = SpriteFlags::from_bits_truncate(val)
        }
    }
}
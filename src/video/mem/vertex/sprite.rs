// Dealing with sprites.
use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use bitflags::bitflags;

use crate::mem::MemDevice;

use super::{
    Corner, Vertex, VertexBuffer
};

use std::sync::Arc;

const SPRITE_WIDTH: f32 = (8.0 / 160.0) * 2.0;
const SPRITE_HEIGHT: f32 = (8.0 / 144.0) * 2.0;

const GB_OBJ_PALETTE_0: u32 = 2 << 10;
const GB_OBJ_PALETTE_1: u32 = 3 << 10;

bitflags! {
    #[derive(Default)]
    struct SpriteFlags: u8 {
        const PRIORITY  = 0b10000000;
        const Y_FLIP    = 0b01000000;
        const X_FLIP    = 0b00100000;
        const PALETTE   = 0b00010000;
        const VRAM_BANK = 0b00001000;
        const CGB_PAL_2 = 0b00000100;
        const CGB_PAL_1 = 0b00000010;
        const CGB_PAL_0 = 0b00000001;
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

    // Get vertices for sprite. Needs to know if this is 8x8 or 8x16.
    pub fn make_vertices(&self, large: bool, lo_priority: bool, cgb_mode: bool) -> Vec<Vertex> {
        // This sprite should be in the batch.
        if lo_priority == self.flags.contains(SpriteFlags::PRIORITY) {
            let lo_x = ((self.x as f32 - 8.0) / 80.0) - 1.0;
            let hi_x = lo_x + SPRITE_WIDTH;

            let base_y = ((self.y as f32 - 16.0) / 72.0) - 1.0;
            let lo_y = if large && self.flags.contains(SpriteFlags::Y_FLIP) {base_y + SPRITE_HEIGHT} else {base_y};
            let hi_y = lo_y + SPRITE_HEIGHT;

            let (top_left, bottom_left, top_right, bottom_right) = match (self.flags.contains(SpriteFlags::X_FLIP), self.flags.contains(SpriteFlags::Y_FLIP)) {
                (false, false)  => (Corner::TopLeft, Corner::BottomLeft, Corner::TopRight, Corner::BottomRight),
                (true, false)   => (Corner::TopRight, Corner::BottomRight, Corner::TopLeft, Corner::BottomLeft),
                (false, true)   => (Corner::BottomLeft, Corner::TopLeft, Corner::BottomRight, Corner::TopRight),
                (true, true)    => (Corner::BottomRight, Corner::TopRight, Corner::BottomLeft, Corner::TopLeft)
            };

            let tl = top_left as u32;
            let bl = bottom_left as u32;
            let tr = top_right as u32;
            let br = bottom_right as u32;
            let tile_num = self.tile_num as u32;
            let palette_num = if cgb_mode {
                ((self.flags.bits() & 0xF) as u32) << 10
            } else {
                if self.flags.contains(SpriteFlags::PALETTE) {GB_OBJ_PALETTE_1} else {GB_OBJ_PALETTE_0}
            };

            let mut vertices = Vec::with_capacity(if large {12} else {6});
            vertices.push(Vertex{ position: [lo_x, lo_y], data: palette_num | tile_num | tl });
            vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
            vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
            vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
            vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
            vertices.push(Vertex{ position: [hi_x, hi_y], data: palette_num | tile_num | br });

            if large {
                let lo_y = if large && self.flags.contains(SpriteFlags::Y_FLIP) {base_y} else {hi_y};
                let hi_y = lo_y + SPRITE_HEIGHT;
                let tile_num = self.tile_num as u32 + 1;
                vertices.push(Vertex{ position: [lo_x, lo_y], data: palette_num | tile_num | tl });
                vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
                vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
                vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
                vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
                vertices.push(Vertex{ position: [hi_x, hi_y], data: palette_num | tile_num | br });
            }

            vertices
        } else {    // This sprite is not part of this batch.
            Vec::new()
        }
    }
}

pub struct ObjectMem {
    objects: Vec<Sprite>,
    large_objects: bool,
    buffer_pool: CpuBufferPool<Vertex>,
    current_lo_buffer: Option<VertexBuffer>,
    current_hi_buffer: Option<VertexBuffer>
}

impl ObjectMem {
    pub fn new(device: &Arc<Device>) -> Self {
        ObjectMem {
            objects: vec![Sprite::new(); 40],
            large_objects: false,
            buffer_pool: CpuBufferPool::vertex_buffer(device.clone()),
            current_lo_buffer: None,
            current_hi_buffer: None
        }
    }

    // Makes a new vertex buffer if the data has changed. Else, retrieves the current one.
    // Only retrieves the vertices that appear below the background.
    pub fn get_lo_vertex_buffer(&mut self, large: bool, cgb_mode: bool) -> Option<VertexBuffer> {
        if self.large_objects != large {
            self.current_lo_buffer = None;
            self.current_hi_buffer = None;
            self.large_objects = large;
        }

        if let Some(buf) = &self.current_lo_buffer {
            Some(buf.clone())
        } else {
            let objects = self.objects.iter().rev()
                .map(|o| o.make_vertices(large, true, cgb_mode))
                .fold(Vec::new(), |mut v, mut o| {v.append(&mut o); v});

            let buf = if objects.is_empty() {
                None
            } else {
                Some(self.buffer_pool.chunk(objects).unwrap())
            };
            
            self.current_lo_buffer = buf.clone();
            buf
        }
    }

    // Makes a new vertex buffer if the data has changed. Else, retrieves the current one.
    // Only retrieves the vertices that appear above the background.
    pub fn get_hi_vertex_buffer(&mut self, large: bool, cgb_mode: bool) -> Option<VertexBuffer> {
        if self.large_objects != large {
            self.current_lo_buffer = None;
            self.current_hi_buffer = None;
            self.large_objects = large;
        }

        if let Some(buf) = &self.current_hi_buffer {
            Some(buf.clone())
        } else {
            let objects = self.objects.iter().rev()
                .map(|o| o.make_vertices(large, false, cgb_mode))
                .fold(Vec::new(), |mut v, mut o| {v.append(&mut o); v});

            let buf = if objects.is_empty() {
                None
            } else {
                Some(self.buffer_pool.chunk(objects).unwrap())
            };
            
            self.current_hi_buffer = buf.clone();
            buf
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

        self.current_lo_buffer = None;
        self.current_hi_buffer = None;
    }
}
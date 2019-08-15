// Dealing with sprites.

use bitflags::bitflags;

use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use crate::mem::MemDevice;

use crate::video::{
    mem::vertexgrid::{
        Corner, VertexBuffer
    },
    renderer::Vertex
};

use std::sync::Arc;

const SPRITE_WIDTH: f32 = (8.0 / 160.0) * 2.0;
const SPRITE_HEIGHT: f32 = (8.0 / 144.0) * 2.0;

const OBJ_PALETTE_0: u32 = 1 << 10;
const OBJ_PALETTE_1: u32 = 2 << 10;

bitflags! {
    #[derive(Default)]
    struct SpriteFlags: u8 {
        const PRIORITY  = 0b10000000;
        const Y_FLIP    = 0b01000000;
        const X_FLIP    = 0b00100000;
        const PALETTE   = 0b00010000;
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

    pub fn make_vertices(&self, large: bool) -> Vec<Vertex> {
        let lo_x = ((self.x as f32 - 8.0) / 80.0) - 1.0;
        let lo_y = ((self.y as f32 - 16.0) / 72.0) - 1.0;
        let hi_x = lo_x + SPRITE_WIDTH;
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
        let palette_num = if self.flags.contains(SpriteFlags::PALETTE) {OBJ_PALETTE_1} else {OBJ_PALETTE_0};

        let mut vertices = Vec::with_capacity(if large {12} else {6});
        vertices.push(Vertex{ position: [lo_x, lo_y], data: palette_num | tile_num | tl });
        vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
        vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
        vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
        vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
        vertices.push(Vertex{ position: [hi_x, hi_y], data: palette_num | tile_num | br });

        if large {
            let lo_y = hi_y;
            let hi_y = hi_y + SPRITE_HEIGHT;
            let tile_num = self.tile_num as u32 + 1;
            vertices.push(Vertex{ position: [lo_x, lo_y], data: palette_num | tile_num | tl });
            vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
            vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
            vertices.push(Vertex{ position: [lo_x, hi_y], data: palette_num | tile_num | bl });
            vertices.push(Vertex{ position: [hi_x, lo_y], data: palette_num | tile_num | tr });
            vertices.push(Vertex{ position: [hi_x, hi_y], data: palette_num | tile_num | br });
        }

        vertices
    }
}

pub struct ObjectMem {
    objects: Vec<Sprite>,
    large_objects: bool,
    buffer_pool: CpuBufferPool<Vertex>,
    current_buffer: Option<VertexBuffer>
}

impl ObjectMem {
    pub fn new(device: &Arc<Device>) -> Self {
        ObjectMem {
            objects: vec![Sprite::new(); 40],
            large_objects: false,
            buffer_pool: CpuBufferPool::vertex_buffer(device.clone()),
            current_buffer: None
        }
    }

    // Makes a new vertex buffer if the data has changed. Else, retrieves the current one.
    pub fn get_vertex_buffer(&mut self, large: bool) -> VertexBuffer {
        if self.large_objects != large {
            self.current_buffer = None;
            self.large_objects = large;
        }

        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.chunk(
                self.objects.iter()
                    .map(|o| o.make_vertices(large))
                    .fold(Vec::new(), |mut v, mut o| {v.append(&mut o); v})
            ).unwrap();
            self.current_buffer = Some(buf.clone());
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

        self.current_buffer = None;
    }
}
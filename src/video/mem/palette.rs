use vulkano::{
    buffer::CpuBufferPool,
    buffer::cpu_pool::CpuBufferPoolSubbuffer,
    device::Device,
    memory::pool::StdMemoryPool
};

use cgmath::{
    Matrix4,
    Vector4
};

use crate::mem::MemDevice;

use std::sync::Arc;

pub type PaletteBuffer = CpuBufferPoolSubbuffer<Matrix4<f32>, Arc<StdMemoryPool>>;

pub struct Palette {
    colours: Matrix4<f32>,
    raw: u8,
    buffer_pool: CpuBufferPool<Matrix4<f32>>,
    current_buffer: Option<PaletteBuffer>
}

impl Palette {
    pub fn new_monochrome(device: &Arc<Device>) -> Self {
        Palette {
            colours: Matrix4::from_cols(
                Vector4::new(0.0, 0.0, 0.0, 1.0),
                Vector4::new(0.3, 0.3, 0.3, 1.0),
                Vector4::new(0.6, 0.6, 0.6, 1.0),
                Vector4::new(1.0, 1.0, 1.0, 1.0)
            ),
            raw: 0,
            buffer_pool: CpuBufferPool::uniform_buffer(device.clone()),
            current_buffer: None
        }
    }

    pub fn get_buffer(&mut self) -> PaletteBuffer {
        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.next(
                Matrix4::from_cols(
                    self.colours[(self.raw & 0b00000011) as usize],
                    self.colours[(self.raw & 0b00001100) as usize],
                    self.colours[(self.raw & 0b00110000) as usize],
                    self.colours[(self.raw & 0b11000000) as usize]
                )
            ).unwrap();
            self.current_buffer = Some(buf.clone());
            buf
        }
    }

    pub fn get_obj_buffer(&mut self) -> PaletteBuffer {
        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.next(
                Matrix4::from_cols(
                    Vector4::new(0.0, 0.0, 0.0, 0.0),
                    self.colours[(self.raw & 0b00001100) as usize],
                    self.colours[(self.raw & 0b00110000) as usize],
                    self.colours[(self.raw & 0b11000000) as usize]
                )
            ).unwrap();
            self.current_buffer = Some(buf.clone());
            buf
        }
    }
}

impl MemDevice for Palette {
    fn read(&self, loc: u16) -> u8 {
        self.raw
    }

    fn write(&mut self, loc: u16, val: u8) {
        self.raw = val;
        self.current_buffer = None;
    }
}
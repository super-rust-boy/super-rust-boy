use vulkano::{
    buffer::CpuBufferPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
    device::Device,
    memory::pool::StdMemoryPool
};

use cgmath::{
    Matrix4,
    Vector4
};

use std::sync::Arc;

pub type PaletteBuffer = CpuBufferPoolChunk<Matrix4<f32>, Arc<StdMemoryPool>>;

// A single palette.
struct Palette {
    pub colours: Matrix4<f32>,
    pub raw: u8,
    pub object: bool
}

impl Palette {
    pub fn new_monochrome(device: &Arc<Device>, object: bool) -> Self {
        Palette {
            colours: Matrix4::from_cols(
                Vector4::new(1.0, 1.0, 1.0, 1.0),
                Vector4::new(0.6, 0.6, 0.6, 1.0),
                Vector4::new(0.3, 0.3, 0.3, 1.0),
                Vector4::new(0.0, 0.0, 0.0, 1.0)
            ),
            raw: 0,
            object: object
        }
    }

    pub fn get_palette(&self) -> Matrix4<f32> {
        let colour_0 = self.raw & 0b00000011;
        let colour_1 = (self.raw & 0b00001100) >> 2;
        let colour_2 = (self.raw & 0b00110000) >> 4;
        let colour_3 = (self.raw & 0b11000000) >> 6;

        let col_0 = if self.object {
            Vector4::new(0.0, 0.0, 0.0, 0.0)
        } else {
            self.colours[colour_0 as usize]
        };

        Matrix4::from_cols(
            col_0,
            self.colours[colour_1 as usize],
            self.colours[colour_2 as usize],
            self.colours[colour_3 as usize]
        )
    }

    pub fn read(&self) -> u8 {
        self.raw
    }

    pub fn write(&mut self, val: u8) {
        self.raw = val;
    }
}

// A group of palettes
pub struct PaletteMem {
    palettes: Vec<Palette>,
    buffer_pool: CpuBufferPool<Matrix4<f32>>,
    current_buffer: Option<PaletteBuffer>
}

impl PaletteMem {
    pub fn new(device: &Arc<Device>) -> Self {
        PaletteMem {
            palettes: vec![Palette::new_monochrome(device, false), Palette::new_monochrome(device, true), Palette::new_monochrome(device, true)],
            buffer_pool: CpuBufferPool::uniform_buffer(device.clone()),
            current_buffer: None
        }
    }

    pub fn get_buffer(&mut self) -> PaletteBuffer {
        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.chunk(
                self.palettes.iter().map(|p| p.get_palette())
            ).unwrap();
            self.current_buffer = Some(buf.clone());
            buf
        }
    }

    pub fn read(&self, which: usize) -> u8 {
        self.palettes[which].read()
    }

    pub fn write(&mut self, which: usize, val: u8) {
        self.palettes[which].write(val);

        self.current_buffer = None;
    }
}
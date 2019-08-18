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
    colours: Matrix4<f32>,
    raw: u8
}

impl Palette {
    pub fn new_monochrome_bw() -> Self {
        Palette {
            colours: Matrix4::from_cols(
                Vector4::new(1.0, 1.0, 1.0, 1.0),
                Vector4::new(0.65, 0.65, 0.65, 1.0),
                Vector4::new(0.33, 0.33, 0.33, 1.0),
                Vector4::new(0.0, 0.0, 0.0, 1.0)
            ),
            raw: 0
        }
    }

    #[allow(dead_code)]
    pub fn new_monochrome_green() -> Self {
        Palette {
            colours: Matrix4::from_cols(
                Vector4::new(0.647, 0.765, 0.086, 1.0),
                Vector4::new(0.596, 0.702, 0.165, 1.0),
                Vector4::new(0.184, 0.388, 0.145, 1.0),
                Vector4::new(0.055, 0.208, 0.059, 1.0)
            ),
            raw: 0
        }
    }

    pub fn get_palette(&self, transparent: bool) -> Matrix4<f32> {
        let colour_1 = (self.raw & 0b00001100) >> 2;
        let colour_2 = (self.raw & 0b00110000) >> 4;
        let colour_3 = (self.raw & 0b11000000) >> 6;

        let col_0 = if transparent {
            Vector4::new(0.0, 0.0, 0.0, 0.0)
        } else {
            let colour_0 = self.raw & 0b00000011;
            self.colours[colour_0 as usize]
        };

        Matrix4::from_cols(
            col_0,
            self.colours[colour_1 as usize],
            self.colours[colour_2 as usize],
            self.colours[colour_3 as usize]
        )
    }

    pub fn get_colour_0(&self) -> Vector4<f32> {
        let colour_0 = (self.raw & 0b00000011) as usize;
        self.colours[colour_0]
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
    pub fn new_bw(device: &Arc<Device>) -> Self {
        PaletteMem {
            palettes: vec![
                Palette::new_monochrome_bw(),
                Palette::new_monochrome_bw(),
                Palette::new_monochrome_bw()
            ],
            buffer_pool: CpuBufferPool::uniform_buffer(device.clone()),
            current_buffer: None
        }
    }

    pub fn new_green(device: &Arc<Device>) -> Self {
        PaletteMem {
            palettes: vec![
                Palette::new_monochrome_green(),
                Palette::new_monochrome_green(),
                Palette::new_monochrome_green()
            ],
            buffer_pool: CpuBufferPool::uniform_buffer(device.clone()),
            current_buffer: None
        }
    }

    pub fn get_buffer(&mut self) -> PaletteBuffer {
        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.chunk([
                self.palettes[0].get_palette(true),     // BG
                self.palettes[0].get_palette(false),    // Window
                self.palettes[1].get_palette(true),     // Sprite 0
                self.palettes[2].get_palette(true)      // Sprite 1
            ].iter().cloned()).unwrap();
            self.current_buffer = Some(buf.clone());
            buf
        }
    }

    pub fn get_colour_0(&self) -> [f32; 4] {
        let colour = self.palettes[0].get_colour_0();
        [colour[0], colour[1], colour[2], colour[3]]
    }

    pub fn read(&self, which: usize) -> u8 {
        self.palettes[which].read()
    }

    pub fn write(&mut self, which: usize, val: u8) {
        self.palettes[which].write(val);

        self.current_buffer = None;
    }
}
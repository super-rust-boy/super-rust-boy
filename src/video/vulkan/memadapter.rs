// Wraps the raw memory for vulkan.

use vulkano::{
    device::{
        Device,
        Queue
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::{
        now, GpuFuture
    },
    buffer::{
        CpuBufferPool,
        cpu_pool::CpuBufferPoolChunk
    },
    memory::pool::StdMemoryPool
};

use std::sync::Arc;

use crate::video::{
    PaletteColours,
    types::Vertex,
    mem::{
        VideoMem,
        consts::{TEX_WIDTH, TEX_HEIGHT, MAP_SIZE}
    }
};

const BG_OAM_PRIORITY: u32 = 1 << 19;

// Memory types.
pub type TileImage = Arc<ImmutableImage<R8Uint>>;
pub type TileFuture = Box<dyn GpuFuture>;
pub type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;
pub type PaletteBuffer = CpuBufferPoolChunk<PaletteColours, Arc<StdMemoryPool>>;

pub struct MemAdapter {
    // Image for tile pattern memory
    image:                  Option<TileImage>,
    atlas_size:             (u32, u32),

    // Vertex buffers for tile map & sprites
    vertex_buffer_pool:     CpuBufferPool<Vertex>,
    lo_vertex_buffers:      Vec<Option<VertexBuffer>>,
    hi_vertex_buffers:      Vec<Option<VertexBuffer>>,

    // Buffers for palettes
    palette_buffer_pool:    CpuBufferPool<PaletteColours>,
    current_palette_buffer: Option<PaletteBuffer>
}

impl MemAdapter {
    pub fn new(mem: &VideoMem, device: &Arc<Device>) -> Self {
        let atlas_size_tiles = mem.get_atlas_size();
        let atlas_size_x = (atlas_size_tiles[0] as usize * TEX_WIDTH) as u32;
        let atlas_size_y = (atlas_size_tiles[1] as usize * TEX_HEIGHT) as u32;

        let vertex_buffers = vec![None; MAP_SIZE * TEX_HEIGHT];

        MemAdapter {
            image:                  None,
            atlas_size:             (atlas_size_x, atlas_size_y),

            vertex_buffer_pool:     CpuBufferPool::vertex_buffer(device.clone()),
            lo_vertex_buffers:      vertex_buffers.clone(),
            hi_vertex_buffers:      vertex_buffers,

            palette_buffer_pool:    CpuBufferPool::uniform_buffer(device.clone()),
            current_palette_buffer: None
        }
    }

    // Make an image from the pattern memory.
    pub fn get_image(&mut self, mem: &mut VideoMem, device: &Arc<Device>, queue: &Arc<Queue>) -> (TileImage, TileFuture) {
        if let Some(data) = mem.ref_tile_atlas() {
            let (image, future) = ImmutableImage::from_iter(
                data.iter().cloned(),
                Dimensions::Dim2d { width: self.atlas_size.0, height: self.atlas_size.1 },
                R8Uint,
                queue.clone()
            ).expect("Couldn't create image.");

            self.image = Some(image.clone());

            (image, Box::new(future))
        } else if let Some(image) = &self.image {
            (image.clone(), Box::new(now(device.clone())))
        } else {
            // TODO: make this a bit cleaner.
            panic!("No dirty pattern memory or image!");
        }
    }

    // Get background vertices for given y line.
    pub fn get_background(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        if mem.is_cgb_mode() || mem.get_background_priority() {
            self.get_lo_vertex_buffer(mem.ref_background(y), y)
        } else {
            None
        }
    }

    // Get background vertices with priority bit set for given y line.
    pub fn get_background_hi(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        if mem.is_cgb_mode() {
            self.get_hi_vertex_buffer(mem.ref_background(y), y)
        } else {
            None
        }
    }

    // Get window vertices for given y line.
    pub fn get_window(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        if mem.get_window_enable() {
            self.get_lo_vertex_buffer(mem.ref_window(y), y)
        } else {
            None
        }
    }

    // Get window vertices with priority bit set for given y line.
    pub fn get_window_hi(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        if mem.is_cgb_mode() {
            self.get_hi_vertex_buffer(mem.ref_window(y), y)
        } else {
            None
        }
    }

    // Get low-priority sprites (below the background) for given y line.
    pub fn get_sprites_lo(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        mem.ref_sprites_lo(y).and_then(|vertices| Some(self.vertex_buffer_pool.chunk(vertices.drain(..)).unwrap()))
    }

    // Get high-priority sprites (above the background) for given y line.
    pub fn get_sprites_hi(&mut self, mem: &mut VideoMem, y: u8) -> Option<VertexBuffer> {
        mem.ref_sprites_hi(y).and_then(|vertices| Some(self.vertex_buffer_pool.chunk(vertices.drain(..)).unwrap()))
    }

    // Get palettes
    pub fn get_palette_buffer(&mut self, mem: &mut VideoMem) -> PaletteBuffer {
        if let Some(palettes) = mem.make_palettes() {
            let buf = self.palette_buffer_pool.chunk(
                palettes.into_iter()
            ).unwrap();
            self.current_palette_buffer = Some(buf.clone());
            buf
        } else if let Some(buf) = &self.current_palette_buffer {
            buf.clone()
        } else {
            panic!("Cannot find palette buffer!");
        }
    }
}

// Internal
impl MemAdapter {
    // Get a line of vertices.
    // Only retrieves the vertices that appear below the objects.
    // (This will get the whole background in GB mode).
    fn get_lo_vertex_buffer(&mut self, buffer: Option<&[Vertex]>, y: u8) -> Option<VertexBuffer> {
        let row = y as usize;
        let cached_buffer = &mut self.lo_vertex_buffers[row];

        if let Some(data) = buffer {
            let tile_map = data.iter()
                .cloned()
                .filter(|v| (v.data & BG_OAM_PRIORITY) == 0)
                .collect::<Vec<_>>();

            if tile_map.is_empty() {
                *cached_buffer = None;
                None
            } else {
                let buffer = Some(self.vertex_buffer_pool.chunk(tile_map).unwrap());
                *cached_buffer = buffer.clone();
                buffer
            }
        } else {
            cached_buffer.clone()
        }
    }

    // Only retrieves the vertices that appear above the objects.
    fn get_hi_vertex_buffer(&mut self, buffer: Option<&[Vertex]>, y: u8) -> Option<VertexBuffer> {
        let row = y as usize;
        let cached_buffer = &mut self.hi_vertex_buffers[row];

        if let Some(data) = buffer {
            let tile_map = data.iter()
                .cloned()
                .filter(|v| (v.data & BG_OAM_PRIORITY) != 0)
                .collect::<Vec<_>>();

            if tile_map.is_empty() {
                *cached_buffer = None;
                None
            } else {
                let buffer = Some(self.vertex_buffer_pool.chunk(tile_map).unwrap());
                *cached_buffer = buffer.clone();
                buffer
            }
        } else {
            cached_buffer.clone()
        }
    }
}
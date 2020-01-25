// Vulkan renderer.
use vulkano::{
    instance::{
        Instance, PhysicalDevice
    },
    device::{
        Device, DeviceExtensions, Queue
    },
    framebuffer::{
        Framebuffer, Subpass, FramebufferAbstract, RenderPassAbstract
    },
    buffer::{
        BufferUsage,
        CpuAccessibleBuffer
    },
    pipeline::{
        GraphicsPipeline,
        vertex::SingleBufferDefinition,
        viewport::Viewport,
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        AutoCommandBuffer,
        DynamicState
    },
    image::{
        StorageImage,
        Dimensions,
        ImageUsage
    },
    format::Format,
    sampler::{
        Filter,
        MipmapMode,
        Sampler,
        SamplerAddressMode
    },
    sync::{
        now, GpuFuture
    },
    descriptor::{
        descriptor_set::{
            PersistentDescriptorSetBuf,
            PersistentDescriptorSetImg,
            PersistentDescriptorSetSampler,
            FixedSizeDescriptorSet,
            FixedSizeDescriptorSetsPool
        },
        pipeline_layout::PipelineLayoutAbstract
    }
};

use bitflags::bitflags;

use std::sync::Arc;

use super::super::{
    types::*,
    mem::{
        PaletteBuffer,
        TileImage,
        VideoMem
    }
};

#[derive(Clone, Debug)]
struct PushConstants {
    pub tex_size:       [f32; 2],
    pub atlas_size:     [f32; 2],
    pub vertex_offset:  [f32; 2],
    pub tex_offset:     u32,
    pub palette_offset: u32,
    pub flags:          u32
}

bitflags!{
    #[derive(Default)]
    struct ShaderFlags: u32 {
        const WRAPAROUND =      1;
        const BLOCK_COLOUR =    2;
    }
}

vulkano::impl_vertex!(Vertex, position, data);

type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>
>;

//type OutputImage = Arc<CpuBufferPoolChunk<u8, Arc<StdMemoryPool>>>;

// Data for a single render
struct RenderData {
    command_buffer: Option<AutoCommandBufferBuilder>,
    image_future:   Box<dyn GpuFuture>,
    pipeline:       Arc<RenderPipeline>,
    set0:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, (((), PersistentDescriptorSetImg<TileImage>), PersistentDescriptorSetSampler)>>,
    set1:           Arc<FixedSizeDescriptorSet<Arc<RenderPipeline>, ((), PersistentDescriptorSetBuf<PaletteBuffer>)>>,
    render_image:   Arc<StorageImage<Format>>,
    output_image:   Arc<CpuAccessibleBuffer<[u32]>>
}

pub struct VulkanRenderer {
    // Core
    device:         Arc<Device>,
    queue:          Arc<Queue>,
    pipeline:       Arc<RenderPipeline>,
    // Output
    render_target:  Arc<StorageImage<Format>>,
    framebuffer:    Arc<dyn FramebufferAbstract + Send + Sync>,
    dynamic_state:  DynamicState,
    //output_pool:    CpuBufferPool<u8>,
    output_image:   Arc<CpuAccessibleBuffer<[u32]>>,
    // Uniforms
    sampler:        Arc<Sampler>,
    set_pools:      Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    // Frame data
    previous_frame_future:  Box<dyn GpuFuture>,
    render_data:            Option<RenderData>
}

impl VulkanRenderer {
    // Create and initialise renderer.
    pub fn new(/*window_type: WindowType*/) -> Box<Self> {
        // Make instance with window extensions.
        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("Failed to create vulkan instance")
        };

        // Get graphics device.
        let physical = PhysicalDevice::enumerate(&instance).next()
            .expect("No device available");

        // Get graphics command queue family from graphics device.
        let queue_family = physical.queue_families()
            .find(|&q| q.supports_graphics())
            .expect("Could not find a graphical queue family");

        // Make software device and queue iterator of the graphics family.
        let (device, mut queues) = {
            let device_ext = DeviceExtensions{
                khr_swapchain: true,
                .. DeviceExtensions::none()
            };
            
            Device::new(physical, physical.supported_features(), &device_ext,
                        [(queue_family, 0.5)].iter().cloned())
                .expect("Failed to create device")
        };

        // Get a queue from the iterator.
        let queue = queues.next().unwrap();

        // Make a surface.
        /*let surface = match window_type {
            WindowType::Winit(events_loop) => WindowBuilder::new()
                .with_dimensions((320, 288).into())
                .with_title("Super Rust Boy")
                .build_vk_surface(&events_loop, instance.clone())
                .expect("Couldn't create surface"),
            WindowType::IOS { ui_view, window } => unsafe { Surface::from_ios_moltenvk(
                instance.clone(),
                ui_view,
                window
            )}.expect("Couldn't create iOS surface"),
            WindowType::MacOS { ns_view, window } => unsafe { Surface::from_macos_moltenvk(
                instance.clone(),
                ns_view,
                window
            )}.expect("Couldn't create macOS surface")
        };*/

        // Make the sampler for the texture.
        let sampler = Sampler::new(
            device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0, 1.0, 0.0, 0.0
        ).expect("Couldn't create sampler!");

        // Get a swapchain and images for use with the swapchain, as well as the dynamic state.
        /*let ((swapchain, images), dynamic_state) = {

            let caps = surface.capabilities(physical)
                    .expect("Failed to get surface capabilities");
            let dimensions = caps.current_extent.unwrap_or([160, 144]);

            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            (Swapchain::new(device.clone(), surface.clone(),
                caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &queue,
                SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).expect("Failed to create swapchain"),
            DynamicState {
                viewports: Some(vec![Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0 .. 1.0,
                }]),
                .. DynamicState::none()
            })
        };*/

        /*let image = StorageImage::with_usage(
            device.clone(),
            Dimensions::Dim2d{ width: 160, height: 144 },
            Format::R8G8B8A8Unorm,
            ImageUsage {
                transfer_source: true,
                storage: true,
                input_attachment: true,
                color_attachment: true,
                .. ImageUsage::none()
            },
            vec![queue_family].into_iter()
        ).unwrap();*/

        let image = StorageImage::new(
            device.clone(),
            Dimensions::Dim2d{ width: 160, height: 144 },
            Format::R8G8B8A8Unorm,
            vec![queue_family].into_iter()
        ).unwrap();

        let dynamic_state = DynamicState {
            viewports: Some(vec![Viewport {
                origin: [0.0, 0.0],
                dimensions: [160.0, 144.0],
                depth_range: 0.0 .. 1.0,
            }]),
            .. DynamicState::none()
        };

        // Make the render pass to insert into the command queue.
        let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: Format::R8G8B8A8Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()) as Arc<dyn RenderPassAbstract + Send + Sync>;

        /*let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();*/

        let framebuffer = Arc::new(
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        ) as Arc<dyn FramebufferAbstract + Send + Sync>;

        //let output_buffer = CpuBufferPool::download(device.clone());

        // Assemble
        let vs = super::shaders::vs::Shader::load(device.clone()).expect("failed to create vertex shader");
        let fs = super::shaders::fs::Shader::load(device.clone()).expect("failed to create fragment shader");

        // Make pipeline.
        let pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vs.main_entry_point(), ())
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .blend_alpha_blending()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let set_pools = vec![
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 1)
        ];

        Box::new(VulkanRenderer {
            device:         device.clone(),
            queue:          queue,
            pipeline:       pipeline,

            render_target:  image,
            framebuffer:    framebuffer,
            dynamic_state:  dynamic_state,
            //output_pool:    output_buffer,
            output_image:   CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage {
                transfer_destination: true,
                storage_texel_buffer: true,
                .. BufferUsage::none()
            }, (0..160*144).map(|_| 33)).expect("Unable to make cpu access buffer"),

            sampler:        sampler,
            set_pools:      set_pools,

            previous_frame_future:  Box::new(now(device.clone())),
            render_data:            None
        })
    }

    // Re-create the swapchain and framebuffers.
    /*fn create_swapchain(&mut self) {
        let window = self.surface.window();
        let dimensions = if let Some(dimensions) = window.get_inner_size() {
            let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            return;
        };

        // Get a swapchain and images for use with the swapchain.
        let (new_swapchain, images) = self.swapchain.recreate_with_dimension(dimensions).unwrap();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0 .. 1.0,
        };

        self.dynamic_state.viewports = Some(vec![viewport]);

        self.framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(self.render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        self.swapchain = new_swapchain;
    }*/

    pub fn get_device(&self) -> Arc<Device> {
        self.device.clone()
    }
}

impl Renderer for VulkanRenderer {
    // Start the process of rendering a frame.
    fn frame_start(&mut self, video_mem: &mut VideoMem) {
        // Get image with current texture.
        let (image, write_future) = video_mem.get_tile_atlas(&self.device, &self.queue);

        // Make descriptor set to bind texture atlas.
        let set0 = Arc::new(self.set_pools[0].next()
            .add_sampled_image(image, self.sampler.clone()).unwrap()
            .build().unwrap());

        // Make descriptor set for palette.
        let set1 = Arc::new(self.set_pools[1].next()
            .add_buffer(video_mem.get_palette_buffer().clone()).unwrap()
            .build().unwrap());
        
        // Start building command buffer using pipeline and framebuffer, starting with the background vertices.
        let command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffer.clone(), false, vec![video_mem.get_clear_colour().into()]).unwrap();

        // DEBUG
        //command_buffer_builder = self.draw_debug(video_mem, command_buffer_builder, image);

        //let output_image = Arc::new(self.output_pool.chunk((0..160*144*4).map(|_| 0_u8)).expect("Couldn't allocate output buffer."));
        //self.output_image = Some(output_image.clone());

        self.render_data = Some(RenderData{
            command_buffer: Some(command_buffer_builder),
            image_future:   write_future,
            pipeline:       self.pipeline.clone(),
            set0:           set0,
            set1:           set1,
            render_image:   self.render_target.clone(),
            output_image:   self.output_image.clone()
        });
    }

    fn frame_end(&mut self) {
        let render_data = std::mem::replace(&mut self.render_data, None);

        if let Some(render_data) = render_data {
            // Finish command buffer.
            let (command_buffer, image_future) = render_data.finish_drawing();

            // Wait until previous frame is done.
            let mut now_future = Box::new(now(self.device.clone())) as Box<dyn GpuFuture>;
            std::mem::swap(&mut self.previous_frame_future, &mut now_future);

            // Wait until previous frame is done,
            // _and_ the texture has been uploaded.
            let future = now_future.join(image_future)
                .then_execute(self.queue.clone(), command_buffer).unwrap()  // Run the commands (pipeline and render)
                .then_signal_fence_and_flush().unwrap().wait(None);         // Signal done and flush the pipeline.

            match future {
                Ok(_) => self.previous_frame_future = Box::new(now(self.device.clone())) as Box<_>,
                Err(e) => println!("Err: {:?}", e),
            }

            self.previous_frame_future.cleanup_finished();
        }
    }

    // Draw a scan-line.
    fn draw_line(&mut self, y: u8, video_mem: &mut VideoMem, cgb_mode: bool) {
        if let Some(render_data) = &mut self.render_data {
            if cgb_mode {
                render_data.draw_cgb_line(y, video_mem, &self.dynamic_state);
            } else {
                render_data.draw_gb_line(y, video_mem, &self.dynamic_state);
            }
        }
    }

    fn on_resize(&mut self) {
    }

    fn transfer_image(&mut self, image_out: &mut [u32]) {
        let buffer = self.output_image.read().unwrap();

        for (o, i) in image_out.iter_mut().zip(&buffer[..]) {
            *o = *i;
        }
    }
}

// Internal render functions.
impl RenderData {
    fn draw_gb_line(
        &mut self,
        y: u8,
        video_mem: &mut VideoMem,
        dynamic_state: &DynamicState
    ) {
        if video_mem.display_enabled() {
            let mut command_buffer = std::mem::replace(&mut self.command_buffer, None).unwrap();

            // Make push constants for sprites.
            let sprite_push_constants = PushConstants {
                tex_size: video_mem.get_tile_size(),
                atlas_size: video_mem.get_atlas_size(),
                vertex_offset: [0.0, 0.0],
                tex_offset: 0,
                palette_offset: 0,
                flags: ShaderFlags::default().bits()
            };

            let bg_y = (y as u16 + video_mem.get_scroll_y() as u16) as u8;
            if let Some(bg_vertices) = video_mem.get_background(bg_y) {
                // Add sprites below background.
                if let Some(sprite_vertices) = video_mem.get_sprites_lo(y) {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        dynamic_state,
                        sprite_vertices,
                        (self.set0.clone(), self.set1.clone()),
                        sprite_push_constants.clone()
                    ).unwrap();
                }

                // Make push constants for background.
                let background_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 0,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                // Add the background.
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    bg_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    background_push_constants
                ).unwrap();

                // Add the window if it is enabled.
                let window_y = match y as i16 - video_mem.get_window_y() as i16 {
                    val if val >= 0 => val as u8,
                    _               => 0,
                };
                if let Some(window_vertices) = video_mem.get_window(window_y) {
                    let window_push_constants = PushConstants {
                        tex_size: video_mem.get_tile_size(),
                        atlas_size: video_mem.get_atlas_size(),
                        vertex_offset: video_mem.get_window_position(),
                        tex_offset: video_mem.get_tile_data_offset(),
                        palette_offset: 1,
                        flags: ShaderFlags::default().bits()
                    };

                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        dynamic_state,
                        window_vertices,
                        (self.set0.clone(), self.set1.clone()),
                        window_push_constants
                    ).unwrap();
                }

                // Add sprites above background.
                if let Some(sprite_vertices) = video_mem.get_sprites_hi(y) {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        dynamic_state,
                        sprite_vertices,
                        (self.set0.clone(), self.set1.clone()),
                        sprite_push_constants
                    ).unwrap();
                }
            } else {
                // Add just sprites.
                if let Some(sprite_vertices) = video_mem.get_sprites_lo(y) {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        dynamic_state,
                        sprite_vertices,
                        (self.set0.clone(), self.set1.clone()),
                        sprite_push_constants.clone()
                    ).unwrap();
                }
                if let Some(sprite_vertices) = video_mem.get_sprites_hi(y) {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        dynamic_state,
                        sprite_vertices,
                        (self.set0.clone(), self.set1.clone()),
                        sprite_push_constants
                    ).unwrap();
                }
            }

            self.command_buffer = Some(command_buffer);
        }
    }

    fn draw_cgb_line(
        &mut self,
        y: u8,
        video_mem: &mut VideoMem,
        dynamic_state: &DynamicState
    ) {
        let mut command_buffer = std::mem::replace(&mut self.command_buffer, None).unwrap();

        // Make push constants for sprites.
        let sprite_push_constants = PushConstants {
            tex_size: video_mem.get_tile_size(),
            atlas_size: video_mem.get_atlas_size(),
            vertex_offset: [0.0, 0.0],
            tex_offset: 0,
            palette_offset: 16,
            flags: ShaderFlags::default().bits()
        };

        if video_mem.get_background_priority() {
            // Draw background tile clear colours
            let bg_y = (y as u16 + video_mem.get_scroll_y() as u16) as u8;
            let window_y = match y as i16 - video_mem.get_window_y() as i16 {
                val if val >= 0 => val as u8,
                _               => 0,
            };

            if let Some(background) = video_mem.get_background(bg_y) {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: (ShaderFlags::WRAPAROUND | ShaderFlags::BLOCK_COLOUR).bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    background,
                    (self.set0.clone(), self.set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Draw sprites below background.
            if let Some(sprite_vertices) = video_mem.get_sprites_lo(y) {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    sprite_vertices.clone(),
                    (self.set0.clone(), self.set1.clone()),
                    sprite_push_constants.clone()
                ).unwrap();
            }

            // Add background.
            if let Some(background) = video_mem.get_background(bg_y) {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 0,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    background,
                    (self.set0.clone(), self.set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Add the window if it is enabled.
            if let Some(window_vertices) = video_mem.get_window(window_y) {
                let window_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_window_position(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    window_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    window_push_constants
                ).unwrap();
            }

            // Add sprites above background.
            if let Some(sprite_vertices) = video_mem.get_sprites_hi(y) {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    sprite_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    sprite_push_constants
                ).unwrap();
            }

            // Add high priority background and window.
            if let Some(background) = video_mem.get_background_hi(bg_y) {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    background,
                    (self.set0.clone(), self.set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Add high priority window.
            if let Some(window_vertices) = video_mem.get_window_hi(window_y) {
                let window_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_window_position(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    window_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    window_push_constants
                ).unwrap();
            }
        } else {
            // Ignore priority bits.

            // Add the background.
            // Make push constants for background.
            let background_push_constants = PushConstants {
                tex_size: video_mem.get_tile_size(),
                atlas_size: video_mem.get_atlas_size(),
                vertex_offset: video_mem.get_bg_scroll(),
                tex_offset: video_mem.get_tile_data_offset(),
                palette_offset: 8,
                flags: ShaderFlags::WRAPAROUND.bits()
            };

            let bg_y = (y as u16 + video_mem.get_scroll_y() as u16) as u8;
            command_buffer = command_buffer.draw(
                self.pipeline.clone(),
                dynamic_state,
                video_mem.get_background(bg_y).unwrap(),
                (self.set0.clone(), self.set1.clone()),
                background_push_constants
            ).unwrap();

            // Add the window if it is enabled.
            let window_y = match y as i16 - video_mem.get_window_y() as i16 {
                val if val >= 0 => val as u8,
                _               => 0,
            };
            if let Some(window_vertices) = video_mem.get_window(window_y) {
                let window_push_constants = PushConstants {
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    vertex_offset: video_mem.get_window_position(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    window_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    window_push_constants
                ).unwrap();
            }

            // Add all sprites.
            if let Some(sprite_vertices) = video_mem.get_sprites_lo(y) {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    sprite_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    sprite_push_constants.clone()
                ).unwrap();
            }
            if let Some(sprite_vertices) = video_mem.get_sprites_hi(y) {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    dynamic_state,
                    sprite_vertices,
                    (self.set0.clone(), self.set1.clone()),
                    sprite_push_constants
                ).unwrap();
            }
        }

        self.command_buffer = Some(command_buffer);
    }

    fn finish_drawing(self) -> (AutoCommandBuffer, Box<dyn GpuFuture>) {
        (
            self.command_buffer.unwrap()
                .end_render_pass().unwrap()
                .copy_image_to_buffer(self.render_image, self.output_image).unwrap()
                .build().unwrap(),
            self.image_future,
        )
    }
}

    /*#[allow(dead_code)]
    fn draw_debug(
        &mut self,
        video_mem: &mut super::mem::VideoMem,
        mut command_buffer: AutoCommandBufferBuilder,
        image: Arc<ImmutableImage<R8Uint>>
    ) -> AutoCommandBufferBuilder {
        // Make descriptor set to bind texture atlas.
        let set0 = Arc::new(self.set_pools[0].next()
            .add_sampled_image(image, self.sampler.clone()).unwrap()
            .build().unwrap());

        // Make descriptor set for palette.
        let set1 = Arc::new(self.set_pools[1].next()
            .add_buffer(video_mem.get_palette_buffer().clone()).unwrap()
            .build().unwrap());

        let mut v = Vec::new();
        let tl = super::mem::vertex::Corner::TopLeft as u32;
        let bl = super::mem::vertex::Corner::BottomLeft as u32;
        let tr = super::mem::vertex::Corner::TopRight as u32;
        let br = super::mem::vertex::Corner::BottomRight as u32;
        let tile_size = 1.0 / 8.0;

        for y in 0..16 {
            for x in 0..16 {
                let tex_num = (y * 16) + x;
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0, y as f32 / 8.0 - 1.0], data: tex_num | tl});
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0, y as f32 / 8.0 - 1.0 + tile_size], data: tex_num | bl});
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0 + tile_size, y as f32 / 8.0 - 1.0], data: tex_num | tr});
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0, y as f32 / 8.0 - 1.0 + tile_size], data: tex_num | bl});
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0 + tile_size, y as f32 / 8.0 - 1.0], data: tex_num | tr});
                v.push(Vertex {position: [x as f32 / 8.0 - 1.0 + tile_size, y as f32 / 8.0 - 1.0 + tile_size], data: tex_num | br});
            }
        }

        let vertices = vulkano::buffer::CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            vulkano::buffer::BufferUsage::all(),
            v.iter().cloned()
        ).unwrap();

        let push_constants = PushConstants {
            vertex_offset: [0.0, 0.0],
            tex_size: video_mem.get_tile_size(),
            atlas_size: video_mem.get_atlas_size(),
            tex_offset: 0,  // 256
            palette_offset: 0,
            flags: ShaderFlags::default().bits()
        };

        command_buffer = command_buffer.draw(
            self.pipeline.clone(),
            &self.dynamic_state,
            vertices,
            (set0.clone(), set1.clone()),
            push_constants
        ).unwrap();

        command_buffer
    }*/
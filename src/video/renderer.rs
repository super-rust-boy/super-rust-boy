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
    pipeline::{
        GraphicsPipeline,
        viewport::Viewport,
        vertex::SingleBufferDefinition
    },
    command_buffer::{
        AutoCommandBufferBuilder, DynamicState
    },
    sampler::{
        Filter,
        MipmapMode,
        Sampler,
        SamplerAddressMode
    },
    swapchain::{
        Swapchain, Surface, SurfaceTransform, PresentMode, acquire_next_image
    },
    sync::{
        now, GpuFuture
    },
    descriptor::{
        descriptor_set::FixedSizeDescriptorSetsPool,
        pipeline_layout::PipelineLayoutAbstract
    },
    image::immutable::ImmutableImage,
    format::R8Uint
};

use vulkano_win::VkSurfaceBuild;

use winit::{
    EventsLoop,
    Window,
    WindowBuilder
};

use bitflags::bitflags;

use std::sync::Arc;

bitflags!{
    #[derive(Default)]
    struct ShaderFlags: u32 {
        const WRAPAROUND =      1;
        const BLOCK_COLOUR =    2;
    }
}

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

#[derive(Clone)]
struct PushConstants {
    pub vertex_offset:  [f32; 2],
    pub tex_size:       [f32; 2],
    pub atlas_size:     [f32; 2],
    pub tex_offset:     u32,
    pub palette_offset: u32,
    pub flags:          u32
}

vulkano::impl_vertex!(Vertex, position, data);

type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<dyn PipelineLayoutAbstract + Send + Sync>,
    Arc<dyn RenderPassAbstract + Send + Sync>
>;

pub struct Renderer {
    // Core
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<RenderPipeline>,
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    surface: Arc<Surface<Window>>,
    // Uniforms
    sampler: Arc<Sampler>,
    set_pools: Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    // Vulkan data
    swapchain: Arc<Swapchain<Window>>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    dynamic_state: DynamicState,
    previous_frame_future: Box<dyn GpuFuture>
    // Custom data
}

impl Renderer {
    // Create and initialise renderer.
    pub fn new(events_loop: &EventsLoop) -> Self {
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
        let surface = WindowBuilder::new()
            .with_dimensions((320, 288).into())
            .with_title("Super Rust Boy")
            .build_vk_surface(&events_loop, instance.clone())
            .expect("Couldn't create surface");

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
        let ((swapchain, images), dynamic_state) = {

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
        };

        // Make the render pass to insert into the command queue.
        let render_pass = Arc::new(vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),//Format::R8G8B8A8Unorm,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap()) as Arc<dyn RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

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

        Renderer {
            device: device.clone(),
            queue: queue,
            pipeline: pipeline,
            render_pass: render_pass,
            surface: surface,

            sampler: sampler,
            set_pools: set_pools,

            swapchain: swapchain,
            framebuffers: framebuffers,
            dynamic_state: dynamic_state,
            previous_frame_future: Box::new(now(device.clone())) as Box<dyn GpuFuture>
        }
    }

    // Re-create the swapchain and framebuffers.
    pub fn create_swapchain(&mut self) {
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
    }

    // Render a frame.
    pub fn render(&mut self, video_mem: &mut super::mem::VideoMem, cgb_mode: bool) {
        self.previous_frame_future.cleanup_finished();

        // Get current framebuffer index from the swapchain.
        let (image_num, acquire_future) = acquire_next_image(self.swapchain.clone(), None)
            .expect("Didn't get next image");

        // Make image with current texture.
        // TODO: only re-create the image when the data has changed.
        let (image, write_future) = video_mem.get_tile_atlas(&self.device, &self.queue);
        
        // Start building command buffer using pipeline and framebuffer, starting with the background vertices.
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![video_mem.get_clear_colour().into()]).unwrap();

        // Render in the specified mode.
        command_buffer_builder = if cgb_mode {
            self.draw_cgb(video_mem, command_buffer_builder, image)
        } else {
            self.draw_gb(video_mem, command_buffer_builder, image)
        };

        // DEBUG
        //command_buffer_builder = self.draw_debug(video_mem, command_buffer_builder, image);

        // Finish command buffer.
        let command_buffer = command_buffer_builder.end_render_pass().unwrap().build().unwrap();

        // Wait until previous frame is done.
        let mut now_future = Box::new(now(self.device.clone())) as Box<dyn GpuFuture>;
        std::mem::swap(&mut self.previous_frame_future, &mut now_future);

        // Wait until previous frame is done,
        // _and_ the framebuffer has been acquired,
        // _and_ the texture has been uploaded.
        let future = now_future.join(acquire_future)
            .join(write_future)
            .then_execute(self.queue.clone(), command_buffer).unwrap()                      // Run the commands (pipeline and render)
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)  // Present newly rendered image.
            .then_signal_fence_and_flush();                                                 // Signal done and flush the pipeline.

        match future {
            Ok(future) => self.previous_frame_future = Box::new(future) as Box<_>,
            Err(e) => println!("Err: {:?}", e),
        }
    }


    pub fn get_device(&self) -> Arc<Device> {
        self.device.clone()
    }
}

// Internal render functions.
impl Renderer {
    // Render a frame in GB mode.
    fn draw_gb(
        &mut self,
        video_mem: &mut super::mem::VideoMem,
        mut command_buffer: AutoCommandBufferBuilder,
        image: Arc<ImmutableImage<R8Uint>>
    ) -> AutoCommandBufferBuilder {

        if video_mem.display_enabled() {
            // Make descriptor set to bind texture atlas.
            let set0 = Arc::new(self.set_pools[0].next()
                .add_sampled_image(image, self.sampler.clone()).unwrap()
                .build().unwrap());

            // Make descriptor set for palette.
            let set1 = Arc::new(self.set_pools[1].next()
                .add_buffer(video_mem.get_palette_buffer().clone()).unwrap()
                .build().unwrap());

            // Make push constants for sprites.
            let sprite_push_constants = PushConstants {
                vertex_offset: [0.0, 0.0],
                tex_size: video_mem.get_tile_size(),
                atlas_size: video_mem.get_atlas_size(),
                tex_offset: 0,
                palette_offset: 0,
                flags: ShaderFlags::default().bits()
            };

            if let Some(bg_vertices) = video_mem.get_background() {
                // Add sprites below background.
                if let Some(sprite_vertices) = video_mem.get_sprites_lo() {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        &self.dynamic_state,
                        sprite_vertices,
                        (set0.clone(), set1.clone()),
                        sprite_push_constants.clone()
                    ).unwrap();
                }

                // Make push constants for background.
                let background_push_constants = PushConstants {
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 0,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                // Add the background.
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    bg_vertices,
                    (set0.clone(), set1.clone()),
                    background_push_constants
                ).unwrap();

                // Add the window if it is enabled.
                if let Some(window_vertices) = video_mem.get_window() {
                    let window_push_constants = PushConstants {
                        vertex_offset: video_mem.get_window_position(),
                        tex_size: video_mem.get_tile_size(),
                        atlas_size: video_mem.get_atlas_size(),
                        tex_offset: video_mem.get_tile_data_offset(),
                        palette_offset: 1,
                        flags: ShaderFlags::default().bits()
                    };

                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        &self.dynamic_state,
                        window_vertices,
                        (set0.clone(), set1.clone()),
                        window_push_constants
                    ).unwrap();
                }

                // Add sprites above background.
                if let Some(sprite_vertices) = video_mem.get_sprites_hi() {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        &self.dynamic_state,
                        sprite_vertices,
                        (set0.clone(), set1.clone()),
                        sprite_push_constants
                    ).unwrap();
                }
            } else {
                // Add just sprites.
                if let Some(sprite_vertices) = video_mem.get_sprites_lo() {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        &self.dynamic_state,
                        sprite_vertices,
                        (set0.clone(), set1.clone()),
                        sprite_push_constants.clone()
                    ).unwrap();
                }
                if let Some(sprite_vertices) = video_mem.get_sprites_hi() {
                    command_buffer = command_buffer.draw(
                        self.pipeline.clone(),
                        &self.dynamic_state,
                        sprite_vertices,
                        (set0.clone(), set1.clone()),
                        sprite_push_constants
                    ).unwrap();
                }
            }
        }
        
        command_buffer
    }

    // Render a frame in CGB mode.
    fn draw_cgb(
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

        // Make push constants for sprites.
        let sprite_push_constants = PushConstants {
            vertex_offset: [0.0, 0.0],
            tex_size: video_mem.get_tile_size(),
            atlas_size: video_mem.get_atlas_size(),
            tex_offset: 0,
            palette_offset: 16,
            flags: ShaderFlags::default().bits()
        };

        if video_mem.get_background_priority() {
            // Draw background tile clear colours
            if let Some(background) = video_mem.get_background() {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: (ShaderFlags::WRAPAROUND | ShaderFlags::BLOCK_COLOUR).bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    background,
                    (set0.clone(), set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Draw sprites below background.
            if let Some(sprite_vertices) = video_mem.get_sprites_lo() {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    sprite_vertices.clone(),
                    (set0.clone(), set1.clone()),
                    sprite_push_constants.clone()
                ).unwrap();
            }

            // Add background.
            if let Some(background) = video_mem.get_background() {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 0,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    background,
                    (set0.clone(), set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Add the window if it is enabled.
            if let Some(window_vertices) = video_mem.get_window() {
                let window_push_constants = PushConstants {
                    vertex_offset: video_mem.get_window_position(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    window_vertices,
                    (set0.clone(), set1.clone()),
                    window_push_constants
                ).unwrap();
            }

            // Add sprites above background.
            if let Some(sprite_vertices) = video_mem.get_sprites_hi() {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    sprite_vertices,
                    (set0.clone(), set1.clone()),
                    sprite_push_constants
                ).unwrap();
            }

            // Add high priority background and window.
            if let Some(background) = video_mem.get_background_hi() {
                // Make push constants for background.
                let background_push_constants = PushConstants {
                    vertex_offset: video_mem.get_bg_scroll(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::WRAPAROUND.bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    background,
                    (set0.clone(), set1.clone()),
                    background_push_constants
                ).unwrap();
            }

            // Add high priority window.
            if let Some(window_vertices) = video_mem.get_window_hi() {
                let window_push_constants = PushConstants {
                    vertex_offset: video_mem.get_window_position(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    window_vertices,
                    (set0.clone(), set1.clone()),
                    window_push_constants
                ).unwrap();
            }
        } else {
            // Ignore priority bits.

            // Add the background.
            // Make push constants for background.
            let background_push_constants = PushConstants {
                vertex_offset: video_mem.get_bg_scroll(),
                tex_size: video_mem.get_tile_size(),
                atlas_size: video_mem.get_atlas_size(),
                tex_offset: video_mem.get_tile_data_offset(),
                palette_offset: 8,
                flags: ShaderFlags::WRAPAROUND.bits()
            };

            command_buffer = command_buffer.draw(
                self.pipeline.clone(),
                &self.dynamic_state,
                video_mem.get_background().unwrap(),
                (set0.clone(), set1.clone()),
                background_push_constants
            ).unwrap();

            // Add the window if it is enabled.
            if let Some(window_vertices) = video_mem.get_window() {
                let window_push_constants = PushConstants {
                    vertex_offset: video_mem.get_window_position(),
                    tex_size: video_mem.get_tile_size(),
                    atlas_size: video_mem.get_atlas_size(),
                    tex_offset: video_mem.get_tile_data_offset(),
                    palette_offset: 8,
                    flags: ShaderFlags::default().bits()
                };

                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    window_vertices,
                    (set0.clone(), set1.clone()),
                    window_push_constants
                ).unwrap();
            }

            // Add all sprites.
            if let Some(sprite_vertices) = video_mem.get_sprites_lo() {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    sprite_vertices,
                    (set0.clone(), set1.clone()),
                    sprite_push_constants.clone()
                ).unwrap();
            }
            if let Some(sprite_vertices) = video_mem.get_sprites_hi() {
                command_buffer = command_buffer.draw(
                    self.pipeline.clone(),
                    &self.dynamic_state,
                    sprite_vertices,
                    (set0.clone(), set1.clone()),
                    sprite_push_constants
                ).unwrap();
            }
        }

        command_buffer
    }

    #[allow(dead_code)]
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
    }
}

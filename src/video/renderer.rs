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
    }
};

use vulkano_win::VkSurfaceBuild;

use winit::{
    EventsLoop,
    Window,
    WindowBuilder
};

use std::sync::Arc;

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_corner_offset: [f32; 2],
    pub tex_num: i32
}

struct PushConstants {
    pub vertex_offset: [f32; 2],
    pub tex_offset: i32
}

vulkano::impl_vertex!(Vertex, position, tex_corner_offset, tex_num);

type RenderPipeline = GraphicsPipeline<
    SingleBufferDefinition<Vertex>,
    Box<PipelineLayoutAbstract + Send + Sync>,
    Arc<RenderPassAbstract + Send + Sync>
>;

pub struct Renderer {
    // Core
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<RenderPipeline>,
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    surface: Arc<Surface<Window>>,
    // Uniforms
    sampler: Arc<Sampler>,
    set_pools: Vec<FixedSizeDescriptorSetsPool<Arc<RenderPipeline>>>,
    // Vulkan data
    swapchain: Arc<Swapchain<Window>>,
    framebuffers: Vec<Arc<FramebufferAbstract + Send + Sync>>,
    dynamic_state: DynamicState,
    previous_frame_future: Box<GpuFuture>
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
        let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();

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
            let dimensions = caps.current_extent.unwrap_or([512, 512]);

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
        ).unwrap()) as Arc<RenderPassAbstract + Send + Sync>;

        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            ) as Arc<FramebufferAbstract + Send + Sync>
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
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
        );

        // Make descriptor set pools.
        let set_pools = vec![
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 0),
            FixedSizeDescriptorSetsPool::new(pipeline.clone(), 1)
        ];

        //Box::new(now(device.clone()).join(palette_future)) as Box<GpuFuture>;

        Renderer {
            device: device,
            queue: queue,
            pipeline: pipeline,
            render_pass: render_pass,
            surface: surface,

            sampler: sampler,
            set_pools: set_pools,

            swapchain: swapchain,
            framebuffers: framebuffers,
            dynamic_state: dynamic_state,
            previous_frame_future: Box::new(now(device.clone())) as Box<GpuFuture>
        }
    }

    // Re-create the swapchain and framebuffers.
    pub fn create_swapchain(&mut self) {
        let caps = self.surface.capabilities(self.device.physical_device())
                .expect("Failed to get surface capabilities");
        let dimensions = caps.current_extent.unwrap_or([512, 512]);

        // Get a swapchain and images for use with the swapchain.
        let (swapchain, images) = {
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            Swapchain::new(self.device.clone(), self.surface.clone(),
                caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &self.queue,
                SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None
            ).expect("Failed to create swapchain")
        };

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
            ) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        self.swapchain = swapchain;
    }

    // Render a frame
    pub fn render(&mut self, video_mem: &mut super::mem::VideoMem) {
        self.previous_frame_future.cleanup_finished();

        // Get current framebuffer index from the swapchain.
        let (image_num, acquire_future) = acquire_next_image(self.swapchain.clone(), None)
            .expect("Didn't get next image");

        // Make image with current texture.
        // TODO: only re-create the image when the data has changed.
        let (image, write_future) = video_mem.get_tile_atlas(self.queue.clone());

        // Make descriptor set to bind texture atlas.
        let set0 = self.set_pools[0].next()
            .add_sampled_image(image.clone(), self.sampler.clone()).unwrap()
            .build().unwrap();

        // Make descriptor set for palettes.
        let set1 = self.set_pools[1].next()
            .add_buffer(video_mem.get_palette_buffer().clone()).unwrap()
            .build().unwrap();
        
        // Start building command buffer using pipeline and framebuffer, starting with the background vertices.
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary_one_time_submit(self.device.clone(), self.queue.family()).unwrap()
            .begin_render_pass(self.framebuffers[image_num].clone(), false, vec![[1.0, 1.0, 1.0, 1.0].into()]).unwrap()
            .draw(
                self.pipeline.clone(),
                &self.dynamic_state,
                video_mem.get_background().clone(),
                (set0, set1),
                PushConstants {vertex_offset: video_mem.get_bg_scroll(), tex_offset: video_mem.get_tile_data_offset()}
            ).unwrap();

        // Add the window if it is enabled.
        if let Some(window_vertices) = video_mem.get_window() {
            command_buffer_builder.draw(
                self.pipeline.clone(),
                &self.dynamic_state,
                window_vertices.clone(),
                (set0, set1),
                PushConstants {vertex_offset: video_mem.get_window_position(), tex_offset: video_mem.get_tile_data_offset()}
            ).unwrap();
        }

        // Add sprites.


        // Finish command buffer.
        let command_buffer = command_buffer_builder.end_render_pass().unwrap().build().unwrap();

        // Wait until previous frame is done.
        let mut now_future = Box::new(now(self.device.clone())) as Box<GpuFuture>;
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

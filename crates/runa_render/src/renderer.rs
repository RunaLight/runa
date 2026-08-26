use std::{collections::HashMap, num::NonZero, sync::Arc};

use crate::{
    font::FontManager, pipelines::BackgroundPipeline, pipelines::BackgroundUniforms,
    pipelines::MeshPipeline, pipelines::MeshUniforms, pipelines::PointLightUniform,
    pipelines::PostProcessPipeline, pipelines::PostProcessUniforms, pipelines::SpritePipeline,
    pipelines::UIPipeline, pipelines::UITexturedVertex, pipelines::UIUniforms, pipelines::UIVertex,
    pipelines::MAX_POINT_LIGHTS, resources::texture::GpuTexture,
};
use glam::Vec2;
use runa_asset::TextureAsset;
use runa_render_api::{BackgroundModeData, FontId, InstanceData, RenderCommands, RenderQueue};
use wgpu::util::DeviceExt;
use wgpu::{MemoryHints::Performance, Trace};
use wgpu::{Texture, TextureView};
use winit::window::Window;

/// Vertex structure for sprite quads.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

/// Global uniform buffer data containing view-projection matrix and aspect ratio.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
    pub view_proj: [[f32; 4]; 4],
    pub aspect: f32,
    pub _padding: [f32; 7],
}

/// Offscreen render target used by the editor viewport and previews.
pub struct RenderTarget {
    _color_texture: Texture,
    render_color_view: TextureView,
    sample_color_view: TextureView,
    _depth_texture: Texture,
    depth_view: TextureView,
    size: (u32, u32),
    _render_format: wgpu::TextureFormat,
    _sample_format: wgpu::TextureFormat,
}

impl RenderTarget {
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn render_view(&self) -> &TextureView {
        &self.render_color_view
    }

    pub fn sample_view(&self) -> &TextureView {
        &self.sample_color_view
    }
}

/// Main renderer struct managing GPU resources and rendering.
pub struct Renderer<'window> {
    pub surface: wgpu::Surface<'window>,
    pub surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    sprite_pipeline: SpritePipeline,
    background_pipeline: BackgroundPipeline,

    mesh_pipeline: MeshPipeline,

    ui_pipeline: UIPipeline,
    ui_uniform_buffer: wgpu::Buffer,
    ui_bind_group: Option<wgpu::BindGroup>,

    globals_buffer: wgpu::Buffer,

    textures: HashMap<usize, Arc<GpuTexture>>,
    nearest_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    linear_sampler: wgpu::Sampler,

    font_manager: FontManager,

    textures_cache: HashMap<usize, Arc<TextureAsset>>,
    bind_group_cache: HashMap<usize, wgpu::BindGroup>,

    depth_view: TextureView,
    depth_texture: Texture,

    post_process_pipeline: PostProcessPipeline,

    /// Intermediate render target for post-processing
    intermediate_texture: Option<Texture>,
    intermediate_view: Option<TextureView>,

    /// Clamp-to-edge sampler for post-process sampling
    post_sampler: wgpu::Sampler,

    /// Base quad vertices (6 vertices, static).
    quad_buffer: wgpu::Buffer,
    /// Dynamic instance buffer - resized as needed.
    instance_buffer: wgpu::Buffer,
    /// Current capacity of instance buffer in number of instances.
    instance_buffer_capacity: usize,

    /// GPU cache for 3D meshes: mesh_id -> (vertex_buffer, index_buffer).
    mesh_gpu_cache: HashMap<u64, (wgpu::Buffer, wgpu::Buffer)>,
    /// Persistent uniform buffer for all 3D meshes (written each frame via write_buffer).
    mesh_uniform_buffer: wgpu::Buffer,
    /// Current capacity of mesh uniform buffer in bytes.
    mesh_uniform_capacity: u64,
    /// Aligned stride per mesh uniform (respects min_uniform_buffer_offset_alignment).
    uniform_stride: u64,

    /// Persistent uniform buffer for background (reused each frame).
    background_uniform_buffer: wgpu::Buffer,
    /// Persistent uniform buffer for post-process (reused each frame).
    postprocess_uniform_buffer: wgpu::Buffer,

    // Per-frame temp containers — reused to avoid alloc/dealloc churn.
    all_instances: Vec<InstanceData>,
    sprite_instances: Vec<(i32, f32, usize, usize, InstanceData)>,
    mesh_items: Vec<(i32, f32, usize)>,
    ui_vertices: Vec<UIVertex>,
    batches: Vec<(i32, f32, usize, usize, usize, usize)>,
    orders: Vec<i32>,
}

impl<'window> Renderer<'window> {
    /// Creates a new renderer with the given window and vsync setting.
    ///
    /// # Arguments
    /// * `window` - The window to render to
    /// * `vsync` - Enable vertical sync for frame presentation
    pub async fn new_async(window: Arc<Window>, vsync: bool) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: Default::default(),
                memory_hints: Performance,
                trace: Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let size = window.inner_size();

        let capabilities = surface.get_capabilities(&adapter);
        let preferred_format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| *format == wgpu::TextureFormat::Rgba8UnormSrgb)
            .or_else(|| {
                capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(|format| *format == wgpu::TextureFormat::Bgra8UnormSrgb)
            })
            .or_else(|| {
                capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(|format| *format == wgpu::TextureFormat::Rgba8Unorm)
            })
            .or_else(|| {
                capabilities
                    .formats
                    .iter()
                    .copied()
                    .find(|format| *format == wgpu::TextureFormat::Bgra8Unorm)
            })
            .unwrap_or(capabilities.formats[0]);

        let surface_config: wgpu::SurfaceConfiguration = if vsync {
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: preferred_format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }
        } else {
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: preferred_format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::AutoNoVsync,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }
        };

        surface.configure(&device, &surface_config);

        let sprite_pipeline = SpritePipeline::new(
            &device,
            surface_config.format,
            wgpu::TextureFormat::Depth32Float,
        );
        let background_pipeline = BackgroundPipeline::new(&device, surface_config.format);

        let identity_mat = glam::Mat4::IDENTITY.to_cols_array_2d();
        let globals = Globals {
            view_proj: identity_mat,
            aspect: surface_config.width as f32 / surface_config.height as f32,
            _padding: [0.0; 7],
        };

        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globals Buffer"),
            contents: bytemuck::bytes_of(&globals),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Pixel Art Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Linear Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("PostProcess Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let post_process_pipeline = PostProcessPipeline::new(&device, surface_config.format);
        let intermediate_texture = Self::create_intermediate_texture(
            &device,
            (surface_config.width, surface_config.height),
            surface_config.format,
        );
        let intermediate_view = intermediate_texture
            .as_ref()
            .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));

        let font_manager = FontManager::new(&device, &queue);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("Depth Texture"),
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 3D mesh pipeline
        let mesh_pipeline = MeshPipeline::new(
            &device,
            surface_config.format,
            wgpu::TextureFormat::Depth32Float,
        );

        // UI pipeline for debug rectangles and text
        let ui_pipeline = UIPipeline::new(&device, surface_config.format);

        let ui_uniforms = UIUniforms {
            screen_width: surface_config.width as f32,
            screen_height: surface_config.height as f32,
        };

        let ui_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Uniform Buffer"),
            contents: bytemuck::bytes_of(&ui_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let ui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &ui_pipeline.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ui_uniform_buffer.as_entire_binding(),
            }],
            label: Some("UI Bind Group"),
        });

        const QUAD_VERTICES: &[Vertex] = &[
            Vertex {
                position: [-0.5, -0.5, 0.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.0],
                tex_coords: [0.0, 0.0],
            },
        ];

        let quad_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Instance buffer with initial capacity
        const INITIAL_INSTANCE_CAPACITY: usize = 1000;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: (size_of::<InstanceData>() * INITIAL_INSTANCE_CAPACITY) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment;
        let uniform_stride = u64::from(
            (size_of::<MeshUniforms>() as u32).div_ceil(uniform_alignment) * uniform_alignment,
        );
        const INITIAL_MESH_UNIFORM_SIZE: u64 = 65536; // 64KB — room for ~40 meshes
        let mesh_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh Uniform Buffer"),
            size: INITIAL_MESH_UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let background_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Background Uniform Buffer"),
            size: size_of::<BackgroundUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let postprocess_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("PostProcess Uniform Buffer"),
            size: size_of::<PostProcessUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface,
            surface_config,
            device,
            queue,
            sprite_pipeline,
            background_pipeline,
            mesh_pipeline,
            ui_pipeline,
            ui_uniform_buffer,
            ui_bind_group: Some(ui_bind_group),
            globals_buffer,
            textures: HashMap::new(),
            nearest_sampler,
            linear_sampler,
            font_manager,
            textures_cache: HashMap::new(),
            bind_group_cache: HashMap::new(),
            depth_view,
            depth_texture,
            post_process_pipeline,
            intermediate_texture,
            intermediate_view,
            post_sampler,
            quad_buffer,
            instance_buffer,
            instance_buffer_capacity: INITIAL_INSTANCE_CAPACITY,
            mesh_gpu_cache: HashMap::new(),
            mesh_uniform_buffer,
            mesh_uniform_capacity: INITIAL_MESH_UNIFORM_SIZE,
            uniform_stride,
            background_uniform_buffer,
            postprocess_uniform_buffer,
            all_instances: Vec::new(),
            sprite_instances: Vec::new(),
            mesh_items: Vec::new(),
            ui_vertices: Vec::new(),
            batches: Vec::new(),
            orders: Vec::new(),
        }
    }

    /// Creates a new renderer synchronously (blocking).
    pub fn new(window: Arc<Window>, vsync: bool) -> Self {
        pollster::block_on(Self::new_async(window, vsync))
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn capture_render_target_rgba8(
        &self,
        target: &RenderTarget,
    ) -> Result<(u32, u32, Vec<u8>), String> {
        let (width, height) = target.size();
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let output_size = padded_bytes_per_row as u64 * height as u64;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Render Target Readback"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Target Readback Encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target._color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result.map_err(|error| error.to_string()));
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|error| error.to_string())?
            .map_err(|error| format!("Failed to map readback buffer: {error}"))?;

        let mapped = slice.get_mapped_range();
        let mut pixels = vec![0u8; (width * height * bytes_per_pixel) as usize];
        let is_bgra = matches!(
            target._sample_format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );

        for row in 0..height as usize {
            let src_offset = row * padded_bytes_per_row as usize;
            let dst_offset = row * unpadded_bytes_per_row as usize;
            let src = &mapped[src_offset..src_offset + unpadded_bytes_per_row as usize];
            let dst = &mut pixels[dst_offset..dst_offset + unpadded_bytes_per_row as usize];

            if is_bgra {
                let (src_px, _) = src.as_chunks::<4>();
                let (dst_px, _) = dst.as_chunks_mut::<4>();
                for (src_px, dst_px) in src_px.iter().zip(dst_px.iter_mut()) {
                    dst_px[0] = src_px[2];
                    dst_px[1] = src_px[1];
                    dst_px[2] = src_px[0];
                    dst_px[3] = src_px[3];
                }
            } else {
                dst.copy_from_slice(src);
            }
        }

        drop(mapped);
        buffer.unmap();
        Ok((width, height, pixels))
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    pub fn create_render_target(&self, size: (u32, u32)) -> RenderTarget {
        Self::build_render_target(&self.device, size, self.surface_config.format)
    }

    /// Resizes the surface and recreates the depth texture.
    pub fn resize(&mut self, new_size: (u32, u32)) {
        let (width, height) = new_size;
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
        let depth = Self::create_depth_texture(
            &self.device,
            (self.surface_config.width, self.surface_config.height),
        );
        self.depth_texture = depth.0;
        self.depth_view = depth.1;

        // Recreate intermediate texture
        self.intermediate_texture = Self::create_intermediate_texture(
            &self.device,
            (self.surface_config.width, self.surface_config.height),
            self.surface_config.format,
        );
        self.intermediate_view = self
            .intermediate_texture
            .as_ref()
            .map(|tex| tex.create_view(&wgpu::TextureViewDescriptor::default()));

        // Update UI uniforms
        let ui_uniforms = UIUniforms {
            screen_width: self.surface_config.width as f32,
            screen_height: self.surface_config.height as f32,
        };
        self.queue
            .write_buffer(&self.ui_uniform_buffer, 0, bytemuck::bytes_of(&ui_uniforms));
    }

    /// Renders the current frame using the provided render queue and camera matrix.
    pub fn draw(&mut self, queue: &RenderQueue, camera_matrix: glam::Mat4, virtual_size: Vec2) {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            _ => return, // Surface lost, timeout, etc.
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: None,
                ..Default::default()
            });

        let target_size = (self.surface_config.width, self.surface_config.height);
        let has_effects = queue.screen_effects.enabled.has_any();

        if has_effects {
            // Render to intermediate texture first
            let intermediate_view = self
                .intermediate_view
                .clone()
                .expect("Intermediate texture not available");
            let depth_view = self.depth_view.clone();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Scene Encoder"),
                });
            self.encode_render_passes(
                &mut encoder,
                &intermediate_view,
                &depth_view,
                target_size,
                queue,
                camera_matrix,
                virtual_size,
            );
            self.queue.submit(Some(encoder.finish()));

            // Now do post-process pass: intermediate -> surface
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("PostProcess Encoder"),
                });
            self.encode_post_process_pass(
                &mut encoder,
                &surface_view,
                target_size,
                &queue.screen_effects,
            );
            self.queue.submit(Some(encoder.finish()));
        } else {
            // Render directly to surface
            let depth_view = self.depth_view.clone();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });
            self.encode_render_passes(
                &mut encoder,
                &surface_view,
                &depth_view,
                target_size,
                queue,
                camera_matrix,
                virtual_size,
            );
            self.queue.submit(Some(encoder.finish()));
        }

        let _ = self.device.poll(wgpu::PollType::Poll);
        surface_texture.present();
    }

    pub fn draw_to_target(
        &mut self,
        target: &RenderTarget,
        queue: &RenderQueue,
        camera_matrix: glam::Mat4,
        virtual_size: Vec2,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Offscreen Render Encoder"),
            });
        self.encode_render_passes(
            &mut encoder,
            target.render_view(),
            &target.depth_view,
            target.size(),
            queue,
            camera_matrix,
            virtual_size,
        );
        self.queue.submit(Some(encoder.finish()));
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_render_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &TextureView,
        depth_view: &TextureView,
        target_size: (u32, u32),
        queue: &RenderQueue,
        camera_matrix: glam::Mat4,
        virtual_size: Vec2,
    ) {
        let target_aspect = target_size.0.max(1) as f32 / target_size.1.max(1) as f32;
        self.queue.write_buffer(
            &self.globals_buffer,
            0,
            bytemuck::bytes_of(&Globals {
                view_proj: camera_matrix.to_cols_array_2d(),
                aspect: (virtual_size.x / virtual_size.y) / target_aspect,
                _padding: [0.0; 7],
            }),
        );

        let ui_uniforms = UIUniforms {
            screen_width: target_size.0.max(1) as f32,
            screen_height: target_size.1.max(1) as f32,
        };
        self.queue
            .write_buffer(&self.ui_uniform_buffer, 0, bytemuck::bytes_of(&ui_uniforms));

        self.encode_background_pass(encoder, target_view, &queue.atmosphere, camera_matrix);

        self.all_instances.clear();
        self.sprite_instances.clear();
        self.mesh_items.clear();
        self.ui_vertices.clear();
        let mut ui_font_vertices_map: std::collections::HashMap<usize, Vec<UITexturedVertex>> =
            std::collections::HashMap::new();
        let mut ui_image_vertices_map: std::collections::HashMap<usize, Vec<UITexturedVertex>> =
            std::collections::HashMap::new();
        let has_lighting = !queue.directional_lights.is_empty() || !queue.point_lights.is_empty();
        let directional = queue.directional_lights.first().copied();
        let mut point_lights = [PointLightUniform::default(); MAX_POINT_LIGHTS];
        for (target, light) in point_lights
            .iter_mut()
            .zip(queue.point_lights.iter().take(MAX_POINT_LIGHTS))
        {
            *target = PointLightUniform {
                position_radius: [
                    light.position.x,
                    light.position.y,
                    light.position.z,
                    light.radius,
                ],
                color_intensity: [light.color.x, light.color.y, light.color.z, light.intensity],
                params: [light.falloff, 0.0, 0.0, 0.0],
            };
        }
        let point_light_count = queue.point_lights.len().min(MAX_POINT_LIGHTS) as u32;

        for (cmd_index, cmd) in queue.commands.iter().enumerate() {
            match cmd {
                RenderCommands::Mesh3D(params) => {
                    self.mesh_items
                        .push((params.order, params.depth, cmd_index));
                }
                RenderCommands::Sprite {
                    texture,
                    position,
                    rotation,
                    scale,
                    color,
                    uv_rect,
                    order,
                    replace_color,
                    flip_x,
                    flip_y,
                } => {
                    let tex_width = texture.width as f32;
                    let tex_height = texture.height as f32;
                    let pixels_per_unit = scale.z.max(f32::EPSILON);
                    let world_scale_x = scale.x * ((tex_width * uv_rect[2]) / pixels_per_unit);
                    let world_scale_y = scale.y * ((tex_height * uv_rect[3]) / pixels_per_unit);

                    let flip = (if *replace_color { 4 } else { 0 })
                        | (*flip_x as u32)
                        | ((*flip_y as u32) << 1);
                    let instance = InstanceData {
                        position: [position.x, position.y, position.z],
                        rotation: rotation.z,
                        scale: [world_scale_x, world_scale_y, 1.0],
                        color: *color,
                        uv_offset: [uv_rect[0], uv_rect[1]],
                        uv_size: [uv_rect[2], uv_rect[3]],
                        flip,
                    };

                    let key = Arc::as_ptr(texture) as usize;
                    self.textures_cache
                        .entry(key)
                        .or_insert_with(|| texture.clone());

                    self.sprite_instances
                        .push((*order, position.z, cmd_index, key, instance));
                }
                RenderCommands::Tile(params) => {
                    let instance = InstanceData {
                        position: [params.position.x, params.position.y, params.position.z],
                        rotation: 0.0,
                        scale: [params.size.x, params.size.y, 1.0],
                        color: params.color,
                        uv_offset: [params.uv_rect[0], params.uv_rect[1]],
                        uv_size: [params.uv_rect[2], params.uv_rect[3]],
                        flip: (params.flip_x as u32) | ((params.flip_y as u32) << 1),
                    };

                    let key = Arc::as_ptr(&params.texture) as usize;
                    self.textures_cache
                        .entry(key)
                        .or_insert_with(|| params.texture.clone());

                    self.sprite_instances.push((
                        params.order,
                        params.position.z,
                        cmd_index,
                        key,
                        instance,
                    ));
                }
                RenderCommands::TileBatch {
                    texture,
                    instances,
                    order,
                    depth,
                } => {
                    let key = Arc::as_ptr(texture) as usize;
                    self.textures_cache
                        .entry(key)
                        .or_insert_with(|| texture.clone());
                    for instance in instances {
                        self.sprite_instances
                            .push((*order, *depth, cmd_index, key, *instance));
                    }
                }
                RenderCommands::DebugRect {
                    position,
                    size,
                    color,
                } => {
                    let left = position.x - size.x / 2.0;
                    let top = position.y - size.y / 2.0;
                    let right = left + size.x;
                    let bottom = top + size.y;

                    self.ui_vertices.extend_from_slice(&[
                        UIVertex {
                            position: [left, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [left, bottom],
                            color: *color,
                        },
                        UIVertex {
                            position: [left, bottom],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, bottom],
                            color: *color,
                        },
                    ]);
                }
                RenderCommands::DebugLine {
                    start,
                    end,
                    color,
                    width,
                } => {
                    let dx = end.x - start.x;
                    let dy = end.y - start.y;
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > f32::EPSILON {
                        let nx = -dy / len * width * 0.5;
                        let ny = dx / len * width * 0.5;
                        self.ui_vertices.extend_from_slice(&[
                            UIVertex {
                                position: [start.x + nx, start.y + ny],
                                color: *color,
                            },
                            UIVertex {
                                position: [end.x + nx, end.y + ny],
                                color: *color,
                            },
                            UIVertex {
                                position: [start.x - nx, start.y - ny],
                                color: *color,
                            },
                            UIVertex {
                                position: [start.x - nx, start.y - ny],
                                color: *color,
                            },
                            UIVertex {
                                position: [end.x + nx, end.y + ny],
                                color: *color,
                            },
                            UIVertex {
                                position: [end.x - nx, end.y - ny],
                                color: *color,
                            },
                        ]);
                    }
                }
                RenderCommands::Text {
                    text,
                    position,
                    color,
                    size,
                    outline,
                } => {
                    let (char_width, char_height) = self.font_manager.char_size();
                    let scale = *size / self.font_manager.base_font_size();
                    let char_w = char_width as f32 * scale;
                    let char_h = char_height as f32 * scale;

                    if let Some(ol) = outline {
                        // Outline: 4 cardinal offsets in outline color
                        let offsets = [
                            (-ol.width, 0.0),
                            (ol.width, 0.0),
                            (0.0, -ol.width),
                            (0.0, ol.width),
                        ];
                        for (ox, oy) in offsets {
                            self.emit_text_vertices(
                                text,
                                position.x + ox,
                                position.y + oy,
                                &ol.color,
                                char_w,
                                char_h,
                                scale,
                                char_width as f32,
                                &mut ui_font_vertices_map,
                            );
                        }
                    }
                    // Main text
                    self.emit_text_vertices(
                        text,
                        position.x,
                        position.y,
                        color,
                        char_w,
                        char_h,
                        scale,
                        char_width as f32,
                        &mut ui_font_vertices_map,
                    );
                }
                RenderCommands::UiRect {
                    rect,
                    color,
                    z_index: _,
                } => {
                    let left = rect.x - rect.w / 2.0;
                    let top = rect.y - rect.h / 2.0;
                    let right = left + rect.w;
                    let bottom = top + rect.h;

                    self.ui_vertices.extend_from_slice(&[
                        UIVertex {
                            position: [left, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [left, bottom],
                            color: *color,
                        },
                        UIVertex {
                            position: [left, bottom],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, top],
                            color: *color,
                        },
                        UIVertex {
                            position: [right, bottom],
                            color: *color,
                        },
                    ]);
                }
                RenderCommands::UiImage {
                    texture,
                    rect,
                    tint,
                    uv_rect,
                    z_index: _,
                } => {
                    let key = Arc::as_ptr(texture) as usize;
                    self.textures_cache
                        .entry(key)
                        .or_insert_with(|| texture.clone());

                    // Normalize UVs if caller supplied pixel-based UVs (>1.0)
                    let mut uv_n = *uv_rect;
                    if uv_n[0] > 1.0 || uv_n[1] > 1.0 || uv_n[2] > 1.0 || uv_n[3] > 1.0 {
                        uv_n = [
                            uv_rect[0] / texture.width as f32,
                            uv_rect[1] / texture.height as f32,
                            uv_rect[2] / texture.width as f32,
                            uv_rect[3] / texture.height as f32,
                        ];
                    }

                    let left = rect.x - rect.w / 2.0;
                    let top = rect.y - rect.h / 2.0;
                    let right = left + rect.w;
                    let bottom = top + rect.h;

                    // Add textured vertices for this image using normalized UVs
                    // For regular textures (not font atlas) flip V coordinate because texture assets are top-left origin
                    let entry = ui_image_vertices_map.entry(key).or_default();
                    let u0 = uv_n[0];
                    let v0 = uv_n[1];
                    let uw = uv_n[2];
                    let vh = uv_n[3];
                    // Use UVs as provided for assets (no vertical flip)
                    let v_top = v0;
                    let v_bottom = v0 + vh;

                    entry.extend_from_slice(&[
                        UITexturedVertex {
                            position: [left, top],
                            tex_coords: [u0, v_top],
                            color: *tint,
                        },
                        UITexturedVertex {
                            position: [right, top],
                            tex_coords: [u0 + uw, v_top],
                            color: *tint,
                        },
                        UITexturedVertex {
                            position: [left, bottom],
                            tex_coords: [u0, v_bottom],
                            color: *tint,
                        },
                        UITexturedVertex {
                            position: [left, bottom],
                            tex_coords: [u0, v_bottom],
                            color: *tint,
                        },
                        UITexturedVertex {
                            position: [right, top],
                            tex_coords: [u0 + uw, v_top],
                            color: *tint,
                        },
                        UITexturedVertex {
                            position: [right, bottom],
                            tex_coords: [u0 + uw, v_bottom],
                            color: *tint,
                        },
                    ]);
                }
                RenderCommands::UiText {
                    text,
                    rect,
                    color,
                    font_size,
                    z_index: _,
                    font_id,
                    segments,
                } => {
                    let fid = font_id.unwrap_or(FontId::DEFAULT);
                    let scale = *font_size as f32 / self.font_manager.base_font_size_for(fid);
                    let (char_width, _) = self.font_manager.char_size_for(fid);
                    let char_h = self.font_manager.line_height_for(fid) * scale;
                    let y = if rect.h > char_h {
                        rect.y + (rect.h - char_h) * 0.5
                    } else {
                        rect.y
                    };

                    let emit_glyph = |x: &mut f32,
                                      ch: char,
                                      seg_color: &[f32; 4],
                                      bold: bool,
                                      ui_font_vertices_map: &mut HashMap<
                        usize,
                        Vec<UITexturedVertex>,
                    >| {
                        let char_w = self
                            .font_manager
                            .get_char_advance_for(fid, ch)
                            .unwrap_or(char_width as f32)
                            * scale;

                        if ch == ' ' {
                            *x += char_w;
                            return;
                        }

                        if let Some(glyph_info) = self.font_manager.get_glyph_info_for(fid, ch) {
                            let char_uv = glyph_info.uv;
                            let base_left = *x + char_uv.bearing_x * scale;
                            let top = y + char_uv.bearing_y * scale;
                            let right = base_left + char_uv.width * scale;
                            let bottom = top + char_uv.height * scale;

                            if let Some(atlas_tex) = self.font_manager.get_atlas_texture_for(fid) {
                                let atlas_key = Arc::as_ptr(atlas_tex) as usize;
                                let entry = ui_font_vertices_map.entry(atlas_key).or_default();

                                let offsets: &[f32] = if bold { &[0.0, 1.0] } else { &[0.0] };
                                for &dx in offsets {
                                    let left = base_left + dx;
                                    entry.extend_from_slice(&[
                                        UITexturedVertex {
                                            position: [left, top],
                                            tex_coords: [char_uv.u, char_uv.v],
                                            color: *seg_color,
                                        },
                                        UITexturedVertex {
                                            position: [right + dx, top],
                                            tex_coords: [char_uv.u + char_uv.u_width, char_uv.v],
                                            color: *seg_color,
                                        },
                                        UITexturedVertex {
                                            position: [left, bottom],
                                            tex_coords: [char_uv.u, char_uv.v + char_uv.v_height],
                                            color: *seg_color,
                                        },
                                        UITexturedVertex {
                                            position: [left, bottom],
                                            tex_coords: [char_uv.u, char_uv.v + char_uv.v_height],
                                            color: *seg_color,
                                        },
                                        UITexturedVertex {
                                            position: [right + dx, top],
                                            tex_coords: [char_uv.u + char_uv.u_width, char_uv.v],
                                            color: *seg_color,
                                        },
                                        UITexturedVertex {
                                            position: [right + dx, bottom],
                                            tex_coords: [
                                                char_uv.u + char_uv.u_width,
                                                char_uv.v + char_uv.v_height,
                                            ],
                                            color: *seg_color,
                                        },
                                    ]);
                                }
                            }
                        }

                        *x += char_w;
                    };

                    if segments.is_empty() {
                        let mut x = rect.x - rect.w * 0.5;
                        for ch in text.chars() {
                            emit_glyph(&mut x, ch, color, false, &mut ui_font_vertices_map);
                        }
                    } else {
                        // compute total width for centering
                        let mut total_w = 0.0_f32;
                        for seg in segments {
                            for ch in seg.text.chars() {
                                let cw = self
                                    .font_manager
                                    .get_char_advance_for(fid, ch)
                                    .unwrap_or(char_width as f32)
                                    * scale;
                                total_w += cw;
                            }
                        }
                        let mut x = rect.x - total_w * 0.5;
                        for seg in segments {
                            for ch in seg.text.chars() {
                                emit_glyph(
                                    &mut x,
                                    ch,
                                    &seg.color,
                                    seg.bold,
                                    &mut ui_font_vertices_map,
                                );
                            }
                        }
                    }
                }
            }
        }

        self.sprite_instances.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| {
                    left.1
                        .partial_cmp(&right.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.2.cmp(&right.2))
        });
        self.batches.clear();
        for (order, depth, sequence, texture_key, instance) in self.sprite_instances.drain(..) {
            let offset = self.all_instances.len();
            self.all_instances.push(instance);
            if let Some(last) = self.batches.last_mut() {
                if last.0 == order && last.3 == texture_key {
                    last.5 += 1;
                    continue;
                }
            }
            self.batches
                .push((order, depth, sequence, texture_key, offset, 1));
        }

        if self.all_instances.len() > self.instance_buffer_capacity {
            let new_capacity = (self.all_instances.len() * 3 / 2).max(1000);
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: (size_of::<InstanceData>() * new_capacity) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.instance_buffer_capacity = new_capacity;
        }

        if !self.all_instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.all_instances),
            );
        }

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        // Single sort for meshes by (order, depth, cmd_index)
        self.mesh_items.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.2.cmp(&b.2))
        });

        let orders = &mut self.orders;
        orders.clear();
        orders.extend(self.mesh_items.iter().map(|(order, _, _)| *order));
        orders.extend(self.batches.iter().map(|(order, _, _, _, _, _)| *order));
        orders.sort_unstable();
        orders.dedup();

        let mut mesh_uniform_offset: u64 = 0;

        for order in orders.iter() {
            // Draw meshes for this order — they are already sorted globally
            let order_start = self.mesh_items.partition_point(|(o, _, _)| o < order);
            let order_end = self.mesh_items.partition_point(|(o, _, _)| o <= order);
            for cmd_index in self.mesh_items[order_start..order_end]
                .iter()
                .map(|(_, _, i)| *i)
            {
                let RenderCommands::Mesh3D(params) = &queue.commands[cmd_index] else {
                    continue;
                };

                let directional_direction = directional
                    .map(|light| [light.direction.x, light.direction.y, light.direction.z, 0.0])
                    .unwrap_or([0.0, -1.0, 0.0, 0.0]);
                let directional_color_intensity = directional
                    .map(|light| [light.color.x, light.color.y, light.color.z, light.intensity])
                    .unwrap_or([0.0, 0.0, 0.0, 0.0]);
                let mesh_uniforms = MeshUniforms {
                    view_proj: camera_matrix.to_cols_array_2d(),
                    model: params.model_matrix.to_cols_array_2d(),
                    base_color: params.color,
                    emission: [
                        params.emission[0],
                        params.emission[1],
                        params.emission[2],
                        0.0,
                    ],
                    directional_direction,
                    directional_color_intensity,
                    ambient_color_intensity: [
                        queue.atmosphere.ambient_color.x,
                        queue.atmosphere.ambient_color.y,
                        queue.atmosphere.ambient_color.z,
                        queue.atmosphere.ambient_intensity,
                    ],
                    flags: [
                        has_lighting as u32,
                        params.use_vertex_color as u32,
                        point_light_count,
                        directional.is_some() as u32,
                    ],
                    point_lights,
                };

                // Grow uniform buffer if needed
                let needed_size = mesh_uniform_offset + self.uniform_stride;
                if needed_size > self.mesh_uniform_capacity {
                    let new_size = (needed_size * 3 / 2).max(65536);
                    self.mesh_uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Mesh Uniform Buffer"),
                        size: new_size,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.mesh_uniform_capacity = new_size;
                }

                self.queue.write_buffer(
                    &self.mesh_uniform_buffer,
                    mesh_uniform_offset,
                    bytemuck::bytes_of(&mesh_uniforms),
                );

                let mesh_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.mesh_pipeline.bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.mesh_uniform_buffer,
                            offset: mesh_uniform_offset,
                            size: NonZero::new(self.uniform_stride),
                        }),
                    }],
                    label: Some("Mesh Bind Group"),
                });
                mesh_uniform_offset += self.uniform_stride;

                // Cache vertex/index GPU buffers by mesh_id
                let (vertex_buffer, index_buffer) =
                    if let Some(cached) = self.mesh_gpu_cache.get(&params.mesh_id) {
                        (cached.0.clone(), cached.1.clone())
                    } else {
                        let vb = self.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Mesh Vertex Buffer"),
                            size: (params.vertices.len() * size_of::<runa_render_api::Vertex3D>())
                                as u64,
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                        self.queue
                            .write_buffer(&vb, 0, bytemuck::cast_slice(&params.vertices));

                        let ib = self.device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("Mesh Index Buffer"),
                            size: (params.indices.len() * 4) as u64,
                            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                        self.queue
                            .write_buffer(&ib, 0, bytemuck::cast_slice(&params.indices));

                        self.mesh_gpu_cache.insert(params.mesh_id, (vb, ib));
                        self.mesh_gpu_cache.get(&params.mesh_id).cloned().unwrap()
                    };

                rpass.set_pipeline(&self.mesh_pipeline.pipeline);
                rpass.set_bind_group(0, &mesh_bind_group, &[]);
                rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rpass.draw_indexed(0..params.indices.len() as u32, 0, 0..1);
            }

            for (_, _, _, texture_key, instance_offset, instance_count) in self
                .batches
                .iter()
                .filter(|(sprite_order, _, _, _, _, _)| *sprite_order == *order)
            {
                if !self.textures.contains_key(texture_key) {
                    let texture = self.textures_cache.get(texture_key).unwrap();
                    let gpu_tex =
                        Arc::new(GpuTexture::from_asset(&self.device, &self.queue, texture));
                    self.textures.insert(*texture_key, gpu_tex);
                }
                let gpu_texture = self.textures.get(texture_key).unwrap().clone();

                let bind_group = self
                    .bind_group_cache
                    .entry(*texture_key)
                    .or_insert_with(|| {
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: &self.sprite_pipeline.bind_group_layout,
                            entries: &[
                                wgpu::BindGroupEntry {
                                    binding: 0,
                                    resource: self.globals_buffer.as_entire_binding(),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 1,
                                    resource: wgpu::BindingResource::TextureView(&gpu_texture.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 2,
                                    resource: wgpu::BindingResource::Sampler(&self.nearest_sampler),
                                },
                            ],
                            label: Some("BindGroup"),
                        })
                    });

                rpass.set_pipeline(&self.sprite_pipeline.pipeline);
                rpass.set_bind_group(0, &*bind_group, &[]);
                rpass.set_vertex_buffer(0, self.quad_buffer.slice(..));
                rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                rpass.draw(
                    0..6,
                    *instance_offset as u32..(*instance_offset + *instance_count) as u32,
                );
            }
        }

        if !self.ui_vertices.is_empty() {
            let ui_vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UI Vertex Buffer"),
                size: (size_of::<UIVertex>() * self.ui_vertices.len()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(
                &ui_vertex_buffer,
                0,
                bytemuck::cast_slice(&self.ui_vertices),
            );

            rpass.set_pipeline(&self.ui_pipeline.pipeline);
            rpass.set_bind_group(0, self.ui_bind_group.as_ref().unwrap(), &[]);
            rpass.set_vertex_buffer(0, ui_vertex_buffer.slice(..));
            rpass.draw(0..self.ui_vertices.len() as u32, 0..1);
        }

        // Render font text with nearest sampler (crisp pixel font at any scale)
        if !ui_font_vertices_map.is_empty() {
            let sampler = self.nearest_sampler.clone();
            self.render_textured_ui_batch(&mut rpass, &mut ui_font_vertices_map, &sampler);
        }

        // Render UI images with nearest sampler
        if !ui_image_vertices_map.is_empty() {
            let sampler = self.nearest_sampler.clone();
            self.render_textured_ui_batch(&mut rpass, &mut ui_image_vertices_map, &sampler);
        }
    }

    fn render_textured_ui_batch(
        &mut self,
        rpass: &mut wgpu::RenderPass,
        vertices_map: &mut std::collections::HashMap<usize, Vec<UITexturedVertex>>,
        sampler: &wgpu::Sampler,
    ) {
        for (tex_key, vertices) in vertices_map.drain() {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("UI Textured Vertex Buffer"),
                size: (size_of::<UITexturedVertex>() * vertices.len()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue
                .write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));

            let gpu_texture: Arc<GpuTexture> =
                if let Some(atlas_tex) = self.font_manager.get_atlas_texture() {
                    if Arc::as_ptr(atlas_tex) as usize == tex_key {
                        atlas_tex.clone()
                    } else {
                        if !self.textures.contains_key(&tex_key) {
                            let texture_asset = self.textures_cache.get(&tex_key).unwrap().clone();
                            let gpu_tex = Arc::new(GpuTexture::from_asset(
                                &self.device,
                                &self.queue,
                                &texture_asset,
                            ));
                            self.textures.insert(tex_key, gpu_tex);
                        }
                        self.textures.get(&tex_key).unwrap().clone()
                    }
                } else {
                    if !self.textures.contains_key(&tex_key) {
                        let texture_asset = self.textures_cache.get(&tex_key).unwrap().clone();
                        let gpu_tex = Arc::new(GpuTexture::from_asset(
                            &self.device,
                            &self.queue,
                            &texture_asset,
                        ));
                        self.textures.insert(tex_key, gpu_tex);
                    }
                    self.textures.get(&tex_key).unwrap().clone()
                };

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.ui_pipeline.textured_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.ui_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&gpu_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
                label: Some("UI Textured Bind Group"),
            });

            rpass.set_pipeline(&self.ui_pipeline.textured_pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.set_vertex_buffer(0, buffer.slice(..));
            rpass.draw(0..vertices.len() as u32, 0..1);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_text_vertices(
        &self,
        text: &str,
        origin_x: f32,
        origin_y: f32,
        color: &[f32; 4],
        char_w: f32,
        char_h: f32,
        scale: f32,
        char_advance: f32,
        ui_font_vertices_map: &mut HashMap<usize, Vec<UITexturedVertex>>,
    ) {
        let mut x = origin_x;
        let y = origin_y;
        for ch in text.chars() {
            if ch == ' ' {
                x += char_w;
                continue;
            }
            if let Some(char_uv) = self.font_manager.get_char_uv(ch) {
                let left = x;
                let top = y;
                let right = x + char_w;
                let bottom = y + char_h;
                if let Some(atlas_tex) = self.font_manager.get_atlas_texture() {
                    let atlas_key = Arc::as_ptr(atlas_tex) as usize;
                    let entry = ui_font_vertices_map.entry(atlas_key).or_default();
                    entry.extend_from_slice(&[
                        UITexturedVertex {
                            position: [left, top],
                            tex_coords: [char_uv.u, char_uv.v],
                            color: *color,
                        },
                        UITexturedVertex {
                            position: [right, top],
                            tex_coords: [char_uv.u + char_uv.u_width, char_uv.v],
                            color: *color,
                        },
                        UITexturedVertex {
                            position: [left, bottom],
                            tex_coords: [char_uv.u, char_uv.v + char_uv.v_height],
                            color: *color,
                        },
                        UITexturedVertex {
                            position: [left, bottom],
                            tex_coords: [char_uv.u, char_uv.v + char_uv.v_height],
                            color: *color,
                        },
                        UITexturedVertex {
                            position: [right, top],
                            tex_coords: [char_uv.u + char_uv.u_width, char_uv.v],
                            color: *color,
                        },
                        UITexturedVertex {
                            position: [right, bottom],
                            tex_coords: [char_uv.u + char_uv.u_width, char_uv.v + char_uv.v_height],
                            color: *color,
                        },
                    ]);
                }
            }
            x += self
                .font_manager
                .get_char_advance(ch)
                .unwrap_or(char_advance)
                * scale;
        }
    }

    fn encode_background_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &TextureView,
        atmosphere: &runa_render_api::AtmosphereData,
        camera_matrix: glam::Mat4,
    ) {
        let uniforms = background_uniforms(atmosphere, camera_matrix);
        self.queue.write_buffer(
            &self.background_uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background Bind Group"),
            layout: &self.background_pipeline.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.background_uniform_buffer.as_entire_binding(),
            }],
        });

        let mut r_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Background Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        r_pass.set_pipeline(&self.background_pipeline.pipeline);
        r_pass.set_bind_group(0, &bind_group, &[]);
        r_pass.draw(0..3, 0..1);
    }

    fn build_render_target(
        device: &wgpu::Device,
        size: (u32, u32),
        render_format: wgpu::TextureFormat,
    ) -> RenderTarget {
        let (width, height) = (size.0.max(1), size.1.max(1));
        let sample_format = render_format.remove_srgb_suffix();
        let mut view_formats = Vec::new();
        if sample_format != render_format {
            view_formats.push(render_format);
        }
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Color Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: sample_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &view_formats,
        });
        let render_color_view = color_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(render_format),
            ..Default::default()
        });
        let sample_color_view = color_texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(sample_format),
            ..Default::default()
        });
        let (depth_texture, depth_view) = Self::create_depth_texture(device, (width, height));

        RenderTarget {
            _color_texture: color_texture,
            render_color_view,
            sample_color_view,
            _depth_texture: depth_texture,
            depth_view,
            size: (width, height),
            _render_format: render_format,
            _sample_format: sample_format,
        }
    }

    fn create_intermediate_texture(
        device: &wgpu::Device,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Option<Texture> {
        if size.0 == 0 || size.1 == 0 {
            return None;
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Intermediate Render Texture"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Some(texture)
    }

    fn create_depth_texture(device: &wgpu::Device, size: (u32, u32)) -> (Texture, TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("Depth Texture"),
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    fn encode_post_process_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &TextureView,
        _target_size: (u32, u32),
        effects: &runa_render_api::ScreenEffectData,
    ) {
        let flags = effects.enabled.to_u32();
        if flags == 0 {
            return;
        }

        let uniforms = PostProcessUniforms {
            fade_color: effects.fade_color,
            vignette_strength: effects.vignette_strength,
            vignette_radius: if effects.vignette_radius <= 0.0 {
                0.5
            } else {
                effects.vignette_radius
            },
            vignette_softness: effects.vignette_softness.max(0.001),
            rgb_shift: effects.rgb_shift,
            _pad1: [0.0; 2],
            tint_color: effects.tint_color,
            brightness: effects.brightness,
            contrast: effects.contrast,
            flags,
            _pad2: [0u32; 3],
        };

        let Some(intermediate_view) = self.intermediate_view.as_ref() else {
            return;
        };

        self.queue.write_buffer(
            &self.postprocess_uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("PostProcess Bind Group"),
            layout: &self.post_process_pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(intermediate_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.postprocess_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("PostProcess Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        rpass.set_pipeline(&self.post_process_pipeline.pipeline);
        rpass.set_bind_group(0, &bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }
}

fn background_uniforms(
    atmosphere: &runa_render_api::AtmosphereData,
    camera_matrix: glam::Mat4,
) -> BackgroundUniforms {
    let mut uniforms = BackgroundUniforms {
        inverse_view_proj: camera_matrix.inverse().to_cols_array_2d(),
        mode: [1, 0, 0, 0],
        background_params: [atmosphere.background_intensity, 0.0, 0.0, 0.0],
        solid_color: [0.0, 0.0, 0.0, 1.0],
        zenith_color: [0.2, 0.4, 0.8, 0.5],
        horizon_color: [0.8, 0.9, 1.0, 0.25],
        ground_color: [0.6, 0.6, 0.7, 0.0],
    };

    match atmosphere.background {
        BackgroundModeData::SolidColor { color } => {
            uniforms.mode = [0, 0, 0, 0];
            uniforms.solid_color = [color.x, color.y, color.z, 1.0];
        }
        BackgroundModeData::VerticalGradient {
            zenith_color,
            horizon_color,
            ground_color,
            horizon_height,
            smoothness,
        } => {
            uniforms.mode = [1, 0, 0, 0];
            uniforms.zenith_color = [
                zenith_color.x,
                zenith_color.y,
                zenith_color.z,
                horizon_height.clamp(0.0, 1.0),
            ];
            uniforms.horizon_color = [
                horizon_color.x,
                horizon_color.y,
                horizon_color.z,
                smoothness.max(0.001),
            ];
            uniforms.ground_color = [ground_color.x, ground_color.y, ground_color.z, 0.0];
        }
        BackgroundModeData::Sky => {
            // TODO: Route this to a skybox/skysphere/HDRI pass. For now, keep gradient fallback.
            uniforms.mode = [2, 0, 0, 0];
        }
    }

    uniforms
}

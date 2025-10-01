//! Metal-specific WGPU integration for IOSurface textures.
//!
//! This module provides functionality to create WGPU textures from Metal IOSurfaces,
//! which is essential for efficient GPU memory sharing on macOS. It includes texture
//! flipping operations to handle coordinate system differences between Metal and other APIs.

use std::fmt;
use std::sync::OnceLock;

use objc2::runtime::NSObject;
use objc2::{msg_send, rc::Retained};
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{MTLPixelFormat, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage};

use foreign_types_shared::ForeignType;
use wgpu::Error as WgpuError;
use winit::dpi::PhysicalSize;

/// Errors that can occur during Metal texture operations.
#[derive(Debug)]
pub enum MetalError {
    /// Failed to create Metal texture from IOSurface
    TextureCreationFailed(String),
    /// Failed to get Metal device from WGPU device
    DeviceExtractionFailed(String),
    /// Generic WGPU error
    WgpuError(WgpuError),
}

impl fmt::Display for MetalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetalError::TextureCreationFailed(msg) => write!(f, "Texture creation failed: {}", msg),
            MetalError::DeviceExtractionFailed(msg) => {
                write!(f, "Device extraction failed: {}", msg)
            }
            MetalError::WgpuError(err) => write!(f, "WGPU error: {:?}", err),
        }
    }
}

impl std::error::Error for MetalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MetalError::WgpuError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<WgpuError> for MetalError {
    fn from(err: WgpuError) -> Self {
        MetalError::WgpuError(err)
    }
}

/// Cached render resources to avoid recreating expensive objects.
struct RenderResourceCache {
    vertex_shader: OnceLock<wgpu::ShaderModule>,
    fragment_shader: OnceLock<wgpu::ShaderModule>,
    bind_group_layout: OnceLock<wgpu::BindGroupLayout>,
    render_pipeline: OnceLock<wgpu::RenderPipeline>,
    sampler: OnceLock<wgpu::Sampler>,
}

impl RenderResourceCache {
    const fn new() -> Self {
        Self {
            vertex_shader: OnceLock::new(),
            fragment_shader: OnceLock::new(),
            bind_group_layout: OnceLock::new(),
            render_pipeline: OnceLock::new(),
            sampler: OnceLock::new(),
        }
    }
}

/// Global cache for render resources to avoid recreation.
static RENDER_CACHE: RenderResourceCache = RenderResourceCache::new();

/// WGPU texture wrapper for Metal IOSurface textures.
///
/// This struct provides functionality to create WGPU textures from Metal IOSurfaces
/// and perform coordinate system transformations.
pub struct WPGPUTextureFromMetal {
    pub size: PhysicalSize<u32>,
}

impl WPGPUTextureFromMetal {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        Self { size }
    }

    pub fn get(
        &self,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
        surfman_device: &surfman::Device,
        surfman_surface: &surfman::Surface,
    ) -> Result<wgpu::Texture, MetalError> {
        let objc2_metal_texture =
            self.objc2_metal_texture(wgpu_device, surfman_device, surfman_surface)?;

        let texture = self.wgpu_hal_texture(wgpu_device, objc2_metal_texture)?;

        self.create_flipped_texture_render(wgpu_device, wgpu_queue, &texture)
    }

    /// Creates a WGPU texture descriptor with standard settings for this use case.
    fn create_wgpu_texture_descriptor(
        size: PhysicalSize<u32>,
        label: &str,
        usage: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
    ) -> wgpu::TextureDescriptor<'_> {
        wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        }
    }

    /// Creates a Metal texture from an IOSurface using Objective-C messaging.
    ///
    /// This function uses unsafe Objective-C messaging. The caller must ensure:
    /// - The device pointer is valid and points to a Metal device
    /// - The descriptor contains valid configuration
    /// - The IOSurface is valid and compatible with the descriptor
    fn create_texture_from_iosurface(
        &self,
        device: &objc2::runtime::NSObject,
        descriptor: &MTLTextureDescriptor,
        iosurface: &IOSurfaceRef,
        plane: objc2_foundation::NSUInteger,
    ) -> Option<Retained<NSObject>> {
        unsafe {
            msg_send![device, newTextureWithDescriptor:descriptor, iosurface:iosurface, plane:plane]
        }
    }

    /// Creates a Metal texture object from an IOSurface using the WGPU Metal backend.
    ///
    /// This method extracts the Metal device from the WGPU device and uses it to create
    /// a Metal texture directly from the IOSurface contained in the surfman surface.
    ///
    /// This function contains unsafe code for:
    /// - Extracting the raw Metal device from WGPU
    /// - Converting device pointers for Objective-C messaging
    fn objc2_metal_texture(
        &self,
        wgpu_device: &wgpu::Device,
        surfman_device: &surfman::Device,
        surfman_surface: &surfman::Surface,
    ) -> Result<Retained<NSObject>, MetalError> {
        // SAFETY: We're working with WGPU Metal backend, so the device extraction
        // and pointer manipulations are safe within this controlled context.
        unsafe {
            let metal_device = wgpu_device
                .as_hal::<wgpu::wgc::api::Metal>()
                .ok_or_else(|| {
                    MetalError::DeviceExtractionFailed(
                        "WGPU device is not using Metal backend".to_string(),
                    )
                })?;

            let device_raw = metal_device.raw_device().lock().clone();

            let descriptor = MTLTextureDescriptor::new();
            descriptor.setDepth(1);
            descriptor.setMipmapLevelCount(1);
            descriptor.setSampleCount(1);
            descriptor.setUsage(MTLTextureUsage::ShaderRead);
            descriptor.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
            descriptor.setTextureType(MTLTextureType::Type2D);
            descriptor.setWidth(self.size.width as usize);
            descriptor.setHeight(self.size.height as usize);

            // let texture_descriptor = Self::create_metal_texture_descriptor(self.size);

            let native_surface = surfman_device.native_surface(surfman_surface);
            let io_surface = native_surface.0;

            // SAFETY: The device_raw pointer is valid (obtained from WGPU Metal backend)
            // and we're casting it appropriately for Objective-C messaging.
            let texture = self
                .create_texture_from_iosurface(
                    &*(device_raw.as_ptr() as *mut objc2::runtime::NSObject),
                    &descriptor,
                    &io_surface,
                    0,
                )
                .ok_or_else(|| {
                    MetalError::TextureCreationFailed(
                        "Failed to create Metal texture from IOSurface".to_string(),
                    )
                })?;

            Ok(texture)
        }
    }

    /// Converts a Metal texture object into a WGPU texture.
    ///
    /// This method takes a Metal texture (as an NSObject) and wraps it in WGPU's
    /// texture abstraction, allowing it to be used with WGPU rendering operations.
    ///
    /// This function contains unsafe code for:
    /// - Converting Objective-C objects to Metal API objects
    /// - Creating HAL textures from raw Metal textures
    /// - Managing memory ownership transfer between different APIs
    fn wgpu_hal_texture(
        &self,
        wgpu_device: &wgpu::Device,
        metal_texture: Retained<NSObject>,
    ) -> Result<wgpu::Texture, MetalError> {
        // SAFETY: We're converting between compatible object types within the same
        // Metal/WGPU ecosystem. The ownership transfer is handled correctly.
        unsafe {
            let ptr: *mut objc2_foundation::NSObject = Retained::into_raw(metal_texture);

            // SAFETY: The ptr comes from a valid Metal texture object
            let metal_texture = metal::Texture::from_ptr(ptr as *mut _);

            let hal_texture = wgpu::hal::metal::Device::texture_from_raw(
                metal_texture,
                wgpu::TextureFormat::Bgra8Unorm,
                metal::MTLTextureType::D2,
                0,
                0,
                wgpu::hal::CopyExtent {
                    width: self.size.width,
                    height: self.size.height,
                    depth: 0,
                },
            );

            let wgpu_descriptor = Self::create_wgpu_texture_descriptor(
                self.size,
                "Metal IOSurface Texture",
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                wgpu::TextureFormat::Bgra8Unorm,
            );

            Ok(wgpu_device
                .create_texture_from_hal::<wgpu::wgc::api::Metal>(hal_texture, &wgpu_descriptor))
        }
    }

    /// Creates and applies a texture flipping render operation.
    ///
    /// This method creates a new texture with the same dimensions as the input,
    /// then uses a render pass to copy and vertically flip the source texture.
    pub fn create_flipped_texture_render(
        &self,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
        source_texture: &wgpu::Texture,
    ) -> Result<wgpu::Texture, MetalError> {
        // Create the output texture
        let flipped_texture = self.create_output_texture(wgpu_device)?;

        // Get or create cached render resources
        let render_pipeline = self.get_or_create_render_pipeline(wgpu_device);
        let sampler = self.get_or_create_sampler(wgpu_device);
        let bind_group_layout = self.get_or_create_bind_group_layout(wgpu_device);

        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Metal Texture Flip Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        // Execute the render pass
        let target_view = &flipped_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Metal Texture Flip Command Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Metal Texture Flip Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1); // Draw a fullscreen triangle
        }

        wgpu_queue.submit(std::iter::once(encoder.finish()));

        Ok(flipped_texture)
    }

    /// Creates the output texture for the flipping operation.
    fn create_output_texture(
        &self,
        wgpu_device: &wgpu::Device,
    ) -> Result<wgpu::Texture, MetalError> {
        let descriptor = Self::create_wgpu_texture_descriptor(
            self.size,
            "Flipped Metal IOSurface Texture",
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        Ok(wgpu_device.create_texture(&descriptor))
    }

    /// Gets or creates the vertex shader module with caching.
    fn get_or_create_vertex_shader(&self, wgpu_device: &wgpu::Device) -> &wgpu::ShaderModule {
        RENDER_CACHE
            .vertex_shader
            .get_or_init(|| {
                wgpu_device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Metal Texture Flip Vertex Shader"),
                        source: wgpu::ShaderSource::Wgsl(r#"
                            @vertex
                            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                                let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
                                return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
                                
                            }
                        "#.into()),
                    })
            })
    }

    /// Gets or creates the fragment shader module with caching.
    fn get_or_create_fragment_shader(&self, wgpu_device: &wgpu::Device) -> &wgpu::ShaderModule {
        RENDER_CACHE
            .fragment_shader
            .get_or_init(|| {
                wgpu_device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("Metal Texture Flip Fragment Shader"),
                        source: wgpu::ShaderSource::Wgsl(r#"
                            @group(0) @binding(0) var source_texture: texture_2d<f32>;
                            @group(0) @binding(1) var source_sampler: sampler;

                            @fragment
                            fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
                                let size = textureDimensions(source_texture);
                                let uv = position.xy / vec2<f32>(f32(size.x), f32(size.y));
                                // Flip vertically by inverting the V coordinate
                                let flipped_uv = vec2<f32>(uv.x, 1.0 - uv.y);
                                let color = textureSample(source_texture, source_sampler, flipped_uv);
                                return color;
                            }
                        "#.into()),
                    })
            })
    }

    /// Gets or creates the bind group layout with caching.
    fn get_or_create_bind_group_layout(
        &self,
        wgpu_device: &wgpu::Device,
    ) -> &wgpu::BindGroupLayout {
        RENDER_CACHE.bind_group_layout.get_or_init(|| {
            wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Metal Texture Flip Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            })
        })
    }

    /// Gets or creates the sampler with caching.
    fn get_or_create_sampler(&self, wgpu_device: &wgpu::Device) -> &wgpu::Sampler {
        let descriptior = wgpu::SamplerDescriptor {
            label: Some("Metal Texture Sampler"),
            compare: None,
            border_color: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            anisotropy_clamp: 1,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
        };

        RENDER_CACHE
            .sampler
            .get_or_init(|| wgpu_device.create_sampler(&descriptior))
    }

    /// Gets or creates the render pipeline with caching.
    fn get_or_create_render_pipeline(&self, wgpu_device: &wgpu::Device) -> &wgpu::RenderPipeline {
        RENDER_CACHE.render_pipeline.get_or_init(|| {
            let vertex_shader = self.get_or_create_vertex_shader(wgpu_device);
            let fragment_shader = self.get_or_create_fragment_shader(wgpu_device);
            let bind_group_layout = self.get_or_create_bind_group_layout(wgpu_device);

            let pipeline_layout =
                wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Metal Texture Flip Pipeline Layout"),
                    bind_group_layouts: &[bind_group_layout],
                    push_constant_ranges: &[],
                });

            wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Metal Texture Flip Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vertex_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: fragment_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        })
    }
}

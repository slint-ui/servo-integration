use objc2::runtime::NSObject;
use objc2::{msg_send, rc::Retained};
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{MTLPixelFormat, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage};

use foreign_types_shared::ForeignType;
use winit::dpi::PhysicalSize;

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
    ) -> wgpu::Texture {
        let objc2_metla_texture =
            self.objc2_metla_texture(wgpu_device, surfman_device, surfman_surface);

        let texture = self.wgpu_hal_texture(wgpu_device, objc2_metla_texture);

        self.create_flipped_texture_render(wgpu_device, wgpu_queue, &texture)
    }

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

    fn objc2_metla_texture(
        &self,
        wgpu_device: &wgpu::Device,
        surfman_device: &surfman::Device,
        surfman_surface: &surfman::Surface,
    ) -> Retained<NSObject> {
        unsafe {
            let metal_device = wgpu_device.as_hal::<wgpu::wgc::api::Metal>().unwrap();
            let device_raw = metal_device.raw_device().lock().clone();

            let texture_descriptor = MTLTextureDescriptor::new();
            texture_descriptor.setDepth(1);
            texture_descriptor.setMipmapLevelCount(1);
            texture_descriptor.setSampleCount(1);
            texture_descriptor.setUsage(MTLTextureUsage::ShaderRead);
            texture_descriptor.setPixelFormat(MTLPixelFormat::RGBA8Unorm);
            texture_descriptor.setTextureType(MTLTextureType::Type2D);
            texture_descriptor.setWidth(self.size.width as usize);
            texture_descriptor.setHeight(self.size.height as usize);

            let native_surface = surfman_device.native_surface(&surfman_surface);
            let io_surface = native_surface.0;

            return self
                .create_texture_from_iosurface(
                    &*(device_raw.as_ptr() as *mut objc2::runtime::NSObject),
                    &texture_descriptor,
                    &io_surface,
                    0,
                )
                .unwrap();
        }
    }

    pub fn create_flipped_texture_render(
        &self,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
        source_texture: &wgpu::Texture,
    ) -> wgpu::Texture {
        let flipped_texture = wgpu_device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Flipped Metal IOSurface Texture"),
            size: wgpu::Extent3d {
                width: self.size.width,
                height: self.size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // Create shader modules
        let vertex_shader_module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(r#"
                @vertex
                fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                    var positions = array<vec2<f32>, 6>(
                        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0),
                        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0,  1.0), vec2<f32>(-1.0,  1.0)
                    );
                    return vec4<f32>(positions[vertex_index], 0.0, 1.0);
                }
            "#.into()),
        });

        let fragment_shader_module = wgpu_device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
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
                    
                    // Swap R and B channels since we changed from BGRA to RGBA format
                    return vec4<f32>(color.b, color.g, color.r, color.a);  // Swap R and B channels
                }
            "#.into()),
        });

        // Create texture views
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = flipped_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler
        let sampler = wgpu_device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        // Create bind group layout
        let bind_group_layout = wgpu_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
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
        });

        // Create bind group
        let bind_group = wgpu_device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = wgpu_device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = wgpu_device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Flip Texture Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader_module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader_module,
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
        });

        // Create command encoder and execute the render pass
        let mut encoder = wgpu_device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flip Texture Command Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Flip Texture Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // Submit the command buffer
        wgpu_queue.submit(std::iter::once(encoder.finish()));

        flipped_texture
    }

    fn wgpu_hal_texture(
        &self,
        wgpu_device: &wgpu::Device,
        metal_texture: Retained<NSObject>,
    ) -> wgpu::Texture {
        unsafe {
            let ptr: *mut objc2_foundation::NSObject = Retained::into_raw(metal_texture);

            let metal_texture = metal::Texture::from_ptr(ptr as *mut _);

            let hal_texture = wgpu::hal::metal::Device::texture_from_raw(
                metal_texture,
                wgpu::wgt::TextureFormat::Rgba8Unorm,
                metal::MTLTextureType::D2,
                0,
                0,
                wgpu::hal::CopyExtent {
                    width: self.size.width,
                    height: self.size.height,
                    depth: 0,
                },
            );

            let wgpu_descriptor = wgpu::TextureDescriptor {
                label: Some("Metal IOSurface Texture"),
                size: wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            return wgpu_device
                .create_texture_from_hal::<wgpu::wgc::api::Metal>(hal_texture, &wgpu_descriptor);
        };
    }
}

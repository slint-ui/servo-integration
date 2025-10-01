use std::{cell::Cell, rc::Rc, sync::Arc};

use euclid::default::Size2D;

use crate::gl_bindings as gl;
use image::RgbaImage;
use servo::RenderingContext;
use slint::wgpu_26::wgpu;
use webrender_api::units::DeviceIntRect;
use winit::dpi::PhysicalSize;

use surfman::{
    Connection, Device, Error, Surface, SurfaceTexture, SurfaceType,
    chains::{PreserveBuffer, SwapChain},
};

use crate::rendering_context::surfman_context::SurfmanRenderingContext;

pub struct CustomRenderingContext {
    pub size: Cell<PhysicalSize<u32>>,
    pub surfman_rendering_info: SurfmanRenderingContext,
    pub swap_chain: SwapChain<Device>,
}

impl Drop for CustomRenderingContext {
    fn drop(&mut self) {
        let device = &mut self.surfman_rendering_info.device.borrow_mut();
        let context = &mut self.surfman_rendering_info.context.borrow_mut();
        let _ = self.swap_chain.destroy(device, context);
    }
}

impl CustomRenderingContext {
    pub fn new(size: PhysicalSize<u32>) -> Result<Self, Error> {
        let connection = Connection::new()?;

        let adapter = connection.create_adapter()?;

        let surfman_rendering_info = SurfmanRenderingContext::new(&connection, &adapter)?;

        let surfman_size = Size2D::new(size.width as i32, size.height as i32);

        let surface =
            surfman_rendering_info.create_surface(SurfaceType::Generic { size: surfman_size })?;

        surfman_rendering_info.bind_surface(surface)?;

        surfman_rendering_info.make_current()?;

        let swap_chain = surfman_rendering_info.create_attached_swap_chain()?;

        Ok(Self {
            size: Cell::new(size),
            surfman_rendering_info,
            swap_chain,
        })
    }

    pub fn get_wgpu_texture_from_vulkan(
        &self,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
    ) -> Result<wgpu::Texture, Error> {
        let device = &self.surfman_rendering_info.device.borrow();
        let mut context = self.surfman_rendering_info.context.borrow_mut();

        let surface = device.unbind_surface_from_context(&mut context)?.unwrap();

        let tex_size = self.size.get();
        let height = tex_size.height as i32;

        let width = tex_size.width as i32;

        let texture = unsafe {
            use glow::HasContext;

            let vulkan_device = wgpu_device.as_hal::<wgpu::wgc::api::Vulkan>().unwrap();

            let (vulkan_texture, memory_handle, size) = vulkan_device
                .texture_funky(&wgpu_hal::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: tex_size.width,
                        height: tex_size.height,
                        depth_or_array_layers: 1,
                    },
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    dimension: wgpu::TextureDimension::D2,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUses::RESOURCE | wgpu::TextureUses::COLOR_TARGET,
                    view_formats: vec![],
                    memory_flags: wgpu_hal::MemoryFlags::empty(),
                })
                .unwrap();

            let gl = &self.surfman_rendering_info.glow_gl;

            let gl_with_extensions =
                gl::Gl::load_with(|function_name| device.get_proc_address(&context, function_name));

            let mut memory_object = 0;
            gl_with_extensions.CreateMemoryObjectsEXT(1, &mut memory_object);
            // We're using a dedicated allocation.
            gl_with_extensions.MemoryObjectParameterivEXT(memory_object, gl::DEDICATED_MEMORY_OBJECT_EXT, &1);
            gl_with_extensions.ImportMemoryFdEXT(
                memory_object,
                size,
                gl::HANDLE_TYPE_OPAQUE_FD_EXT,
                memory_handle,
            );
            // Create a texture and bind it to the imported memory.
            let mut texture = gl.create_texture().unwrap();
            gl.bind_texture(gl::TEXTURE_2D, Some(texture));
            gl_with_extensions.TexStorageMem2DEXT(
                gl::TEXTURE_2D,
                1,
                gl::RGBA8,
                width,
                height,
                memory_object,
                0,
            );


            let mut framebuffer = gl.create_framebuffer().unwrap();
            // Bind a destination framebuffer targeting our texture.
            gl.bind_framebuffer(gl::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.clear_color(0.7, 0.6, 0.5, 1.0);
            gl.clear(gl::COLOR_BUFFER_BIT);
            gl.flush();


            wgpu_device.create_texture_from_hal::<wgpu::wgc::api::Vulkan>(
                vulkan_texture,
                &wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: tex_size.width,
                        height: tex_size.height,
                        depth_or_array_layers: 1,
                    },
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    dimension: wgpu::TextureDimension::D2,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
            )
        };

        let _ = device
            .bind_surface_to_context(&mut context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(&mut context, &mut surface);
                err
            });

        Ok(texture)
    }

    /*
    pub fn get_wgpu_texture_from_metal(
        &self,
        wgpu_device: &wgpu::Device,
        wgpu_queue: &wgpu::Queue,
    ) -> Result<wgpu::Texture, Error> {
        let device = &self.surfman_rendering_info.device.borrow();
        let mut context = self.surfman_rendering_info.context.borrow_mut();

        let surface = device.unbind_surface_from_context(&mut context)?.unwrap();

        let size = self.size.get();

        let wgpu_texture = crate::rendering_context::metal::WPGPUTextureFromMetal::new(size)
            .get(wgpu_device, wgpu_queue, device, &surface)
            .expect("Failed to get WGPU texture from Metal texture");

        let _ = device
            .bind_surface_to_context(&mut context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(&mut context, &mut surface);
                err
            });

        Ok(wgpu_texture)
    }
    */
}

impl RenderingContext for CustomRenderingContext {
    fn prepare_for_rendering(&self) {
        self.surfman_rendering_info.prepare_for_rendering();
    }

    fn read_to_image(&self, source_rectangle: DeviceIntRect) -> Option<RgbaImage> {
        self.surfman_rendering_info.read_to_image(source_rectangle)
    }

    fn size(&self) -> PhysicalSize<u32> {
        self.size.get()
    }

    fn resize(&self, size: PhysicalSize<u32>) {
        if self.size.get() == size {
            return;
        }

        self.size.set(size);

        let mut device = self.surfman_rendering_info.device.borrow_mut();
        let mut context = self.surfman_rendering_info.context.borrow_mut();
        let size = Size2D::new(size.width as i32, size.height as i32);
        let _ = self.swap_chain.resize(&mut *device, &mut *context, size);
    }

    fn present(&self) {
        let mut device = self.surfman_rendering_info.device.borrow_mut();
        let mut context = self.surfman_rendering_info.context.borrow_mut();
        let _ = self
            .swap_chain
            .swap_buffers(&mut *device, &mut *context, PreserveBuffer::No);
    }

    fn make_current(&self) -> std::result::Result<(), surfman::Error> {
        self.surfman_rendering_info.make_current()
    }

    fn gleam_gl_api(&self) -> Rc<dyn gleam::gl::Gl> {
        self.surfman_rendering_info.gleam_gl.clone()
    }

    fn glow_gl_api(&self) -> Arc<glow::Context> {
        self.surfman_rendering_info.glow_gl.clone()
    }

    fn create_texture(&self, surface: Surface) -> Option<(SurfaceTexture, u32, Size2D<i32>)> {
        self.surfman_rendering_info.create_texture(surface)
    }

    fn destroy_texture(&self, surface_texture: SurfaceTexture) -> Option<Surface> {
        self.surfman_rendering_info.destroy_texture(surface_texture)
    }

    fn connection(&self) -> Option<Connection> {
        self.surfman_rendering_info.connection()
    }
}

use std::{cell::Cell, rc::Rc, sync::Arc};

use euclid::default::Size2D;

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

mod dma_buf;

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
    ) -> wgpu::Texture {
        let device = &self.surfman_rendering_info.device.borrow();
        let mut context = self.surfman_rendering_info.context.borrow_mut();

        let surface = device
            .unbind_surface_from_context(&mut context)
            .unwrap()
            .unwrap();

        let info = device.surface_info(&surface);

        //dbg!(
        //    device.surface_gl_texture_target(),
        //    info.size,
        //    info.id,
        //    device.surface_texture_object(&surface)
        //);
        //
        //dbg!(&device.native_device());

        let size = self.size.get();

        /*
        let _ = device
            .bind_surface_to_context(&mut context, surface)
            .map_err(|(err, mut surface)| {
                let _ = device.destroy_surface(&mut context, &mut surface);
                err
            });
            */

        let texture_usage =
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;

        let mip_level_count = 1;
        let sample_count = 1;
        let dimension = wgpu::wgt::TextureDimension::D2;
        let format = wgpu::wgt::TextureFormat::Rgba8Unorm;
        let label = None;
        let size = self.size.get();
        let size = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };

        let egl_image = info.id.0 as khronos_egl::EGLImage;
        let dma_buffers = dma_buf::DMABuffersForSurface::try_from(egl_image).unwrap();

        eprintln!("exported {:#?}", dma_buffers);

        // todo
        let file_descriptor = todo!();

        let mut memory_import = ash::vk::ImportMemoryFdInfoKHR::default()
            .handle_type(ash::vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT)
            .fd(file_descriptor);

        unsafe {
            let vulkan_device = wgpu_device.as_hal::<wgpu::wgc::api::Vulkan>().unwrap();

            let ash_device = vulkan_device.raw_device();

            let memory = ash_device
                .allocate_memory(
                    // todo: fill out
                    &ash::vk::MemoryAllocateInfo::default().push_next(&mut memory_import),
                    None,
                )
                .unwrap();

            let image = ash_device
                .create_image(
                    // todo: fill out
                    &ash::vk::ImageCreateInfo::default(),
                    None,
                )
                .unwrap();
            // todo
            let offset = 0;
            ash_device.bind_image_memory(image, memory, offset);

            let hal_texture = vulkan_device.texture_from_raw(
                image,
                &wgpu::hal::TextureDescriptor {
                    label,
                    size,
                    mip_level_count,
                    sample_count,
                    dimension,
                    format,
                    usage: wgpu::wgt::TextureUses::COLOR_TARGET,
                    // todo: complete
                    memory_flags: wgpu_hal::MemoryFlags::empty(),
                    view_formats: vec![],
                },
                None,
            );

            wgpu_device.create_texture_from_hal::<wgpu::wgc::api::Vulkan>(
                hal_texture,
                &wgpu::TextureDescriptor {
                    mip_level_count,
                    sample_count,
                    dimension,
                    format,
                    label,
                    size,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    // todo
                    view_formats: &[],
                },
            )
        }
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
        }*/
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

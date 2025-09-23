use std::{cell::Cell, rc::Rc, sync::Arc};

use euclid::default::Size2D;

use image::RgbaImage;
use objc2::rc::Retained;
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLPixelFormat, MTLTexture, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage,
};
use servo::RenderingContext;
use slint::wgpu_26::wgpu;
use webrender_api::units::DeviceIntRect;
use winit::dpi::PhysicalSize;

use surfman::{
    Connection, Device, Error, Surface, SurfaceTexture, SurfaceType,
    chains::{PreserveBuffer, SwapChain},
};

use foreign_types_shared::ForeignType;

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
    pub fn new(size: PhysicalSize<u32>) -> Self {
        let connection = Connection::new().unwrap();

        let adapter = connection.create_adapter().unwrap();

        let surfman_rendering_info = SurfmanRenderingContext::new(&connection, &adapter).unwrap();

        let surfman_size = Size2D::new(size.width as i32, size.height as i32);

        let surface = surfman_rendering_info
            .create_surface(SurfaceType::Generic { size: surfman_size })
            .unwrap();

        surfman_rendering_info.bind_surface(surface).unwrap();

        surfman_rendering_info.make_current().unwrap();

        let swap_chain = surfman_rendering_info.create_attached_swap_chain().unwrap();

        CustomRenderingContext {
            size: Cell::new(size),
            surfman_rendering_info,
            swap_chain,
        }
    }

    pub fn get_wgpu_texture_from_metal(&self, wgpu_device: &wgpu::Device) -> wgpu::Texture {
        let device = &self.surfman_rendering_info.device.borrow();
        let mut context = self.surfman_rendering_info.context.borrow_mut();

        let surface = &device
            .unbind_surface_from_context(&mut context)
            .unwrap()
            .unwrap();

        let size = self.size.get();

        let native_surface = device.native_surface(surface);
        let io_surface = native_surface.0;

        let wgpu_descriptor = wgpu::TextureDescriptor {
            label: Some("Metal IOSurface Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

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
            texture_descriptor.setWidth(size.width as usize);
            texture_descriptor.setHeight(size.height as usize);

            let texture = create_texture_from_iosurface(
                &*(device_raw.as_ptr() as *mut objc2::runtime::NSObject),
                &texture_descriptor,
                &io_surface,
                0,
            )
            .unwrap();

            let ptr = Retained::into_raw(texture);

            // Create the wgpu_hal Metal texture
            let metal_texture = metal::Texture::from_ptr(ptr as *mut _);

            let hal_texture = wgpu::hal::metal::Device::texture_from_raw(
                metal_texture,
                wgpu::wgt::TextureFormat::Rgba8Unorm,
                metal::MTLTextureType::D2,
                0,
                0,
                wgpu::hal::CopyExtent {
                    width: size.width,
                    height: size.height,
                    depth: 0,
                },
            );

            return wgpu_device
                .create_texture_from_hal::<wgpu::wgc::api::Metal>(hal_texture, &wgpu_descriptor);
        };
    }
}

unsafe fn create_texture_from_iosurface(
    device: &objc2::runtime::NSObject,
    descriptor: &MTLTextureDescriptor,
    iosurface: &IOSurfaceRef,
    plane: objc2_foundation::NSUInteger,
) -> Option<Retained<objc2::runtime::NSObject>> {
    use objc2::msg_send;
    msg_send![device, newTextureWithDescriptor:descriptor, iosurface:iosurface, plane:plane]
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

    fn make_current(&self) -> Result<(), Error> {
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

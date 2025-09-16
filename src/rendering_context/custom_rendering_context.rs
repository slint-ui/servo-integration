use std::{cell::Cell, rc::Rc, sync::Arc};

use euclid::default::Size2D;

use image::RgbaImage;
use servo::RenderingContext;
use surfman::{
    Connection, Device, Error, Surface, SurfaceTexture, SurfaceType,
    chains::{PreserveBuffer, SwapChain},
};
use webrender_api::units::DeviceIntRect;
use winit::dpi::PhysicalSize;

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

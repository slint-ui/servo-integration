use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
    sync::Arc,
};

use euclid::default::{Rect, Size2D};

use glow::{HasContext, NativeTexture};
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
    size: Cell<PhysicalSize<u32>>,
    surfman_rendering_info: SurfmanRenderingContext,
    swap_chain: SwapChain<Device>,
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

        let adapter = connection.create_software_adapter().unwrap();

        let surfman_rendering_info = SurfmanRenderingContext::new(&connection, &adapter).unwrap();

        let surfman_size = Size2D::new(size.width as i32, size.height as i32);

        let surface = surfman_rendering_info
            .create_surface(SurfaceType::Generic { size: surfman_size })
            .unwrap();

        surfman_rendering_info
            .bind_surface(surface)
            .unwrap();

        surfman_rendering_info.make_current().unwrap();

        let swap_chain = surfman_rendering_info.create_attached_swap_chain().unwrap();

        CustomRenderingContext {
            size: Cell::new(size),
            surfman_rendering_info,
            swap_chain,
        }
    }

    pub fn get_texture(&self) -> NativeTexture {
        let device_ref = self.surfman_rendering_info.device.borrow();
        let context_ref = self.surfman_rendering_info.context.borrow();

        // Get surface info to check what we have available
        if let Some(surface_info) = device_ref.context_surface_info(&*context_ref).unwrap() {
            // SurfaceInfo has: size, id, context_id, framebuffer_object
            // We'll work with the framebuffer approach below
        }
        
        // If no color texture, try to create one from the current surface
        // This is a more complex approach - let's first try to read from current GL state
        let gl = &self.surfman_rendering_info.glow_gl;
        
        // Get the current framebuffer's color attachment
        unsafe {
            let current_fbo = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
            if current_fbo != 0 {
                // We have a non-default framebuffer, try to get its color attachment
                // This is a workaround - we'll need to create a texture and copy the content
                
                let size = self.size();
                let texture = gl.create_texture().unwrap();
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    size.width as i32,
                    size.height as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                
                // Copy the framebuffer content to our texture
                gl.copy_tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    0,
                    0,
                    size.width as i32,
                    size.height as i32,
                );
                
                return texture;
            }
        }
        
        // Fallback - return dummy texture
        glow::NativeTexture(std::num::NonZeroU32::new(1).unwrap())
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

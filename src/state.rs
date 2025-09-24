use std::{cell::RefCell, rc::Rc};

use servo::{Servo, WebView};
use slint::{ComponentHandle, Weak, wgpu_26::wgpu};

use crate::{MyApp, rendering_context::CustomRenderingContext};

pub struct State {
    pub app: Weak<MyApp>,
    pub device: RefCell<Option<wgpu::Device>>,
    pub queue: RefCell<Option<wgpu::Queue>>,
    pub scale_factor: RefCell<f32>,
    pub servo: RefCell<Option<Servo>>,
    pub webview: RefCell<Option<WebView>>,
    pub rendering_context: RefCell<Option<Rc<CustomRenderingContext>>>,
}

impl State {
    pub fn new(app: Weak<MyApp>) -> Self {
        Self {
            app,
            device: RefCell::new(None),
            queue: RefCell::new(None),
            servo: RefCell::new(None),
            webview: RefCell::new(None),
            scale_factor: RefCell::new(1.0),
            rendering_context: RefCell::new(None),
        }
    }

    pub fn update_web_content_with_latest_frame(&self) {
        let rendering_context_ref = self.rendering_context.borrow();
        let rendering_context = rendering_context_ref.as_ref().unwrap();

        // let size = rendering_context.size2d().to_i32();

        // let viewport_rect = DeviceIntRect::from_origin_and_size(Point2D::origin(), size);

        // let image_buffer = rendering_context.read_to_image(viewport_rect).unwrap();

        // let (width, height) = image_buffer.dimensions();
        // let pixel_slice = image_buffer.into_raw();

        // let shared_pixel_buffer = SharedPixelBuffer::clone_from_slice(&pixel_slice, width, height);

        // let slint_image = Image::from_rgba8(shared_pixel_buffer);

        let wgpu_device = self.device.borrow();
        let wgpu_device = wgpu_device.as_ref().unwrap();
        
        let wgpu_queue = self.queue.borrow();
        let wgpu_queue = wgpu_queue.as_ref().unwrap();

        let texture = rendering_context.get_wgpu_texture_from_metal(wgpu_device, wgpu_queue);

        let slint_image = slint::Image::try_from(texture)
            .expect("Failed to create slint image from wgpu texture");

        let app = self.app.upgrade().unwrap();

        app.set_web_content(slint_image);
        app.window().request_redraw();
    }
}

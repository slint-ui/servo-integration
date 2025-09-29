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
        println!("Creating new application state");
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
        let rendering_context = self.rendering_context.borrow();
        let rendering_context = rendering_context
            .as_ref()
            .expect("Rendering context not initialized - ensure Servo setup completed");

        let wgpu_device = self.device.borrow();
        let wgpu_device = wgpu_device
            .as_ref()
            .expect("WGPU device not initialized - ensure rendering setup completed");

        let wgpu_queue = self.queue.borrow();
        let wgpu_queue = wgpu_queue
            .as_ref()
            .expect("WGPU queue not initialized - ensure rendering setup completed");

        let texture = rendering_context.get_wgpu_texture_from_vulkan(wgpu_device, wgpu_queue);
        //.expect(
        //    "Failed to get WGPU texture from Vulkan texture - ensure rendering context is valid",
        //);

        let slint_image = slint::Image::try_from(texture).expect(
            "Failed to create Slint image from WGPU texture - check texture format compatibility",
        );

        let app = self
            .app
            .upgrade()
            .expect("Application reference is no longer valid - UI may have been destroyed");

        app.set_web_content(slint_image);
        app.window().request_redraw();
    }
}

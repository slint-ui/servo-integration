use std::cell::RefCell;

use servo::{Servo, WebView};
use slint::{ComponentHandle, Weak};
use smol::channel::Sender;

#[cfg(not(target_os = "android"))]
use slint::wgpu_26::wgpu;

use crate::{MyApp, rendering_context::ServoRenderingAdapter};

pub struct SlintServoAdapter {
    pub app: Weak<MyApp>,
    pub waker_sender: Sender<()>,
    pub scale_factor: RefCell<f32>,
    pub servo: RefCell<Option<Servo>>,
    pub webview: RefCell<Option<WebView>>,
    pub rendering_adapter: RefCell<Option<Box<dyn ServoRenderingAdapter>>>,
    #[cfg(not(target_os = "android"))]
    pub device: RefCell<Option<wgpu::Device>>,
    #[cfg(not(target_os = "android"))]
    pub queue: RefCell<Option<wgpu::Queue>>,
}

impl SlintServoAdapter {
    pub fn new(app: Weak<MyApp>, waker_sender: Sender<()>) -> Self {
        Self {
            app,
            waker_sender,
            servo: RefCell::new(None),
            webview: RefCell::new(None),
            scale_factor: RefCell::new(1.0),
            rendering_adapter: RefCell::new(None),
            #[cfg(not(target_os = "android"))]
            device: RefCell::new(None),
            #[cfg(not(target_os = "android"))]
            queue: RefCell::new(None),
        }
    }

    pub fn update_web_content_with_latest_frame(&self) {
        let rendering_adapter = self.rendering_adapter.borrow();
        let rendering_adapter = rendering_adapter
            .as_ref()
            .expect("Failed to get rendering adpater check if it initliazed first");

        let slint_image = rendering_adapter.current_framebuffer_as_image();

        let app = self
            .app
            .upgrade()
            .expect("Application reference is no longer valid - UI may have been destroyed");

        app.set_web_content(slint_image);
        app.window().request_redraw();
    }
}

use std::cell::RefCell;
use std::rc::Rc;

use euclid::Point2D;
use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView};
use slint::{ComponentHandle, Weak, wgpu_26::wgpu};
use smol::channel::Sender;
use webrender_api::units::DeviceIntRect;

use crate::{
    MyApp,
    rendering_context::{self, ServoRenderingAdapter},
};

pub struct SlintServoAdapter {
    pub app: Weak<MyApp>,
    pub waker_sender: Sender<()>,
    // pub device: RefCell<Option<wgpu::Device>>,
    // pub queue: RefCell<Option<wgpu::Queue>>,
    pub scale_factor: RefCell<f32>,
    pub servo: RefCell<Option<Servo>>,
    pub webview: RefCell<Option<WebView>>,
    // pub rendering_adapter: RefCell<Option<Box<dyn ServoRenderingAdapter>>>,
    pub rendering_context: RefCell<Option<Rc<SoftwareRenderingContext>>>,
}

impl SlintServoAdapter {
    pub fn new(app: Weak<MyApp>, waker_sender: Sender<()>) -> Self {
        Self {
            app,
            waker_sender,
            // device: RefCell::new(None),
            // queue: RefCell::new(None),
            servo: RefCell::new(None),
            webview: RefCell::new(None),
            scale_factor: RefCell::new(1.0),
            // rendering_adapter: RefCell::new(None),
            rendering_context: RefCell::new(None),
        }
    }

    pub fn update_web_content_with_latest_frame(&self) {
        // let rendering_adapter = self.rendering_adapter.borrow();
        // let rendering_adapter = rendering_adapter
        //     .as_ref()
        //     .expect("Failed to get rendering adpater check if it initliazed first");

        // let slint_image = rendering_adapter.current_framebuffer_as_image();

        let rendering_context = self.rendering_context.borrow();
        let rendering_context = rendering_context.as_ref().unwrap();

        let size = rendering_context.size2d().to_i32();
        let viewport_rect = DeviceIntRect::from_origin_and_size(Point2D::origin(), size);

        let image_buffer = rendering_context.read_to_image(viewport_rect).unwrap();
        let (width, height) = image_buffer.dimensions();

        eprintln!("Captured image dimensions: {}x{}", width, height);

        // Check if image has any non-white content for verification
        let has_non_white_content = image_buffer.pixels().any(|pixel| {
            let rgba = pixel.0;
            // Check if any pixel is not white (255,255,255) with full alpha
            rgba[0] != 255 || rgba[1] != 255 || rgba[2] != 255 || rgba[3] != 255
        });

        eprintln!("Image has non-white content: {}", has_non_white_content);

        // Sample a few pixels for verification
        let mut pixel_samples = Vec::new();
        for (i, pixel) in image_buffer.pixels().enumerate() {
            if i < 5 {
                pixel_samples.push(pixel.0);
            }
            if pixel_samples.len() >= 5 {
                break;
            }
        }
        eprintln!("First 5 pixel samples: {:?}", pixel_samples);

        // Convert the ImageBuffer to raw RGBA bytes
        let rgba_data: Vec<u8> = image_buffer.into_raw();
        let buffer = slint::SharedPixelBuffer::clone_from_slice(&rgba_data, width, height);

        // Create a Slint Image from the raw RGBA data
        let slint_image = slint::Image::from_rgba8(buffer);

        let app = self
            .app
            .upgrade()
            .expect("Application reference is no longer valid - UI may have been destroyed");

        app.set_web_content(slint_image);
        app.window().request_redraw();
    }
}

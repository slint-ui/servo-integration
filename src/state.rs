use std::{cell::RefCell, rc::Rc};

use euclid::Point2D;
use webrender_api::units::DeviceIntRect;

use slint::{ComponentHandle, Image, SharedPixelBuffer};

use servo::{RenderingContext, Servo, SoftwareRenderingContext, WebView};

use crate::MyApp;

pub struct State {
    pub app: MyApp,
    pub scale_factor: RefCell<f32>,
    pub servo: RefCell<Option<Servo>>,
    pub webview: RefCell<Option<WebView>>,
    pub rendering_context: RefCell<Option<Rc<SoftwareRenderingContext>>>,
}

impl State {
    pub fn new(app: MyApp) -> Self {
        Self {
            app,
            servo: RefCell::new(None),
            webview: RefCell::new(None),
            scale_factor: RefCell::new(1.0),
            rendering_context: RefCell::new(None),
        }
    }

    pub fn get_slint_image(&self) -> Image {
        let rendering_context_ref = self.rendering_context.borrow();
        let rendering_context = rendering_context_ref.as_ref().unwrap();
        slint_image_from_rendering_context(rendering_context)
    }

    pub fn update_web_content_with_latest_frame(&self) {
        let image = self.get_slint_image();
        self.app.set_web_content(image);
        self.app.window().request_redraw();
    }
}

pub fn slint_image_from_rendering_context<T>(rendering_context: &Rc<T>) -> Image
where
    T: RenderingContext + ?Sized,
{
    let size = rendering_context.size2d().to_i32();
    let viewport_rect = DeviceIntRect::from_origin_and_size(Point2D::origin(), size);

    let image_buffer = rendering_context.read_to_image(viewport_rect).unwrap();

    let (width, height) = image_buffer.dimensions();
    let pixel_slice = image_buffer.into_raw();

    let shared_pixel_buffer = SharedPixelBuffer::clone_from_slice(&pixel_slice, width, height);

    Image::from_rgba8(shared_pixel_buffer)
}

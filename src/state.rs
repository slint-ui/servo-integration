use std::{cell::RefCell, rc::Rc};


use servo::{RenderingContext, Servo, WebView};
use slint::ComponentHandle;

use crate::{MyApp, rendering_context::CustomRenderingContext};

pub struct State {
    pub app: MyApp,
    pub scale_factor: RefCell<f32>,
    pub servo: RefCell<Option<Servo>>,
    pub webview: RefCell<Option<WebView>>,
    pub rendering_context: RefCell<Option<Rc<CustomRenderingContext>>>,
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

    pub fn update_web_content_with_latest_frame(&self) {
        let rendering_context_ref = self.rendering_context.borrow();
        let rendering_context = rendering_context_ref.as_ref().unwrap();

        let size = rendering_context.size();

        let texture = rendering_context.get_texture();

        let slint_image = unsafe {
            slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                texture.0,
                (size.width, size.height).into(),
            )
            .build()
        };

        self.app.set_web_content(slint_image);
        self.app.window().request_redraw();
    }
}

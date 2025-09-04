#![allow(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::rc::Rc;

use url::Url;

use winit::dpi;

use slint::winit_030::WinitWindowAccessor;

use slint::RenderingState;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use servo::{
    RenderingContext, Servo, ServoBuilder, WebView, WebViewBuilder, WindowRenderingContext,
};

slint::slint! {

    export component MyApp inherits Window {
        width: 1024px;
        height: 768px;

        in property <image> web_content <=> image.source;

        image :=  Image {
            width: 100%;
            height: 100%;
        }
    }
}
struct AppDelegate {
    app: MyApp,
}

impl servo::WebViewDelegate for AppDelegate {
    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.app.window().request_redraw();
    }
}

fn main() {
    let app = MyApp::new().unwrap();
    let app_weak = app.as_weak();

    //let window = app.window();

    let servo: Rc<RefCell<Option<Servo>>> = Rc::new(RefCell::new(None));
    let servo_clone = servo.clone();

    let webview: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));
    let webview_clone = webview.clone();

    slint::spawn_local({
        let app_weak = app_weak.clone();
        async move {
            let app = app_weak.upgrade().unwrap();
            let winit_window = app.window().winit_window().await.unwrap();
            let slint_window_handle = app.window().window_handle();

            let window_handle = slint_window_handle.window_handle().unwrap();
            let display_handle = slint_window_handle.display_handle().unwrap();

            let window_size = app.window().size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);

            let rendering_context =
                WindowRenderingContext::new(display_handle, window_handle, size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let offscreen_context = rendering_context_rc.offscreen_context(size);
            let offscreen_context_rc = Rc::new(offscreen_context);

            let _ = offscreen_context_rc.make_current();

            let servo_instance = ServoBuilder::new(rendering_context_rc).build();
            servo_instance.setup_logging();

            let delegate = Rc::new(AppDelegate { app: app });

            let url = Url::parse("https://example.com/").unwrap();
            let webview_instance = WebViewBuilder::new(&servo_instance)
                .url(url)
                .delegate(delegate.clone())
                .build();

            *webview_clone.borrow_mut() = Some(webview_instance);
            *servo_clone.borrow_mut() = Some(servo_instance);
        }
    })
    .unwrap();

    /* 
    window
        .set_rendering_notifier(move |state, _graphics_api| match state {
            RenderingState::RenderingSetup => {
                let app = app_weak.upgrade().unwrap();

                let window = app.window();
                let slint_window_handle = window.window_handle();

                let window_handle = slint_window_handle.window_handle().unwrap();
                let display_handle = slint_window_handle.display_handle().unwrap();

                let window_size = window.size();
                let size = dpi::PhysicalSize::new(window_size.width, window_size.height);

                let rendering_context =
                    WindowRenderingContext::new(display_handle, window_handle, size).unwrap();
                let rendering_context_rc = Rc::new(rendering_context);

                let offscreen_context = rendering_context_rc.offscreen_context(size);
                let offscreen_context_rc = Rc::new(offscreen_context);

                let _ = offscreen_context_rc.make_current();

                let servo_instance = ServoBuilder::new(offscreen_context_rc).build();
                servo_instance.setup_logging();

                let delegate = Rc::new(AppDelegate { app: app });

                let url = Url::parse("https://example.com/").unwrap();
                let webview_instance = WebViewBuilder::new(&servo_instance)
                    .url(url)
                    .delegate(delegate.clone())
                    .build();

                *webview_clone.borrow_mut() = Some(webview_instance);
                *servo_clone.borrow_mut() = Some(servo_instance);
            }
            RenderingState::RenderingTeardown => {
                *webview_clone.borrow_mut() = None;
                if let Some(servo_instance) = servo_clone.borrow_mut().take() {
                    servo_instance.deinit();
                }
            }
            _ => {}
        })
        .unwrap();
*/

    app.run().unwrap();
}

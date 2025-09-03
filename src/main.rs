#![allow(unsafe_op_in_unsafe_fn)]

mod servo_util;

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use servo::RenderingContext;

use i_slint_backend_winit::{Backend, CustomApplicationHandler, WinitWindowEventResult};
use slint::{RenderingState, Window, platform::set_platform};

use crate::servo_util::MyServo;
use crate::servo_util::render_context::get_rendering_context;
use crate::servo_util::webview::WebView;

slint::slint! {
    export component MyApp inherits Window {
        width: 1024px;
        height: 768px;

        in property <image> web_content <=> image.source;

        image := Image {
            width: 100%;
            height: 100%;
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    let app_state = Rc::new(RefCell::new(AppState::default()));

    let handler = ApplicationHandler {
        app_state: app_state.clone(),
    };

    let backend = Backend::builder()
        .with_custom_application_handler(handler)
        .build()?;

    let proxy = backend.create_winit_event_loop_proxy();

    set_platform(Box::new(backend))?;

    let app = Rc::new(MyApp::new()?);

    let app_for_closure = app.clone();
    let app_state_for_closer = app_state.clone();

    app.window()
        .set_rendering_notifier(move |state, _graphics_api| match state {
            RenderingState::RenderingSetup => {
                let proxy = proxy.clone().expect("Event loop proxy not available");

                let window = app_for_closure.window();

                let context = get_rendering_context(window);

                let servo = MyServo::new(proxy, context.clone());

                let webview = servo.create_webview(window);

                servo.spin_event_loop(window);

                let new_app_state = AppState {
                    is_initialized: true,
                    servo: Some(servo),
                    rendering_context: Some(context),
                    webview: Some(webview),
                    app: Some(app_for_closure.clone()),
                };

                *app_state_for_closer.borrow_mut() = new_app_state;
            }
            RenderingState::RenderingTeardown => {
                let mut app_state_mut = app_state_for_closer.borrow_mut();
                if let Some(servo) = app_state_mut.servo.take() {
                    servo.deinit();
                }
            }
            _ => {}
        })?;

    app.run()?;

    Ok(())
}

#[derive(Default)]
pub struct AppState {
    pub is_initialized: bool,
    pub servo: Option<MyServo>,
    pub rendering_context: Option<Rc<dyn RenderingContext>>,
    pub webview: Option<WebView>,
    pub app: Option<Rc<MyApp>>,
}

struct ApplicationHandler {
    app_state: Rc<RefCell<AppState>>,
}

impl CustomApplicationHandler for ApplicationHandler {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _: WindowId,
        _: Option<&winit::window::Window>,
        _: Option<&Window>,
        event: &WindowEvent,
    ) -> WinitWindowEventResult {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let app_state = self.app_state.borrow();

                if app_state.is_initialized {
                    let servo = app_state.servo.as_ref().expect("Servo not available");

                    let app = app_state.app.as_ref().expect("App not available");

                    // Continue spinning the servo event loop
                    if !servo.spin_event_loop(&app.window()) {
                        // Servo has shut down
                        event_loop.exit();
                        return WinitWindowEventResult::Propagate;
                    }

                    let rendered_image = servo.get_rendered_image();
                    //     .expect("Failed to get rendered image");

                    // app.set_web_content(rendered_image);
                }
            }
            _ => (),
        }
        WinitWindowEventResult::Propagate
    }
}

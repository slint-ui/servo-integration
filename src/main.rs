#![allow(unsafe_op_in_unsafe_fn)]
use std::cell::RefCell;
use std::rc::Rc;

use euclid::Point2D;
use smol::{channel, channel::Sender};
use url::Url;
use winit::dpi;

use slint::{Image, SharedPixelBuffer, winit_030::WinitWindowAccessor};

use embedder_traits::EventLoopWaker;
use servo::{
    RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder,
    WebViewDelegate,
};
use webrender_api::units::DeviceIntRect;

slint::slint! {
    export component MyApp inherits Window {
        width: 1024px;
        height: 768px;

        in property <image> web_content <=> image.source;

        VerticalLayout {
            Text {
                text: "Hello from Slint";
            }
            image := Image {
                    width: 100%;
                    height: 100%;
                }
        }
    }
}

struct AppDelegate {
    state: Rc<State>,
}

impl WebViewDelegate for AppDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        webview.show(true);
        webview.paint();

        if let Some(ref rendering_context) = *self.state.rendering_context.borrow() {
            let image = get_slint_image(rendering_context);
            self.state.app.set_web_content(image);
            self.state.app.window().request_redraw();
        }
    }
}

struct State {
    app: MyApp,
    servo: RefCell<Option<Servo>>,
    webview: RefCell<Option<WebView>>,
    rendering_context: RefCell<Option<Rc<SoftwareRenderingContext>>>,
}

impl State {}

fn main() {
    let state = Rc::new(State {
        app: MyApp::new().unwrap(),
        servo: RefCell::new(None),
        webview: RefCell::new(None),
        rendering_context: RefCell::new(None),
    });

    let state_weak = Rc::downgrade(&state);

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    slint::spawn_local({
        async move {
            let state = state_weak.upgrade().unwrap();

            let winit_window = state.app.window().winit_window().await.unwrap();

            let window_size = winit_window.inner_size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);

            let rendering_context = SoftwareRenderingContext::new(size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let servo_instance = ServoBuilder::new(rendering_context_rc.clone())
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
                .build();

            let delegate = Rc::new(AppDelegate {
                state: state.clone(),
            });

            let url = Url::parse("https://slint.dev/").unwrap();

            let webview_instance = WebViewBuilder::new(&servo_instance)
                .url(url)
                .delegate(delegate)
                .build();

            *state.servo.borrow_mut() = Some(servo_instance);
            *state.webview.borrow_mut() = Some(webview_instance);
            *state.rendering_context.borrow_mut() = Some(rendering_context_rc.clone());
        }
    })
    .unwrap();

    {
        let state = state.clone();
        slint::spawn_local({
            async move {
                loop {
                    let _ = waker_receiver.recv().await;
                    if let Some(ref servo) = *state.servo.borrow() {
                        servo.spin_event_loop();
                    }
                }
            }
        })
        .unwrap();
    }

    state.app.run().unwrap();
}

#[derive(Clone)]
struct Waker(Sender<()>);

impl Waker {
    fn new(sender: Sender<()>) -> Self {
        Self(sender)
    }
}

impl EventLoopWaker for Waker {
    fn wake(&self) {
        let _ = self.0.try_send(());
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
}

pub fn get_slint_image<T>(rendering_context: &Rc<T>) -> Image
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

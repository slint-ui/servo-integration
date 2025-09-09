#![allow(unsafe_op_in_unsafe_fn)]

mod pointer_event;

use std::{cell::RefCell, rc::Rc};

use euclid::{Point2D, vec2};
use url::Url;
use winit::dpi;

use smol::{channel, channel::Sender};

use slint::{ComponentHandle, Image, SharedPixelBuffer, winit_030::WinitWindowAccessor};

use embedder_traits::EventLoopWaker;

use webrender_api::{
    ScrollLocation,
    units::{DeviceIntPoint, DeviceIntRect},
};

use servo::{
    RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder,
    WebViewDelegate,
};

use crate::pointer_event::convert_slint_pointer_event_to_servo_input_event;

slint::slint! {
    export component MyApp inherits Window {
        width: 1024px;
        height: 768px;

        in property <image> web_content <=> image.source;
        out property <length> mouse_x <=> touch_area.mouse-x;
        out property <length> mouse_y <=> touch_area.mouse-y;

        callback scroll(length, length);
        callback pointer_event(PointerEvent);

        touch_area := TouchArea {
            image := Image {
                width: 100%;
                height: 100%;
            }
            pointer-event(event) => {
                root.pointer_event(event);
            }
            scroll-event(event) => {
                scroll(event.delta-x, event.delta-y);
                return accept;
            }
        }

    }
}

fn main() {
    let url_string = "https://slint.dev/";
    // let url_string = "https://demo.servo.org/experiments/twgl-tunnel/";

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state = Rc::new(State::new(MyApp::new().unwrap()));

    let state_weak = Rc::downgrade(&state);
    slint::spawn_local({
        async move {
            let state = state_weak.upgrade().unwrap();
            loop {
                let _ = waker_receiver.recv().await;
                if let Some(ref servo) = *state.servo.borrow() {
                    servo.spin_event_loop();
                }
            }
        }
    })
    .unwrap();

    let state_weak = Rc::downgrade(&state);
    state.app.on_scroll(move |x, y| {
        let state = state_weak.upgrade().unwrap();

        let webview_ref = state.webview.borrow();
        let webview = webview_ref.as_ref().unwrap();

        let dx = -(x as f32);
        let dy = -(y as f32);

        let moved_by = vec2(dx, dy);
        let point = DeviceIntPoint::new(10, 10);

        webview.notify_scroll_event(ScrollLocation::Delta(moved_by), point);
    });

    let state_weak = Rc::downgrade(&state);
    state.app.on_pointer_event(move |event| {
        let state = state_weak.upgrade().unwrap();

        let webview_ref = state.webview.borrow();
        let webview = webview_ref.as_ref().unwrap();

        let event_str = format!("{:?}", event);
        println!("Pointer event: {}", event_str);

        let mouse_x = state.app.get_mouse_x();
        let mouse_y = state.app.get_mouse_y();
        let scale_factor = *state.scale_factor.borrow();

        let input_event = convert_slint_pointer_event_to_servo_input_event(
            &event_str,
            mouse_x,
            mouse_y,
            scale_factor,
        );

        webview.notify_input_event(input_event);
    });

    let state_weak = Rc::downgrade(&state);
    slint::spawn_local({
        async move {
            let state = state_weak.upgrade().unwrap();

            let winit_window = state.app.window().winit_window().await.unwrap();

            let window_size = winit_window.inner_size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);
            let scale_factor = winit_window.scale_factor() as f32;

            let rendering_context = SoftwareRenderingContext::new(size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let servo = ServoBuilder::new(rendering_context_rc.clone())
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
                .build();

            let url = Url::parse(url_string).unwrap();
            let delegate = Rc::new(AppDelegate::new(state.clone()));

            let webview = WebViewBuilder::new(&servo)
                .url(url)
                .delegate(delegate)
                .build();

            webview.show(true);

            *state.servo.borrow_mut() = Some(servo);
            *state.scale_factor.borrow_mut() = scale_factor;
            *state.webview.borrow_mut() = Some(webview);
            *state.rendering_context.borrow_mut() = Some(rendering_context_rc.clone());
        }
    })
    .unwrap();

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

pub struct AppDelegate {
    pub state: Rc<State>,
}

impl AppDelegate {
    pub fn new(state: Rc<State>) -> Self {
        Self { state }
    }
}

impl WebViewDelegate for AppDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        webview.paint();
        self.state.update_web_content_with_latest_frame();
    }
}

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

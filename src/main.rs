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
    app_weak: slint::Weak<MyApp>,
    rendering_context: Rc<SoftwareRenderingContext>,
}

impl WebViewDelegate for AppDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        webview.show(true);
        webview.paint();

        let image = get_slint_image(&self.rendering_context);

        let app = self.app_weak.upgrade().unwrap();
        app.set_web_content(image);
        app.window().request_redraw();
    }
}

fn main() {
    let app = MyApp::new().unwrap();
    let app_weak = app.as_weak();

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let servo: Rc<RefCell<Option<Servo>>> = Rc::new(RefCell::new(None));
    let webview: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));

    slint::spawn_local({
        let app_weak_clone = app_weak.clone();
        let servo_clone = servo.clone();
        let webview_clone = webview.clone();

        async move {
            let app = app_weak_clone.upgrade().unwrap();
            let winit_window = app.window().winit_window().await.unwrap();

            let window_size = winit_window.inner_size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);

            let rendering_context = SoftwareRenderingContext::new(size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let servo_instance = ServoBuilder::new(rendering_context_rc.clone())
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
                .build();

            let delegate = Rc::new(AppDelegate {
                app_weak: app_weak_clone.clone(),
                rendering_context: rendering_context_rc,
            });

            let url = Url::parse("https://slint.dev/").unwrap();

            let webview_instance = WebViewBuilder::new(&servo_instance)
                .url(url)
                .delegate(delegate)
                .build();

            *webview_clone.borrow_mut() = Some(webview_instance);
            *servo_clone.borrow_mut() = Some(servo_instance);
        }
    })
    .unwrap();

    slint::spawn_local({
        let servo_clone = servo.clone();

        async move {
            loop {
                let _ = waker_receiver.recv().await;
                if let Some(servo) = servo_clone.borrow().as_ref() {
                    servo.spin_event_loop();
                }
            }
        }
    })
    .unwrap();

    app.run().unwrap();
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

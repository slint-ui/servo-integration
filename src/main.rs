#![allow(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::rc::Rc;

use euclid::Point2D;
use image::{DynamicImage, ImageFormat};
use url::Url;

use webrender_api::units::DeviceIntRect;
use winit::dpi;

use slint::winit_030::WinitWindowAccessor;

use servo::{
    LoadStatus, RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView,
    WebViewBuilder,
};

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
    rendering_context: Rc<SoftwareRenderingContext>,
}

impl servo::WebViewDelegate for AppDelegate {
    fn notify_load_status_changed(&self, webview: WebView, status: LoadStatus) {
        if status == LoadStatus::Complete {
            eprintln!("Load finished for {:?}", webview.page_title());
            save_output_image(&self.rendering_context);
            self.rendering_context.present();
        }
    }
}

fn main() {
    let app = MyApp::new().unwrap();
    let app_weak = app.as_weak();

    let (waker_sender, waker_receiver) = smol::channel::unbounded::<()>();

    let servo: Rc<RefCell<Option<Servo>>> = Rc::new(RefCell::new(None));

    let webview: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));
    let webview_clone = webview.clone();

    slint::spawn_local({
        let app_weak = app_weak.clone();
        let servo_clone = servo.clone();

        async move {
            let app = app_weak.upgrade().unwrap();
            let winit_window = app.window().winit_window().await.unwrap();

            let window_size = winit_window.inner_size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);

            let rendering_context = SoftwareRenderingContext::new(size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let servo_instance = ServoBuilder::new(rendering_context_rc.clone())
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
                .build();
            servo_instance.setup_logging();

            let delegate = Rc::new(AppDelegate {
                rendering_context: rendering_context_rc,
            });

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
struct Waker(smol::channel::Sender<()>);

impl Waker {
    fn new(sender: smol::channel::Sender<()>) -> Self {
        Self(sender)
    }
}

impl embedder_traits::EventLoopWaker for Waker {
    fn wake(&self) {
        self.0.send_blocking(()).unwrap();
    }

    fn clone_box(&self) -> Box<dyn embedder_traits::EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
}

/// This needs to be done before presenting(), because `ReneringContext::read_to_image` reads
/// from the back buffer.
pub fn save_output_image<T>(rendering_context: &Rc<T>)
where
    T: RenderingContext + ?Sized,
{
    let size = rendering_context.size2d().to_i32();

    let viewport_rect = DeviceIntRect::from_origin_and_size(Point2D::origin(), size);

    let image = rendering_context.read_to_image(viewport_rect).unwrap();

    let output_path = "./output.png";

    let image_format = ImageFormat::from_path(output_path).unwrap_or(ImageFormat::Png);

    DynamicImage::ImageRgba8(image)
        .save_with_format(output_path, image_format)
        .unwrap()
}

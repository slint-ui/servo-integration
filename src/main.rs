#![allow(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::rc::Rc;

use euclid::Point2D;
use url::Url;
use webrender_api::units::DeviceIntRect;
use winit::dpi;

use slint::winit_030::WinitWindowAccessor;

use servo::{
    RenderingContext, Servo, ServoBuilder, SoftwareRenderingContext, WebView, WebViewBuilder,
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
    frame_sender: smol::channel::Sender<slint::Image>,
}

impl servo::WebViewDelegate for AppDelegate {
    fn notify_new_frame_ready(&self, webview: WebView) {
        eprintln!("New frame ready for {:?}", webview.page_title());
        webview.show(true);
        webview.paint();

        // Send updated frame to Slint with verification
        let image = get_slint_image_with_verification(&self.rendering_context);
        let _ = self.frame_sender.try_send(image);
    }
}

fn main() {
    let app = MyApp::new().unwrap();
    let app_weak = app.as_weak();

    let (waker_sender, waker_receiver) = smol::channel::unbounded::<()>();

    // Create channels for communication between UI and Servo rendering
    let (frame_sender, frame_receiver) = smol::channel::unbounded::<slint::Image>();

    let servo: Rc<RefCell<Option<Servo>>> = Rc::new(RefCell::new(None));

    let webview: Rc<RefCell<Option<WebView>>> = Rc::new(RefCell::new(None));
    let webview_clone = webview.clone();

    slint::spawn_local({
        let app_weak = app_weak.clone();
        let servo_clone = servo.clone();
        let frame_sender_clone = frame_sender.clone();

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
                frame_sender: frame_sender_clone,
            });

            let url = Url::parse("https://slint.dev/").unwrap();
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

    // Use timer to efficiently update web content from Servo frames
    let app_weak_timer = app.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16), // ~60 FPS
        move || {
            // Check for new frames from Servo
            if let Ok(image) = frame_receiver.try_recv() {
                if let Some(app) = app_weak_timer.upgrade() {
                    eprintln!("Updating Slint UI with new frame from timer");
                    app.set_web_content(image);
                    // Only request redraw when we have new content
                    app.window().request_redraw();
                }
            }
        },
    );

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
        // Use try_send to avoid blocking and handle errors gracefully
        let _ = self.0.try_send(());
    }

    fn clone_box(&self) -> Box<dyn embedder_traits::EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }
}

pub fn get_slint_image_with_verification<T>(rendering_context: &Rc<T>) -> slint::Image
where
    T: RenderingContext + ?Sized,
{
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
    return slint::Image::from_rgba8(buffer);
}

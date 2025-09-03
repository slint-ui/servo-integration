pub mod compositor;
pub mod constellation;
pub mod embedder;
pub mod render_context;
pub mod waker;
pub mod webrender;
pub mod webview;

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use servo::script;
use url::Url;
use winit::event_loop::EventLoopProxy;
use i_slint_backend_winit::SlintEvent;

use webgl::WebGLComm;
use webrender_api::units::DeviceIntRect;

use embedder_traits::EmbedderMsg;
use embedder_traits::user_content_manager::UserContentManager;

use compositing::{IOCompositor, InitialCompositorState};
use compositing_traits::WebrenderExternalImageHandlers;
use constellation::ConstellationProxy;

use compositing_traits::WebrenderImageHandlerType;
use constellation_traits::EmbedderToConstellationMessage;
use crossbeam_channel::Receiver;
use image::DynamicImage;
use image::ImageFormat;
use log::debug;
use log::warn;
use servo::RenderingContext;
use servo::ShutdownState;
use servo::ViewportDetails;
use servo::base::id::WebViewId;
use servo::base::id::{PipelineNamespace, PipelineNamespaceId};
use servo::canvas_traits::webgl::GlType;
use servo::euclid::{Point2D, Scale};
use servo::media::WindowGLContext;
use servo::net::protocols::ProtocolRegistry;
use servo::profile::{mem as profile_mem, time as profile_time};
use slint::Window;

use crate::servo_util::compositor::create_compositor_channel;
use crate::servo_util::constellation::create_constellation;
use crate::servo_util::embedder::create_embedder_channel;
use crate::servo_util::waker::Waker;
use crate::servo_util::webrender::get_webrender;
use crate::servo_util::webview::WebView;

pub struct MyServo {
    pub rendering_context: Rc<dyn RenderingContext>,
    pub compositor: Rc<RefCell<IOCompositor>>,
    pub constellation_proxy: ConstellationProxy,
    pub embedder_receiver: Receiver<EmbedderMsg>,
    pub webview: Rc<RefCell<Option<WebView>>>,
    pub shutdown_state: Rc<Cell<ShutdownState>>,
    pub webrender_gl: Rc<dyn gleam::gl::Gl>,
}

impl MyServo {
    pub fn new(
        proxy: EventLoopProxy<SlintEvent>,
        rendering_context: Rc<dyn RenderingContext>,
    ) -> Self {
        // Get GL bindings
        let webrender_gl = rendering_context.gleam_gl_api();

        // Make sure the gl context is made current.
        if let Err(err) = rendering_context.make_current() {
            warn!("Failed to make the rendering context current: {:?}", err);
        }
        debug_assert_eq!(webrender_gl.get_error(), gleam::gl::NO_ERROR,);

        // Reserving a namespace to create WebViewId.
        PipelineNamespace::install(PipelineNamespaceId(0));

        let event_loop_waker = Box::new(Waker(proxy));

        let (compositor_proxy, compositor_receiver) =
            create_compositor_channel(event_loop_waker.clone());

        let (embedder_proxy, embedder_receiver) = create_embedder_channel(event_loop_waker.clone());

        let time_profiler_chan = profile_time::Profiler::create(&None, None);

        let mem_profiler_chan = profile_mem::Profiler::create();

        let (mut webrender, webrender_api_sender, webrender_api, webrender_document) =
            get_webrender(
                rendering_context.clone(),
                compositor_proxy.clone(),
                webrender_gl.clone(),
            );

        // Important that this call is done in a single-threaded fashion, we
        // can't defer it after `create_constellation` has started.
        let js_engine_setup = if false { Some(script::init()) } else { None };

        // Create the webgl thread
        let gl_type = match webrender_gl.get_type() {
            gleam::gl::GlType::Gl => GlType::Gl,
            gleam::gl::GlType::Gles => GlType::Gles,
        };

        let (external_image_handlers, external_images) = WebrenderExternalImageHandlers::new();
        let mut external_image_handlers = Box::new(external_image_handlers);

        let WebGLComm {
            webgl_threads,
            image_handler,
        } = WebGLComm::new(
            rendering_context.clone(),
            compositor_proxy.cross_process_compositor_api.clone(),
            webrender_api.create_sender(),
            external_images.clone(),
            gl_type,
        );

        // Set webrender external image handler for WebGL textures
        external_image_handlers.set_handler(image_handler, WebrenderImageHandlerType::WebGL);

        WindowGLContext::initialize_image_handler(
            &mut external_image_handlers,
            external_images.clone(),
        );

        webrender.set_external_image_handler(external_image_handlers);

        // Create the constellation, which maintains the engine pipelines, including script and
        // layout, as well as the navigation context.
        let protocols = ProtocolRegistry::with_internal_protocols();

        let constellation_chan = create_constellation(
            embedder_proxy,
            compositor_proxy.clone(),
            time_profiler_chan.clone(),
            mem_profiler_chan.clone(),
            webrender_document,
            webrender_api_sender,
            external_images,
            protocols,
            UserContentManager::default(),
        );

        // The compositor coordinates with the client window to create the final
        // rendered page and display it somewhere.
        let shutdown_state = Rc::new(Cell::new(ShutdownState::NotShuttingDown));

        let state = InitialCompositorState {
            sender: compositor_proxy,
            receiver: compositor_receiver,
            constellation_chan: constellation_chan.clone(),
            time_profiler_chan,
            mem_profiler_chan,
            webrender,
            webrender_document,
            webrender_api,
            rendering_context: rendering_context.clone(),
            webrender_gl: webrender_gl.clone(),
            shutdown_state: shutdown_state.clone(),
            event_loop_waker,
        };

        // The compositor coordinates with the client window to create the final
        // rendered page and display it somewhere.
        let io_compositor = IOCompositor::new(state, false);

        let compositor = Rc::new(RefCell::new(io_compositor));

        let constellation_proxy = ConstellationProxy::new(constellation_chan.clone());

        Self {
            rendering_context: rendering_context.clone(),
            compositor,
            constellation_proxy,
            embedder_receiver,
            webview: Rc::new(RefCell::new(None)),
            shutdown_state,
            webrender_gl: webrender_gl.clone(),
        }
    }

    pub fn deinit(self) {
        self.compositor.borrow_mut().deinit();
    }

    fn send_new_frame_ready_messages(&self, window: &Window) {
        if !self.compositor.borrow().needs_repaint() {
            return;
        }

        window.request_redraw();
    }

    pub fn create_webview(&self, window: &Window) -> WebView {
        let url = Url::parse("https://example.com/").unwrap();

        let id = WebViewId::new();

        let hidpi_scale_factor = Scale::new(window.scale_factor());

        let size = self.rendering_context.size2d().to_f32() / hidpi_scale_factor;

        let viewport_details = ViewportDetails {
            size,
            hidpi_scale_factor,
        };

        let webview = WebView {
            id,
            compositor: self.compositor.clone(),
            constellation_proxy: self.constellation_proxy.clone(),
        };

        *self.webview.borrow_mut() = Some(webview.clone());

        let webview_box = Box::new(webview.clone());

        self.compositor
            .borrow_mut()
            .add_webview(webview_box, viewport_details);

        self.constellation_proxy
            .send(EmbedderToConstellationMessage::NewWebView(
                url.into(),
                id,
                viewport_details,
            ));

        webview.focus();
        webview.raise_to_top(true);

        return webview;
    }

    /// Spin the Servo event loop, which:
    ///
    ///   - Performs updates in the compositor, such as queued pinch zoom events
    ///   - Runs delebgate methods on all `WebView`s and `Servo` itself
    ///   - Maybe update the rendered compositor output, but *without* swapping buffers.
    ///
    /// The return value of this method indicates whether or not Servo, false indicates that Servo
    /// has finished shutting down and you should not spin the event loop any longer.
    pub fn spin_event_loop(&self, window: &Window) -> bool {
        if self.shutdown_state.get() == ShutdownState::FinishedShuttingDown {
            return false;
        }

        // Handle compositor messages
        {
            let mut compositor = self.compositor.borrow_mut();
            let mut messages = Vec::new();
            while let Ok(message) = compositor.receiver().try_recv() {
                messages.push(message);
            }
            compositor.handle_messages(messages);
        }

        // Handle embedder messages
        // Only handle incoming embedder messages if the compositor hasn't already started shutting down.
        while let Ok(message) = self.embedder_receiver.try_recv() {
            self.handle_embedder_message(message);

            if self.shutdown_state.get() == ShutdownState::FinishedShuttingDown {
                break;
            }
        }

        if self.constellation_proxy.disconnected() {
            warn!("Lost connection to constellation");
            return false;
        }

        self.compositor.borrow_mut().perform_updates();
        self.send_new_frame_ready_messages(window);

        if self.shutdown_state.get() == ShutdownState::FinishedShuttingDown {
            return false;
        }

        return true;
    }

    fn handle_embedder_message(&self, message: EmbedderMsg) {
        match message {
            EmbedderMsg::ShutdownComplete => {
                debug!("Servo received message that Constellation shutdown is complete");
                self.shutdown_state.set(ShutdownState::FinishedShuttingDown);
                self.compositor.borrow_mut().finish_shutting_down();
            }
            _ => {}
        }
    }

    pub fn get_rendered_image(&self) -> Option<slint::Image> {
        let size = self.rendering_context.size2d().to_i32();
        let rect = DeviceIntRect::from_origin_and_size(Point2D::origin(), size);

        let image = self.rendering_context.read_to_image(rect).unwrap();

        let image_size = image.dimensions();

        println!("{:?}", image_size);

        let output_path = "./output.png";

        let image_format = ImageFormat::from_path(output_path).unwrap_or(ImageFormat::Png);

        DynamicImage::ImageRgba8(image).save_with_format(output_path, image_format);

        None
    }
}

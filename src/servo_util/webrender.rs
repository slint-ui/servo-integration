use std::rc::Rc;

use compositing_traits::{CompositorMsg, CompositorProxy};

use servo::{gl::{Gl, RENDERER}, RenderingContext};
use servo::config::pref;

use webrender::{
    ONE_TIME_USAGE_HINT, RenderApi, RenderApiSender, Renderer, ShaderPrecacheFlags, UploadMethod,
};
use webrender_api::{ColorF, DocumentId, FramePublishId, FrameReadyParams};

#[derive(Clone)]
struct RenderNotifier {
    compositor_proxy: CompositorProxy,
}

impl RenderNotifier {
    pub fn new(compositor_proxy: CompositorProxy) -> RenderNotifier {
        RenderNotifier { compositor_proxy }
    }
}

impl webrender_api::RenderNotifier for RenderNotifier {
    fn clone(&self) -> Box<dyn webrender_api::RenderNotifier> {
        Box::new(RenderNotifier::new(self.compositor_proxy.clone()))
    }

    fn wake_up(&self, _composite_needed: bool) {}

    fn new_frame_ready(
        &self,
        document_id: DocumentId,
        _: FramePublishId,
        frame_ready_params: &FrameReadyParams,
    ) {
        self.compositor_proxy
            .send(CompositorMsg::NewWebRenderFrameReady(
                document_id,
                frame_ready_params.render,
            ));
    }
}

pub fn get_webrender(
    rendering_context: Rc<dyn RenderingContext>,
    compositor_proxy: CompositorProxy,
    webrender_gl: Rc<dyn Gl>,
) -> (Renderer, RenderApiSender, RenderApi, DocumentId) {
    let debug_flags = webrender::DebugFlags::empty();

    rendering_context.prepare_for_rendering();
    let render_notifier = Box::new(RenderNotifier::new(compositor_proxy.clone()));
    let clear_color = pref!(shell_background_color_rgba);
    let clear_color = ColorF::new(
        clear_color[0] as f32,
        clear_color[1] as f32,
        clear_color[2] as f32,
        clear_color[3] as f32,
    );

    // Use same texture upload method as Gecko with ANGLE:
    // https://searchfox.org/mozilla-central/source/gfx/webrender_bindings/src/bindings.rs#1215-1219
    let upload_method = if webrender_gl.get_string(RENDERER).starts_with("ANGLE") {
        UploadMethod::Immediate
    } else {
        UploadMethod::PixelBuffer(ONE_TIME_USAGE_HINT)
    };

    let (webrender, webrender_api_sender) = webrender::create_webrender_instance(
        webrender_gl.clone(),
        render_notifier,
        webrender::WebRenderOptions {
            // We force the use of optimized shaders here because rendering is broken
            // on Android emulators with unoptimized shaders. This is due to a known
            // issue in the emulator's OpenGL emulation layer.
            // See: https://github.com/servo/servo/issues/31726
            use_optimized_shaders: true,
            resource_override_path: None,
            debug_flags,
            precache_flags: if pref!(gfx_precache_shaders) {
                ShaderPrecacheFlags::FULL_COMPILE
            } else {
                ShaderPrecacheFlags::empty()
            },
            enable_aa: pref!(gfx_text_antialiasing_enabled),
            enable_subpixel_aa: pref!(gfx_subpixel_text_antialiasing_enabled),
            allow_texture_swizzling: pref!(gfx_texture_swizzling_enabled),
            clear_color,
            upload_method,
            size_of_op: Some(servo_allocator::usable_size),
            ..Default::default()
        },
        None,
    )
    .expect("Unable to initialize webrender!");

    let webrender_api = webrender_api_sender.create_api();
    let webrender_document = webrender_api.add_document(rendering_context.size2d().to_i32());

    return (
        webrender,
        webrender_api_sender,
        webrender_api,
        webrender_document,
    );
}
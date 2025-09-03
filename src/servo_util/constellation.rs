use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use log::warn;
use layout::LayoutFactoryImpl;
use crossbeam_channel::{SendError, Sender};

use compositing_traits::{CompositorProxy, WebrenderExternalImageRegistry};

use webrender::RenderApiSender;
use webrender_api::DocumentId;

use constellation::{Constellation, InitialConstellationState};
use constellation_traits::EmbedderToConstellationMessage;

use servo::EmbedderProxy;
use servo::fonts::SystemFontService;
use servo::net::protocols::ProtocolRegistry;
use servo::net::resource_thread::new_resource_threads;
use servo::profile_traits::{mem, time};
use servo::script::{ScriptThread, ServiceWorkerManager};
use servo::user_content_manager::UserContentManager;

#[derive(Clone)]
pub struct ConstellationProxy {
    sender: Sender<EmbedderToConstellationMessage>,
    disconnected: Arc<AtomicBool>,
}

impl ConstellationProxy {
    pub fn new(sender: Sender<EmbedderToConstellationMessage>) -> Self {
        Self {
            sender,
            disconnected: Arc::default(),
        }
    }

    pub fn disconnected(&self) -> bool {
        self.disconnected.load(Ordering::SeqCst)
    }

    pub fn send(&self, msg: EmbedderToConstellationMessage) {
        if self.try_send(msg).is_err() {
            warn!("Lost connection to Constellation. Will report to embedder.")
        }
    }

    fn try_send(
        &self,
        msg: EmbedderToConstellationMessage,
    ) -> Result<(), SendError<EmbedderToConstellationMessage>> {
        if self.disconnected() {
            return Err(SendError(msg));
        }
        if let Err(error) = self.sender.send(msg) {
            self.disconnected.store(true, Ordering::SeqCst);
            return Err(error);
        }

        Ok(())
    }

    pub fn sender(&self) -> Sender<EmbedderToConstellationMessage> {
        self.sender.clone()
    }
}

pub fn create_constellation(
    embedder_proxy: EmbedderProxy,
    compositor_proxy: CompositorProxy,
    time_profiler_chan: time::ProfilerChan,
    mem_profiler_chan: mem::ProfilerChan,
    webrender_document: DocumentId,
    webrender_api_sender: RenderApiSender,
    external_images: Arc<Mutex<WebrenderExternalImageRegistry>>,
    protocols: ProtocolRegistry,
    user_content_manager: UserContentManager,
) -> Sender<EmbedderToConstellationMessage> {
    let (public_resource_threads, private_resource_threads) = new_resource_threads(
        None,
        time_profiler_chan.clone(),
        mem_profiler_chan.clone(),
        embedder_proxy.clone(),
        None,
        None,
        true,
        Arc::new(protocols),
    );

    let system_font_service = Arc::new(
        SystemFontService::spawn(
            compositor_proxy.cross_process_compositor_api.clone(),
            mem_profiler_chan.clone(),
        )
        .to_proxy(),
    );

    let initial_state = InitialConstellationState {
        compositor_proxy,
        embedder_proxy,
        devtools_sender: None,
        system_font_service,
        public_resource_threads,
        private_resource_threads,
        time_profiler_chan,
        mem_profiler_chan,
        webrender_document,
        webrender_api_sender,
        webxr_registry: None,
        webgl_threads: None,
        webrender_external_images: external_images,
        user_content_manager,
    };

    let layout_factory = Arc::new(LayoutFactoryImpl());

    Constellation::<ScriptThread, ServiceWorkerManager>::start(
        initial_state,
        layout_factory,
        None,
        None,
        false,
    )
}

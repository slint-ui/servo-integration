
use crossbeam_channel::{Receiver, unbounded};

use compositing_traits::{CompositorMsg, CompositorProxy, CrossProcessCompositorApi};

use servo::{
    EventLoopWaker,
    ipc_channel::{ipc, router::ROUTER},
};

pub fn create_compositor_channel(
    event_loop_waker: Box<dyn EventLoopWaker>,
) -> (CompositorProxy, Receiver<CompositorMsg>) {
    let (sender, receiver) = unbounded();

    let (compositor_ipc_sender, compositor_ipc_receiver) =
        ipc::channel().expect("ipc channel failure");

    let cross_process_compositor_api = CrossProcessCompositorApi(compositor_ipc_sender);
    let compositor_proxy = CompositorProxy {
        sender,
        cross_process_compositor_api,
        event_loop_waker,
    };

    let compositor_proxy_clone = compositor_proxy.clone();
    ROUTER.add_typed_route(
        compositor_ipc_receiver,
        Box::new(move |message| {
            compositor_proxy_clone.send(message.expect("Could not convert Compositor message"));
        }),
    );

    (compositor_proxy, receiver)
}

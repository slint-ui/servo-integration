use crossbeam_channel::{unbounded, Receiver};

use servo::{EmbedderMsg, EmbedderProxy, EventLoopWaker};

pub fn create_embedder_channel(
    event_loop_waker: Box<dyn EventLoopWaker>,
) -> (EmbedderProxy, Receiver<EmbedderMsg>) {
    let (sender, receiver) = unbounded();
    (
        EmbedderProxy {
            sender,
            event_loop_waker,
        },
        receiver,
    )
}
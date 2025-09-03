use i_slint_backend_winit::SlintEvent;
use i_slint_backend_winit::CustomEvent;
use log::warn;
use servo::EventLoopWaker;
use winit::event_loop::EventLoopProxy;

#[derive(Debug, Clone)]
pub struct Waker(pub EventLoopProxy<SlintEvent>);

impl EventLoopWaker for Waker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Self(self.0.clone()))
    }

    fn wake(&self) {

        let event = SlintEvent(CustomEvent::UserEvent(Box::new(|| {})));

        if let Err(error) = self.0.send_event(event) {
            warn!("{} Failed to wake event loop", error);
        }
    }
}

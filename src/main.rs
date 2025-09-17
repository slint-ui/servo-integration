mod delegate;
mod on_events;
mod rendering_context;
mod servo_util;
mod state;
mod waker;

use smol::channel;
use std::{cell::RefCell, rc::Rc};

use slint::ComponentHandle;

use crate::{
    on_events::{on_pointer_event, on_scroll_event},
    servo_util::{init_servo_webview, spin_servo_event_loop},
    state::State,
};

slint::include_modules!();

fn main() {
    let url_string = "https://slint.dev/";

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state_placeholder = Rc::new(RefCell::new(None));

    let app = MyApp::new().unwrap();

    let state = Rc::new(State::new(app));

    // Update the placeholder with the actual state
    *state_placeholder.borrow_mut() = Some(state.clone());

    init_servo_webview(url_string.to_string(), state.clone(), waker_sender);

    spin_servo_event_loop(state.clone(), waker_receiver);

    on_scroll_event(state.clone());

    on_pointer_event(state.clone());

    state.app.run().unwrap();
}

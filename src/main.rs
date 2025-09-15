mod application_handler;
mod delegate;
mod on_events;
mod servo_util;
mod state;
mod waker;
mod rendering_context;

use smol::channel;
use std::{cell::RefCell, rc::Rc};

use slint::{BackendSelector, ComponentHandle};

use crate::{
    on_events::{on_pointer_event, on_scroll_event},
    servo_util::{init_servo_webview, spin_servo_event_loop},
    state::State,
    application_handler::ApplicationHandler,
};

slint::include_modules!();

fn main() {
    let url_string = "https://slint.dev/";

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state_placeholder = Rc::new(RefCell::new(None));

    let application_handler = ApplicationHandler::new(state_placeholder.clone());

    BackendSelector::new()
        .with_winit_custom_application_handler(application_handler)
        .select()
        .unwrap();

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

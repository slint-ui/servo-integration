use std::{cell::RefCell, rc::Rc};

use termcolor::Color;

use slint::winit_030::{CustomApplicationHandler, EventResult};

use crate::{on_events::print_time, state::State};

pub struct ApplicationHandler {
    pub state: Rc<RefCell<Option<Rc<State>>>>,
}

impl ApplicationHandler {
    pub fn new(state: Rc<RefCell<Option<Rc<State>>>>) -> Self {
        Self { state }
    }
}

impl CustomApplicationHandler for ApplicationHandler {
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        winit_window: Option<&winit::window::Window>,
        slint_window: Option<&slint::Window>,
        event: &winit::event::WindowEvent,
    ) -> EventResult {
        let state = self.state.borrow();
        let state = state.as_ref().unwrap();

        let _ = state.waker_sender.try_send(());
        print_time("Window event wake sent", Color::Yellow);

        return EventResult::Propagate;
    }
}

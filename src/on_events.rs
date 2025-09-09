use std::rc::Rc;

use euclid::Vector2D;
use webrender_api::{ScrollLocation, units::DeviceIntPoint};

use crate::{pointer_event::convert_slint_pointer_event_to_servo_input_event, state::State};

pub fn on_scroll_event(state: Rc<State>) {
    let state_weak = Rc::downgrade(&state);
    state.app.on_scroll_event(move |dx, dy| {
        let state = state_weak.upgrade().unwrap();

        let webview_ref = state.webview.borrow();
        let webview = webview_ref.as_ref().unwrap();

        let point = DeviceIntPoint::new(0, 0);
        let moved_by = Vector2D::new(-dx as f32, -dy as f32);

        webview.notify_scroll_event(ScrollLocation::Delta(moved_by), point);
    });
}

pub fn on_pointer_event(state: Rc<State>) {
    let state_weak = Rc::downgrade(&state);
    state.app.on_pointer_event(move |event| {
        let state = state_weak.upgrade().unwrap();

        let webview_ref = state.webview.borrow();
        let webview = webview_ref.as_ref().unwrap();

        let event_str = format!("{:?}", event);
        // println!("Pointer event: {}", event_str);

        let mouse_x = state.app.get_mouse_x();
        let mouse_y = state.app.get_mouse_y();
        let scale_factor = *state.scale_factor.borrow();

        let input_event = convert_slint_pointer_event_to_servo_input_event(
            &event_str,
            mouse_x,
            mouse_y,
            scale_factor,
        );

        webview.notify_input_event(input_event);
    });
}

use std::rc::Rc;

use euclid::{Point2D, Vector2D};

use webrender_api::ScrollLocation;

use servo::{InputEvent, MouseButton, MouseButtonAction, MouseButtonEvent, MouseMoveEvent};

use crate::state::State;

pub fn on_scroll_event(state: Rc<State>) {
    let state_weak = Rc::downgrade(&state);
    state.app.on_scroll_event(move |dx, dy| {
        let state = state_weak.upgrade().unwrap();

        let webview_ref = state.webview.borrow();
        let webview = webview_ref.as_ref().unwrap();

        let x = state.app.get_mouse_x();
        let y = state.app.get_mouse_y();

        println!("dx:{dx:?} dy:{dy:?}");

        let point = Point2D::new(x, y);

        let moved_by = Vector2D::new(-dx, -dy);
        webview.notify_scroll_event(ScrollLocation::Delta(moved_by), point.to_i32());
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

        let input_event =
            convert_slint_pointer_event_to_servo_input_event(&event_str, mouse_x, mouse_y);

        webview.notify_input_event(input_event);
    });
}

pub fn convert_slint_pointer_event_to_servo_input_event(
    event_str: &str,
    mouse_x: f32,
    mouse_y: f32,
) -> InputEvent {
    let point = Point2D::new(mouse_x, mouse_y);

    if event_str.contains("kind: Down") {
        let button = get_mouse_button(event_str);
        return InputEvent::MouseButton(MouseButtonEvent::new(
            MouseButtonAction::Down,
            button,
            point,
        ));
    } else if event_str.contains("kind: Up") {
        let button = get_mouse_button(event_str);
        return InputEvent::MouseButton(MouseButtonEvent::new(
            MouseButtonAction::Up,
            button,
            point,
        ));
    } else {
        return InputEvent::MouseMove(MouseMoveEvent::new(point));
    }
}

fn get_mouse_button(event_str: &str) -> MouseButton {
    if event_str.contains("button: Left") {
        return MouseButton::Left;
    } else if event_str.contains("button: Right") {
        return MouseButton::Right;
    } else if event_str.contains("button: Middle") {
        return MouseButton::Middle;
    } else {
        return MouseButton::Left;
    }
}

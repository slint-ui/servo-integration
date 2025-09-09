use webrender_api::units::DevicePoint;

use servo::{InputEvent, MouseButton, MouseButtonAction, MouseButtonEvent, MouseMoveEvent};

pub fn convert_slint_pointer_event_to_servo_input_event(
    event_str: &str,
    mouse_x: f32,
    mouse_y: f32,
    scale_factor: f32,
) -> InputEvent {
    let point = DevicePoint::new(mouse_x * scale_factor, mouse_y * scale_factor);

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

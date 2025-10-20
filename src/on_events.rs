use std::rc::Rc;

use euclid::{Box2D, Point2D, Size2D, Vector2D};

use servo::{
    InputEvent, MouseButton, MouseButtonAction, MouseButtonEvent, MouseMoveEvent,
    webrender_api::{
        ScrollLocation,
        units::{DevicePixel, DevicePoint},
    },
};
use slint::ComponentHandle;
use winit::dpi::PhysicalSize;

use crate::{WebviewLogic, adapter::SlintServoAdapter};

pub fn on_resize(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    let app = state
        .app
        .upgrade()
        .expect("Failed to upgrade app weak reference");

    app.global::<WebviewLogic>()
        .on_resize(move |width, height| {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in servo init");

            println!("physical_size {}, {}", width, height);

            let webview = state.webview.borrow();
            let webview = webview
                .as_ref()
                .expect("Webview not initialized for scroll event");

            let scale_factor = state.scale_factor.borrow();
            let scale_factor = *scale_factor;

            let size = Size2D::new(width, height) * scale_factor;

            let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);

            let rect: Box2D<f32, DevicePixel> =
                Box2D::from_origin_and_size(Point2D::origin(), size);

            webview.move_resize(rect);
            webview.resize(physical_size);
        });
}

pub fn on_buttons(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    let app = state
        .app
        .upgrade()
        .expect("Failed to upgrade app weak reference");

    app.on_back(move || {
        let state = state_weak
            .upgrade()
            .expect("Failed to upgrade state weak reference in scroll event");

        let webview = state.webview.borrow();
        let webview = webview
            .as_ref()
            .expect("Webview not initialized for scroll event");

        webview.go_back(1);
    });

    let state_weak = Rc::downgrade(&state);
    app.on_forward(move || {
        let state = state_weak
            .upgrade()
            .expect("Failed to upgrade state weak reference in scroll event");

        let webview = state.webview.borrow();
        let webview = webview
            .as_ref()
            .expect("Webview not initialized for scroll event");

        webview.go_forward(1);
    });

    let state_weak = Rc::downgrade(&state);
    app.on_reload(move || {
        let state = state_weak
            .upgrade()
            .expect("Failed to upgrade state weak reference in scroll event");

        let webview = state.webview.borrow();
        let webview = webview
            .as_ref()
            .expect("Webview not initialized for scroll event");

        webview.reload();
    });
}

pub fn on_scroll_event(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    let app = state
        .app
        .upgrade()
        .expect("Failed to upgrade app weak reference");

    app.global::<WebviewLogic>()
        .on_scroll_event(move |dx, dy, mouse_x, mouse_y| {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in scroll event");

            let webview = state.webview.borrow();
            let webview = webview
                .as_ref()
                .expect("Webview not initialized for scroll event");

            let scale_factor = *state.scale_factor.borrow();

            let point = DevicePoint::new(mouse_x * scale_factor, mouse_y * scale_factor);

            let moved_by = Vector2D::new(dx, dy);
            let servo_delta = -moved_by;

            webview.notify_scroll_event(ScrollLocation::Delta(servo_delta), point.to_i32());
        });
}

pub fn on_pointer_event(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    let app = state
        .app
        .upgrade()
        .expect("Failed to upgrade app weak reference for pointer events");

    app.global::<WebviewLogic>()
        .on_pointer_event(move |event, mouse_x, mouse_y| {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in pointer event");

            let webview = state.webview.borrow();
            let webview = webview
                .as_ref()
                .expect("Webview not initialized for pointer event");

            let scale_factor = *state.scale_factor.borrow();

            let event_str = format!("{:?}", event);

            let point = DevicePoint::new(mouse_x * scale_factor, mouse_y * scale_factor);

            let input_event = convert_slint_pointer_event_to_servo_input_event(&event_str, point);

            webview.notify_input_event(input_event);
        });
}

fn convert_slint_pointer_event_to_servo_input_event(
    event_str: &str,
    point: DevicePoint,
) -> InputEvent {
    let button = get_mouse_button(event_str);

    if event_str.contains("kind: Down") {
        InputEvent::MouseButton(MouseButtonEvent::new(
            MouseButtonAction::Down,
            button,
            point,
        ))
    } else if event_str.contains("kind: Up") {
        InputEvent::MouseButton(MouseButtonEvent::new(MouseButtonAction::Up, button, point))
    } else {
        InputEvent::MouseMove(MouseMoveEvent::new(point))
    }
}

fn get_mouse_button(event_str: &str) -> MouseButton {
    if event_str.contains("button: Left") {
        MouseButton::Left
    } else if event_str.contains("button: Right") {
        MouseButton::Right
    } else if event_str.contains("button: Middle") {
        MouseButton::Middle
    } else {
        MouseButton::Left
    }
}

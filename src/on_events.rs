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

use crate::{
    WebviewLogic,
    adapter::{SlintServoAdapter, upgrade_adapter},
};

pub fn on_resize(adapter: Rc<SlintServoAdapter>) {
    let adpater_weak = Rc::downgrade(&adapter);

    let app = adapter.app();

    app.global::<WebviewLogic>()
        .on_resize(move |width, height| {
            let adapter = upgrade_adapter(&adpater_weak);

            if let Some(webview) = adapter.try_get_webview() {
                let scale_factor = adapter.scale_factor();

                let size = Size2D::new(width, height) * scale_factor;

                let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);

                let rect: Box2D<f32, DevicePixel> =
                    Box2D::from_origin_and_size(Point2D::origin(), size);

                webview.move_resize(rect);
                webview.resize(physical_size);
            }
        });
}

pub fn on_buttons(adapter: Rc<SlintServoAdapter>) {
    let app = adapter.app();

    let adapter_weak = Rc::downgrade(&adapter);
    app.on_back(move || {
        let adapter = upgrade_adapter(&adapter_weak);

        let webview = adapter.webview();

        webview.go_back(1);
    });

    let adapter_weak = Rc::downgrade(&adapter);
    app.on_forward(move || {
        let adapter = upgrade_adapter(&adapter_weak);

        let webview = adapter.webview();

        webview.go_forward(1);
    });

    let adapter_weak = Rc::downgrade(&adapter);
    app.on_reload(move || {
        let adapter = upgrade_adapter(&adapter_weak);

        let webview = adapter.webview();

        webview.reload();
    });
}

pub fn on_scroll_event(adapter: Rc<SlintServoAdapter>) {
    let adapter_weak = Rc::downgrade(&adapter);

    let app = adapter.app();

    app.global::<WebviewLogic>()
        .on_scroll_event(move |dx, dy, mouse_x, mouse_y| {
            let adapter = upgrade_adapter(&adapter_weak);

            let webview = adapter.webview();

            let scale_factor = adapter.scale_factor();

            let point = DevicePoint::new(mouse_x * scale_factor, mouse_y * scale_factor);

            let moved_by = Vector2D::new(dx, dy);
            let servo_delta = -moved_by;

            webview.notify_scroll_event(ScrollLocation::Delta(servo_delta), point.to_i32());
        });
}

pub fn on_pointer_event(adapter: Rc<SlintServoAdapter>) {
    let adapter_weak = Rc::downgrade(&adapter);

    let app = adapter.app();

    app.global::<WebviewLogic>()
        .on_pointer_event(move |event, mouse_x, mouse_y| {
            let adapter = upgrade_adapter(&adapter_weak);

            let webview = adapter.webview();

            let scale_factor = adapter.scale_factor();

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

use std::rc::Rc;

use euclid::{Box2D, Point2D, Size2D, Vector2D};

use servo::{
    InputEvent, MouseButton, MouseButtonAction, MouseButtonEvent, MouseMoveEvent, TouchEvent,
    TouchEventType, TouchId,
    webrender_api::{
        ScrollLocation,
        units::{DevicePixel, DevicePoint},
    },
};
use slint::ComponentHandle;
use url::Url;
use winit::dpi::PhysicalSize;

use crate::{
    WebviewLogic,
    adapter::{SlintServoAdapter, upgrade_adapter},
};

pub fn on_app_callbacks(adapter: Rc<SlintServoAdapter>) {
    on_resize(adapter.clone());
    on_buttons(adapter.clone());
    on_scroll(adapter.clone());
    on_pointer(adapter.clone());
}

fn on_buttons(adapter: Rc<SlintServoAdapter>) {
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

    let adpater_weak = Rc::downgrade(&adapter);
    app.on_go(move |url| {
        let adapter = upgrade_adapter(&adpater_weak);
        let webview = adapter.webview();
        let url = Url::parse(url.as_str()).expect("Failed to parse url");
        webview.load(url);
    });
}

fn on_resize(adapter: Rc<SlintServoAdapter>) {
    let app = adapter.app();

    let adapter_weak = Rc::downgrade(&adapter);
    app.global::<WebviewLogic>()
        .on_resize(move |width, height| {
            let adapter = upgrade_adapter(&adapter_weak);

            let webview = adapter.webview();

            let scale_factor = adapter.scale_factor();

            let size = Size2D::new(width, height) * scale_factor;

            let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);

            let rect: Box2D<f32, DevicePixel> =
                Box2D::from_origin_and_size(Point2D::origin(), size);

            webview.move_resize(rect);
            webview.resize(physical_size);
        });
}

fn on_scroll(adapter: Rc<SlintServoAdapter>) {
    let app = adapter.app();

    let adapter_weak = Rc::downgrade(&adapter);
    app.global::<WebviewLogic>()
        .on_scroll(move |initial_x, initial_y, delta_x, delta_y| {
            let adapter = upgrade_adapter(&adapter_weak);

            println!(
                "Scroll event initial_x:{} initial_y:{} delta_x:{} delta_y:{}",
                initial_x, initial_y, delta_x, delta_y
            );

            let webview = adapter.webview();

            let scale_factor = adapter.scale_factor();

            let point = DevicePoint::new(initial_x * scale_factor, initial_y * scale_factor);

            let moved_by = Vector2D::new(delta_x, delta_y);
            let servo_delta = -moved_by;

            webview.notify_scroll_event(ScrollLocation::Delta(servo_delta), point.to_i32());
        });
}

fn on_pointer(adapter: Rc<SlintServoAdapter>) {
    let app = adapter.app();

    let adapter_weak = Rc::downgrade(&adapter);
    app.global::<WebviewLogic>().on_pointer(move |event, x, y| {
        let event_str = format!("{:?}", event);

        println!("Pointer event event:{} x:{} y:{}", event_str, x, y);

        let adapter = upgrade_adapter(&adapter_weak);

        let webview = adapter.webview();

        let scale_factor = adapter.scale_factor();

        let event_str = format!("{:?}", event);

        let point = DevicePoint::new(x * scale_factor, y * scale_factor);

        let input_event = if cfg!(target_os = "android") {
            android_convert_slint_pointer_event_to_servo_input_event(&event_str, point)
        } else {
            convert_slint_pointer_event_to_servo_input_event(&event_str, point)
        };

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

fn android_convert_slint_pointer_event_to_servo_input_event(
    event_str: &str,
    point: DevicePoint,
) -> InputEvent {
    let touch_id = TouchId(1);

    if event_str.contains("kind: Down") {
        let touch_event = TouchEvent::new(TouchEventType::Down, touch_id, point);
        InputEvent::Touch(touch_event)
    } else if event_str.contains("kind: Up") {
        let touch_event = TouchEvent::new(TouchEventType::Up, touch_id, point);
        InputEvent::Touch(touch_event)
    } else {
        let touch_event = TouchEvent::new(TouchEventType::Move, touch_id, point);
        InputEvent::Touch(touch_event)
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

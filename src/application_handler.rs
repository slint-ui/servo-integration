use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use euclid::{Point2D, Scale, Vector2D};

use winit::{
    dpi::{LogicalPosition, PhysicalPosition},
    event::{ElementState, MouseScrollDelta, WindowEvent},
};

use slint::winit_030::{CustomApplicationHandler, EventResult};

use servo::{
    InputEvent, MouseButtonAction, MouseButtonEvent, MouseLeftViewportEvent, MouseMoveEvent,
    WebView, WheelDelta, WheelEvent, WheelMode, servo_geometry::DeviceIndependentPixel,
};

use webrender_api::{
    ScrollLocation,
    units::{DevicePixel, DevicePoint},
};

use crate::state::State;

// This should vary by zoom level and maybe actual text size (focused or under cursor)
pub(crate) const LINE_HEIGHT: f32 = 76.0;
pub(crate) const LINE_WIDTH: f32 = 76.0;

// MouseScrollDelta::PixelDelta is default for MacOS, which is high precision and very slow
// in winit. Therefore we use a factor of 4.0 to make it more usable.
// See https://github.com/servo/servo/pull/34063#discussion_r2197729507
pub(crate) const PIXEL_DELTA_FACTOR: f64 = 4.0;

pub struct ApplicationHandler {
    pub state: Rc<RefCell<Option<Rc<State>>>>,
    pub webview_relative_mouse_point: Cell<DevicePoint>,
}

impl ApplicationHandler {
    pub fn new(state: Rc<RefCell<Option<Rc<State>>>>) -> Self {
        Self {
            state,
            webview_relative_mouse_point: Cell::new(Point2D::zero()),
        }
    }

    fn get_webview(&self) -> WebView {
        let state_borrow = self.state.borrow();
        let state = state_borrow.as_ref().unwrap();
        let webview_borrow = state.webview.borrow();
        let webview = webview_borrow.as_ref().unwrap();
        return webview.clone();
    }

    fn get_scale_factor(&self) -> f32 {
        let state_borrow = self.state.borrow();
        let state = state_borrow.as_ref().unwrap();
        let scale_factor = state.scale_factor.borrow();
        return *scale_factor;
    }

    fn device_hidpi_scale_factor(&self) -> Scale<f32, DeviceIndependentPixel, DevicePixel> {
        let scale_factor = self.get_scale_factor();
        Scale::new(scale_factor)
    }

    fn handle_mouse(&self, button: &servo::MouseButton, action: &ElementState) {
        let webview = self.get_webview();

        let point = self.webview_relative_mouse_point.get();
        // `point` can be outside viewport, such as at toolbar with negative y-coordinate.
        if !webview.rect().contains(point) {
            return;
        }
        let action = match action {
            ElementState::Pressed => MouseButtonAction::Down,
            ElementState::Released => MouseButtonAction::Up,
        };

        webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
            action, *button, point,
        )));
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
        // println!("{:?}", event);
        match event {
            WindowEvent::MouseInput { state, button, .. } => {
                let servo_button = winit_mouse_button_to_servo(*button);
                self.handle_mouse(&servo_button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut point = winit_position_to_euclid_point(*position).to_f32();
                // point.y -= (self.toolbar_height() * self.hidpi_scale_factor()).0;

                let webview = self.get_webview();

                let previous_point = self.webview_relative_mouse_point.get();
                if webview.rect().contains(point) {
                    webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(point)));
                } else if webview.rect().contains(previous_point) {
                    webview.notify_input_event(InputEvent::MouseLeftViewport(
                        MouseLeftViewportEvent::default(),
                    ));
                }

                self.webview_relative_mouse_point.set(point);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (mut dx, mut dy, mode) = match delta {
                    MouseScrollDelta::LineDelta(dx, dy) => (
                        (dx * LINE_WIDTH) as f64,
                        (dy * LINE_HEIGHT) as f64,
                        WheelMode::DeltaLine,
                    ),
                    MouseScrollDelta::PixelDelta(position) => {
                        let position: LogicalPosition<f64> =
                            position.to_logical(self.device_hidpi_scale_factor().get() as f64);
                        (
                            position.x * PIXEL_DELTA_FACTOR,
                            position.y * PIXEL_DELTA_FACTOR,
                            WheelMode::DeltaPixel,
                        )
                    }
                };

                // Create wheel event before snapping to the major axis of movement
                let delta = WheelDelta {
                    x: dx,
                    y: dy,
                    z: 0.0,
                    mode,
                };
                let point = self.webview_relative_mouse_point.get();

                // Scroll events snap to the major axis of movement, with vertical
                // preferred over horizontal.
                if dy.abs() >= dx.abs() {
                    dx = 0.0;
                } else {
                    dy = 0.0;
                }

                let webview = self.get_webview();

                // Send events
                webview.notify_input_event(InputEvent::Wheel(WheelEvent::new(delta, point)));

                let scroll_location = ScrollLocation::Delta(-Vector2D::new(dx as f32, dy as f32));

                webview.notify_scroll_event(scroll_location, point.to_i32());
            }
            _ => (),
        }
        return EventResult::Propagate;
    }
}

pub fn winit_position_to_euclid_point<T>(position: PhysicalPosition<T>) -> Point2D<T, DevicePixel> {
    Point2D::new(position.x, position.y)
}

// Convert winit MouseButton to servo MouseButton
fn winit_mouse_button_to_servo(button: winit::event::MouseButton) -> servo::MouseButton {
    match button {
        winit::event::MouseButton::Left => servo::MouseButton::Left,
        winit::event::MouseButton::Right => servo::MouseButton::Right,
        winit::event::MouseButton::Middle => servo::MouseButton::Middle,
        winit::event::MouseButton::Back => servo::MouseButton::Back,
        winit::event::MouseButton::Forward => servo::MouseButton::Forward,
        winit::event::MouseButton::Other(value) => servo::MouseButton::Other(value),
    }
}

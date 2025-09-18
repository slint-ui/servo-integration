mod delegate;
mod on_events;
mod rendering_context;
mod servo_util;
mod state;
mod waker;

use smol::channel;
use std::{cell::RefCell, rc::Rc};

use slint::{
    ComponentHandle,
    wgpu_26::{WGPUConfiguration, WGPUSettings, wgpu},
};

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

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_features = wgpu::Features::PUSH_CONSTANTS;
    wgpu_settings.device_required_limits.max_push_constant_size = 16;

    slint::BackendSelector::new()
        .require_wgpu_26(WGPUConfiguration::Automatic(wgpu_settings))
        .select()
        .unwrap();

    let app = MyApp::new().unwrap();

    let app_weak = app.as_weak();

    let state = Rc::new(State::new(app_weak));

    let state_weak = Rc::downgrade(&state);

    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            //eprintln!("rendering state {:#?} {:#?}", state, graphics_api);

            match state {
                slint::RenderingState::RenderingSetup => {
                    match graphics_api {
                        slint::GraphicsAPI::WGPU26 { device, queue, .. } => {
                            let state = state_weak.upgrade().unwrap();
                            *state.device.borrow_mut() = Some(device.clone());
                            *state.queue.borrow_mut() = Some(queue.clone());
                        }
                        _ => return,
                    };
                }
                slint::RenderingState::BeforeRendering => {}
                slint::RenderingState::AfterRendering => {}
                slint::RenderingState::RenderingTeardown => {}
                _ => {}
            }
        })
        .expect("Unable to set rendering notifier");

    // Update the placeholder with the actual state
    *state_placeholder.borrow_mut() = Some(state.clone());

    init_servo_webview(url_string.to_string(), state.clone(), waker_sender);

    spin_servo_event_loop(state.clone(), waker_receiver);

    on_scroll_event(state.clone());

    on_pointer_event(state.clone());

    app.run().unwrap();
}

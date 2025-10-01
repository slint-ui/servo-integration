mod constants;
mod delegate;
mod on_events;
mod rendering_context;
mod servo_util;
mod state;
mod waker;

mod gl_bindings {
    #![allow(unsafe_op_in_unsafe_fn)]

    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

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
    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state_placeholder = Rc::new(RefCell::new(None));

    let mut wgpu_settings = WGPUSettings::default();
    wgpu_settings.device_required_features = wgpu::Features::PUSH_CONSTANTS;
    wgpu_settings.device_required_limits.max_push_constant_size = constants::MAX_PUSH_CONSTANT_SIZE;

    slint::BackendSelector::new()
        .require_wgpu_26(WGPUConfiguration::Automatic(wgpu_settings))
        .select()
        .expect("Failed to create Slint backend with WGPU based renderer - ensure your system supports WGPU");

    let app = MyApp::new().expect("Failed to create Slint application - check UI resources");

    let app_weak = app.as_weak();

    let state = Rc::new(State::new(app_weak));

    let state_weak = Rc::downgrade(&state);

    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            //eprintln!("rendering state {:#?} {:#?}", state, graphics_api);

            match state {
                slint::RenderingState::RenderingSetup => {
                    if let slint::GraphicsAPI::WGPU26 { device, queue, .. } = graphics_api {
                        if let Some(state) = state_weak.upgrade() {
                            *state.device.borrow_mut() = Some(device.clone());
                            *state.queue.borrow_mut() = Some(queue.clone());
                            println!("WGPU device and queue initialized successfully");
                        }
                    }
                }
                slint::RenderingState::BeforeRendering => {}
                slint::RenderingState::AfterRendering => {}
                slint::RenderingState::RenderingTeardown => {}
                _ => {}
            }
        })
        .expect("Failed to set rendering notifier - WGPU integration may not be available");

    // Update the placeholder with the actual state
    *state_placeholder.borrow_mut() = Some(state.clone());

    init_servo_webview(state.clone(), waker_sender);

    spin_servo_event_loop(state.clone(), waker_receiver);

    on_scroll_event(state.clone());

    on_pointer_event(state.clone());

    app.run()
        .expect("Application failed to run - check for runtime errors");
}

mod adapter;
#[cfg(not(target_os = "android"))]
mod application_handler;
mod constants;
mod debug_helper;
mod delegate;
mod on_events;
mod rendering_context;
mod servo_util;
mod waker;

use smol::channel;
use std::{cell::RefCell, rc::Rc};

use slint::{
    ComponentHandle,
    wgpu_26::{WGPUConfiguration, WGPUSettings, wgpu},
};

use crate::{
    adapter::SlintServoAdapter,
    on_events::{on_pointer_event, on_scroll_event},
    servo_util::{init_servo_webview, spin_servo_event_loop},
};

#[cfg(not(target_os = "android"))]
use crate::application_handler::ApplicationHandler;

slint::include_modules!();

pub fn main() {
    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state_placeholder = Rc::new(RefCell::new(None));

    #[cfg(not(target_os = "android"))]
    {
        let application_handler = ApplicationHandler::new(state_placeholder.clone());

        let mut wgpu_settings = WGPUSettings::default();
        wgpu_settings.device_required_features = wgpu::Features::PUSH_CONSTANTS;
        wgpu_settings.device_required_limits.max_push_constant_size =
            constants::MAX_PUSH_CONSTANT_SIZE;

        slint::BackendSelector::new()
            .require_wgpu_26(WGPUConfiguration::Automatic(wgpu_settings))
            .with_winit_custom_application_handler(application_handler)
            .select()
            .expect("Failed to create Slint backend with WGPU based renderer - ensure your system supports WGPU");
    }

    #[cfg(target_os = "android")]
    {
        // For Android, use the default backend without winit
        slint::BackendSelector::new()
            .select()
            .expect("Failed to create Slint backend for Android");
    }

    let app = MyApp::new().expect("Failed to create Slint application - check UI resources");

    let app_weak = app.as_weak();

    let state = Rc::new(SlintServoAdapter::new(app_weak, waker_sender.clone()));

    let state_weak = Rc::downgrade(&state);

    app.window()
        .set_rendering_notifier(move |state, graphics_api| match state {
            slint::RenderingState::RenderingSetup => {
                #[cfg(not(target_os = "android"))]
                if let slint::GraphicsAPI::WGPU26 { device, queue, .. } = graphics_api {
                    if let Some(state) = state_weak.upgrade() {
                        *state.device.borrow_mut() = Some(device.clone());
                        *state.queue.borrow_mut() = Some(queue.clone());
                    }
                }
            }
            slint::RenderingState::BeforeRendering => {}
            slint::RenderingState::AfterRendering => {}
            slint::RenderingState::RenderingTeardown => {}
            _ => {}
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

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).unwrap();
    main();
}

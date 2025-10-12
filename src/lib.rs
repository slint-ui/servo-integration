mod adapter;
mod constants;
mod debug_helper;
mod delegate;
mod on_events;
mod rendering_context;
mod servo_util;
mod waker;

#[cfg(not(target_os = "android"))]
mod application_handler;

#[cfg(target_os = "linux")]
mod gl_bindings {
    #![allow(unsafe_op_in_unsafe_fn)]

    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

use slint::ComponentHandle;
use smol::channel;
use std::{cell::RefCell, rc::Rc};

use crate::{
    adapter::SlintServoAdapter,
    on_events::{on_pointer_event, on_scroll_event},
    servo_util::spin_servo_event_loop,
};

slint::include_modules!();

#[cfg(not(target_os = "android"))]
use {
    crate::application_handler::ApplicationHandler,
    crate::servo_util::init_servo_webview,
    slint::wgpu_27::{WGPUConfiguration, WGPUSettings, wgpu},
};

#[cfg(target_os = "android")]
use {
    crate::servo_util::android_init_servo_webview,
    i_slint_backend_android_activity::AndroidPlatform,
    i_slint_backend_android_activity::android_activity::MainEvent,
    i_slint_backend_android_activity::android_activity::PollEvent,
};

#[cfg(not(target_os = "android"))]
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
        .require_wgpu_27(WGPUConfiguration::Automatic(wgpu_settings))
        .with_winit_custom_application_handler(application_handler)
        .select()
        .expect("Failed to create Slint backend with WGPU based renderer - ensure your system supports WGPU");
    }

    let app = MyApp::new().expect("Failed to create Slint application - check UI resources");

    let app_weak = app.as_weak();

    let state = Rc::new(SlintServoAdapter::new(
        app_weak,
        waker_sender.clone(),
        waker_receiver.clone(),
    ));

    let state_weak = Rc::downgrade(&state);

    app.window()
        .set_rendering_notifier(move |state, graphics_api| match state {
            slint::RenderingState::RenderingSetup => {
                #[cfg(not(target_os = "android"))]
                if let slint::GraphicsAPI::WGPU27 { device, queue, .. } = graphics_api {
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

    init_servo_webview(state.clone());

    spin_servo_event_loop(state.clone());

    on_scroll_event(state.clone());

    on_pointer_event(state.clone());

    app.run()
        .expect("Application failed to run - check for runtime errors");
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(android_app: slint::android::AndroidApp) {
    let mut platform = AndroidPlatform::new(android_app.clone());

    let listener_handle = platform.event_listener_handle();

    slint::platform::set_platform(Box::new(platform)).unwrap();

    let app = MyApp::new().expect("Failed to create Slint application - check UI resources");

    let app_weak = app.as_weak();

    let (waker_sender, waker_receiver) = channel::unbounded::<()>();

    let state = Rc::new(SlintServoAdapter::new(
        app_weak,
        waker_sender.clone(),
        waker_receiver.clone(),
    ));

    let state_clone = state.clone();

    listener_handle.set(move |event| {
        on_android_event(event, state_clone.clone());
    });

    spin_servo_event_loop(state.clone());

    on_scroll_event(state.clone());

    on_pointer_event(state.clone());

    app.run()
        .expect("Application failed to run - check for runtime errors");
}

#[cfg(target_os = "android")]
fn on_android_event(poll_event: &PollEvent<'_>, state: Rc<SlintServoAdapter>) {
    match poll_event {
        PollEvent::Main(main_event) => match main_event {
            MainEvent::InitWindow { .. } => {
                android_init_servo_webview(state.clone());
            }
            MainEvent::InputAvailable => {
                let _ = state.waker_sender.try_send(());
            }
            _ => {}
        },
        _ => {}
    }
}

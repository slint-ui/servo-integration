use std::rc::Rc;

use euclid::{Scale, Size2D};
use slint::ComponentHandle;
use url::Url;
use winit::dpi::PhysicalSize;

use servo::{ServoBuilder, WebViewBuilder, webrender_api::units::DevicePixel};

#[cfg(not(target_os = "android"))]
use slint::winit_030::WinitWindowAccessor;

use crate::{adapter::SlintServoAdapter, constants, delegate::AppDelegate, waker::Waker};

#[cfg(not(target_os = "android"))]
use crate::rendering_context::try_create_gpu_context;

#[cfg(target_os = "android")]
use crate::rendering_context::create_software_context;

use crate::WebviewLogic;

pub fn spin_servo_event_loop(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    slint::spawn_local({
        async move {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in servo event loop");

            loop {
                let _ = state.waker_receiver.recv().await;
                if let Some(ref servo) = *state.servo.borrow() {
                    servo.spin_event_loop();
                }
            }
        }
    })
    .expect("Failed to spawn servo event loop task");
}

#[cfg(not(target_os = "android"))]
pub fn init_servo_webview(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    slint::spawn_local({
        async move {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in servo init");

            let app = state
                .app
                .upgrade()
                .expect("Failed to upgrade app weak reference in servo init");

            let winit_window = app
                .window()
                .winit_window()
                .await
                .expect("Failed to get winit window");

            let scale_factor = winit_window.scale_factor() as f32;

            let width = app.global::<WebviewLogic>().get_viewport_width();
            let height = app.global::<WebviewLogic>().get_viewport_height();

            let size: Size2D<f32, DevicePixel> = Size2D::new(width, height) * scale_factor;

            let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);

            let wgpu_device = state.wgpu_device();
            let wgpu_queue = state.wgpu_queue();

            let rendering_adapter =
                try_create_gpu_context(wgpu_device, wgpu_queue, physical_size).unwrap();

            let rendering_context = rendering_adapter.get_rendering_context();

            let servo = ServoBuilder::new(rendering_context)
                .event_loop_waker(Box::new(Waker::new(state.waker_sender.clone())))
                .build();

            let url = Url::parse(constants::DEFAULT_URL).expect("Failed to parse default URL");
            let delegate = Rc::new(AppDelegate::new(state.clone()));
            let scale = Scale::new(scale_factor);

            let webview = WebViewBuilder::new(&servo)
                .url(url)
                .size(physical_size)
                .delegate(delegate)
                .hidpi_scale_factor(scale)
                .build();

            webview.show(true);

            state.set_inner(servo, webview, scale_factor, rendering_adapter);
        }
    })
    .expect("Failed to spawn servo initialization task");
}

#[cfg(target_os = "android")]
pub fn android_init_servo_webview(state: Rc<SlintServoAdapter>) {
    let state_weak = Rc::downgrade(&state);

    slint::spawn_local({
        async move {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in servo init");

            let app = state
                .app
                .upgrade()
                .expect("Failed to upgrade app weak reference in servo init");

            let window = app.window();

            let window_size = window.size();

            let scale_factor = window.scale_factor() as f32;

            let width = app.global::<WebviewLogic>().get_viewport_width();
            let height = app.global::<WebviewLogic>().get_viewport_height();

            let size: Size2D<f32, DevicePixel> = Size2D::new(width, height) * scale_factor;

            let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);

            let rendering_adapter = create_software_context(physical_size);

            let rendering_context = rendering_adapter.get_rendering_context();

            let servo = ServoBuilder::new(rendering_context)
                .event_loop_waker(Box::new(Waker::new(state.waker_sender.clone())))
                .build();

            let url = Url::parse(constants::DEFAULT_URL).expect("Failed to parse default URL");
            let delegate = Rc::new(AppDelegate::new(state.clone()));
            let scale = Scale::new(scale_factor);

            let webview = WebViewBuilder::new(&servo)
                .url(url)
                .size(physical_size)
                .delegate(delegate)
                .hidpi_scale_factor(scale)
                .build();

            webview.show(true);

            state.set_inner(servo, webview, scale_factor, rendering_adapter);
        }
    })
    .expect("Failed to spawn servo initialization task for Android");
}

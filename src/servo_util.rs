use std::rc::Rc;

use euclid::Scale;
use url::Url;
use winit::dpi::PhysicalSize;

use smol::channel::{Receiver, Sender};

use slint::{ComponentHandle, winit_030::WinitWindowAccessor};

use servo::{ServoBuilder, WebViewBuilder};

use crate::{
    constants,
    delegate::AppDelegate,
    rendering_context::try_create_gpu_context,
    adapter::ServoSlintAdapter,
    waker::Waker,
};

pub fn spin_servo_event_loop(state: Rc<ServoSlintAdapter>, waker_receiver: Receiver<()>) {
    let state_weak = Rc::downgrade(&state);

    slint::spawn_local({
        async move {
            let state = state_weak
                .upgrade()
                .expect("Failed to upgrade state weak reference in servo event loop");

            loop {
                let _ = waker_receiver.recv().await;
                if let Some(ref servo) = *state.servo.borrow() {
                    servo.spin_event_loop();
                }
            }
        }
    })
    .expect("Failed to spawn servo event loop task");
}

pub fn init_servo_webview(state: Rc<ServoSlintAdapter>, waker_sender: Sender<()>) {
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

            let window_size = winit_window.inner_size();
            let scale_factor = winit_window.scale_factor() as f32;

            let physical_size = PhysicalSize::new(window_size.width, window_size.height);

            let wgpu_device = state.device.borrow();
            let wgpu_device = wgpu_device
                .as_ref()
                .expect("WGPU device not initialized - ensure rendering setup completed");

            let wgpu_queue = state.queue.borrow();
            let wgpu_queue = wgpu_queue
                .as_ref()
                .expect("WGPU queue not initialized - ensure rendering setup completed");

            let rendering_adapter =
                try_create_gpu_context(wgpu_device, wgpu_queue, physical_size).unwrap();

            let rendering_context = rendering_adapter.get_rendering_context();
            
            let servo = ServoBuilder::new(rendering_context)
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
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

            *state.servo.borrow_mut() = Some(servo);
            *state.webview.borrow_mut() = Some(webview);
            *state.scale_factor.borrow_mut() = scale_factor;
            *state.rendering_adapter.borrow_mut() = Some(rendering_adapter);
        }
    })
    .expect("Failed to spawn servo initialization task");
}

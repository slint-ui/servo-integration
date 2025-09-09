use std::rc::Rc;

use smol::channel::{Receiver, Sender};
use url::Url;
use winit::dpi;

use slint::{ComponentHandle, winit_030::WinitWindowAccessor};

use servo::{ServoBuilder, SoftwareRenderingContext, WebViewBuilder};

use crate::{delegate::AppDelegate, state::State, waker::Waker};

pub fn spin_servo_event_loop(state: Rc<State>, waker_receiver: Receiver<()>) {
    let state_weak = Rc::downgrade(&state);
    slint::spawn_local({
        async move {
            let state = state_weak.upgrade().unwrap();
            loop {
                let _ = waker_receiver.recv().await;
                if let Some(ref servo) = *state.servo.borrow() {
                    servo.spin_event_loop();
                }
            }
        }
    })
    .unwrap();
}

pub fn init_servo_webview(url_string: String, state: Rc<State>, waker_sender: Sender<()>) {
    let state_weak = Rc::downgrade(&state);
    slint::spawn_local({
        async move {
            let state = state_weak.upgrade().unwrap();

            let winit_window = state.app.window().winit_window().await.unwrap();

            let window_size = winit_window.inner_size();
            let size = dpi::PhysicalSize::new(window_size.width, window_size.height);
            let scale_factor = winit_window.scale_factor() as f32;

            let rendering_context = SoftwareRenderingContext::new(size).unwrap();
            let rendering_context_rc = Rc::new(rendering_context);

            let servo = ServoBuilder::new(rendering_context_rc.clone())
                .event_loop_waker(Box::new(Waker::new(waker_sender)))
                .build();

            let url = Url::parse(&url_string).unwrap();
            let delegate = Rc::new(AppDelegate::new(state.clone()));

            let webview = WebViewBuilder::new(&servo)
                .url(url)
                .delegate(delegate)
                .build();

            webview.show(true);

            *state.servo.borrow_mut() = Some(servo);
            *state.scale_factor.borrow_mut() = scale_factor;
            *state.webview.borrow_mut() = Some(webview);
            *state.rendering_context.borrow_mut() = Some(rendering_context_rc.clone());
        }
    })
    .unwrap();
}

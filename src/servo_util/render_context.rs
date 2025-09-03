use std::rc::Rc;

use servo::{RenderingContext, WindowRenderingContext};
use slint::Window;
use winit::dpi::PhysicalSize;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub fn get_rendering_context(window: &Window) -> Rc<dyn RenderingContext>{
    let slint_window_handle = window.window_handle();

    let window_handle = slint_window_handle.window_handle().unwrap();
    let display_handle = slint_window_handle.display_handle().unwrap();

    let slint_size = window.size();
    let size = PhysicalSize::new(slint_size.width, slint_size.height);

    let rendering_context =
        WindowRenderingContext::new(display_handle, window_handle, size).unwrap();
    let rendering_context_rc = Rc::new(rendering_context);

    return rendering_context_rc;
}

mod gpu_rendering_context;

mod servo_rendering_context;
mod surfman_context;

pub use gpu_rendering_context::GPURenderingContext;
pub use servo_rendering_context::ServoRenderingAdapter;
pub use servo_rendering_context::create_software_context;

#[cfg(not(target_os = "android"))]
pub use servo_rendering_context::try_create_gpu_context;

#[cfg(target_os = "macos")]
mod metal;

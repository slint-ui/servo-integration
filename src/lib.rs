// Library crate for servo-integration
// This provides the cdylib interface when building for Android or other platforms

pub mod adapter;
pub mod application_handler;
pub mod constants;
pub mod debug_helper;
pub mod delegate;
pub mod on_events;
pub mod rendering_context;
pub mod servo_util;
pub mod waker;

// Include Slint modules for library compilation
slint::include_modules!();

// Re-export main functionality for library consumers
pub use adapter::SlintServoAdapter;
pub use application_handler::ApplicationHandler;

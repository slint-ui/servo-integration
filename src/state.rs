//! Application state management.
//!
//! This module contains the central `State` struct that manages all shared
//! application resources including WGPU devices, Servo instances, and the
//! rendering context.

use std::{cell::RefCell, rc::Rc};

use servo::{Servo, WebView};
use slint::{ComponentHandle, Weak, wgpu_26::wgpu};

use crate::{MyApp, rendering_context::CustomRenderingContext};

/// Central application state managing all shared resources.
///
/// This struct coordinates between the Slint UI, Servo web engine,
/// and WGPU graphics pipeline. All fields use interior mutability
/// to allow safe sharing across the application.
pub struct State {
    /// Weak reference to the Slint application (avoids circular references)
    pub app: Weak<MyApp>,

    /// WGPU device for GPU operations (set during rendering setup)
    pub device: RefCell<Option<wgpu::Device>>,

    /// WGPU queue for command submission (set during rendering setup)
    pub queue: RefCell<Option<wgpu::Queue>>,

    /// Display scale factor for high-DPI support
    pub scale_factor: RefCell<f32>,

    /// Servo web engine instance
    pub servo: RefCell<Option<Servo>>,

    /// WebView instance for web content rendering
    pub webview: RefCell<Option<WebView>>,

    /// Custom rendering context managing Metal/WGPU integration
    pub rendering_context: RefCell<Option<Rc<CustomRenderingContext>>>,
}

impl State {
    /// Create a new application state with the given Slint app reference.
    ///
    /// # Arguments
    ///
    /// * `app` - Weak reference to the Slint application to avoid circular references
    pub fn new(app: Weak<MyApp>) -> Self {
        println!("Creating new application state");
        Self {
            app,
            device: RefCell::new(None),
            queue: RefCell::new(None),
            servo: RefCell::new(None),
            webview: RefCell::new(None),
            scale_factor: RefCell::new(1.0),
            rendering_context: RefCell::new(None),
        }
    }

    /// Update the web content display with the latest rendered frame.
    ///
    /// This method retrieves the current frame from the rendering context,
    /// converts it to a format suitable for Slint, and updates the UI.
    ///
    /// # Panics
    ///
    /// This method may panic if:
    /// - The rendering context is not initialized
    /// - WGPU device or queue are not available
    /// - Texture conversion fails
    /// - The application reference is no longer valid
    pub fn update_web_content_with_latest_frame(&self) {
        // Get rendering context with better error message
        let rendering_context_ref = self.rendering_context.borrow();
        let rendering_context = rendering_context_ref
            .as_ref()
            .expect("Rendering context not initialized - ensure Servo setup completed");

        // Get WGPU device and queue with better error messages
        let wgpu_device = self.device.borrow();
        let wgpu_device = wgpu_device
            .as_ref()
            .expect("WGPU device not initialized - ensure rendering setup completed");

        let wgpu_queue = self.queue.borrow();
        let wgpu_queue = wgpu_queue
            .as_ref()
            .expect("WGPU queue not initialized - ensure rendering setup completed");

        // Create texture from Metal surface
        let texture = rendering_context.get_wgpu_texture_from_metal(wgpu_device, wgpu_queue);

        // Convert to Slint image with better error message
        let slint_image = slint::Image::try_from(texture).expect(
            "Failed to create Slint image from WGPU texture - check texture format compatibility",
        );

        // Update the UI with better error message
        let app = self
            .app
            .upgrade()
            .expect("Application reference is no longer valid - UI may have been destroyed");

        app.set_web_content(slint_image);
        app.window().request_redraw();

        // Uncomment for debugging frame updates
        // println!("Web content updated successfully");
    }
}

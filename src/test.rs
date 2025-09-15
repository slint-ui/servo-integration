use euclid::Size2D;

use slint::{ComponentHandle, Image};

use surfman::{
    Connection, ContextAttributeFlags, ContextAttributes, GLVersion, SurfaceAccess, SurfaceType,
};

slint::slint! {
    export component MyApp inherits Window {
        width: 1024px;
        height: 768px;

        in property <image> image_source <=> image.source;

        image := Image {
            width: 100%;
            height: 100%;
        }
    }
}

const WIDTH: i32 = 1024;
const HEIGHT: i32 = 768;

fn main() {
    let app = MyApp::new().unwrap();

    let app_weak = app.as_weak();

    app.window()
        .set_rendering_notifier(move |state, _graphics_api| match state {
            slint::RenderingState::BeforeRendering => {
                let image = slint_image_from_surfman_surface();
                let app = app_weak.upgrade().unwrap();
                app.set_image_source(image);
                app.window().request_redraw();
            }
            _ => {}
        })
        .unwrap();

    app.run().unwrap();
}

pub fn slint_image_from_surfman_surface() -> Image {
    let connection = Connection::new().unwrap();

    let adapter = connection.create_adapter().unwrap();

    let mut device = connection.create_device(&adapter).unwrap();

    let context_attributes = ContextAttributes {
        version: GLVersion::new(3, 3),
        flags: ContextAttributeFlags::empty(),
    };

    let context_descriptor = device
        .create_context_descriptor(&context_attributes)
        .unwrap();

    let mut context = device.create_context(&context_descriptor, None).unwrap();

    let surface = device
        .create_surface(
            &context,
            SurfaceAccess::GPUOnly,
            SurfaceType::Generic {
                size: Size2D::new(WIDTH, HEIGHT),
            },
        )
        .unwrap();

    device
        .bind_surface_to_context(&mut context, surface)
        .unwrap();

    device.make_context_current(&context).unwrap();

    gl::load_with(|symbol_name| device.get_proc_address(&context, symbol_name));

    let surface_info = device.context_surface_info(&context).unwrap().unwrap();

    unsafe {
        gl::BindFramebuffer(
            gl::FRAMEBUFFER,
            surface_info.framebuffer_object.map_or(0, |fbo| fbo.0.get()),
        );
        gl::Viewport(0, 0, WIDTH, HEIGHT);

        gl::ClearColor(0.2, 0.3, 0.8, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
    }

    let surface = device
        .unbind_surface_from_context(&mut context)
        .unwrap()
        .unwrap();

    let surface_texture = device
        .create_surface_texture(&mut context, surface)
        .unwrap();

    device.destroy_context(&mut context).unwrap();

    let texture = device.surface_texture_object(&surface_texture).unwrap();

    let texture_id = texture.0;

    let result_texture = unsafe {
        slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
            texture_id,
            Size2D::new(WIDTH as u32, HEIGHT as u32),
        )
        .build()
    };

    return result_texture;
}

use khronos_egl::{Boolean as EGLBoolean, EGLDisplay, EGLImage as EGLImageKHR};

#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetCurrentDisplay() -> EGLDisplay;

    fn eglGetError() -> std::ffi::c_int;

    fn eglGetProcAddress(procname: *const i8) -> extern "system" fn();

}

#[allow(non_snake_case)]
struct ExtensionFunctions {
    eglExportDMABUFImageQueryMESA: fn(
        dpy: EGLDisplay,
        image: EGLImageKHR,
        fourcc: *mut std::ffi::c_int,
        num_planes: *mut std::ffi::c_int,
        modifiers: *mut u64,
    ) -> EGLBoolean,

    eglExportDMABUFImageMESA: fn(
        dpy: EGLDisplay,
        image: EGLImageKHR,
        fds: *mut std::ffi::c_int,
        strides: *mut std::ffi::c_int,
        offsets: *mut std::ffi::c_int,
    ) -> EGLBoolean,
}

thread_local! {
    static EGL_DMABUF_EXTENSIONS: std::cell::LazyCell<ExtensionFunctions> = std::cell::LazyCell::new(|| {
        unsafe {
            ExtensionFunctions {
                eglExportDMABUFImageQueryMESA: std::mem::transmute(eglGetProcAddress(c"eglExportDMABUFImageQueryMESA".as_ptr())),
                eglExportDMABUFImageMESA: std::mem::transmute(eglGetProcAddress(c"eglExportDMABUFImageMESA".as_ptr())),
            }
        }
    });
}

#[derive(Debug)]
pub struct DMABuffersForSurface {
    pub num_planes: i32, // at most 4
    /// IMPORTANT: the fd here is not owned. Take care of closing it or making sure that the
    /// consumer takes ownership.
    pub fds: [std::os::fd::RawFd; 4],
    pub fourcc_format: i32,
    pub modifiers: [u64; 4],
    pub strides: [std::ffi::c_int; 4],
    pub offsets: [std::ffi::c_int; 4],
}

impl TryFrom<khronos_egl::EGLImage> for DMABuffersForSurface {
    type Error = khronos_egl::Error;

    fn try_from(image: khronos_egl::EGLImage) -> Result<Self, Self::Error> {
        EGL_DMABUF_EXTENSIONS.with(|egl_extensions| {
            let mut fourcc_format = 0;
            let mut num_planes = 0;
            let mut modifiers = [0, 0, 0, 0];
            let egl_display = unsafe { eglGetCurrentDisplay() };
            if unsafe {
                (egl_extensions.eglExportDMABUFImageQueryMESA)(
                    egl_display,
                    image,
                    &mut fourcc_format,
                    &mut num_planes,
                    modifiers.as_mut_ptr(),
                )
            } == khronos_egl::FALSE
            {
                return Err(unsafe { eglGetError() }.try_into().unwrap());
            }
            let mut fds = [0, 0, 0, 0];
            let mut strides = [0, 0, 0, 0];
            let mut offsets = [0, 0, 0, 0];
            if unsafe {
                (egl_extensions.eglExportDMABUFImageMESA)(
                    egl_display,
                    image,
                    fds.as_mut_ptr(),
                    strides.as_mut_ptr(),
                    offsets.as_mut_ptr(),
                )
            } == khronos_egl::FALSE
            {
                return Err(unsafe { eglGetError() }.try_into().unwrap());
            }

            Ok(Self {
                num_planes,
                fds,
                fourcc_format,
                modifiers,
                strides,
                offsets,
            })
        })
    }
}

extern crate gl_generator;

use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("gl_bindings.rs")).unwrap();

    // Config copied from https://github.com/YaLTeR/bxt-rs/blob/9f621251b8ce5c2af00b67d2feab731e48d1dae9/build.rs.

    Registry::new(
        Api::Gl,
        (4, 6),
        Profile::Compatibility,
        Fallbacks::All,
        [
            "GL_EXT_memory_object",
            "GL_EXT_memory_object_fd",
            "GL_EXT_memory_object_win32",
            "GL_EXT_semaphore",
            "GL_EXT_semaphore_fd",
            "GL_EXT_semaphore_win32",
        ],
    )
    .write_bindings(StructGenerator, &mut file)
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    slint_build::compile("ui/app.slint").unwrap();
}

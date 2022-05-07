extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    // libretro bindings
    // Monitor changes in headers
    println!("cargo:rerun-if-changed=libretro.h");

    let libretro_bindings = bindgen::Builder::default()
        .header("libretro.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Generation of libretro bindings failed");

    libretro_bindings
        .write_to_file(output.join("libretro_bindings.rs"))
        .expect("Failed to write libretro bindings");
}

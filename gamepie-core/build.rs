extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    // libretro bindings
    // Monitor changes in headers
    println!("cargo:rerun-if-changed=cshim");

    let log_bindings = bindgen::Builder::default()
        .header("cshim/cshim.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Generation of libretro bindings failed");

    log_bindings
        .write_to_file(output.join("log_bindings.rs"))
        .expect("Failed to write log bindings");

    cc::Build::new().file("cshim/cshim.c").compile("libcshim.a");
}

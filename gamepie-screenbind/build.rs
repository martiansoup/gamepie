extern crate bindgen;

use cmake::Config;
use std::env;
use std::path::PathBuf;

fn main() {
    let output = PathBuf::from(env::var("OUT_DIR").unwrap());

    // screen driver library
    // Monitor changes in screen driver
    println!("cargo:rerun-if-changed=screen");

    let dst = Config::new("screen")
        //.define("PIMORONI_DISPLAY_HAT_MINI", "ON")
        .define("PIRATE_AUDIO_ST7789_HAT", "ON")
        .define("SPI_BUS_CLOCK_DIVISOR", "8")
        .define("DISPLAY_BREAK_ASPECT_RATIO_WHEN_SCALING", "ON")
        .define("STATISTICS", "0")
        .define("LOW_BATTERY_PIN", "26")
        .build();
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=screen");
    println!("cargo:rustc-link-lib=dylib=bcm_host");

    let screen_bindings = bindgen::Builder::default()
        .header("screen/fbcp-ili9341.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Generation of screen bindings failed");

    screen_bindings
        .write_to_file(output.join("screen_bindings.rs"))
        .expect("Failed to write screen bindings");
}

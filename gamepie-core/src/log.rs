use log::log;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use std::ffi::CStr;

// Enum to match libretro log levels, but without introducing a dependency on
// the bindings.
#[repr(u32)]
#[derive(FromPrimitive, ToPrimitive, Debug, PartialEq, std::cmp::Eq, std::hash::Hash)]
enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

/// Provide a C interface to the Rust logger,
///
/// # Safety
///
/// This function is unsafe for the same reasons as `CStr::from_ptr()`.
#[no_mangle]
pub unsafe extern "C" fn gamepie_log(level: u32, msg: *const ::std::os::raw::c_char) {
    let l = match FromPrimitive::from_u32(level) {
        Some(LogLevel::Debug) => log::Level::Debug,
        Some(LogLevel::Info) => log::Level::Info,
        Some(LogLevel::Warn) => log::Level::Warn,
        Some(LogLevel::Error) => log::Level::Error,
        None => log::Level::Warn,
    };

    let msg_str = CStr::from_ptr(msg).to_string_lossy();

    let msg = String::from(msg_str);

    log!(l, "{}", msg.trim_matches('\n'));
}

include!(concat!(env!("OUT_DIR"), "/log_bindings.rs"));

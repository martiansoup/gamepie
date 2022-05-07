use log::{error, trace, warn};
use std::error::Error;

use gamepie_libretro::callbacks::retro_environment_callback_inner;
use gamepie_libretro::proxy::{ProxyWarning, RetroProxy};
use gamepie_libretrobind::bind::{
    retro_audio_sample_batch_t, retro_audio_sample_t, retro_environment_t, retro_input_poll_t,
    retro_input_state_t, retro_video_refresh_t, size_t, RETRO_DEVICE_JOYPAD,
};
use gamepie_libretrobind::enums::RetroDevice;

unsafe extern "C" fn retro_environment_callback(
    cmd: ::std::os::raw::c_uint,
    data: *mut ::std::os::raw::c_void,
) -> bool {
    match crate::proxy::libretro::with_proxy(|p| retro_environment_callback_inner(cmd, data, p)) {
        Some(b) => b,
        None => {
            error!("Callback executed before core loaded");
            false
        }
    }
}

pub fn retro_set_environment(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    let cb = Some(
        retro_environment_callback
            as unsafe extern "C" fn(
                cmd: ::std::os::raw::c_uint,
                data: *mut ::std::os::raw::c_void,
            ) -> bool,
    );
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_environment_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_environment");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

unsafe extern "C" fn retro_video_refresh_callback(
    data: *const ::std::os::raw::c_void,
    width: ::std::os::raw::c_uint,
    height: ::std::os::raw::c_uint,
    pitch: size_t,
) {
    if !data.is_null() {
        let w: u16 = width.try_into().expect("giant screen");
        let h: u16 = height.try_into().expect("giant screen");
        let pitch: u16 = pitch.try_into().expect("giant screen");
        let psz: usize = pitch.try_into().expect("giant screen");
        let hsz: usize = height.try_into().expect("giant screen");
        let slice = std::slice::from_raw_parts(data as *const u8, psz * hsz);
        trace!("video refresh {}x{} {}pitch", w, h, pitch);

        let f = |p: &mut RetroProxy| {
            p.draw(w, h, pitch, slice);
        };

        if crate::proxy::libretro::with_proxy(f).is_none() {
            error!("Callback executed before core loaded")
        }
    }
}

pub fn retro_set_video_refresh(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    let cb = Some(
        retro_video_refresh_callback
            as unsafe extern "C" fn(
                data: *const ::std::os::raw::c_void,
                width: ::std::os::raw::c_uint,
                height: ::std::os::raw::c_uint,
                pitch: size_t,
            ),
    );
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_video_refresh_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_video_refresh");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

extern "C" fn retro_input_poll_callback() {
    let f = |p: &mut RetroProxy| {
        p.input_poll();
    };
    if crate::proxy::libretro::with_proxy(f).is_none() {
        error!("Callback executed before core loaded")
    }
}

pub fn retro_set_input_poll(
    lib: &libloading::Library,
    //cb: retro_input_poll_t,
) -> Result<(), Box<dyn Error>> {
    let cb = Some(retro_input_poll_callback as unsafe extern "C" fn());
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_input_poll_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_input_poll");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

extern "C" fn retro_input_state_callback(
    port: ::std::os::raw::c_uint,
    device: ::std::os::raw::c_uint,
    // Index unused as not applicable for joypad
    _index: ::std::os::raw::c_uint,
    id: ::std::os::raw::c_uint,
) -> i16 {
    match crate::proxy::libretro::with_proxy(|p| {
        if port != 0 {
            // Only expect any controller on port 0
            let msg = format!("Trying to get input for port {}", port);
            p.warn_once(ProxyWarning::DevicePort, &msg);
            return 0;
        }

        if device == RETRO_DEVICE_JOYPAD {
            let button = num::FromPrimitive::from_u32(id);
            match button {
                Some(b) => p.input_state(b),
                None => {
                    warn!("Unknown button");
                    0
                }
            }
        } else {
            let msg = format!(
                "Unsupported input device: {}",
                RetroDevice::identify(device)
            );
            p.warn_once(ProxyWarning::DeviceType, &msg);
            0
        }
    }) {
        Some(v) => v,
        None => {
            error!("Callback executed before core loaded");
            0
        }
    }
}

pub fn retro_set_input_state(
    lib: &libloading::Library,
    //cb: retro_input_state_t,
) -> Result<(), Box<dyn Error>> {
    let cb = Some(
        retro_input_state_callback
            as unsafe extern "C" fn(
                port: ::std::os::raw::c_uint,
                device: ::std::os::raw::c_uint,
                index: ::std::os::raw::c_uint,
                id: ::std::os::raw::c_uint,
            ) -> i16,
    );
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_input_state_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_input_state");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

extern "C" fn retro_audio_sample_callback(left: i16, right: i16) {
    trace!("audio sample");
    let f = |p: &mut RetroProxy| {
        // Inefficient as it's sending a single sample at a time but
        // no indication of when there are no more samples available
        // so not trivial to batch up.
        p.audio_sample(vec![left, right]);
    };

    if crate::proxy::libretro::with_proxy(f).is_none() {
        error!("Callback executed before core loaded");
    }
}

pub fn retro_set_audio_sample(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    let cb = Some(retro_audio_sample_callback as unsafe extern "C" fn(left: i16, right: i16));
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_audio_sample_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_audio_sample");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

extern "C" fn retro_audio_sample_batch_callback(
    data: *const i16,
    frames: ::std::os::raw::c_ulong,
) -> ::std::os::raw::c_ulong {
    trace!("audio samples");
    match crate::proxy::libretro::with_proxy(|p| {
        let nframes: usize = frames.try_into().expect("too much audio");
        unsafe {
            // Multiply number of frames by two, as a frame contains
            // a sample for both left and right.
            let slice = std::slice::from_raw_parts(data, nframes * 2);
            p.audio_sample(slice.to_vec());
            frames
        }
    }) {
        Some(n) => n,
        None => {
            error!("Callback executed before core loaded");
            0
        }
    }
}

pub fn retro_set_audio_sample_batch(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    let cb = Some(
        retro_audio_sample_batch_callback
            as unsafe extern "C" fn(
                data: *const i16,
                frames: ::std::os::raw::c_ulong,
            ) -> ::std::os::raw::c_ulong,
    );
    unsafe {
        let func: Result<
            libloading::Symbol<unsafe extern "C" fn(retro_audio_sample_batch_t)>,
            libloading::Error,
        > = lib.get(b"retro_set_audio_sample_batch");
        match func {
            Ok(f) => {
                f(cb);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

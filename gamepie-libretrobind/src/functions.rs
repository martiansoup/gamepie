use log::debug;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;

use gamepie_core::error::GamepieError;
use gamepie_core::portable::PString;
use gamepie_core::RetroSystemInfo;

use crate::bind::{retro_game_info, retro_system_av_info, retro_system_info};
use crate::types::*;

// TODO, should symbols be cached?
// and how to maintain validity of that cache?
// wrap lib in cached lib?

pub fn get_system_info(lib: &libloading::Library) -> Result<RetroSystemInfo, Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut retro_system_info) -> ()> =
            lib.get(b"retro_get_system_info")?;

        let mut info: retro_system_info = std::mem::zeroed();
        func(&mut info as *mut retro_system_info);
        let library_name = PString::from_ptr(info.library_name)?.into();
        let library_version = PString::from_ptr(info.library_version)?.into();
        let valid_extensions = PString::from_ptr(info.valid_extensions)?.into();
        Ok(RetroSystemInfo {
            library_name,
            library_version,
            valid_extensions,
            need_fullpath: info.need_fullpath,
            block_extract: info.block_extract,
        })
    }
}

pub fn get_system_av_info(lib: &libloading::Library) -> Result<RetroSystemAvInfo, Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn(*mut retro_system_av_info) -> ()> =
            lib.get(b"retro_get_system_av_info")?;

        let mut info: retro_system_av_info = std::mem::zeroed();
        func(&mut info as *mut retro_system_av_info);
        let geometry = RetroGameGeometry {
            aspect_ratio: info.geometry.aspect_ratio,
            base_height: info.geometry.base_height,
            base_width: info.geometry.base_width,
            max_height: info.geometry.max_height,
            max_width: info.geometry.max_width,
        };
        let timing = RetroSystemTiming {
            fps: info.timing.fps,
            sample_rate: info.timing.sample_rate,
        };
        Ok(RetroSystemAvInfo { geometry, timing })
    }
}

pub fn frontend_api_version() -> std::os::raw::c_uint {
    crate::bind::RETRO_API_VERSION
}

pub fn api_version(lib: &libloading::Library) -> Result<std::os::raw::c_uint, Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn() -> std::os::raw::c_uint> =
            lib.get(b"retro_api_version")?;

        Ok(func())
    }
}

pub fn init(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"retro_init")?;

        func();
        Ok(())
    }
}

// TODO retro_run symbol (at least) should be cached
pub fn run(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"retro_run")?;
        func();
        Ok(())
    }
}

pub fn deinit(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"retro_unload_game")?;
        func();
        let func: libloading::Symbol<unsafe extern "C" fn()> = lib.get(b"retro_deinit")?;
        func();
        Ok(())
    }
}

pub struct RetroGameInfo {
    path: String, // TODO data/size/meta for need_fullpath cores
}

impl RetroGameInfo {
    pub fn new(path: &str) -> Self {
        RetroGameInfo {
            path: String::from(path),
        }
    }
}

pub fn load_game(
    lib: &libloading::Library,
    info: &RetroSystemInfo,
    game_info: RetroGameInfo,
) -> Result<bool, Box<dyn Error>> {
    unsafe {
        let c_path = PString::from_str(&game_info.path)?;
        let c_meta = PString::from_str("")?;
        let mut buffer = Vec::new();
        let c_info = if info.need_fullpath {
            retro_game_info {
                path: c_path.as_ptr(),
                meta: c_meta.as_ptr(),
                size: 0,
                data: std::ptr::null::<std::os::raw::c_void>(),
            }
        } else {
            let mut game_file = File::open(&game_info.path)?;

            let size = game_file.read_to_end(&mut buffer)?;

            retro_game_info {
                path: c_path.as_ptr(),
                meta: c_meta.as_ptr(),
                size: size.try_into()?,
                data: buffer.as_ptr() as *const std::os::raw::c_void,
            }
        };

        let func: libloading::Symbol<unsafe extern "C" fn(game: *const retro_game_info) -> bool> =
            lib.get(b"retro_load_game")?;

        Ok(func(&c_info as *const retro_game_info))
    }
}

pub fn set_controller_port_device(lib: &libloading::Library) -> Result<(), Box<dyn Error>> {
    // Currently supports NES, GB, GBC, GBA
    // Only NES supports a second player, but only support a single controller
    // at a time, so always connect a joypad to port/player 0
    unsafe {
        let func: libloading::Symbol<
            unsafe extern "C" fn(::std::os::raw::c_uint, ::std::os::raw::c_uint),
        > = lib.get(b"retro_set_controller_port_device")?;

        func(0, crate::bind::RETRO_DEVICE_JOYPAD);
        Ok(())
    }
}

pub fn get_memory_size(lib: &libloading::Library, id: u32) -> Result<usize, Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<
            unsafe extern "C" fn(::std::os::raw::c_uint) -> ::std::os::raw::c_uint,
        > = lib.get(b"retro_get_memory_size")?;

        Ok(func(id).try_into().expect("u32 to usize"))
    }
}

pub fn get_memory_data(
    lib: &libloading::Library,
    id: u32,
) -> Result<*mut ::std::os::raw::c_void, Box<dyn Error>> {
    unsafe {
        let func: libloading::Symbol<
            unsafe extern "C" fn(::std::os::raw::c_uint) -> *mut ::std::os::raw::c_void,
        > = lib.get(b"retro_get_memory_data")?;

        Ok(func(id))
    }
}

// Libraries are not cached as this can cause problems with some emulators that
// don't reinitialise everything correctly causing broken audio etc.
pub fn load_library<P>(path: P) -> Result<Arc<libloading::Library>, Box<dyn Error>>
where
    P: AsRef<OsStr>,
{
    unsafe {
        let key = path.as_ref().to_str().ok_or(GamepieError::String)?;
        debug!("Loading library: '{}'", key);
        let lib = libloading::Library::new(key)?;
        let arc = Arc::new(lib);
        Ok(arc)
    }
}

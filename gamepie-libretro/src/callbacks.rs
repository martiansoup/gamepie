use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use log::{debug, error, info, trace, warn};
use std::error::Error;
use std::ffi::CStr;
use std::time::Duration;

use gamepie_core::commands::{ScreenMessage, ScreenToast};
use gamepie_core::error::GamepieError;
use gamepie_core::log::gamepie_log_shim;
use gamepie_core::portable::PStr;
use gamepie_core::problem::Problem;
use gamepie_libretrobind::bind::{
    retro_controller_description, retro_controller_info, retro_core_option_definition,
    retro_core_option_display, retro_core_option_value, retro_core_options_intl,
    retro_game_geometry, retro_input_descriptor, retro_language_RETRO_LANGUAGE_ENGLISH,
    retro_log_callback, retro_memory_map, retro_message, retro_pixel_format,
    retro_pixel_format_RETRO_PIXEL_FORMAT_RGB565, retro_variable, RETRO_ENVIRONMENT_EXPERIMENTAL,
    RETRO_ENVIRONMENT_PRIVATE,
};
use gamepie_libretrobind::enums::{identify_button, RetroDevice, RetroEnvironment};

use crate::proxy::RetroProxy;

// TODO could have the proxy in a RwLock so quicker for callbacks that
// are only reading from the proxy. Or RefCell to allow mutating just the
// bits that require mutability.

unsafe fn set_variables_v0(
    vars: *const retro_variable,
    proxy: &mut RetroProxy,
) -> Result<(), Box<dyn Error>> {
    let mut offset = 0;
    let mut var: retro_variable = *vars.offset(offset);
    while !var.key.is_null() {
        let key = PStr::from_ptr(var.key)?;
        let descr = PStr::from_ptr(var.value)?;
        proxy.add_var_v0(&key, &descr);

        offset += 1;
        var = *vars.offset(offset);
    }
    proxy.log_vars();
    Ok(())
}

unsafe fn set_variables_v1(
    vars: *const retro_core_option_definition,
    proxy: &mut RetroProxy,
) -> Result<(), Box<dyn Error>> {
    let mut offset = 0;
    let mut var: retro_core_option_definition = *vars.offset(offset);
    while !var.key.is_null() {
        let key = PStr::from_ptr(var.key)?;
        let descr = PStr::from_ptr(var.desc)?;
        let info = PStr::from_ptr(var.info)?;
        let mut values = Vec::new();

        let mut voffset = 0;
        let mut vvar: retro_core_option_value = var.values[voffset];

        while !vvar.value.is_null() {
            let val = PStr::from_ptr(vvar.value)?;

            let vdes = if vvar.label.is_null() {
                None
            } else {
                Some(PStr::from_ptr(vvar.label)?)
            };

            values.push((val, vdes));
            voffset += 1;
            vvar = var.values[voffset];
        }

        let default = PStr::from_ptr(var.default_value).ok();

        proxy.add_var_v1(&key, &descr, &info, values.as_slice(), default.as_ref());

        offset += 1;
        var = *vars.offset(offset);
    }
    proxy.log_vars();
    Ok(())
}

/// Libretro Environment callback
///
/// # Safety
///
/// Safety depends on `data` matching the type expected by the command.
pub unsafe extern "C" fn retro_environment_callback_inner(
    cmd: ::std::os::raw::c_uint,
    data: *mut ::std::os::raw::c_void,
    proxy: &mut RetroProxy,
) -> bool {
    let c = num::FromPrimitive::from_u32(cmd);
    let experimental = (cmd & RETRO_ENVIRONMENT_EXPERIMENTAL) == RETRO_ENVIRONMENT_EXPERIMENTAL;
    let private = (cmd & RETRO_ENVIRONMENT_PRIVATE) == RETRO_ENVIRONMENT_PRIVATE;
    let e_str = if experimental {
        "Experimental"
    } else {
        "Stable"
    };
    let p_str = if private { "Private" } else { "Public" };
    match c {
        Some(RetroEnvironment::GetLogInterface) => {
            let var = data as *mut retro_log_callback;
            (*var).log = Some(gamepie_log_shim);
            true
        }
        Some(RetroEnvironment::SetHwRender) => {
            error!("Hardware rendering not supported");
            false
        }
        Some(RetroEnvironment::SetGeometry) => {
            let var = data as *const retro_game_geometry;
            let new_width = (*var).base_width;
            let new_height = (*var).base_height;
            match proxy.get_av() {
                Some(av) => {
                    let (width, height) = (av.geometry.base_width, av.geometry.base_height);
                    if new_width != width || new_height != height {
                        warn!(
                            "Trying to set geometry to {}x{}, doesn't match expected dimensions {}x{}",
                            (*var).base_width,
                            (*var).base_height,
                            width,
                            height
                        );
                        proxy.problem(Problem::warn(ScreenToast::error(ScreenMessage::VideoIssue)));
                        false
                    } else {
                        true
                    }
                }
                None => true,
            }
        }
        Some(RetroEnvironment::GetSystemDirectory) => {
            let var = data as *mut *const std::os::raw::c_char;
            *var = proxy.sys_dir().as_ptr();
            false
        }
        Some(RetroEnvironment::GetVariable) => {
            let var = data as *mut retro_variable;

            let k = CStr::from_ptr((*var).key);
            if let Ok(key) = k.to_str() {
                (*var).value = proxy.get_var(key);
                true
            } else {
                false
            }
        }
        Some(RetroEnvironment::GetVariableUpdate) => {
            let updated = data as *mut bool;
            if proxy.vars_updated() {
                debug!("Variables updated");
                *updated = true;
            } else {
                *updated = false;
            }
            true
        }
        Some(RetroEnvironment::SetVariable) => {
            let var = data as *mut retro_variable;

            if var.is_null() {
                trace!("Set variable supported");
                // No update on null, just checking support
                return true;
            }

            let k = CStr::from_ptr((*var).key);
            let v = PStr::from_ptr((*var).value);
            if let Ok(key) = k.to_str() {
                if let Ok(val) = v {
                    proxy.set_var(key, &val)
                } else {
                    warn!("Set variable with invalid value");
                    false
                }
            } else {
                false
            }
        }
        Some(RetroEnvironment::SetVariables) => {
            debug!("Setting core options (v0)");
            let vars = data as *const retro_variable;
            set_variables_v0(vars, proxy)
                .map_err(|e| {
                    error!("Variable error: {}", e);
                })
                .is_ok()
        }
        Some(RetroEnvironment::SetCoreOptions) => {
            debug!("Setting core options (v1)");
            let vars = data as *const retro_core_option_definition;
            set_variables_v1(vars, proxy)
                .map_err(|e| {
                    error!("Variable error: {}", e);
                })
                .is_ok()
        }
        Some(RetroEnvironment::SetCoreOptionsIntl) => {
            debug!("Setting core options (v1-intl)");
            let vars_intl = data as *const retro_core_options_intl;
            let vars = (*vars_intl).us as *const retro_core_option_definition;
            // US is default English options and must be present,
            // with no language support can always use this
            set_variables_v1(vars, proxy)
                .map_err(|e| {
                    error!("Variable error: {}", e);
                })
                .is_ok()
        }
        Some(RetroEnvironment::SetPixelFormat) => {
            let pfmt = data as *const retro_pixel_format;
            if *pfmt == retro_pixel_format_RETRO_PIXEL_FORMAT_RGB565 {
                debug!("Set pixel formal to RGB565");
                true
            } else {
                warn!("Tried to use a non-RGB565 pixel format");
                proxy.problem(Problem::fatal(GamepieError::UnsupportedVideo));
                false
            }
        }
        Some(RetroEnvironment::GetAudioVideoEnable) => {
            let avint = data as *mut std::os::raw::c_int;
            let mut val = 0;
            // Bit 0 - Video enable
            if proxy.video_enabled() {
                val |= 1
            }
            // Bit 1 - Audio enable
            if proxy.audio_enabled() {
                val |= 2
            }
            // Bit 2 - Use fast save states
            // Bit 4 - Hard disable audio
            *avint = val;
            true
        }
        Some(RetroEnvironment::SetControllerInfo) => {
            let mut any_error = false;
            let info_arr = data as *const retro_controller_info;
            let mut offset = 0;
            let mut info: retro_controller_info = *info_arr.offset(offset);
            while !info.types.is_null() {
                info!("Port {} controllers", info.num_types);
                let num: isize = info.num_types.try_into().expect("too many controllers");
                for i in 0..num {
                    let controller: retro_controller_description = *(info.types).offset(i);
                    let n = CStr::from_ptr(controller.desc);
                    match n.to_str() {
                        Ok(name) => {
                            let dev_type = RetroDevice::identify(controller.id);
                            info!("  {} ({})", name, dev_type);
                        }
                        Err(_) => {
                            any_error = true;
                            warn!("Invalid string for controller name");
                        }
                    }
                }
                offset += 1;
                info = *info_arr.offset(offset);
            }
            !any_error
        }
        Some(RetroEnvironment::SetMemoryMaps) => {
            let maps = data as *const retro_memory_map;
            let num = (*maps).num_descriptors;
            if num != 0 {
                debug!("Memory map:")
            }
            for i in 0..num {
                let isz: isize = i.try_into().expect("too much memory");
                let map = (*maps).descriptors.offset(isz);
                let start = (*map).start;
                let end = (*map).start + (*map).len;
                if (*map).addrspace.is_null() {
                    debug!("  {:#010x} -> {:#010x}", start, end);
                } else {
                    let n = CStr::from_ptr((*map).addrspace);
                    let name = n.to_str().expect("non UTF-8");
                    debug!("  {:#010x} -> {:#010x} {}", start, end, name);
                }
            }
            true
        }
        Some(RetroEnvironment::GetCoreOptionsVersion) => {
            // Not many cores seem to support v2 options,
            // so only support v1.
            let version = data as *mut u32;
            *version = 1;
            true
        }
        Some(RetroEnvironment::GetVfsInterface) => {
            // TODO VFS support
            false
        }
        Some(RetroEnvironment::GetCanDupe) => {
            let dupe = data as *mut bool;
            *dupe = true;
            true
        }
        Some(RetroEnvironment::SetPerformanceLevel) => {
            let perf = data as *const ::std::os::raw::c_uint;
            info!("Performance level: {}", *perf);
            true
        }
        Some(RetroEnvironment::GetLanguage) => {
            let lang = data as *mut ::std::os::raw::c_uint;
            *lang = retro_language_RETRO_LANGUAGE_ENGLISH;
            true
        }
        Some(RetroEnvironment::SetInputDescriptors) => {
            let descriptors = data as *const retro_input_descriptor;
            let mut offset = 0;
            let mut descriptor: retro_input_descriptor = *descriptors.offset(offset);
            let mut mappings = Vec::new();

            while !descriptor.description.is_null() {
                mappings.push(descriptor);

                offset += 1;
                descriptor = *descriptors.offset(offset);
            }

            log_mappings(mappings);

            true
        }
        Some(RetroEnvironment::SetMessage) => {
            let msg = data as *const retro_message;

            match PStr::from_ptr((*msg).msg) {
                Ok(message) => {
                    let frames = (*msg).frames;
                    let duration = match proxy.get_av() {
                        Some(av) => {
                            Duration::from_secs_f64((1.0 / av.timing.fps) * (frames as f64))
                        }
                        None => Duration::from_secs(1),
                    };
                    let smsg = ScreenToast::new(
                        ScreenMessage::Message(message.to_string()),
                        duration,
                        Rgb565::WHITE,
                    );
                    proxy.problem(Problem::warn(smsg));
                    debug!("'{}' for {} frames", message, frames);
                    true
                }
                Err(_) => false,
            }
        }
        Some(RetroEnvironment::SetCoreOptionsDisplay) => {
            let disp = data as *const retro_core_option_display;

            match PStr::from_ptr((*disp).key) {
                Ok(key) => {
                    let visible = (*disp).visible;
                    let k = key.to_string();
                    proxy.set_var_visible(&k, visible);
                    true
                }
                Err(_) => false,
            }
        }
        Some(RetroEnvironment::GetInputBitmasks) => true,
        Some(RetroEnvironment::SetSupportAchievements) => false,
        Some(RetroEnvironment::GetRumbleInterface) => false,
        Some(c) => {
            warn!("Unsupported command: {:?} ({},{})", c, p_str, e_str);
            false
        }
        None => {
            error!("Unknown libretro environment command: {}", cmd);
            false
        }
    }
}

pub fn log_mappings(mappings: Vec<retro_input_descriptor>) {
    let mut lines = Vec::new();
    let mut col0 = 0;
    let mut col1 = 0;
    let mut col2 = 0;
    let mut col3 = 0;

    for mapping in mappings {
        let c0 = format!("{}", mapping.port);
        let c1 = RetroDevice::identify(mapping.device);
        let c2 = identify_button(mapping.device, mapping.id).to_string();
        let d = unsafe { CStr::from_ptr(mapping.description) };
        let c3 = d.to_string_lossy();

        // Index ignored as unused for basic retropad
        col0 = std::cmp::max(c0.len(), col0);
        col1 = std::cmp::max(c1.len(), col1);
        col2 = std::cmp::max(c2.len(), col2);
        col3 = std::cmp::max(c3.len(), col3);

        lines.push((c0, c1, c2, c3));
    }

    for l in lines {
        info!(
            "Port {:>w0$} - {:>w1$} {:>w2$} <=> {:<w3$}",
            l.0,
            l.1,
            l.2,
            l.3,
            w0 = col0,
            w1 = col1,
            w2 = col2,
            w3 = col3
        );
    }
}

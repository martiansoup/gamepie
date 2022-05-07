use log::{debug, error, info};
use std::error::Error;

use gamepie_core::error::GamepieError;

use crate::bind::RETRO_MEMORY_SAVE_RAM;

pub fn has_save_memory(lib: &libloading::Library) -> Result<bool, Box<dyn Error>> {
    let mem_size = crate::functions::get_memory_size(lib, RETRO_MEMORY_SAVE_RAM)?;
    Ok(mem_size != 0)
}

pub fn try_read_into_save_mem(
    lib: &libloading::Library,
    save_path: &str,
) -> Result<(), Box<dyn Error>> {
    match std::fs::read(save_path) {
        Ok(data) => {
            let save_size = crate::functions::get_memory_size(lib, RETRO_MEMORY_SAVE_RAM)?;
            if save_size == data.len() {
                let save_ptr = crate::functions::get_memory_data(lib, RETRO_MEMORY_SAVE_RAM)?;
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), save_ptr as *mut u8, save_size);
                }
                debug!("Save data loaded from '{}'", save_path);
                Ok(())
            } else {
                error!(
                    "Save length {} doesn't match expected length {}",
                    data.len(),
                    save_size
                );
                Err(Box::new(GamepieError::MismatchSave))
            }
        }
        Err(_) => {
            info!("No save data to load");
            Ok(())
        }
    }
}

pub fn save_to_file(lib: &libloading::Library, save_path: &str) -> Result<(), Box<dyn Error>> {
    let save_size = crate::functions::get_memory_size(lib, RETRO_MEMORY_SAVE_RAM)?;
    let save_ptr = crate::functions::get_memory_data(lib, RETRO_MEMORY_SAVE_RAM)?;
    let save_slice = unsafe { std::slice::from_raw_parts(save_ptr as *mut u8, save_size) };
    std::fs::write(save_path, save_slice)?;
    info!("Saved to '{}'", save_path);
    Ok(())
}

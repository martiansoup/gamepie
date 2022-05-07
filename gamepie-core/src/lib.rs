use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use std::time::Duration;

pub mod commands;
pub mod error;
pub mod log;
pub mod portable;
pub mod problem;

mod types;

pub use types::*;

pub const EMU_PATH: &str = "emulators";
pub const ROM_PATH: &str = "roms";
pub const SAVE_PATH: &str = "saves";
pub const SYS_PATH: &str = "sys";

pub const METADATA_EXT: &str = "toml";
pub const SAVEDATA_EXT: &str = "sav";

const SPLASH_TIME_SECS: u64 = 3;
const MENU_FRAME_TIME_MS: u64 = 30;
const BUTTON_BLANK_MS: u64 = 500;
// For toast popups, show for slightly under debounce to prevent popups
// building up.
const BUTTON_TOAST_MS: u64 = BUTTON_BLANK_MS - (MENU_FRAME_TIME_MS * 2);

pub const MENU_FRAME_DURATION: Duration = Duration::from_millis(MENU_FRAME_TIME_MS);
pub const BUTTON_BLANK_DURATION: Duration = Duration::from_millis(BUTTON_BLANK_MS);

pub const SPLASH_DURATION: Duration = Duration::from_secs(SPLASH_TIME_SECS);

pub const ERROR_DURATION: Duration = Duration::from_secs(SPLASH_TIME_SECS);
pub const ERROR_COLOUR: Rgb565 = Rgb565::RED;

pub const INFO_DURATION: Duration = Duration::from_millis(BUTTON_TOAST_MS);
pub const INFO_COLOUR: Rgb565 = Rgb565::WHITE;

// Menu colours
pub const BACKGROUND_COLOUR: Rgb565 = Rgb565::new(19, 6, 21);
pub const ERROR_BACKGROUND_COLOUR: Rgb565 = Rgb565::new(0, 30, 26);

pub const TEXT_COLOUR: Rgb565 = Rgb565::new(30, 54, 23);
pub const TEXT_SEL_COLOUR: Rgb565 = Rgb565::new(30, 46, 15);
pub const ERROR_TEXT_COLOUR: Rgb565 = Rgb565::WHITE;

// Unwrap a result that cannot fail, will provide guarantees
// that this is only used for Infallible results.
pub fn discard_error<T>(r: Result<T, std::convert::Infallible>) -> T {
    match r {
        Ok(r) => r,
        Err(e) => match e {},
    }
}

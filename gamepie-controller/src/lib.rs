use evdev_rs::InputEvent;
use gamepie_libretrobind::enums::RetroPadButton;

mod controller;
mod mapping;

pub use controller::*;

pub(crate) type MappingFn = fn(InputEvent) -> Vec<(RetroPadButton, i16)>;

use evdev_rs::enums::{EventCode, EventType, EV_ABS, EV_KEY};
use evdev_rs::{Device, DeviceWrapper, InputEvent};
use log::{trace, warn};

use gamepie_libretrobind::enums::RetroPadButton;

use crate::MappingFn;

// Mappings are defined as a function from an input event to a list of
// RetroPad button values.
// Currently these are hard-coded but would be better as configuration
// files if supporting more varied controllers.

pub(crate) fn get_mapping(device: &Device) -> Option<MappingFn> {
    let vid = device.vendor_id();
    let pid = device.product_id();
    match device.name() {
        Some(name) => trace!("Input device: '{}'", name),
        None => trace!("Input device: UNNAMED"),
    }
    trace!("Input device: {:#04x}:{:#04x}", vid, pid);

    match (vid, pid) {
        (0x45e, 0x2e0) => Some(map_8bitdo),
        (0x20d6, 0xa711) => Some(map_switchwired),
        _ => None,
    }
}

fn map_switchwired(event: InputEvent) -> Vec<(RetroPadButton, i16)> {
    let mut result = Vec::new();
    if event.is_type(&EventType::EV_KEY) {
        let id = match event.event_code {
            EventCode::EV_KEY(key) => match key {
                EV_KEY::BTN_C => Some(RetroPadButton::A),
                EV_KEY::BTN_EAST => Some(RetroPadButton::B),
                EV_KEY::BTN_NORTH => Some(RetroPadButton::X),
                EV_KEY::BTN_SOUTH => Some(RetroPadButton::Y),
                EV_KEY::BTN_Z => Some(RetroPadButton::R),
                EV_KEY::BTN_TR => Some(RetroPadButton::R2),
                EV_KEY::BTN_START => Some(RetroPadButton::R3),
                EV_KEY::BTN_WEST => Some(RetroPadButton::L),
                EV_KEY::BTN_TL => Some(RetroPadButton::L2),
                EV_KEY::BTN_SELECT => Some(RetroPadButton::L3),
                EV_KEY::BTN_TL2 => Some(RetroPadButton::Select),
                EV_KEY::BTN_TR2 => Some(RetroPadButton::Start),
                EV_KEY::BTN_THUMBL => Some(RetroPadButton::Select),
                EV_KEY::BTN_MODE => Some(RetroPadButton::Start),
                _ => {
                    warn!("Unexpected key: {:?}", key);
                    None
                }
            },
            _ => {
                warn!("Key event with mismatched code: {:?}", event);
                None
            }
        };
        let value = match event.value.try_into() {
            Ok(v) => Some(v),
            Err(_) => {
                warn!("Input value out of range");
                None
            }
        };
        if let (Some(id), Some(val)) = (id, value) {
            result.push((id, val));
        }
    } else if event.is_type(&EventType::EV_ABS) {
        match event.event_code {
            EventCode::EV_ABS(abs) => match abs {
                EV_ABS::ABS_HAT0Y => match event.value {
                    -1 => {
                        result.push((RetroPadButton::Up, 1));
                        result.push((RetroPadButton::Down, 0));
                    }
                    0 => {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 0));
                    }
                    1 => {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 1));
                    }
                    _ => {
                        warn!("Unexpected axis value: {}", event.value);
                    }
                },
                EV_ABS::ABS_HAT0X => match event.value {
                    -1 => {
                        result.push((RetroPadButton::Left, 1));
                        result.push((RetroPadButton::Right, 0));
                    }
                    0 => {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 0));
                    }
                    1 => {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 1));
                    }
                    _ => {
                        warn!("Unexpected axis value: {}", event.value);
                    }
                },
                EV_ABS::ABS_X => {
                    // Axis is from 0 to 255
                    let upper_bits = (event.value >> 6) & 0x3;
                    if upper_bits == 0 {
                        result.push((RetroPadButton::Left, 1));
                        result.push((RetroPadButton::Right, 0));
                    } else if upper_bits == 3 {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 1));
                    } else {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 0));
                    }
                }
                EV_ABS::ABS_Y => {
                    let upper_bits = (event.value >> 6) & 0x3;
                    if upper_bits == 0 {
                        result.push((RetroPadButton::Up, 1));
                        result.push((RetroPadButton::Down, 0));
                    } else if upper_bits == 3 {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 1));
                    } else {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 0));
                    }
                }
                EV_ABS::ABS_Z => {
                    // Z-Axis unused
                }
                EV_ABS::ABS_RZ => {
                    // RZ-Axis unused
                }
                _ => {
                    warn!("Unexpected axis event: {:?}", event);
                }
            },
            _ => {
                warn!("Key event with mismatched code: {:?}", event);
            }
        }
    } else if event.is_type(&EventType::EV_SYN) || event.is_type(&EventType::EV_MSC) {
        // SYN/MSC unused
    } else {
        warn!("Event: {:?}", event);
    }
    result
}

fn map_8bitdo(event: InputEvent) -> Vec<(RetroPadButton, i16)> {
    let mut result = Vec::new();
    if event.is_type(&EventType::EV_KEY) {
        let id = match event.event_code {
            EventCode::EV_KEY(key) => match key {
                EV_KEY::BTN_TR => Some(RetroPadButton::Start),
                EV_KEY::BTN_TL => Some(RetroPadButton::Select),
                EV_KEY::BTN_EAST => Some(RetroPadButton::A),
                EV_KEY::BTN_SOUTH => Some(RetroPadButton::B),
                EV_KEY::BTN_WEST => Some(RetroPadButton::L),
                EV_KEY::BTN_Z => Some(RetroPadButton::R),
                EV_KEY::BTN_NORTH => Some(RetroPadButton::X),
                EV_KEY::BTN_C => Some(RetroPadButton::Y),
                _ => {
                    warn!("Unexpected key: {:?}", key);
                    None
                }
            },
            _ => {
                warn!("Key event with mismatched code: {:?}", event);
                None
            }
        };
        let value = match event.value.try_into() {
            Ok(v) => Some(v),
            Err(_) => {
                warn!("Input value out of range");
                None
            }
        };
        if let (Some(id), Some(val)) = (id, value) {
            result.push((id, val));
        }
    } else if event.is_type(&EventType::EV_ABS) {
        match event.event_code {
            EventCode::EV_ABS(abs) => match abs {
                EV_ABS::ABS_Y => match event.value {
                    0 => {
                        result.push((RetroPadButton::Up, 1));
                        result.push((RetroPadButton::Down, 0));
                    }
                    32768 => {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 0));
                    }
                    65535 => {
                        result.push((RetroPadButton::Up, 0));
                        result.push((RetroPadButton::Down, 1));
                    }
                    _ => {
                        warn!("Unexpected axis value: {}", event.value);
                    }
                },
                EV_ABS::ABS_X => match event.value {
                    0 => {
                        result.push((RetroPadButton::Left, 1));
                        result.push((RetroPadButton::Right, 0));
                    }
                    32768 => {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 0));
                    }
                    65535 => {
                        result.push((RetroPadButton::Left, 0));
                        result.push((RetroPadButton::Right, 1));
                    }
                    _ => {
                        warn!("Unexpected axis value: {}", event.value);
                    }
                },
                _ => {
                    warn!("Unexpected axis event: {:?}", event);
                }
            },
            _ => {
                warn!("Key event with mismatched code: {:?}", event);
            }
        }
    } else if event.is_type(&EventType::EV_SYN) || event.is_type(&EventType::EV_MSC) {
        // SYN/MSC unused
    } else {
        match event.event_type() {
            Some(t) => warn!("Event type '{}' unexpected", t),
            None => warn!("Event with no type: {:?}", event),
        }
    }
    result
}

pub(crate) fn map_empty(_: InputEvent) -> Vec<(RetroPadButton, i16)> {
    Vec::with_capacity(0)
}

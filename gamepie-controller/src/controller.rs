use evdev_rs::{Device, DeviceWrapper, InputEvent, ReadFlag};
use glob::glob;
use log::{error, info, trace, warn};
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

use gamepie_libretrobind::enums::RetroPadButton;

use crate::mapping::{get_mapping, map_empty};

pub struct Controller {
    device: Option<Device>,
    keys: HashMap<RetroPadButton, i16>,
    mapping: fn(InputEvent) -> Vec<(RetroPadButton, i16)>,
}

impl Controller {
    pub fn new() -> Self {
        let mut controller = Self::empty();

        if !controller.try_get_controller() {
            warn!("No input device");
        }

        controller
    }

    fn try_get_controller(&mut self) -> bool {
        trace!("Trying to find controllers");
        let mut found = false;
        let mut devices = Vec::new();
        if let Ok(g) = glob("/dev/input/event*") {
            for d in g.flatten() {
                devices.push(d);
            }
        }

        let mut options = OpenOptions::new();
        options.read(true);
        options.custom_flags(libc::O_NONBLOCK);
        for dev in &devices {
            if let Ok(f) = options.open(dev) {
                if let Ok(d) = Device::new_from_file(f) {
                    let mapping = get_mapping(&d);
                    match mapping {
                        Some(map) => {
                            match d.name() {
                                Some(name) => info!("Input device: '{}'", name),
                                None => info!("Input device: UNNAMED"),
                            }

                            info!(
                                "Input device: {:#04x}:{:#04x}",
                                d.vendor_id(),
                                d.product_id()
                            );

                            self.device = Some(d);
                            self.mapping = map;

                            found = true;
                            break;
                        }
                        None => {
                            trace!("No mapping for: {:?}", dev);
                        }
                    }
                }
            }
        }

        found
    }

    fn empty() -> Self {
        Controller {
            device: None,
            keys: HashMap::new(),
            mapping: map_empty,
        }
    }

    pub fn input_poll(&mut self) {
        if self.device.is_none() {
            self.try_get_controller();
        }

        let mut need_to_destruct = false;

        if let Some(d) = &self.device {
            let mut try_get = true;
            while try_get {
                let ev = d.next_event(ReadFlag::NORMAL);
                match ev {
                    Ok((status, event)) => {
                        if status == evdev_rs::ReadStatus::Sync {
                            warn!("SYNC");
                        }
                        let events = (self.mapping)(event);
                        for (k, v) in events {
                            self.keys.insert(k, v);
                        }
                    }
                    Err(e) => {
                        try_get = false;
                        if let Some(os) = e.raw_os_error() {
                            if os == 19 {
                                // ENODEV
                                // Destruct and try again if device not present
                                need_to_destruct = true;
                            }
                        }
                        match e.kind() {
                            std::io::ErrorKind::WouldBlock => {}
                            _ => {
                                error!("Error kind {:?}", e.kind());
                                error!("Error {:?}", e);
                            }
                        }
                    }
                }
            }
        }

        if need_to_destruct {
            self.device = None;
        }
    }

    pub fn input_state(&self, id: RetroPadButton) -> i16 {
        if id == RetroPadButton::Mask {
            let mut result = 0;
            for (b, val) in &self.keys {
                let id = b.to_u32().expect("button u32");
                result |= val << id;
            }
            result
        } else {
            *self.keys.get(&id).unwrap_or(&0)
        }
    }
}

impl Default for Controller {
    fn default() -> Self {
        Self::new()
    }
}

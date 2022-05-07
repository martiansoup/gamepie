use lazy_static::lazy_static;
use log::{error, trace, warn};
use std::ops::DerefMut;
use std::sync::{mpsc, Mutex};

use gamepie_core::commands::AudioMsg;
use gamepie_core::portable::PString;
use gamepie_core::problem::Problem;
use gamepie_libretro::proxy::RetroProxy;
use gamepie_libretrobind::types::RetroSystemAvInfo;
use gamepie_screen::Screen;

lazy_static! {
    static ref PROXY: Mutex<Option<RetroProxy>> = Mutex::new(None);
}

pub(crate) fn with_proxy<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut RetroProxy) -> T,
{
    let mut guard = match PROXY.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for libretro proxy");
            e.into_inner()
        }
    };

    guard.deref_mut().as_mut().map(f)
}

pub(crate) fn create(
    system_dir: PString,
    screen: Option<Screen>,
    error_channel: mpsc::Sender<Problem>,
    audio_channel: mpsc::Sender<AudioMsg>,
) {
    trace!("Creating proxy object for libretro callbacks");
    let mut guard = match PROXY.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for libretro proxy");
            e.into_inner()
        }
    };
    // Take old proxy to drop if needed
    let old_proxy = (*guard).take();
    let new_screen = match old_proxy {
        Some(mut old_proxy) => {
            // If there is an old proxy, it should have a screen
            assert!(screen.is_none(), "screen passed in with existing proxy");
            old_proxy.take_screen()
        }
        None => {
            // Must take old screen
            screen
        }
    };
    let proxy = RetroProxy::new(system_dir, new_screen, error_channel, audio_channel);
    *guard = Some(proxy);
}

pub(crate) fn set_av(av: RetroSystemAvInfo) {
    trace!("Setting AV info to proxy '{:?}", av);
    let mut guard = match PROXY.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for libretro proxy");
            e.into_inner()
        }
    };
    match (*guard).as_mut() {
        Some(proxy) => {
            proxy.set_av(Some(av));
        }
        None => {
            warn!("Trying to set AV info with no proxy");
        }
    }
}

pub(crate) fn destroy() -> Option<Screen> {
    trace!("Destroying proxy object");
    let mut guard = match PROXY.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for libretro proxy");
            e.into_inner()
        }
    };
    let old_proxy = (*guard).take();
    match old_proxy {
        Some(mut old_proxy) => old_proxy.take_screen(),
        None => None,
    }
}

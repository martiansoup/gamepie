use lazy_static::lazy_static;
use log::{error, trace};
use std::sync::{mpsc, Mutex};

use gamepie_audio::Audio;
use gamepie_core::commands::{AudioMsg, ScreenToast};
use gamepie_core::problem::Problem;

lazy_static! {
    static ref AUDIO: Mutex<Option<Audio>> = Mutex::new(None);
}

pub(crate) fn get() -> mpsc::Sender<AudioMsg> {
    let mut guard = match AUDIO.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for audio proxy");
            e.into_inner()
        }
    };

    match (*guard).as_ref() {
        Some(audio) => audio.get_sender(),
        None => {
            let dummy = Audio::dummy();
            let ch = dummy.get_sender();
            *guard = Some(dummy);
            ch
        }
    }
}

pub(crate) fn try_create(overlay_tx: mpsc::Sender<ScreenToast>, error_tx: mpsc::Sender<Problem>) {
    trace!("Creating proxy object for audio");
    let mut guard = match AUDIO.lock() {
        Ok(g) => g,
        Err(e) => {
            error!("Poisoned mutex for audio proxy");
            e.into_inner()
        }
    };

    let replace = match &*guard {
        Some(audio) => !audio.is_real(),
        None => true,
    };

    if replace {
        let audio = Audio::new(overlay_tx, error_tx);
        *guard = Some(audio);
    }
}

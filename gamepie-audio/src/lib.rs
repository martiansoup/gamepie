use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};
use log::{debug, error, info, warn};
use std::error::Error;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gamepie_core::commands::{AudioCmd, AudioMsg, ScreenMessage, ScreenToast};
use gamepie_core::error::GamepieError;
use gamepie_core::problem::Problem;

pub struct Audio {
    _handle: JoinHandle<()>,
    sender: mpsc::Sender<AudioMsg>,
    real: bool,
}

// GB/NES is 44.1kHz, GBA is 32.768kHz

// Volume is inverse as this is the divisor
const VOL_DEFAULT: i16 = 4;
const VOL_MAX: i16 = 0;
const VOL_MIN: i16 = 15;

const ERROR_REPEAT_TIMEOUT: Duration = Duration::from_secs(4);
const AUDIO_ERROR_TIME: Duration = Duration::from_secs(1);

impl Audio {
    pub fn volume(v: i16) -> f32 {
        let v: f32 = v.into();
        let vmax: f32 = VOL_MAX.into();
        let vmin: f32 = VOL_MIN.into();
        let new_v = (v - vmax) / (vmin - vmax);
        1.0 - new_v
    }

    fn problem() -> Problem {
        Problem::warn(ScreenToast::new(
            ScreenMessage::AudioIssue,
            AUDIO_ERROR_TIME,
            Rgb565::RED,
        ))
    }

    fn send_error_check(
        problem: Problem,
        last_error: &mut Option<Instant>,
        tx: &mpsc::Sender<Problem>,
    ) {
        let should_send = match last_error {
            None => true,
            Some(time) => time.elapsed() > ERROR_REPEAT_TIMEOUT,
        };
        if should_send {
            *last_error = Some(Instant::now());

            if tx.send(problem).is_err() {
                // As this is just the audio channel, don't handle erors by
                // stopping the thread, as it will naturally end when all
                // transmitters are dropped.
                error!("Can't send to error channel");
            }
        }
    }

    fn audio_thread(
        rx: mpsc::Receiver<AudioMsg>,
        overlay_tx: mpsc::Sender<ScreenToast>,
        error_tx: mpsc::Sender<Problem>,
    ) -> Result<(), Box<dyn Error>> {
        let mut last_error = None;

        let sdl = sdl2::init()?;
        let subsys = sdl.audio()?;

        let mut device: Option<sdl2::audio::AudioQueue<i16>> = None;
        let mut volume = VOL_DEFAULT;

        while let Ok(msg) = rx.recv() {
            match msg {
                AudioMsg::Command(cmd) => match cmd {
                    AudioCmd::Start(freq) => {
                        if let Some(d) = &device {
                            d.pause();
                            d.clear();
                            Self::send_error_check(Self::problem(), &mut last_error, &error_tx);
                            warn!("Audio started but device already exists");
                        }
                        info!("Creating audio device: {} Hz", freq);
                        let new_desired = sdl2::audio::AudioSpecDesired {
                            freq: Some(freq),
                            channels: Some(2),
                            samples: Some(2048),
                        };
                        match subsys.open_queue::<i16, _>(None, &new_desired) {
                            Ok(new_device) => {
                                info!("Got audio device: {} Hz", new_device.spec().freq);
                                new_device.resume();
                                device = Some(new_device);
                            }
                            Err(e) => {
                                Self::send_error_check(Self::problem(), &mut last_error, &error_tx);
                                error!("Couldn't initialise audio queue: {}", e)
                            }
                        }
                    }
                    AudioCmd::Stop => {
                        match &device {
                            Some(device) => {
                                device.pause();
                                device.clear();
                            }
                            None => {
                                Self::send_error_check(Self::problem(), &mut last_error, &error_tx);
                                warn!("Audio stopped but no device present");
                            }
                        }
                        device = None;
                    }
                    AudioCmd::VolumeDown => {
                        let new_volume = volume + 1;
                        volume = std::cmp::min(VOL_MIN, new_volume);
                        if overlay_tx
                            .send(ScreenToast::info(ScreenMessage::VolumeDown(Self::volume(
                                volume,
                            ))))
                            .is_err()
                        {
                            warn!("Failed to send volume popup");
                        }
                        debug!("Volume set to {}", volume);
                    }
                    AudioCmd::VolumeUp => {
                        let new_volume = volume - 1;
                        volume = std::cmp::max(VOL_MAX, new_volume);
                        if overlay_tx
                            .send(ScreenToast::info(ScreenMessage::VolumeUp(Self::volume(
                                volume,
                            ))))
                            .is_err()
                        {
                            warn!("Failed to send volume popup");
                        }
                        debug!("Volume set to {}", volume);
                    }
                },
                AudioMsg::Data(data) => match &device {
                    Some(device) => {
                        let mut new_vec = Vec::new();
                        for d in data {
                            new_vec.push(d >> volume);
                        }
                        if device.queue_audio(new_vec.as_ref()).is_err() {
                            Self::send_error_check(Self::problem(), &mut last_error, &error_tx);
                            warn!("Failed to queue audio");
                        }
                    }
                    None => {
                        Self::send_error_check(Self::problem(), &mut last_error, &error_tx);
                        error!("Audio data provided before initialised");
                    }
                },
            }
        }
        Ok(())
    }

    pub fn new(overlay_tx: mpsc::Sender<ScreenToast>, error_tx: mpsc::Sender<Problem>) -> Self {
        let (tx, rx) = mpsc::channel::<AudioMsg>();
        let handle = std::thread::spawn(move || {
            match Self::audio_thread(rx, overlay_tx, error_tx.clone()) {
                Ok(_) => {
                    info!("Audio queue closed cleanly");
                }
                Err(e) => {
                    error!("Audio thread error: {}", e);
                    if error_tx
                        .send(Problem::fatal(GamepieError::NoAudio))
                        .is_err()
                    {
                        // As this is just the audio channel, don't handle erors by
                        // stopping the thread, as it will naturally end when all
                        // transmitters are dropped.
                        error!("Can't send to error channel");
                    }
                }
            }
        });

        Audio {
            _handle: handle,
            sender: tx,
            real: true,
        }
    }

    pub fn dummy() -> Self {
        let (tx, rx) = mpsc::channel::<AudioMsg>();
        let handle = std::thread::spawn(move || while rx.recv().is_ok() {});

        Audio {
            _handle: handle,
            sender: tx,
            real: false,
        }
    }

    pub fn is_real(&self) -> bool {
        self.real
    }

    pub fn get_sender(&self) -> mpsc::Sender<AudioMsg> {
        self.sender.clone()
    }
}

use embedded_graphics::pixelcolor::Rgb565;
use log::{debug, warn};
use std::fmt::Display;
use std::time::{Duration, Instant};

pub enum ScreenMessage {
    VolumeUp(f32),
    VolumeDown(f32),
    AudioIssue,
    Unstable,
    VideoIssue,
    Message(String),
}

impl Display for ScreenMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ScreenMessage::VolumeUp(v) => write!(f, "volume up {:.1}", v),
            ScreenMessage::VolumeDown(v) => write!(f, "volume down {:.1}", v),
            ScreenMessage::AudioIssue => write!(f, "audio problem"),
            ScreenMessage::VideoIssue => write!(f, "video problem"),
            ScreenMessage::Unstable => write!(f, "unstable"),
            ScreenMessage::Message(m) => write!(f, "'{}'", m),
        }
    }
}

pub struct ScreenToast {
    message: ScreenMessage,
    duration: Duration,
    start: Instant,
    colour: Rgb565,
}

impl ScreenToast {
    pub fn new(message: ScreenMessage, duration: Duration, colour: Rgb565) -> Self {
        ScreenToast {
            message,
            duration,
            start: Instant::now(),
            colour,
        }
    }

    pub fn info(message: ScreenMessage) -> Self {
        ScreenToast {
            message,
            duration: crate::INFO_DURATION,
            start: Instant::now(),
            colour: crate::INFO_COLOUR,
        }
    }

    pub fn error(message: ScreenMessage) -> Self {
        ScreenToast {
            message,
            duration: crate::ERROR_DURATION,
            start: Instant::now(),
            colour: crate::ERROR_COLOUR,
        }
    }

    pub fn elapsed(&self) -> bool {
        let diff = Instant::now() - self.start;
        diff > self.duration
    }

    pub fn message(&self) -> &ScreenMessage {
        &self.message
    }

    pub fn colour(&self) -> &Rgb565 {
        &self.colour
    }

    pub fn log(&self) {
        match &self.message {
            ScreenMessage::VolumeUp(_) => {
                debug!("{}", self);
            }
            ScreenMessage::VolumeDown(_) => {
                debug!("{}", self);
            }
            ScreenMessage::AudioIssue => {
                warn!("{}", self);
            }
            ScreenMessage::VideoIssue => {
                warn!("{}", self);
            }
            ScreenMessage::Unstable => {
                warn!("{}", self);
            }
            ScreenMessage::Message(_) => {
                debug!("{}", self);
            }
        }
    }
}

impl Display for ScreenToast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.message.fmt(f)
    }
}

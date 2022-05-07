use std::error::Error;
use std::fmt::Display;

#[derive(Clone, Copy, Debug)]
pub enum GamepieError {
    /// No games found
    NoGames,
    /// Error loading game into emulator
    GameLoadError,
    /// No core found that supports game
    NoCore,
    /// Internal System Error
    System,
    /// Corrupted (wrong length) save data
    MismatchSave,
    /// Unsupported video mode
    UnsupportedVideo,
    /// Audio error
    NoAudio,
    /// Video error
    NoVideo,
    /// String error
    String,
}

impl Display for GamepieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            GamepieError::NoGames => write!(f, "no games found"),
            GamepieError::GameLoadError => write!(f, "game load error"),
            GamepieError::NoCore => write!(f, "no compatible core"),
            GamepieError::System => write!(f, "internal system error"),
            GamepieError::MismatchSave => write!(f, "mismatched save"),
            GamepieError::UnsupportedVideo => write!(f, "unsupported video"),
            GamepieError::NoAudio => write!(f, "audio error"),
            GamepieError::NoVideo => write!(f, "video error"),
            GamepieError::String => write!(f, "string error"),
        }
    }
}

impl Error for GamepieError {}

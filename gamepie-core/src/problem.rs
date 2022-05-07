use crate::commands::ScreenToast;
use crate::error::GamepieError;

pub enum Problem {
    Fatal(GamepieError),
    Warn(ScreenToast),
}

impl Problem {
    pub fn fatal(e: GamepieError) -> Problem {
        Problem::Fatal(e)
    }

    pub fn warn(s: ScreenToast) -> Problem {
        Problem::Warn(s)
    }
}

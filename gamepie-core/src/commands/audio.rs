/// Commands to control an audio channel
pub enum AudioCmd {
    /// Start the audio channel at the specified frequency
    Start(i32),
    VolumeUp,
    VolumeDown,
    /// Stop the audio channel
    Stop,
}

/// The format of a message to the audio channel, which can contain a command
/// or audio samples.
pub enum AudioMsg {
    Command(AudioCmd),
    Data(Vec<i16>),
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("FFmpeg: {0}")]
    Ffmpeg(#[from] ffmpeg_next::Error),
    #[error("FFmpeg {call} failed with code {code}")]
    FfmpegCall { call: &'static str, code: i32 },
    #[error("no {0} encoder is available")]
    NoEncoder(&'static str),
    #[error("the frame is in a buffer this encoder cannot read")]
    WrongBuffer,
    #[error("no encoder on this machine can record these frames")]
    NoUsableEncoder,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Turns a negative FFmpeg return code into an error.
pub(crate) fn check(code: i32, call: &'static str) -> Result<i32, RecordError> {
    if code < 0 {
        Err(RecordError::FfmpegCall { call, code })
    } else {
        Ok(code)
    }
}

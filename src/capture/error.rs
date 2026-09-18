use std::time::Duration;
use thiserror::Error;

/// Errors for the opt-in capture API. The legacy WindowsCaptureError is unchanged.
#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("No window has the requested title")]
    WindowNotFound,
    #[error("The title matches {0} windows; select a specific HWND")]
    AmbiguousWindowTitle(usize),
    #[error("Invalid capture argument: {0}")]
    InvalidInput(&'static str),
    #[error("Capture target is no longer available")]
    TargetClosed,
    #[error("Windows Graphics Capture is not supported in this session")]
    Unsupported,
    #[error("No capture frame arrived within {0:?}")]
    CaptureTimeout(Duration),
    #[error("Capture frame channel closed before a frame arrived")]
    FrameChannelClosed,
    #[error("Invalid capture frame: {0}")]
    InvalidFrame(&'static str),
    #[error("Windows capture operation failed")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, CaptureError>;

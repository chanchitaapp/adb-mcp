use thiserror::Error;

#[derive(Error, Debug)]
pub enum AdbError {
    #[error("ADB command failed: {0}")]
    CommandFailed(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::error::Error),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, AdbError>;

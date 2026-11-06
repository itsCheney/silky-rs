use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{format}: {message}")]
    Invalid {
        format: &'static str,
        message: String,
    },
    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn invalid<T>(format: &'static str, message: impl Into<String>) -> Result<T> {
    Err(Error::Invalid {
        format,
        message: message.into(),
    })
}

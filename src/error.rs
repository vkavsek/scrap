use thiserror::Error;
use tokio::sync::mpsc;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("The selector you are trying to scrape for is missing. Selector: {0}")]
    ParseMissingSelector(String),

    #[error("Io Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tokio Join Error, couldn't await a task! {0}")]
    RuntimeJoin(#[from] tokio::task::JoinError),
    #[error("Couldn't send a page through a channel.")]
    RuntimeSendError,

    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl From<mpsc::error::SendError<Vec<(String, String)>>> for Error {
    fn from(_value: mpsc::error::SendError<Vec<(String, String)>>) -> Self {
        Error::RuntimeSendError
    }
}

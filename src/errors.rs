use derive_more::{Display, From};

pub type Result<T> = anyhow::Result<T, Error>;

#[derive(Debug, From, Display)]
pub enum Error {
    #[from]
    Custom(String),

    InvalidAddress(String),

    ServerReceiverNotFound,

    #[from]
    IO(std::io::Error),
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::Custom(value.to_string())
    }
}

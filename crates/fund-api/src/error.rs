use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("insufficient shares to redeem")]
    Insufficient,
    #[error("invalid fund code: {0}")]
    InvalidCode(String),
    #[error("no data stored for fund code {0}")]
    NoData(String),
    #[error("unknown strategy: {0}")]
    UnknownStrategy(String),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("failed to parse response: {0}")]
    Parse(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid date: {0}")]
    InvalidDate(#[from] chrono::ParseError),
    #[error("wasm strategy error: {0}")]
    Wasm(String),
}

impl From<wasmtime::Error> for Error {
    fn from(err: wasmtime::Error) -> Self {
        Error::Wasm(err.to_string())
    }
}

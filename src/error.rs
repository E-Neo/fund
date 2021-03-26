use derive_more::{Display, Error, From};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Display, Error, From)]
pub enum Error {
    Insufficient,
    Overflow,
    #[from]
    IO(std::io::Error),
}

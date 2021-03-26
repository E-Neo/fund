use derive_more::{Display, Error};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Display, Error, PartialEq)]
pub enum Error {
    Insufficient,
    Overflow,
}

use crate::kind::NBTKind;
use serde::{de, ser};
use std::{
    fmt::{self, Display},
    io,
};
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    Message(String),
    Eof,
    ExpectedRootCompound,
    InvalidTagId,
    MismatchedTag(NBTKind, NBTKind),
    ExpectedBooleanByte(i8),
    Unrepresentable,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::MismatchedTag(received, expected) => formatter.write_fmt(format_args!(
                "Expected {} tag but received {}",
                expected, received
            )),
            Error::ExpectedBooleanByte(byte) => {
                formatter.write_fmt(format_args!("Expected a boolean value but got {}", byte))
            }
            Error::Eof => formatter.write_str("unexpected end of input"),
            _ => todo!("Fill out errors: {}", self),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::IoError(value)
    }
}

impl std::error::Error for Error {}

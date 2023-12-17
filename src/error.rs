use std::{
    fmt::{self, Display},
    io,
};

use serde::{de, ser};

use crate::deserializer::NBTKind;

pub type Result<T> = std::result::Result<T, NBTError>;

#[derive(Debug)]
pub enum NBTError {
    IoError(io::Error),
    Message(String),
    Eof,
    ExpectedRootCompound,
    InvalidTagId,
    MismatchedTag(NBTKind, NBTKind),
    ExpectedBooleanByte(i8),

    ExpectedBoolean,
    ExpectedInteger,
    ExpectedString,
    ExpectedNull,
    ExpectedArray,
    ExpectedArrayComma,
    ExpectedArrayEnd,
    ExpectedMap,
    ExpectedMapColon,
    ExpectedMapComma,
    ExpectedMapEnd,
    ExpectedEnum,
    TrailingCharacters,
}

impl ser::Error for NBTError {
    fn custom<T: Display>(msg: T) -> Self {
        NBTError::Message(msg.to_string())
    }
}

impl de::Error for NBTError {
    fn custom<T: Display>(msg: T) -> Self {
        NBTError::Message(msg.to_string())
    }
}

impl Display for NBTError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NBTError::Message(msg) => formatter.write_str(msg),
            NBTError::MismatchedTag(received, expected) => formatter.write_fmt(format_args!(
                "Expected {} tag but received {}",
                expected, received
            )),
            NBTError::ExpectedBooleanByte(byte) => {
                formatter.write_fmt(format_args!("Expected a boolean value but got {}", byte))
            }
            NBTError::Eof => formatter.write_str("unexpected end of input"),
            _ => todo!("Fill out errors: {}", self),
        }
    }
}

impl From<io::Error> for NBTError {
    fn from(value: io::Error) -> Self {
        NBTError::IoError(value)
    }
}

impl std::error::Error for NBTError {}

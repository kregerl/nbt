use std::fmt::{Display, self};

use serde::{ser, de};

pub type Result<T> = std::result::Result<T, NBTError>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug)]
pub enum NBTError {
    // One or more variants that can be created by data structures through the
    // `ser::Error` and `de::Error` traits. For example the Serialize impl for
    // Mutex<T> might return an error because the mutex is poisoned, or the
    // Deserialize impl for a struct may return an error because a required
    // field is missing.
    Message(String),

    // Zero or more variants that can be created directly by the Serializer and
    // Deserializer without going through `ser::Error` and `de::Error`. These
    // are specific to the format, in this case JSON.
    Eof,
    Syntax,
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
            NBTError::Eof => formatter.write_str("unexpected end of input"),
            _ => todo!("Fill out errors: {}", self)
        }
    }
}

impl std::error::Error for NBTError {}
mod debug;
mod kind;
mod parser;
mod writer;

pub mod tag;
pub mod error;
pub mod de;
pub mod ser;

pub use error::{Error, Result};
pub use de::{from_gzip_reader, from_reader, from_slice, from_zlib_reader};
pub use ser::{to_writer, to_bytes, byte_array, int_array, long_array};
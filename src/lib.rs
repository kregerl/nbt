mod error;
mod kind;
mod de;
mod ser;

pub use error::{Error, Result};
pub use de::{from_reader, from_slice, from_gzip_reader, from_zlib_reader};
use crate::kind::NBTKind;
use byteorder::ReadBytesExt;
use std::io;

// Macro for generating parsing function implementations of number types
macro_rules! parse_number_types {
    ($($typ:ident),+) => {
        paste::item! {
            $(pub(crate)  fn [<parse_ $typ>](&mut self) -> io::Result<$typ> {
                self.reader.[<read_ $typ>]::<byteorder::BigEndian>()
            })*
        }
    };
}

pub(crate) struct Parser<R: io::Read> {
    reader: R,
}

impl<R: io::Read> Parser<R> {
    pub(crate) fn new(reader: R) -> Self {
        Self { reader }
    }

    parse_number_types!(i16, i32, i64, f32, f64);

    pub(crate) fn parse_kind(&mut self) -> io::Result<NBTKind> {
        Ok(NBTKind::from(self.reader.read_u8()?))
    }

    pub(crate) fn parse_string(&mut self) -> io::Result<String> {
        // The first byte in a tag is the tag type (ID)
        // (Note TAG_End is not named and does not contain the extra 2 bytes;
        // the name is assumed to be empty).
        // followed by a two byte big-endian unsigned integer for the length of the name
        let name_length = self.reader.read_u16::<byteorder::BigEndian>()?;
        let mut buffer = vec![0u8; name_length as usize];
        self.reader.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    // Separated from the number type macro since a single byte does not have an endianess.
    pub(crate) fn parse_i8(&mut self) -> io::Result<i8> {
        self.reader.read_i8()
    }
}

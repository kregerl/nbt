use std::{
    collections::HashMap,
    fs,
    io::{self, Cursor, Read},
};

use byteorder::ReadBytesExt;

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum NBTKind {
    End,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    List,
    Compound,
    IntArray,
    LongArray,
}

impl From<u8> for NBTKind {
    fn from(value: u8) -> Self {
        match value {
            0 => NBTKind::End,
            1 => NBTKind::Byte,
            2 => NBTKind::Short,
            3 => NBTKind::Int,
            4 => NBTKind::Long,
            5 => NBTKind::Float,
            6 => NBTKind::Double,
            7 => NBTKind::ByteArray,
            8 => NBTKind::String,
            9 => NBTKind::List,
            10 => NBTKind::Compound,
            11 => NBTKind::IntArray,
            12 => NBTKind::LongArray,
            _ => unreachable!("Unknown ID value for NBTTag {}.", value),
        }
    }
}

#[derive(Debug)]
enum NBTPayload {
    Empty,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NBTPayload>),
    Compound(HashMap<String, NBTPayload>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[derive(Debug)]
pub struct NBTTag {
    kind: NBTKind,
    name: Option<String>,
    payload: NBTPayload,
}

pub struct NBTReader {
    cursor: Cursor<Vec<u8>>,
}

impl NBTReader {
    pub fn new(filename: &str) -> io::Result<Self> {
        let cursor = Cursor::new(fs::read(filename)?);
        Ok(Self { cursor })
    }

    fn has_bytes_left(&self) -> bool {
        let len = self.cursor.get_ref().len();
        (self.cursor.position() as usize) < len - 1
    }

    pub fn parse_nbt(&mut self) -> Option<NBTTag> {
        if !self.has_bytes_left() {
            None
        } else {
            match self.parse_nbt_tag() {
                Ok(tag) => Some(tag),
                Err(e) => panic!("Error reading tag {}", e),
            }
        }
    }

    // Reads the elements of an array of type T
    fn parse_array<T>(
        &mut self,
        element_type: fn(&mut NBTReader) -> io::Result<T>,
    ) -> io::Result<Vec<T>> {
        let length = self.cursor.read_u32::<byteorder::BigEndian>()?;
        let mut array = Vec::with_capacity(length as usize);
        for _ in 0..length {
            array.push(element_type(self)?);
        }
        Ok(array)
    }

    fn parse_nbt_payload(&mut self, kind: &NBTKind) -> io::Result<NBTPayload> {
        Ok(match kind {
            // A single signed byte
            NBTKind::Byte => NBTPayload::Byte(self.cursor.read_i8()?),
            // A single signed, big endian 16 bit integer
            NBTKind::Short => NBTPayload::Short(self.cursor.read_i16::<byteorder::BigEndian>()?),
            // A single signed, big endian 32 bit integer
            NBTKind::Int => NBTPayload::Int(self.cursor.read_i32::<byteorder::BigEndian>()?),
            // A single signed, big endian 64 bit integer
            NBTKind::Long => NBTPayload::Long(self.cursor.read_i64::<byteorder::BigEndian>()?),
            // A single, big endian IEEE-754 single-precision floating point number (NaN possible)
            NBTKind::Float => NBTPayload::Float(self.cursor.read_f32::<byteorder::BigEndian>()?),
            // A single, big endian IEEE-754 double-precision floating point number (NaN possible)
            NBTKind::Double => NBTPayload::Double(self.cursor.read_f64::<byteorder::BigEndian>()?),
            // A length-prefixed array of signed bytes. The prefix is a signed integer (thus 4 bytes)
            NBTKind::ByteArray => {
                let bytes = self.parse_array(|reader| reader.cursor.read_i8())?;
                NBTPayload::ByteArray(bytes)
            }
            // A length-prefixed modified UTF-8 string. The prefix is an unsigned short (thus 2 bytes) signifying the length of the string in bytes
            NBTKind::String => {
                let str_len = self.cursor.read_u16::<byteorder::BigEndian>()?;
                let mut str_bytes = vec![0u8; str_len as usize];
                self.cursor.read_exact(&mut str_bytes)?;
                NBTPayload::String(String::from_utf8(str_bytes).unwrap())
            }
            // A list of nameless tags, all of the same type.
            // The list is prefixed with the Type ID of the items it contains (thus 1 byte),
            // and the length of the list as a signed integer (a further 4 bytes).
            // If the length of the list is 0 or negative, the type may be 0 (TAG_End) but otherwise it
            // must be any other type. (The notchian implementation uses TAG_End in that situation,
            // but another reference implementation by Mojang uses 1 instead; parsers should accept any type
            // if the length is <= 0).
            NBTKind::List => {
                let list_nbt_type = NBTKind::from(self.cursor.read_u8()?);
                let length = self.cursor.read_i32::<byteorder::BigEndian>()?;
                let mut payload = Vec::with_capacity(length as usize);
                for _ in 0..length {
                    let tag_value = self.parse_nbt_payload(&list_nbt_type)?;
                    payload.push(tag_value)
                }

                NBTPayload::List(payload)
            }
            // Effectively a list of named tags. Order is not guaranteed.
            NBTKind::Compound => {
                let mut map: HashMap<String, NBTPayload> = HashMap::new();
                loop {
                    let tag = self.parse_nbt_tag()?;
                    if let NBTKind::End = tag.kind {
                        break;
                    }
                    // TODO: Is it possible to have a nameless tag here?
                    map.insert(tag.name.unwrap(), tag.payload);
                }
                NBTPayload::Compound(map)
            }
            // A length-prefixed array of signed integers. The prefix is a signed integer (thus 4 bytes) and indicates the number of 4 byte integers.
            NBTKind::IntArray => {
                let ints =
                    self.parse_array(|reader| reader.cursor.read_i32::<byteorder::BigEndian>())?;
                NBTPayload::IntArray(ints)
            }
            // A length-prefixed array of signed longs. The prefix is a signed integer (thus 4 bytes) and indicates the number of 8 byte longs.
            NBTKind::LongArray => {
                let longs =
                    self.parse_array(|reader| reader.cursor.read_i64::<byteorder::BigEndian>())?;
                NBTPayload::LongArray(longs)
            }
            _ => NBTPayload::Empty,
        })
    } 

    fn parse_nbt_tag(&mut self) -> io::Result<NBTTag> {
        // The first byte in a tag is the tag type (ID)
        let kind = NBTKind::from(self.cursor.read_u8()?);
        let name = if let NBTKind::End = kind {
            // (Note TAG_End is not named and does not contain the extra 2 bytes; the name is assumed to be empty).
            String::new()
        } else {
            // // followed by a two byte big-endian unsigned integer for the length of the name
            let name_length = self.cursor.read_u16::<byteorder::BigEndian>()?;
            let mut buffer = vec![0u8; name_length as usize];
            self.cursor.read_exact(&mut buffer)?;
            String::from_utf8(buffer).unwrap()
        };

        let payload = self.parse_nbt_payload(&kind)?;

        Ok(NBTTag {
            kind,
            name: Some(name),
            payload,
        })
    }
}

use std::{
    collections::HashMap,
    fs,
    io::{self, Cursor, Read},
    thread::current, fmt::Display, f32::consts::PI,
};

use crate::error::{self, NBTError};
use byteorder::ReadBytesExt;
use serde::{
    de::{self, MapAccess},
    forward_to_deserialize_any, Deserialize, Deserializer,
};

pub struct NBTDeserializer<'de> {
    cursor: Cursor<&'de [u8]>,
    size: u64,
    // bytes: &'de [u8],
}

#[derive(Debug)]
enum NBTTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NBTTag>),
    Compound(HashMap<String, NBTTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

#[repr(u8)]
#[derive(Debug, PartialEq, Copy, Clone)]
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

impl Display for NBTKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", self))
    }
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

impl<'de> NBTDeserializer<'de> {
    fn has_bytes_left(&self) -> bool {
        let len = self.cursor.get_ref().len();
        (self.cursor.position() as usize) < len.saturating_sub(1)
    }

    // Reads the elements of an array of type T
    fn parse_array<T>(
        &mut self,
        element_type: fn(&mut NBTDeserializer) -> io::Result<T>,
    ) -> io::Result<Vec<T>> {
        let length = self.cursor.read_i32::<byteorder::BigEndian>()?;
        let mut array = Vec::with_capacity(length as usize);
        for _ in 0..length {
            array.push(element_type(self)?);
        }
        Ok(array)
    }

    fn parse_kind(&mut self) -> io::Result<NBTKind> {
        Ok(NBTKind::from(self.cursor.read_u8()?))
    }

    fn parse_string(&mut self) -> io::Result<String> {
        // The first byte in a tag is the tag type (ID)
        // (Note TAG_End is not named and does not contain the extra 2 bytes;
        // the name is assumed to be empty).
        // followed by a two byte big-endian unsigned integer for the length of the name
        let name_length = self.cursor.read_u16::<byteorder::BigEndian>()?;
        let mut buffer = vec![0u8; name_length as usize];
        self.cursor.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer).unwrap())
    }

    fn parse_nbt_tag(&mut self, kind: NBTKind) -> io::Result<NBTTag> {
        let tag = match kind {
            // A single signed byte
            NBTKind::Byte => NBTTag::Byte(self.cursor.read_i8()?),
            // A single signed, big endian 16 bit integer
            NBTKind::Short => NBTTag::Short(self.cursor.read_i16::<byteorder::BigEndian>()?),
            // A single signed, big endian 32 bit integer
            NBTKind::Int => NBTTag::Int(self.cursor.read_i32::<byteorder::BigEndian>()?),
            // A single signed, big endian 64 bit integer
            NBTKind::Long => NBTTag::Long(self.cursor.read_i64::<byteorder::BigEndian>()?),
            // A single, big endian IEEE-754 single-precision floating point number (NaN possible)
            NBTKind::Float => NBTTag::Float(self.cursor.read_f32::<byteorder::BigEndian>()?),
            // A single, big endian IEEE-754 double-precision floating point number (NaN possible)
            NBTKind::Double => NBTTag::Double(self.cursor.read_f64::<byteorder::BigEndian>()?),
            // A length-prefixed array of signed bytes. The prefix is a signed integer (thus 4 bytes)
            NBTKind::ByteArray => {
                let bytes = self.parse_array(|reader| reader.cursor.read_i8())?;
                NBTTag::ByteArray(bytes)
            }
            // A length-prefixed modified UTF-8 string. The prefix is an unsigned short (thus 2 bytes) signifying the length of the string in bytes
            NBTKind::String => {
                let s = self.parse_string()?;
                NBTTag::String(s)
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
                    let tag_value = self.parse_nbt_tag(list_nbt_type)?;
                    payload.push(tag_value)
                }

                NBTTag::List(payload)
            }
            // // Effectively a list of named tags. Order is not guaranteed.
            // NBTKind::Compound => {
            //     let mut map: HashMap<String, NBTTag> = HashMap::new();
            //     loop {
            //         let kind = self.parse_kind()?;
            //         let tag = self.parse_nbt_tag(kind)?;
            //         if let NBTTag::End = tag {
            //             break;
            //         }
            //         let name = self.parse_string()?;
            //         // TODO: Is it possible to have a nameless tag here?
            //         map.insert(name, tag);
            //     }
            //     NBTTag::Compound(map)
            // }
            // A length-prefixed array of signed integers. The prefix is a signed integer (thus 4 bytes) and indicates the number of 4 byte integers.
            NBTKind::IntArray => {
                let ints =
                    self.parse_array(|reader| reader.cursor.read_i32::<byteorder::BigEndian>())?;
                NBTTag::IntArray(ints)
            }
            // A length-prefixed array of signed longs. The prefix is a signed integer (thus 4 bytes) and indicates the number of 8 byte longs.
            NBTKind::LongArray => {
                let longs =
                    self.parse_array(|reader| reader.cursor.read_i64::<byteorder::BigEndian>())?;
                NBTTag::LongArray(longs)
            }
            _ => NBTTag::End,
        };

        Ok(tag)
    }
}

impl<'de> NBTDeserializer<'de> {
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        NBTDeserializer {
            cursor: Cursor::new(bytes),
            size: bytes.len() as u64,
        }
    }
}

pub fn from_slice<'a, T>(s: &'a [u8]) -> error::Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = NBTDeserializer::from_slice(s);
    T::deserialize(&mut deserializer)
}

#[derive(Debug, Deserialize)]
struct Server {
    ip: String,
    name: String,
}

#[test]
fn test() {
    let filename = "test.dat";
    let bytes = fs::read(filename).unwrap();
    let x: Server = from_slice(&bytes).unwrap();
    println!("Here: {:#?}", x)
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut NBTDeserializer<'de> {
    type Error = NBTError;

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string bytes byte_buf
        unit seq tuple_struct tuple option enum identifier ignored_any
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(NBTError::ExpectedRootCompound)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if !self.has_bytes_left() {
            return Err(NBTError::Eof);
        }

        let kind = self.parse_kind()?;
        if let NBTKind::Compound = kind {
            let _ = self.parse_string()?;
            visitor.visit_map(NBTMapDeserializer::new(self))
        } else {
            Err(NBTError::ExpectedRootCompound)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}

struct NBTMapDeserializer<'de, 'a> {
    outer: &'a mut NBTDeserializer<'de>,
    kind: Option<NBTKind>,
}

impl<'de, 'a> NBTMapDeserializer<'de, 'a> {
    fn new(outer: &'a mut NBTDeserializer<'de>) -> Self {
        Self { outer, kind: None }
    }
}

impl<'de, 'a> MapAccess<'de> for NBTMapDeserializer<'de, 'a> {
    type Error = NBTError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let kind = self.outer.parse_kind()?;

        if let NBTKind::End = kind {
            return Ok(None);
        }

        self.kind = Some(kind);

        let mut de_impl = NBTDeserializerImpl::new(self.outer, NBTKind::String);

        Ok(Some(seed.deserialize(&mut de_impl)?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.kind {
            Some(kind) => {
                let mut de_impl = NBTDeserializerImpl::new(self.outer, kind);
                Ok(seed.deserialize(&mut de_impl)?)
            }
            None => unreachable!("Cannot get the next value seed if the kind is 'None'"),
        }
    }
}

struct NBTSeqDeserializer<'de, 'a> {
    outer: &'a mut NBTDeserializer<'de>,
    kind: NBTKind,
    length: i32,
    current_pos: i32
}

impl<'de, 'a> NBTSeqDeserializer<'de, 'a> {
    fn from_list(outer: &'a mut NBTDeserializer) -> io::Result<Self> {
        // let kind = outer.parse_kind()?;
        // let tag = outer.parse_nbt_tag(kind)?;
        todo!();
    

    }
}

struct NBTDeserializerImpl<'de, 'a> {
    outer: &'a mut NBTDeserializer<'de>,
    kind: NBTKind,
}

impl<'de, 'a> NBTDeserializerImpl<'de, 'a> {
    pub fn new(outer: &'a mut NBTDeserializer<'de>, kind: NBTKind) -> Self {
        Self { outer, kind }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut NBTDeserializerImpl<'de, 'a> {
    type Error = NBTError;

    forward_to_deserialize_any! {
        u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string bytes byte_buf seq
        map tuple_struct struct tuple enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let tag = self.outer.parse_nbt_tag(self.kind)?;

        match tag {
            NBTTag::Byte(value) => visitor.visit_i8(value),
            NBTTag::Short(value) => visitor.visit_i16(value),
            NBTTag::Int(value) => visitor.visit_i32(value),
            NBTTag::Long(value) => visitor.visit_i64(value),
            NBTTag::Float(value) => visitor.visit_f32(value),
            NBTTag::Double(value) => visitor.visit_f64(value),
            NBTTag::ByteArray(_) => todo!(),
            NBTTag::String(value) => visitor.visit_string(value),
            NBTTag::List(_) => todo!(),
            NBTTag::Compound(_) => todo!(),
            NBTTag::IntArray(_) => todo!(),
            NBTTag::LongArray(_) => todo!(),
            _ => Err(NBTError::InvalidTagId),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let tag = self.outer.parse_nbt_tag(self.kind)?;
        match tag {
            NBTTag::Byte(value) => {
                match value {
                    0 => visitor.visit_bool(false),
                    1 => visitor.visit_bool(true),
                    _ => Err(NBTError::ExpectedBooleanByte(value))
                }
            },
            _ => Err(NBTError::MismatchedTag(self.kind, NBTKind::Byte))
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
}

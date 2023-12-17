use std::io::{self, Cursor, Read};

use crate::{
    error::{self, NBTError},
    kind::NBTKind,
};
use byteorder::ReadBytesExt;
use serde::{
    de::{self, MapAccess, SeqAccess},
    forward_to_deserialize_any, Deserialize,
};

// Wrapper deserializeer that consumes the nameless root compound NBT tag
pub struct NBTDeserializer<'de> {
    cursor: Cursor<&'de [u8]>,
}

// Macro for generating parsing function implementations of number types
macro_rules! parse_number_types {
    ($($typ:ident),+) => {
        paste::item! {
            $(fn [<parse_ $typ>](&mut self) -> io::Result<$typ> {
                self.cursor.[<read_ $typ>]::<byteorder::BigEndian>()
            })*
        }
    };
}

impl<'de> NBTDeserializer<'de> {
    parse_number_types!(i16, i32, i64, f32, f64);

    fn has_bytes_left(&self) -> bool {
        let len = self.cursor.get_ref().len();
        (self.cursor.position() as usize) < len.saturating_sub(1)
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

    // Separated from the number type macro since a single byte does not have an endianess.
    fn parse_i8(&mut self) -> io::Result<i8> {
        self.cursor.read_i8()
    }
}

impl<'de> NBTDeserializer<'de> {
    pub fn from_slice(bytes: &'de [u8]) -> Self {
        NBTDeserializer {
            cursor: Cursor::new(bytes),
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
        // File is not valid if there is no root compound NBT tag.
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
        // EOF if theres no bytes remaining.
        if !self.has_bytes_left() {
            return Err(NBTError::Eof);
        }

        // Error if there is no root compound NBT tag
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

/// Deserializer for compound NBT tags.
/// Holds the outer NBT deserializer since thats where all the parsing functions are.
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

        // Save the kind so 'next_value_seed' can get it.
        self.kind = Some(kind);

        // Treat the key of the compound NBT tag as a string
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

/// Deserializes a compount NBT tag
struct NBTSeqDeserializer<'de, 'a> {
    outer: &'a mut NBTDeserializer<'de>,
    kind: NBTKind,
    length: i32,
    current_pos: i32,
}

impl<'de, 'a> NBTSeqDeserializer<'de, 'a> {
    /// Creates a sequence deserializer for a NBT list where the type is defined as part of the list
    fn from_list(outer: &'a mut NBTDeserializer<'de>) -> io::Result<Self> {
        let kind = outer.parse_kind()?;
        let length = outer.parse_i32()?;
        Ok(Self {
            outer,
            kind,
            length,
            current_pos: 0,
        })
    }

    /// Creates a sequence deserializer for a NBT array of type `kind`
    fn from_array(outer: &'a mut NBTDeserializer<'de>, kind: NBTKind) -> io::Result<Self> {
        let length = outer.parse_i32()?;
        Ok(Self {
            outer,
            kind,
            length,
            current_pos: 0,
        })
    }
}

impl<'de, 'a> SeqAccess<'de> for NBTSeqDeserializer<'de, 'a> {
    type Error = NBTError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.current_pos == self.length {
            return Ok(None);
        }

        // Deserialize the next element in the list/array
        let mut de_impl = NBTDeserializerImpl::new(self.outer, self.kind);
        let value = seed.deserialize(&mut de_impl)?;
        self.current_pos += 1;
        Ok(Some(value))
    }
}

/// Actual implementation of deserializing NBT tags
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
        match self.kind {
            // A single signed byte
            NBTKind::Byte => visitor.visit_i8(self.outer.parse_i8()?),
            // A single signed, big endian 16 bit integer
            NBTKind::Short => visitor.visit_i16(self.outer.parse_i16()?),
            // A single signed, big endian 32 bit integer
            NBTKind::Int => visitor.visit_i32(self.outer.parse_i32()?),
            // A single signed, big endian 64 bit integer
            NBTKind::Long => visitor.visit_i64(self.outer.parse_i64()?),
            // A single, big endian IEEE-754 single-precision floating point number (NaN possible)
            NBTKind::Float => visitor.visit_f32(self.outer.parse_f32()?),
            // A single, big endian IEEE-754 double-precision floating point number (NaN possible)
            NBTKind::Double => visitor.visit_f64(self.outer.parse_f64()?),
            // A length-prefixed array of signed bytes. The prefix is a signed integer (thus 4 bytes)
            NBTKind::ByteArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.outer, NBTKind::Byte)?)
            }
            // A length-prefixed modified UTF-8 string. The prefix is an unsigned short (thus 2 bytes) signifying the length of the string in bytes
            NBTKind::String => visitor.visit_string(self.outer.parse_string()?),
            // A list of nameless tags, all of the same type.
            // The list is prefixed with the Type ID of the items it contains (thus 1 byte),
            // and the length of the list as a signed integer (a further 4 bytes).
            // If the length of the list is 0 or negative, the type may be 0 (TAG_End) but otherwise it
            // must be any other type. (The notchian implementation uses TAG_End in that situation,
            // but another reference implementation by Mojang uses 1 instead; parsers should accept any type
            // if the length is <= 0).
            NBTKind::List => visitor.visit_seq(NBTSeqDeserializer::from_list(self.outer)?),
            // Effectively a list of named tags. Order is not guaranteed.
            NBTKind::Compound => visitor.visit_map(NBTMapDeserializer::new(self.outer)),
            // A length-prefixed array of signed integers. The prefix is a signed integer (thus 4 bytes) and indicates the number of 4 byte integers.
            NBTKind::IntArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.outer, NBTKind::Int)?)
            }
            // A length-prefixed array of signed longs. The prefix is a signed integer (thus 4 bytes) and indicates the number of 8 byte longs.
            NBTKind::LongArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.outer, NBTKind::Long)?)
            }
            _ => Err(NBTError::InvalidTagId),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.kind {
            NBTKind::Byte => match self.outer.parse_i8()? {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                value => Err(NBTError::ExpectedBooleanByte(value)),
            },
            _ => Err(NBTError::MismatchedTag(self.kind, NBTKind::Byte)),
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

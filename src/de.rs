use std::io::{self, Cursor};

use crate::{
    error::{self, Error},
    kind::NBTKind,
    parser::Parser,
};
use flate2::read::{GzDecoder, ZlibDecoder};
use serde::{
    de::{self, MapAccess, SeqAccess},
    forward_to_deserialize_any, Deserialize,
};

// Wrapper deserializeer that consumes the nameless root compound NBT tag
pub struct NBTDeserializer<R: io::Read> {
    parser: Parser<R>,
}

impl NBTDeserializer<Cursor<Vec<u8>>> {
    fn from_slice(bytes: Vec<u8>) -> Self {
        let reader = Cursor::new(bytes);
        NBTDeserializer {
            parser: Parser::new(reader),
        }
    }
}

impl<R: io::Read> NBTDeserializer<R> {
    fn from_reader(reader: R) -> Self {
        NBTDeserializer {
            parser: Parser::new(reader),
        }
    }
}

pub fn from_reader<'a, T, R>(s: R) -> error::Result<T>
where
    T: Deserialize<'a>,
    R: io::Read,
{
    let mut deserializer = NBTDeserializer::from_reader(s);
    T::deserialize(&mut deserializer)
}

pub fn from_gzip_reader<'a, T, R>(s: R) -> error::Result<T>
where
    T: Deserialize<'a>,
    R: io::Read,
{
    let gzip = GzDecoder::new(s);
    from_reader(gzip)
}

pub fn from_zlib_reader<'a, T, R>(s: R) -> error::Result<T>
where
    T: Deserialize<'a>,
    R: io::Read,
{
    let zlib = ZlibDecoder::new(s);
    from_reader(zlib)
}

pub fn from_slice<'a, T>(s: Vec<u8>) -> error::Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer: NBTDeserializer<Cursor<Vec<u8>>> = NBTDeserializer::from_slice(s);
    T::deserialize(&mut deserializer)
}

impl<'de, 'a, R: io::Read> serde::de::Deserializer<'de> for &'a mut NBTDeserializer<R> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string bytes byte_buf
        unit seq tuple_struct tuple option enum identifier ignored_any
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        // File is not valid if there is no root compound NBT tag.
        Err(Error::ExpectedRootCompound)
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
        // Error if there is no root compound NBT tag
        let kind = self.parser.parse_kind()?;
        if let NBTKind::Compound = kind {
            let _ = self.parser.parse_string()?;
            // Effectively a list of named tags. Order is not guaranteed.
            visitor.visit_map(NBTMapDeserializer::new(&mut self.parser))
        } else {
            Err(Error::ExpectedRootCompound)
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
        // Effectively a list of named tags. Order is not guaranteed.
        self.deserialize_map(visitor)
    }
}

/// Deserializer for compound NBT tags.
/// Holds the outer NBT deserializer since thats where all the parsing functions are.
struct NBTMapDeserializer<'a, R: io::Read> {
    parser: &'a mut Parser<R>,
    kind: Option<NBTKind>,
}

impl<'de, 'a, R: io::Read> NBTMapDeserializer<'a, R> {
    fn new(parser: &'a mut Parser<R>) -> Self {
        Self { parser, kind: None }
    }
}

impl<'de, 'a, R: io::Read> MapAccess<'de> for NBTMapDeserializer<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let kind = self.parser.parse_kind()?;

        if let NBTKind::End = kind {
            return Ok(None);
        }

        // Save the kind so 'next_value_seed' can get it.
        self.kind = Some(kind);

        // Treat the key of the compound NBT tag as a string
        let mut de_impl = NBTDeserializerImpl::new(self.parser, NBTKind::String);

        Ok(Some(seed.deserialize(&mut de_impl)?))
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.kind {
            Some(kind) => {
                let mut de_impl = NBTDeserializerImpl::new(self.parser, kind);
                Ok(seed.deserialize(&mut de_impl)?)
            }
            None => unreachable!("Cannot get the next value seed if the kind is 'None'"),
        }
    }
}

/// Deserializes a compound NBT tag
struct NBTSeqDeserializer<'a, R: io::Read> {
    parser: &'a mut Parser<R>,
    kind: NBTKind,
    length: i32,
    current_pos: i32,
}

impl<'a, R: io::Read> NBTSeqDeserializer<'a, R> {
    /// Creates a sequence deserializer for a NBT list where the type is defined as part of the list
    fn from_list(parser: &'a mut Parser<R>) -> io::Result<Self> {
        let kind = parser.parse_kind()?;
        let length = parser.parse_i32()?;
        Ok(Self {
            parser,
            kind,
            length,
            current_pos: 0,
        })
    }

    /// Creates a sequence deserializer for a NBT array of type `kind`
    fn from_array(parser: &'a mut Parser<R>, kind: NBTKind) -> io::Result<Self> {
        let length = parser.parse_i32()?;
        Ok(Self {
            parser,
            kind,
            length,
            current_pos: 0,
        })
    }
}

impl<'de, 'a, R: io::Read> SeqAccess<'de> for NBTSeqDeserializer<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.current_pos == self.length {
            return Ok(None);
        }

        // Deserialize the next element in the list/array
        let mut de_impl = NBTDeserializerImpl::new(self.parser, self.kind);
        let value = seed.deserialize(&mut de_impl)?;
        self.current_pos += 1;
        Ok(Some(value))
    }
}

/// Actual implementation of deserializing NBT tags
struct NBTDeserializerImpl<'a, R: io::Read> {
    parser: &'a mut Parser<R>,
    kind: NBTKind,
}

impl<'a, R: io::Read> NBTDeserializerImpl<'a, R> {
    pub fn new(parser: &'a mut Parser<R>, kind: NBTKind) -> Self {
        Self { parser, kind }
    }
}

impl<'de, 'a, R: io::Read> serde::de::Deserializer<'de> for &'a mut NBTDeserializerImpl<'a, R> {
    type Error = Error;

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
            NBTKind::Byte => visitor.visit_i8(self.parser.parse_i8()?),
            // A single signed, big endian 16 bit integer
            NBTKind::Short => visitor.visit_i16(self.parser.parse_i16()?),
            // A single signed, big endian 32 bit integer
            NBTKind::Int => visitor.visit_i32(self.parser.parse_i32()?),
            // A single signed, big endian 64 bit integer
            NBTKind::Long => visitor.visit_i64(self.parser.parse_i64()?),
            // A single, big endian IEEE-754 single-precision floating point number (NaN possible)
            NBTKind::Float => visitor.visit_f32(self.parser.parse_f32()?),
            // A single, big endian IEEE-754 double-precision floating point number (NaN possible)
            NBTKind::Double => visitor.visit_f64(self.parser.parse_f64()?),
            // A length-prefixed array of signed bytes. The prefix is a signed integer (thus 4 bytes)
            NBTKind::ByteArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.parser, NBTKind::Byte)?)
            }
            // A length-prefixed modified UTF-8 string. The prefix is an unsigned short (thus 2 bytes) signifying the length of the string in bytes
            NBTKind::String => visitor.visit_string(self.parser.parse_string()?),
            // A list of nameless tags, all of the same type.
            // The list is prefixed with the Type ID of the items it contains (thus 1 byte),
            // and the length of the list as a signed integer (a further 4 bytes).
            // If the length of the list is 0 or negative, the type may be 0 (TAG_End) but otherwise it
            // must be any other type. (The notchian implementation uses TAG_End in that situation,
            // but another reference implementation by Mojang uses 1 instead; parsers should accept any type
            // if the length is <= 0).
            NBTKind::List => visitor.visit_seq(NBTSeqDeserializer::from_list(self.parser)?),
            // Effectively a list of named tags. Order is not guaranteed.
            NBTKind::Compound => visitor.visit_map(NBTMapDeserializer::new(self.parser)),
            // A length-prefixed array of signed integers. The prefix is a signed integer (thus 4 bytes) and indicates the number of 4 byte integers.
            NBTKind::IntArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.parser, NBTKind::Int)?)
            }
            // A length-prefixed array of signed longs. The prefix is a signed integer (thus 4 bytes) and indicates the number of 8 byte longs.
            NBTKind::LongArray => {
                visitor.visit_seq(NBTSeqDeserializer::from_array(self.parser, NBTKind::Long)?)
            }
            _ => Err(Error::InvalidTagId),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.kind {
            NBTKind::Byte => match self.parser.parse_i8()? {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                value => Err(Error::ExpectedBooleanByte(value)),
            },
            _ => Err(Error::MismatchedTag(self.kind, NBTKind::Byte)),
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

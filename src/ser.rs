use crate::{
    error,
    error::Error,
    kind::NBTKind,
    writer::{DelayedHeader, Writer},
};
use serde::{
    ser::{
        self, SerializeMap, SerializeSeq, SerializeStruct, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Serialize, Serializer,
};
use std::io;

#[test]
fn test_write() {
    use crate::debug;
    use std::fs::File;
    #[derive(Debug, Serialize)]
    struct Server {
        ip: String,
        name: String,
    }

    #[derive(Debug, Serialize)]
    struct Servers {
        servers: Vec<Server>,
    }

    let file = File::create("test.nbt").unwrap();
    to_writer(
        file,
        &Servers {
            servers: vec![Server {
                ip: "loucaskreger.com".into(),
                name: "Minecraft Server".into(),
            }],
        },
    )
    .unwrap();

    debug::dump_nbt("test.nbt").unwrap();
}

pub fn to_writer<T, W>(w: W, value: &T) -> error::Result<()>
where
    T: Serialize,
    W: io::Write,
{
    let mut serializer = NBTSerializer {
        writer: Writer::new(w),
    };
    value.serialize(&mut serializer)
}

pub fn to_bytes<T>(value: &T) -> error::Result<Vec<u8>>
where
    T: Serialize,
{
    let mut result = Vec::new();
    let mut serializer = NBTSerializer {
        writer: Writer::new(&mut result)
    };
    value.serialize(&mut serializer)?;
    Ok(result)
}

struct NBTSerializer<W: io::Write> {
    writer: Writer<W>,
}

macro_rules! unrepresentable {
    ($name:ident, $typ:ty) => {
        fn $name(self, _: $typ) -> Result<Self::Ok, Self::Error> {
            Err(Error::Unrepresentable)
        }
    };
}

macro_rules! no_root_compound {
    ($name:ident, $typ:ty) => {
        fn $name(self, _: $typ) -> Result<Self::Ok, Self::Error> {
            Err(Error::ExpectedRootCompound)
        }
    };
}

impl<'a, W: io::Write> Serializer for &'a mut NBTSerializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = NBTMapSerializer<'a, W>;
    type SerializeStruct = NBTStructSerializer<'a, W>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    unrepresentable!(serialize_bool, bool);
    unrepresentable!(serialize_u8, u8);
    unrepresentable!(serialize_u16, u16);
    unrepresentable!(serialize_u32, u32);
    unrepresentable!(serialize_u64, u64);
    unrepresentable!(serialize_char, char);

    no_root_compound!(serialize_i8, i8);
    no_root_compound!(serialize_i16, i16);
    no_root_compound!(serialize_i32, i32);
    no_root_compound!(serialize_i64, i64);
    no_root_compound!(serialize_f32, f32);
    no_root_compound!(serialize_f64, f64);
    no_root_compound!(serialize_str, &str);
    no_root_compound!(serialize_bytes, &[u8]);
    no_root_compound!(serialize_unit_struct, &'static str);

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.writer.write_tag_header(NBTKind::Compound, None)?;
        self.writer.write_string("")?;
        Ok(NBTMapSerializer::new(&mut self.writer))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.writer.write_tag_header(NBTKind::Compound, None)?;
        self.writer.write_string("")?;
        Ok(NBTStructSerializer::new(&mut self.writer))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(Error::ExpectedRootCompound)
    }
}

struct NBTMapSerializer<'a, W: io::Write> {
    writer: &'a mut Writer<W>,
}

impl<'a, W: io::Write> NBTMapSerializer<'a, W> {
    pub fn new(writer: &'a mut Writer<W>) -> Self {
        Self { writer }
    }
}

impl<'a, W: io::Write> SerializeMap for NBTMapSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        key.serialize(NBTSerializerImpl::from_writer(&mut self.writer))
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(NBTSerializerImpl::from_writer(&mut self.writer))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_tag_header(NBTKind::End, None)?;
        Ok(())
    }
}

struct NBTStructSerializer<'a, W: io::Write> {
    writer: &'a mut Writer<W>,
}

impl<'a, W: io::Write> NBTStructSerializer<'a, W> {
    pub fn new(writer: &'a mut Writer<W>) -> Self {
        Self { writer }
    }
}

impl<'a, W: io::Write> SerializeStruct for NBTStructSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(NBTSerializerImpl::new(
            &mut self.writer,
            Some(DelayedHeader::MapKey(key)),
        ))
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.writer.write_tag_header(NBTKind::End, None)?;
        Ok(())
    }
}

struct NBTSeqSerializer<'a, W: io::Write> {
    writer: &'a mut Writer<W>,
    delayed_header: Option<DelayedHeader>,
}

impl<'a, W: io::Write> NBTSeqSerializer<'a, W> {
    pub fn new(writer: &'a mut Writer<W>, delayed_header: Option<DelayedHeader>) -> Self {
        Self {
            writer,
            delayed_header,
        }
    }

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        value.serialize(NBTSerializerImpl::new(
            &mut self.writer,
            self.delayed_header,
        ))
    }
}

impl<'a, W: io::Write> SerializeSeq for NBTSeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: io::Write> SerializeTuple for NBTSeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: io::Write> SerializeTupleStruct for NBTSeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, W: io::Write> SerializeTupleVariant for NBTSeqSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_element(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

struct NBTSerializerImpl<'a, W: io::Write> {
    writer: &'a mut Writer<W>,
    delayed_header: Option<DelayedHeader>,
}

impl<'a, W: io::Write> NBTSerializerImpl<'a, W> {
    pub fn from_writer(writer: &'a mut Writer<W>) -> Self {
        Self {
            writer,
            delayed_header: None,
        }
    }

    pub fn new(writer: &'a mut Writer<W>, delayed_header: Option<DelayedHeader>) -> Self {
        Self {
            writer,
            delayed_header,
        }
    }
}

impl<'a, W: io::Write> Serializer for NBTSerializerImpl<'a, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = NBTSeqSerializer<'a, W>;
    type SerializeTuple = NBTSeqSerializer<'a, W>;
    type SerializeTupleStruct = NBTSeqSerializer<'a, W>;
    type SerializeTupleVariant = NBTSeqSerializer<'a, W>;
    type SerializeMap = NBTMapSerializer<'a, W>;
    type SerializeStruct = NBTStructSerializer<'a, W>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    unrepresentable!(serialize_u8, u8);
    unrepresentable!(serialize_u16, u16);
    unrepresentable!(serialize_u32, u32);
    unrepresentable!(serialize_u64, u64);
    unrepresentable!(serialize_char, char);

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.serialize_i8(v as i8)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Byte, self.delayed_header)?;
        self.writer.write_i8(v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Short, self.delayed_header)?;
        self.writer.write_i16(v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Int, self.delayed_header)?;
        self.writer.write_i32(v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::List, self.delayed_header)?;
        self.writer.write_i64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Float, self.delayed_header)?;
        self.writer.write_f32(v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Double, self.delayed_header)?;
        self.writer.write_f64(v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::String, self.delayed_header)?;
        self.writer.write_string(v)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(Error::Unrepresentable)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(Error::Unrepresentable)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::End, self.delayed_header)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(Error::Unrepresentable)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let length = match len {
            Some(len) => len,
            None => 0,
        };
        self.serialize_tuple(length)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::List, self.delayed_header)?;
        if len == 0 {
            self.writer.write_tag_header(NBTKind::End, None)?;
            self.writer.write_i32(0)?;
            Ok(NBTSeqSerializer::new(self.writer, None))
        } else {
            let header = DelayedHeader::List(len);
            Ok(NBTSeqSerializer::new(self.writer, Some(header)))
        }
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(NBTMapSerializer::new(self.writer))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.writer
            .write_tag_header(NBTKind::Compound, self.delayed_header)?;
        Ok(NBTStructSerializer::new(self.writer))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(Error::Unrepresentable)
    }
}

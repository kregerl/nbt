use std::io;

use byteorder::WriteBytesExt;

use crate::{error, kind::NBTKind};

#[derive(Copy, Clone)]
pub(crate) enum DelayedHeader {
    MapKey(&'static str),
    List(usize),
}

pub(crate) struct Writer<W: io::Write> {
    writer: W,
}

impl<W: io::Write> Writer<W> {
    pub(crate) fn new(writer: W) -> Self {
        Self { writer }
    }

    pub(crate) fn write_tag_header(
        &mut self,
        kind: NBTKind,
        delayed_header: Option<DelayedHeader>,
    ) -> error::Result<()> {
        self.writer.write_u8(kind.header_byte())?;
        if let Some(header) = delayed_header {
            match header {
                DelayedHeader::MapKey(key) => self.write_string(key)?,
                DelayedHeader::List(length) => self.write_i32(length as i32)?,
            }
        }
        Ok(())
    }

    pub(crate) fn write_i8(&mut self, n: i8) -> error::Result<()> {
        self.writer.write_i8(n)?;
        Ok(())
    }

    pub(crate) fn write_u16(&mut self, n: u16) -> error::Result<()> {
        self.writer.write_u16::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_i16(&mut self, n: i16) -> error::Result<()> {
        self.writer.write_i16::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_i32(&mut self, n: i32) -> error::Result<()> {
        self.writer.write_i32::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_i64(&mut self, n: i64) -> error::Result<()> {
        self.writer.write_i64::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_f32(&mut self, n: f32) -> error::Result<()> {
        self.writer.write_f32::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_f64(&mut self, n: f64) -> error::Result<()> {
        self.writer.write_f64::<byteorder::BigEndian>(n)?;
        Ok(())
    }

    pub(crate) fn write_string(&mut self, string: &str) -> error::Result<()> {
        self.write_u16(string.len() as u16)?;
        self.writer.write(string.as_bytes())?;
        Ok(())
    }
}

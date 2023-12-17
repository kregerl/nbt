use std::fmt::Display;

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

impl Display for NBTKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", self))
    }
}
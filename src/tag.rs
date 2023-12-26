use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct NBTTag {
    title: String,
    payload: BTreeMap<String, NBTValue>,
}

impl NBTTag {
    pub fn new(title: Option<String>) -> Self {
        let title = match title {
            Some(title) => title,
            None => "".to_owned(),
        };

        Self {
            title,
            payload: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum NBTValue {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NBTValue>),
    Compound(BTreeMap<String, NBTValue>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

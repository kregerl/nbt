use std::{
    fs,
    io::{self, Read},
    marker::PhantomData,
};

use flate2::bufread::{GzDecoder, ZlibDecoder};
use serde::{de, Deserialize};

use crate::{debug, de::from_slice};

#[derive(Debug)]
struct ChunkInfo {
    // Offset of where the chunk is located in the file.
    chunk_offset_bytes: usize,
    // Size of the chunk.
    size: usize,
    // Timestamp of the last time the chunk was modified.
    timestamp: u32,
}

#[derive(Debug)]
struct ChunkHeader {
    length: u32,
    compression_scheme: CompressionScheme,
}

impl From<[u8; 5]> for ChunkHeader {
    fn from(value: [u8; 5]) -> Self {
        Self {
            length: u32::from_be_bytes(value[0..4].try_into().unwrap()),
            compression_scheme: CompressionScheme::from(value[4]),
        }
    }
}

#[derive(Debug)]
enum CompressionScheme {
    Gzip,
    Zlib,
}

impl From<u8> for CompressionScheme {
    fn from(value: u8) -> Self {
        match value {
            1 => CompressionScheme::Gzip,
            2 => CompressionScheme::Zlib,
            _ => unreachable!("Unknown compression scheme {}", value),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Chunk {
    #[serde(rename = "DataVersion")]
    data_version: i32,
    #[serde(rename = "Entities")]
    entities: Vec<Entity>,
    #[serde(rename = "Position")]
    position: [i8; 2],
}

#[derive(Debug, Deserialize)]
struct Entity {
    #[serde(rename = "Air")]
    air: i16,
    #[serde(rename = "FallDistance")]
    fall_distance: f32,
    #[serde(rename = "Fire")]
    fire: i16,
    #[serde(rename = "Invulnerable")]
    invulnerable: i8,
    #[serde(rename = "LootTable")]
    loot_table: Option<String>,
    #[serde(rename = "LootTableSeed")]
    loot_table_seed: Option<i64>,
    #[serde(rename = "Motion")]
    motion: Vec<f64>,
    #[serde(rename = "OnGround")]
    on_ground: i8,
    #[serde(rename = "PortalCooldown")]
    portal_cooldown: i32,
    #[serde(rename = "Pos")]
    position: Vec<f64>,
    #[serde(rename = "Rotation")]
    rotation: Vec<f32>,
    #[serde(rename = "UUID")]
    uuid: [i32; 4],
    id: String,
}

pub fn parse_mca(filename: &str) {
    let bytes = fs::read(filename).unwrap();

    let mut chunks = Vec::new();
    const CHUNK_SIZE: usize = 4096;
    // The first 8KiB of the MCA file is the header which contains the location and timestamp tables for each chunk.
    for (byte_offset, chunk_bytes) in bytes[0..CHUNK_SIZE].chunks(4).enumerate() {
        let int_offset = byte_offset * 4;
        let chunk_offset = u32::from_be_bytes([0, chunk_bytes[0], chunk_bytes[1], chunk_bytes[2]]);
        let size = chunk_bytes[3];
        // If chunk offset and size are 0 then the chunk hasn't been generated yet.
        if chunk_offset != 0 && size != 0 {
            // Should always be a 4 byte timestamp.
            let timestamp_bytes = &bytes[(CHUNK_SIZE + int_offset)..(CHUNK_SIZE + int_offset + 4)];
            let timestamp = u32::from_be_bytes(timestamp_bytes.try_into().expect(&format!(
                "Only expected 4 bytes but got {}",
                timestamp_bytes.len()
            )));
            chunks.push(ChunkInfo {
                chunk_offset_bytes: (chunk_offset as usize) * CHUNK_SIZE,
                size: (size as usize) * CHUNK_SIZE,
                timestamp,
            });
        }
    }
    for chunk in chunks {
        // Read first 5 bytes as chunk header
        let mut current_offset = chunk.chunk_offset_bytes;
        let header_bytes: [u8; 5] = bytes[current_offset..current_offset + 5]
            .try_into()
            .unwrap();
        current_offset += 5;
        // Parse chunk header into meaningful parts
        let header = ChunkHeader::from(header_bytes);
        println!("Chunk: {:#?}", chunk);
        println!("Header: {:#?}", header);
        // Read from chunk header to chunk_header + chunk_length
        let nbt_bytes = &bytes[current_offset..current_offset + header.length as usize];
        let mut decompressed = Vec::new();
        // Decode using the specified compression method
        let mut reader = decompress_bytes_with_scheme(nbt_bytes, header.compression_scheme);
        reader.read_to_end(&mut decompressed).unwrap();

        let x: Chunk = from_slice(decompressed).unwrap();
        println!("Chunk: {:#?}", x);
    }
}

fn decompress_bytes_with_scheme<'a>(bytes: &'a [u8], compression_scheme: CompressionScheme) -> Box<dyn Read + 'a>{
    match compression_scheme {
        CompressionScheme::Gzip => Box::new(GzDecoder::new(bytes)),
        CompressionScheme::Zlib => Box::new(ZlibDecoder::new(bytes)),
    }
}

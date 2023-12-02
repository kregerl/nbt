mod deserializer;
mod error;
mod nbtreader;
use crate::nbtreader::NBTReader;

fn main() {
    let filename = "servers.dat";
    let mut stream = NBTReader::new(filename).unwrap();
    println!("Tag: {:#?}", stream.parse_nbt());
    // println!("Tag: {:#?}", stream.parse_nbt());
    // println!("Tag: {:#?}", stream.parse_nbt());
    // println!("Tag: {:#?}", stream.parse_nbt());
}

mod deserializer;
mod error;
mod nbtreader;
use crate::nbtreader::NBTReader;

fn main() {
    let filename = "test.dat";
    // let filename = "r.0.0.mca";
    let mut stream = NBTReader::new(filename).unwrap();
    println!("Tag: {:#?}", stream.parse_nbt());
}

// mod deserializer;
// mod error;

// fn main() {
//     println!("Hello world");
// }
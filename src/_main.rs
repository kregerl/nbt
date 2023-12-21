// mod debug;
// mod de;
// mod error;
// mod kind;
// mod mca;
// use std::fs;

// use serde::Deserialize;

// use crate::de::from_slice;

// #[derive(Debug, Deserialize)]
// struct Server {
//     ip: String,
//     name: String,
// }

// #[derive(Debug, Deserialize)]
// struct Servers {
//     servers: Vec<Server>,
// }

// fn main() {
//     // debug::dump_nbt("player.dat").unwrap();

//     let filename = "servers.dat";
//     let bytes = fs::read(filename).unwrap();
//     let x: Servers = from_slice(bytes).unwrap();
//     println!("Here: {:#?}", x);

//     // let cursor = Cursor::new(encoded_bytes);
//     // let mut x = read::ZlibDecoder::new(cursor);
//     // let mut new_bytes = Vec::new();
//     // x.read_to_end(&mut new_bytes).unwrap();
//     // debug::dump_nbt_from_bytes(new_bytes).unwrap();
//     let filename = "r.0.0.mca";
//     // let filename = "region.mca";
//     mca::parse_mca(filename);

//     // let bytes = fs::read(filename).unwrap();
//     // let mut i = 0;
//     // for location in bytes[0..4096].chunks(4) {
//     //     let mut offset = [0u8; 4];
//     //     offset[1] = location[0];
//     //     offset[2] = location[1];
//     //     offset[3] = location[2];
//     //     let size = location[3];
//     //     if offset.iter().any(|x| *x != 0) {
//     //         println!("Offset: {:#?}", u32::from_be_bytes(offset));
//     //         println!("Size: {}", size);
//     //         let x = &bytes[(4096 + i)..(4096 + i+ 4)];
//     //         let mut offset2 = [0u8; 4];
//     //         offset2[0] = x[0];
//     //         offset2[1] = x[1];
//     //         offset2[2] = x[2];
//     //         offset2[3] = x[3];
//     //         let timestamp = u32::from_be_bytes(offset2);
//     //         println!("timestamp: {}", timestamp);
//     //     }
//     //     i += 4;
//     // }
// }

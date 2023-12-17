mod debug;
mod deserializer;
mod error;
mod kind;
use std::fs;

use serde::Deserialize;

use crate::deserializer::from_slice;

#[derive(Debug, Deserialize)]
struct Server {
    ip: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct Servers {
    servers: Vec<Server>,
}

fn main() {
    // let filename = "r.0.0.mca";
    debug::dump_nbt("level.dat").unwrap();

    let filename = "servers.dat";
    let bytes = fs::read(filename).unwrap();
    let x: Servers = from_slice(&bytes).unwrap();
    println!("Here: {:#?}", x)
}

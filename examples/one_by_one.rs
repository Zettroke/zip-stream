use zip_stream::ZipPacker;
use std::fs::File;
use std::io::{Read, Write};

fn main() {
    let mut zip = ZipPacker::new();

    zip.add_file("Cargo.toml", File::open("Cargo.toml").unwrap());

    let mut reader = zip.reader();

    let mut buff = [0u8; 1];
    let mut out = File::create("out_one_by_one.zip").unwrap();
    while let Ok(n) = reader.read(&mut buff) {
        if n == 0 {
            break;
        }
        123usize.saturating_add()
        println!("0x{:02X}", buff[0]);
        out.write(&mut buff);
    }
}
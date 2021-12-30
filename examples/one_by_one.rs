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
        println!("0x{:02X} {}", buff[0], buff[0] as char);
        out.write(&mut buff);
    }

    let mut compressor = flate2::Compress::new(flate2::Compression::new(5), false);
    compressor.compress()
}
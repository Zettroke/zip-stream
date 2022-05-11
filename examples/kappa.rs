extern crate bytes;

// use zip_stream::{ZipPacker, ZipEntry};
use std::fs::File;
use std::io::Write;
use zip::CompressionMethod::Deflated;
use zip::write::FileOptions;


use zip_stream::compressor::deflate::DeflateConfig;
use zip_stream::writer::ZipWriter;

fn main() {
    let mut out = Vec::new();
    let mut writer = ZipWriter::new(&mut out);

    {
        let mut file_writer = writer
            .start_file("test/test_kappa")
            .compression(DeflateConfig::default())
            .writer()
            .unwrap();

        std::io::copy(&mut "basically very smol file".as_bytes(), &mut file_writer).unwrap();

        file_writer.finish().unwrap();

    }

    writer.finish().unwrap();
}

fn main1() {

    // panic!("Move file to RAM, do not rape your SSD!!!");
    let mut writer = ZipWriter::new(File::create("test_out_zip_stream.zip").unwrap());

    // writer.append_file("test_big", File::open("Cargo.lock").unwrap()).unwrap();
    writer.start_file("test_big").compression(DeflateConfig::default()).write_data(File::open("Cargo.lock").unwrap()).unwrap();
    let mut data =  b"test_data".iter().cycle();
    writer.append("kappa/keeepo", b"test data" as &[u8]);

    writer.finish().unwrap();


    let mut writer = zip::ZipWriter::new(File::create("test_out_zip.zip").unwrap());

    writer.start_file("test_big", FileOptions::default().compression_method(Deflated));

    // writer.write()
    std::io::copy(&mut "basically very smol file".as_bytes(), &mut writer);

    writer.finish();

    return;

    // let mut zip = ZipPacker::new();
    //
    // // zip.add_file(ZipEntry::new("Cargo.toml", File::open("Cargo.toml").unwrap()));
    // // zip.add_file(ZipEntry::new("Cargo.lock", File::open("Cargo.lock").unwrap()));
    // // zip.add_file(ZipEntry::new("examples/kappa.rs", File::open("examples/kappa.rs").unwrap()));
    // zip.add_file("zip-stream.zip", File::open("zip-stream.zip").unwrap());
    //
    // let mut zip = zip.reader();
    //
    // let mut out = File::create("out.zip").unwrap();
    //
    //
    // let start = std::time::Instant::now();
    //
    // let mut buff = [0u8; 256*1024];
    // while let Ok(n) = zip.read(&mut buff) {
    //     if n > 0 {
    //         out.write_all(&buff[..n]);
    //     } else {
    //         break;
    //     }
    // }
    //
    // // let res = std::io::copy(&mut zip, &mut out);
    // let end = std::time::Instant::now();
    // // println!("{:?}", res);
    // println!("{}", (end - start).as_secs_f64());
}
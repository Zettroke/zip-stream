#![allow(dead_code)]

use std::io::{Cursor, Write};
use zip::CompressionMethod;
use zip::write::FileOptions;
use zip_stream::ZipWriter;

fn generate_data() -> Vec<String> {
    (0..10000).map(|n| n.to_string()).collect::<Vec<_>>()
}


fn actual_zip(out: &mut Cursor<Vec<u8>>, data: &Vec<String>) {
    let mut writer = zip::ZipWriter::new(out);
    // for v in data.iter() {
        writer.start_file("test_kappa", FileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();
        writer.write_all(data[0].as_bytes()).unwrap();
    // }

    writer.finish().unwrap();
}

fn actual_zip_stream(out: &mut Cursor<Vec<u8>>, data: &Vec<String>) {
    let mut writer = ZipWriter::new(out);
    // for v in data.iter() {
        writer.start_file("test_kappa").write_all(data[0].as_bytes()).unwrap();
    // }

    writer.finish().unwrap();
}

fn main() {
    let data = generate_data();
    let mut out = Cursor::new(Vec::new());

    for _ in 0..10_000_000 {
        out.set_position(0);
        // let mut writer = ZipWriter::new(&mut out);
        // for v in data.iter() {
        //     writer.append("test_kappa", v.as_bytes()).unwrap();
        // }
        //
        // writer.finish().unwrap();


        // actual_zip_stream(&mut out, &data);
        actual_zip(&mut out, &data);
    }
}
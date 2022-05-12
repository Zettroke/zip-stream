# Streaming zip writer

```rust
fn main() {
    let mut writer = ZipWriter::new(Vec::new());

    writer.append_data("test_file", b"basically very smol file").unwrap();

    let _out = writer.finish().unwrap();
}
```

```rust
fn main() {
    let mut writer = ZipWriter::new(Vec::new());

    writer
        .start_file("test_file")
        .modification(std::time::SystemTime::now()) // <- time feature
        .write_all(b"basically very smol file")
        .unwrap();

    let _out = writer.finish().unwrap();
}
```

```rust
fn main() {
    let writer = ZipWriter::new(Cursor::new(Vec::new()));

    let mut file_writer = writer.start_file_writer("test").writer().unwrap();
    file_writer.write_all(data).unwrap();
    
    writer = file_writer.finish().unwrap();
    let _out = writer.finish().unwrap();
}
```
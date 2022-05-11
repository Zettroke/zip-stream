# Streaming zip writer

```rust
fn main() {
    let mut out = Vec::new();
    let mut writer = ZipWriter::new(&mut out);

    writer.start_file("test_file").write_all("basically very smol file".as_bytes()).unwrap();

    writer.finish().unwrap();
}
```

```rust
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
    }

    writer.finish().unwrap();
}
```
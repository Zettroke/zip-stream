use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::io::{Cursor, Write};
use zip::CompressionMethod;
use zip_stream::ZipWriter;

use zip::write::{FileOptions, ZipWriter as ZipWriterExternal};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group1 = c.benchmark_group("zip_stream");
    let mut buff = Cursor::new(Vec::new());
    for files_num in [100u32, 1_000, 10_000, 100_000] {
        group1.bench_function(BenchmarkId::new("many_files", files_num), |b| {
            b.iter(|| {
                buff.set_position(0);

                let mut writer = ZipWriter::new(&mut buff);
                for i in 0..files_num {
                    writer
                        .append(i.to_string(), &mut "Test".as_bytes())
                        .unwrap();
                }

                writer.finish().unwrap();
            });
        });
    }
    group1.finish();

    let mut group2 = c.benchmark_group("zip");
    for files_num in [100u32, 1_000, 10_000, 100_000] {
        group2.bench_function(BenchmarkId::new("many_files", files_num), |b| {
            b.iter(|| {
                buff.set_position(0);

                let mut writer = ZipWriterExternal::new(&mut buff);

                for i in 0..files_num {
                    writer
                        .start_file(
                            i.to_string(),
                            FileOptions::default().compression_method(CompressionMethod::Stored),
                        )
                        .unwrap();

                    writer.write_all("Test".as_bytes()).unwrap();
                }

                writer.finish().unwrap();
            });
        });
    }

    group2.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

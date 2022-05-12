#![feature(test)]

use std::fs::File;
use std::io::{Read, Result, Write};
use std::marker::PhantomData;

#[cfg(feature="time")]
use {
    std::time::SystemTime,
    time::OffsetDateTime
};


use crate::compressor::{Compressor, CompressorConfig, HashWriteWrapper, Store, WriterWrapper, WriterWrapperOwned};
pub use crate::zip_impl::{Header, ZipWriter};

mod zip_impl;
pub mod compressor;


impl<W: Write, P: AsRef<str>> ZipWriter<W, P> {
    pub fn new(write: W) -> ZipWriter<W, P> {
        Self {
            write,
            position: 0,
            entries: vec![],
        }
    }

    pub fn append_file(&mut self, path: P, file: File) -> Result<()> {
        #[cfg(feature="time")]
        self.start_file(path).modification_from_file(&file).write_data(file)?;
        #[cfg(not(feature="time"))]
        self.start_file(path).write_data(file)?;

        Ok(())
    }

    pub fn append_data(&mut self, path: P, data: &[u8]) -> Result<()> {
        self.start_file(path).write_all(data)?;

        Ok(())
    }

    pub fn append(&mut self, path: P, file: impl Read) -> Result<()> {
        self.start_file(path).write_data(file)?;

        Ok(())
    }

    pub fn start_file(&mut self, path: P) -> ZipEntryBuilder<P, ZipWriterWrapper<&mut Self, W, P>, compressor::StoreConfig>
    {
        ZipEntryBuilder {
            header: Header::builder(),
            writer: ZipWriterWrapper(self, PhantomData),
            compressor_config: compressor::StoreConfig
        }.path(path)
    }

    pub fn start_file_writer(self, path: P) -> ZipEntryBuilder<P, ZipWriterWrapper<Self, W, P>, compressor::StoreConfig>
    {
        ZipEntryBuilder {
            header: Header::builder(),
            writer: ZipWriterWrapper(self, PhantomData),
            compressor_config: compressor::StoreConfig
        }.path(path)
    }

    pub fn finish(self) -> Result<W> {
        self.write_central_directory()
    }

    /// Can return with incomplete data written in [W]
    pub fn into_inner(self) -> W {
        self.write
    }
}

/// It's needed to hide [Write] implementation on ZipWriter,
/// so user can't screw up format by writing random bytes
pub struct ZipWriterWrapper<T: AsMut<ZipWriter<W, P>>, W: Write, P: AsRef<str>>(T, PhantomData<(W, P)>);

impl<T: AsMut<ZipWriter<W, P>>, W: Write, P: AsRef<str>> Write for ZipWriterWrapper<T, W, P> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.as_mut().write.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.0.as_mut().write.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.0.as_mut().write.write_all(buf)
    }
}

impl<T: AsMut<ZipWriter<W, P>>, W: Write, P: AsRef<str>> WriterWrapper for ZipWriterWrapper<T, W, P> {
    type Inner = T;
    type Path = P;

    fn start_entry(&mut self, header: &mut Header<Self::Path>) -> Result<()> {
        self.0.as_mut().write_entry_header(header)
    }


    fn end_entry(mut self, entry: Header<Self::Path>) -> Result<Self::Inner> {
        self.0.as_mut().position += entry.compressed_size;
        self.0.as_mut().write_entry_data_descriptor(&entry)?;
        self.0.as_mut().entries.push(entry);
        Ok(self.0)
    }
}


impl<W: Write, P: AsRef<str>> WriterWrapperOwned for ZipWriterWrapper<ZipWriter<W, P>, W, P> {}


pub struct ZipFileWriter<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> {
    inner: HashWriteWrapper<C>,
    header: Header<P>
}

impl<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> ZipFileWriter<C, P, ZIP> {
    pub fn finish(self) -> Result<ZIP::Inner> {
        let (crc32, writer) = self.inner.finish();
        let (entry_data, writer) = writer.finish()?;

        let mut header = self.header;

        header.uncompressed_size = entry_data.uncompressed_size;
        header.compressed_size = entry_data.compressed_size;
        header.crc32 = crc32;

        writer.end_entry(header)
    }
}

impl<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> Write for ZipFileWriter<C, P, ZIP> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)
    }
}

pub struct ZipEntryBuilder<P: AsRef<str>, W: WriterWrapper<Path=P>, CC: CompressorConfig<W> = Store<W>> {
    header: HeaderBuilder<P>,
    compressor_config: CC,
    writer: W
}

impl<P: AsRef<str>, W: WriterWrapper<Path=P>, CC: CompressorConfig<W>> ZipEntryBuilder<P, W, CC>{
    pub fn compression<NewCC: CompressorConfig<W>>(self, compressor_config: NewCC) -> ZipEntryBuilder<P, W, NewCC> {
        ZipEntryBuilder {
            compressor_config,
            header: self.header,
            writer: self.writer
        }
    }

    pub fn path(mut self, path: P) -> Self {
        self.header.path = Some(path);
        self
    }

    #[cfg(feature="time")]
    pub fn modification(self, time: SystemTime) -> Self {
        let date_time = OffsetDateTime::from(time);
        self.modification_date_time(date_time)
    }

    #[cfg(feature="time")]
    pub fn modification_date_time(mut self, date_time: OffsetDateTime) -> Self {
        self.header.modification_time = Some(
            (date_time.second() / 2) as u16 | // 0-4 bits
                (date_time.minute() as u16) << 5 | // 5-10 bits
                (date_time.hour() as u16) << 11 // 11-15 bits
        );
        self.header.modification_date = Some(
            (date_time.day() as u16) | // 0-4 bits
                ((date_time.month() as u16) << 5) | // 5-8 bits
                ((date_time.year() - 1980) as u16) << 9 // 9-15 bits
        );

        self
    }

    #[cfg(feature="time")]
    pub fn modification_from_file(self, file: &File) -> Self {
        let modified_at = file.metadata().ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(|| SystemTime::now());

        self.modification(modified_at)
    }

    fn writer_inner(mut self) -> Result<ZipFileWriter<<CC as CompressorConfig<W>>::CompressorTarget, P, W>> {
        let mut header = self.header.build::<CC, W>();
        self.writer.start_entry(&mut header)?;

        let compressor = self.compressor_config.build(self.writer);

        Ok(ZipFileWriter {
            inner: HashWriteWrapper::new(compressor),
            header
        })
    }

    pub fn write_data(self, mut data: impl Read) -> Result<W::Inner> {
        let mut writer = self.writer_inner()?;

        std::io::copy(&mut data, &mut writer)?;

        writer.finish()
    }

    pub fn write_all(self, data: &[u8]) -> Result<W::Inner> {
        let mut writer = self.writer_inner()?;

        writer.write_all(data)?;

        writer.finish()
    }
}

/// To fix issue when user doesn't finish it's writer we implementing
/// getter only we taking ownership of [ZipWriter], so user can't continue writing without finishing writing a file.
/// It can be implemented using drop (although with some trouble), but type shenanigans is funnier
///
/// ```compile_fail
/// use zip_stream::ZipWriter;
/// let mut zip = ZipWriter::new(Vec::new());
///
/// zip.start_file_writer("test");
///
/// zip.finish();
/// ```
///
/// ```compile_fail
/// use zip_stream::ZipWriter;
/// let mut zip = ZipWriter::new(Vec::new());
///
/// zip.start_file("test").writer();
/// ```
impl<P: AsRef<str>, W: WriterWrapper<Path=P> + WriterWrapperOwned, CC: CompressorConfig<W>> ZipEntryBuilder<P, W, CC>{
    pub fn writer(self) -> Result<ZipFileWriter<<CC as CompressorConfig<W>>::CompressorTarget, P, W>> {
        self.writer_inner()
    }
}


pub struct HeaderBuilder<P: AsRef<str>> {
    path: Option<P>,
    modification_time: Option<u16>,
    modification_date: Option<u16>,
}

impl<P: AsRef<str>> HeaderBuilder<P> {

    pub fn build<CC: CompressorConfig<W>, W: WriterWrapper>(self) -> Header<P> {
        Header {
            compression_id: CC::CompressorTarget::compression_id(),
            path: self.path.unwrap(),
            modification_date: self.modification_date.unwrap_or(0),
            modification_time: self.modification_time.unwrap_or(0),
            compressed_size: 0,
            uncompressed_size: 0,
            crc32: 0,
            offset: 0,
        }
    }
}

impl<P: AsRef<str>> Header<P> {
    pub fn builder() -> HeaderBuilder<P> {
        HeaderBuilder {
            path: None,
            modification_date: None,
            modification_time: None,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use std::fs::File;
    use std::io::{Cursor, Read, repeat, Write};
    use test::Bencher;

    use zip::CompressionMethod;
    use zip::write::FileOptions;

    use crate::compressor::deflate::DeflateConfig;
    use crate::ZipWriter;

    #[cfg_attr(target_os = "linux", test)]
    #[allow(dead_code)]
    fn simple_archive_unzip() {
        let file_name = "test_archive-zip-stream.zip";
        let mut file = File::create(file_name).unwrap();
        let mut writer = ZipWriter::new(&mut file);

        writer.append("kappa", "Test data".as_bytes()).unwrap();

        writer.finish().unwrap();

        file.flush().unwrap();
        drop(file);

        let child = std::process::Command::new("unzip").arg("-t").arg(file_name).spawn().unwrap();

        let output = child.wait_with_output().unwrap();
        std::fs::remove_file(file_name).unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn simple_archive2() {
        let data = b"Simple Test";
        let mut out = Cursor::new(Vec::new());

        let mut writer = ZipWriter::new(&mut out);
        writer.append("test", &*data as &[u8]).unwrap();
        writer.finish().unwrap();

        out.set_position(0);

        let mut archive = zip::ZipArchive::new(&mut out).unwrap();
        {
            let mut file_names = archive.file_names();
            assert_eq!(file_names.next(), Some("test"));
            assert_eq!(file_names.next(), None);
        }

        let file = archive.by_index(0).unwrap();
        assert!(file.bytes().map(Result::unwrap).eq((data as &[u8]).bytes().map(Result::unwrap)));
    }

    #[test]
    fn simple_archive_deflate() {
        let data = b"Simple Test" as &[u8];
        let mut out = Cursor::new(Vec::new());

        let mut writer = ZipWriter::new(&mut out);
        writer.start_file("test").compression(DeflateConfig::best()).write_data(data).unwrap();
        writer.finish().unwrap();

        out.set_position(0);

        let mut archive = zip::ZipArchive::new(&mut out).unwrap();
        {
            let mut file_names = archive.file_names();
            assert_eq!(file_names.next(), Some("test"));
            assert_eq!(file_names.next(), None);
        }

        let file = archive.by_index(0).unwrap();
        assert!(file.bytes().map(Result::unwrap).eq((data as &[u8]).bytes().map(Result::unwrap)));
    }

    #[test]
    fn file_writer() {
        let data = b"Simple Test" as &[u8];
        let out1 = {
            let writer = ZipWriter::new(Cursor::new(Vec::new()));
            let mut file_writer = writer.start_file_writer("test").writer().unwrap();
            file_writer.write_all(data).unwrap();

            file_writer.finish().unwrap().finish().unwrap()
        };

        let out2 = {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer.append_data("test", data).unwrap();

            writer.finish().unwrap()
        };

        assert_eq!(out1, out2);
    }

    fn generate_data() -> Vec<String> {
        (0..10000).map(|n| n.to_string()).collect::<Vec<_>>()
    }

    #[bench]
    fn bench_one_file_zip_stream(b: &mut Bencher) {
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = ZipWriter::new(&mut out);

            writer.start_file("test_kappa").write_all("basically very smol file".as_bytes()).unwrap();

            writer.finish().unwrap();
        });

    }

    #[bench]
    fn bench_one_file_zip(b: &mut Bencher) {
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = zip::write::ZipWriter::new(&mut out);

            writer.start_file("test_kappa", FileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();
            writer.write_all("basically very smol file".as_bytes()).unwrap();

            writer.finish().unwrap();
        });

    }


    #[bench]
    fn bench_many_small_files_zip_stream(b: &mut Bencher) {
        let data = generate_data();
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = ZipWriter::new(&mut out);
            for v in data.iter() {
                writer.start_file("test_kappa").write_all(v.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        });
    }

    #[bench]
    fn bench_many_small_files_zip(b: &mut Bencher) {
        let data = generate_data();
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = zip::write::ZipWriter::new(&mut out);
            for v in data.iter() {
                writer.start_file("test_kappa", FileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();
                writer.write_all(v.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        });
    }

    #[bench]
    fn bench_many_small_files_zip_stream_deflate(b: &mut Bencher) {
        let data = generate_data();
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = ZipWriter::new(&mut out);
            for v in data.iter() {
                writer.start_file("test_kappa").compression(DeflateConfig::default()).write_data(v.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        });
    }

    #[bench]
    fn bench_many_small_files_zip_deflate(b: &mut Bencher) {
        let data = generate_data();
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = zip::write::ZipWriter::new(&mut out);
            for v in data.iter() {
                writer.start_file("test_kappa", FileOptions::default().compression_method(CompressionMethod::Deflated)).unwrap();
                writer.write_all(v.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        });
    }

    #[bench]
    fn bench_one_big_file_zip_stream(b: &mut Bencher) {
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = ZipWriter::new(&mut out);

            writer.append("test_kappa", repeat(123u8).take(10*1024*1024)).unwrap();

            writer.finish().unwrap();
        });

    }

    #[bench]
    fn bench_one_big_file_zip(b: &mut Bencher) {
        let mut out = Cursor::new(Vec::new());

        b.iter(|| {
            out.set_position(0);
            let mut writer = zip::write::ZipWriter::new(&mut out);

            writer.start_file("test_kappa", FileOptions::default().compression_method(CompressionMethod::Stored)).unwrap();
            std::io::copy(&mut repeat(123u8).take(10*1024*1024), &mut writer).unwrap();

            writer.finish().unwrap();
        });

    }
}
use std::fs::File;
use std::io::{Read, Result, Write};
use std::path::Path;
use std::time::SystemTime;

use byteorder::{LittleEndian, WriteBytesExt};
use std::cmp::min;
use std::fmt::Arguments;
use time::OffsetDateTime;

use crate::compressor::{Compressor, HashWriteWrapper, WriterWrapper, CompressorConfig, Store};
use crate::compressor;

pub struct ZipWriter<W: Write, P: AsRef<str>> {
    write: W,
    position: u64,
    entries: Vec<Header<P>>,
}

pub struct ZipWriterWrapper<'a, W: Write, P: AsRef<str>> {
    inner: &'a mut ZipWriter<W, P>
}

impl<'a, W: Write, P: AsRef<str>> Write for ZipWriterWrapper<'a, W, P> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.write.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write.write_all(buf)
    }
}

impl<'a, W: Write, P: AsRef<str>> WriterWrapper for ZipWriterWrapper<'a, W, P> {
    type Path = P;

    fn start_entry(&mut self, header: &mut Header<Self::Path>) -> Result<()> {
        self.inner.write_entry_header(header)
    }


    fn end_entry(&mut self, entry: Header<Self::Path>) -> Result<()> {
        println!("end_entry {}", entry.path.as_ref());
        self.inner.position += entry.compressed_size;
        self.inner.write_entry_data_descriptor(&entry)?;
        self.inner.entries.push(entry);
        Ok(())
    }
}

pub struct ZipFileWriter<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> {
    inner: HashWriteWrapper<C>,
    header: Header<P>
}

impl<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> ZipFileWriter<C, P, ZIP> {
    pub fn finish(self) -> Result<C::Inner> {
        let (crc32, writer) = self.inner.finish();
        let (entry_data, mut writer) = writer.finish()?;

        let mut header = self.header;

        header.uncompressed_size = entry_data.uncompressed_size;
        header.compressed_size = entry_data.compressed_size;
        header.crc32 = crc32;

        writer.end_entry(header)?;

        Ok(writer)
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

// impl<C: Compressor<Inner=ZIP>, P: AsRef<str>, ZIP: WriterWrapper<Path=P>> Drop for ZipFileWriter<C, P, ZIP> {
//     /// Can panic, to handle result call [ZipFileWriter::finish]
//     /// When we are dropping file writer we have to call finish to complete it's entry in zip structure.
//     /// But finish can fail, so we panic in this case (there is no other way to reasonably handle it in drop)
//     fn drop(&mut self) {
//         self.finish().unwrap();
//     }
// }


#[derive(Debug)]
pub struct Header<P: AsRef<str>> {
    compression_id: u16,
    path: P,
    modification_time: u16,
    modification_date: u16,

    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    offset: u64,
}

pub struct HeaderWithCompressorConfigBuilder<P: AsRef<str>, W: WriterWrapper<Path=P>, CC: CompressorConfig<W> = Store<W>> {
    header: HeaderBuilder<P>,
    compressor_config: CC,
    writer: W
}

impl<P: AsRef<str>, W: WriterWrapper<Path=P>, CC: CompressorConfig<W>> HeaderWithCompressorConfigBuilder<P, W, CC>{
    pub fn compression<NewCC: CompressorConfig<W>>(self, compressor_config: NewCC) -> HeaderWithCompressorConfigBuilder<P, W, NewCC> {
        HeaderWithCompressorConfigBuilder {
            compressor_config,
            header: self.header,
            writer: self.writer
        }
    }

    pub fn path(mut self, path: P) -> Self {
        self.header.path = Some(path);
        self
    }

    pub fn modification(self, time: SystemTime) -> Self {
        let date_time = OffsetDateTime::from(time);
        self.modification_date_time(date_time)
    }

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

    pub fn modification_from_file(self, file: &File) -> Self {
        let modified_at = file.metadata().ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or_else(|| SystemTime::now());

        self.modification(modified_at)
    }

    pub fn writer(mut self) -> Result<ZipFileWriter<<CC as CompressorConfig<W>>::CompressorTarget, P, W>> {
        let mut header = self.header.build::<CC, W>();
        self.writer.start_entry(&mut header)?;

        let compressor = self.compressor_config.build(self.writer);

        Ok(ZipFileWriter {
            inner: HashWriteWrapper::new(compressor),
            header
        })
    }

    pub fn write_data(self, mut data: impl Read) -> Result<W> {
        let mut writer = self.writer()?;

        std::io::copy(&mut data, &mut writer)?;

        writer.finish()
    }

    pub fn write_all(self, data: &[u8]) -> Result<W> {
        let mut writer = self.writer()?;

        writer.write_all(data)?;

        writer.finish()
    }
}

impl<P: AsRef<str>> Header<P> {
    fn path_str(&self) -> &str {
        self.path.as_ref()
    }

    fn path_bytes(&self) -> &[u8] {
        self.path_str().as_bytes()
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

impl<W: Write, P: AsRef<str>> ZipWriter<W, P> {
    pub fn new(write: W) -> ZipWriter<W, P> {
        Self {
            write,
            position: 0,
            entries: vec![],
        }
    }

    pub fn append_file(&mut self, path: P, file: File) -> Result<()> {
        self.start_file(path).modification_from_file(&file).write_data(file)?;

        Ok(())
    }

    pub fn append(&mut self, path: P, file: impl Read) -> Result<()> {
        self.start_file(path).write_data(file)?;

        Ok(())
    }

    pub fn start_file(&mut self, path: P) -> HeaderWithCompressorConfigBuilder<P, ZipWriterWrapper<'_, W, P>, compressor::StoreConfig>
    {
        HeaderWithCompressorConfigBuilder {
            header: Header::builder(),
            writer: ZipWriterWrapper { inner: self },
            compressor_config: compressor::StoreConfig
        }.path(path)
    }

    fn write_entry_header(&mut self, header: &mut Header<P>) -> Result<()> {
        header.offset = self.position;

        self.write.write_u32::<LittleEndian>(0x04034b50)?; // magic number
        self.write.write_u16::<LittleEndian>(0x2D)?; // version
        self.write.write_u16::<LittleEndian>(0b00000000_00001000 | ((!header.path_str().is_ascii() as u16) << 11))?; // general purpose flag
        self.write.write_u16::<LittleEndian>(header.compression_id)?; // compression method
        self.write.write_u16::<LittleEndian>(header.modification_time)?; // modification_time
        self.write.write_u16::<LittleEndian>(header.modification_date)?; // modification_date
        self.write.write_u32::<LittleEndian>(0)?; // crc-32
        self.write.write_u32::<LittleEndian>(0xFFFFFFFF)?; // compressed size
        self.write.write_u32::<LittleEndian>(0xFFFFFFFF)?; // uncompressed size
        self.write.write_u16::<LittleEndian>(header.path_bytes().len() as u16)?; // file name length
        self.write.write_u16::<LittleEndian>(20)?; // extra field length
        self.write.write_all(header.path_bytes())?; // path

        self.write.write_u16::<LittleEndian>(0x0001)?; // header id (ZIP64)
        self.write.write_u16::<LittleEndian>(16)?;
        self.write.write_u64::<LittleEndian>(0)?;
        self.write.write_u64::<LittleEndian>(0)?;

        self.position += 30 + header.path_bytes().len() as u64 + 20;

        Ok(())
    }

    fn write_entry_data_descriptor(&mut self, header: &Header<P>) -> Result<()> {
        self.write.write_u32::<LittleEndian>(0x08074b50)?; // data descriptor signature
        self.write.write_u32::<LittleEndian>(header.crc32)?;

        if header.uncompressed_size < 0xFFFFFFFF && header.compressed_size < 0xFFFFFFFF {
            self.write.write_u32::<LittleEndian>(header.uncompressed_size as u32)?;
            self.write.write_u32::<LittleEndian>(header.compressed_size as u32)?;

            self.position += 16;
        } else {
            self.write.write_u64::<LittleEndian>(header.uncompressed_size)?;
            self.write.write_u64::<LittleEndian>(header.compressed_size)?;

            self.position += 24;
        }

        Ok(())
    }

    fn write_central_directory(mut self) -> Result<W> {
        let entries_count = self.entries.len() as u64;
        let mut central_directory_size = 0u64;
        let central_directory_offset = self.position;
        for header in self.entries {
            let overflow_fields =
                (header.uncompressed_size >= 0xFFFFFFFF || header.compressed_size >= 0xFFFFFFFF) as u16 * 2 +
                    (header.offset >= 0xFFFFFFFF) as u16;

            self.write.write_u32::<LittleEndian>(0x02014b50)?; // signature
            self.write.write_u16::<LittleEndian>(0x2D)?; // version made by
            self.write.write_u16::<LittleEndian>(0x2D)?; // version to extract
            self.write.write_u16::<LittleEndian>(0b00000000_00001000 | ((!header.path_str().is_ascii() as u16) << 11))?; // general purpose bit flag
            self.write.write_u16::<LittleEndian>(header.compression_id)?; // compression method
            self.write.write_u16::<LittleEndian>(header.modification_time)?; // last mod file time
            self.write.write_u16::<LittleEndian>(header.modification_date)?; // last mod file date
            self.write.write_u32::<LittleEndian>(header.crc32)?; // crc32
            self.write.write_u32::<LittleEndian>(min(header.compressed_size, 0xFFFFFFFF) as u32)?; // compressed size
            self.write.write_u32::<LittleEndian>(min(header.uncompressed_size, 0xFFFFFFFF) as u32)?; // uncompressed size
            self.write.write_u16::<LittleEndian>(header.path_bytes().len() as u16)?; // file name length
            self.write.write_u16::<LittleEndian>(if overflow_fields > 0 { 4 + 8 * overflow_fields } else { 0 })?; // extra field length
            self.write.write_u16::<LittleEndian>(0)?; // file comment length
            self.write.write_u16::<LittleEndian>(0)?; // disk number start
            self.write.write_u16::<LittleEndian>(0)?; // internal file attributes
            self.write.write_u32::<LittleEndian>(0)?; // external file attributes
            self.write.write_u32::<LittleEndian>(min(header.offset, 0xFFFFFFFF) as u32)?; // relative offset of local header
            self.write.write_all(header.path_bytes())?; // file name
            if overflow_fields > 0 {
                self.write.write_u16::<LittleEndian>(0x0001)?; // header id (ZIP64)
                self.write.write_u16::<LittleEndian>(8 * overflow_fields)?;

                if header.uncompressed_size >= 0xFFFFFFFF || header.compressed_size >= 0xFFFFFFFF {
                    self.write.write_u64::<LittleEndian>(header.uncompressed_size)?;
                    self.write.write_u64::<LittleEndian>(header.compressed_size)?;
                }

                if header.offset >= 0xFFFFFFFF {
                    self.write.write_u64::<LittleEndian>(header.offset)?;
                }
            }

            central_directory_size += 46 + header.path_bytes().len() as u64;
            if overflow_fields > 0 {
                central_directory_size += 4 + 8 * overflow_fields as u64;
            }
        }

        self.position += central_directory_size;

        // TODO: conditional zip64 eocd
        // zip64
        self.write.write_u32::<LittleEndian>(0x06064b50)?; // signature (ZIP64 end of central directory)
        self.write.write_u64::<LittleEndian>(44)?;
        self.write.write_u16::<LittleEndian>(0x2D)?; // version made by
        self.write.write_u16::<LittleEndian>(0x2D)?; // version to extract
        self.write.write_u32::<LittleEndian>(0)?; // number of this disk
        self.write.write_u32::<LittleEndian>(0)?; // disk where central directory starts
        self.write.write_u64::<LittleEndian>(entries_count)?; // Number of central directory records on this disk
        self.write.write_u64::<LittleEndian>(entries_count)?; // Total number of central directory records
        self.write.write_u64::<LittleEndian>(central_directory_size)?;
        self.write.write_u64::<LittleEndian>(central_directory_offset)?; // offset of central directory


        // zip64
        self.write.write_u32::<LittleEndian>(0x07064b50)?;
        self.write.write_u32::<LittleEndian>(0)?; // disk number
        self.write.write_u64::<LittleEndian>(self.position)?;
        self.write.write_u32::<LittleEndian>(1)?; // disk total

        // end of central directory
        self.write.write_u32::<LittleEndian>(0x06054b50)?;
        self.write.write_u16::<LittleEndian>(0)?; // number of this disk
        self.write.write_u16::<LittleEndian>(0)?; // disk where central directory starts
        self.write.write_u16::<LittleEndian>(entries_count as u16)?; // number of central directory records on this disk
        self.write.write_u16::<LittleEndian>(entries_count as u16)?; // number of central directory records total
        self.write.write_u32::<LittleEndian>(min(central_directory_size, 0xFFFFFFFF) as u32)?; // size of the central directory
        self.write.write_u32::<LittleEndian>(min(central_directory_offset, 0xFFFFFFFF) as u32)?; // offset of central directory
        self.write.write_u16::<LittleEndian>(0)?; // zip comment length

        Ok(self.write)
    }

    pub fn finish(self) -> Result<W> {
        self.write_central_directory()
    }

    pub fn into_inner(self) -> W {
        self.write
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use std::fs::File;
    use std::io::{Cursor, Read, repeat, Write};
    use std::iter::once;
    use crate::writer::ZipWriter;
    use test::Bencher;
    use zip::write::FileOptions;
    use zip::CompressionMethod;
    use crate::compressor::deflate::DeflateConfig;

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

    fn file_writer_drop() {
        let data = b"Simple Test" as &[u8];
        let mut out = Cursor::new(Vec::new());

        let mut writer = ZipWriter::new(&mut out);
        let mut file_writer = writer.start_file("test").writer().unwrap();
        file_writer.write_all(data).unwrap();
        writer.start_file("test1").writer().unwrap().write_all(data).unwrap();
        writer.finish().unwrap();

        out.set_position(0);

        let mut archive = zip::ZipArchive::new(&mut out).unwrap();
        {
            let mut file_names = archive.file_names();
            assert_eq!(file_names.next(), Some("test"));
            assert_eq!(file_names.next(), Some("test1"));
            assert_eq!(file_names.next(), None);
        }

        let file = archive.by_index(0).unwrap();
        assert!(file.bytes().map(Result::unwrap).eq(data.iter().cloned()));

        let file = archive.by_index(1).unwrap();
        assert!(file.bytes().map(Result::unwrap).eq(data.iter().cloned()));
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
use std::fs::{File, Metadata};
use std::hash::Hash;
use std::io::{Read, Result, Seek, Write};
use std::path::Path;
use std::time::SystemTime;

use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use flate2::write::DeflateEncoder;
use time::PrimitiveDateTime;

pub struct ZipWriter<'a, W: Write> {
    write: W,

    position: u64,

    // TODO: нужны ли они мне?
    compressed_size: u64,
    uncompressed_size: u64,

    entries: Vec<Header<'a>>,
}

pub enum Compression {
    None,
    Deflate(u8),
}

pub struct Header<'a> {
    compression: Compression,
    path: &'a str,
    modification_time: u16,
    modification_date: u16,

    compressed_size: u64,
    uncompressed_size: u64,
    crc32: u32,
    offset: u64,
}

pub struct HeaderBuilder<'a> {
    compression: Compression,
    path: Option<&'a str>,
    modification_time: Option<u16>,
    modification_date: Option<u16>,
}

impl<'a> HeaderBuilder<'a> {
    pub fn compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    pub fn path<P: AsRef<Path>>(mut self, path: &'a P) -> Self {
        self.path = Some(path.as_ref().to_str().unwrap());
        self
    }

    pub fn modification(mut self, time: SystemTime) -> Self {
        self.modification_time = Some(0);
        self.modification_date = Some(0);
        self
    }

    pub fn build(self) -> Header<'a> {
        Header {
            compression: self.compression,
            path: self.path.unwrap(),
            modification_date: self.modification_date.unwrap(),
            modification_time: self.modification_time.unwrap(),
            compressed_size: 0,
            uncompressed_size: 0,
            crc32: 0,
            offset: 0
        }
    }
}

impl Header<'_> {
    pub fn builder<'a>() -> HeaderBuilder<'a> {
        HeaderBuilder {
            compression: Compression::None,
            path: None,
            modification_date: None,
            modification_time: None,
        }
    }
}


struct HashReader<R: Read> {
    reader: R,
    hasher: Hasher
}

impl<R: Read> Read for HashReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let res = self.reader.read(buf)?;

        self.hasher.update(&buf[..res]);

        Ok(res)
    }
}

impl<R: Read> HashReader<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            hasher: Hasher::new()
        }
    }

    fn hash(self) -> u32 {
        self.hasher.finalize()
    }
}


impl<W: Write> ZipWriter<W> {
    pub fn new(write: W) -> ZipWriter<W> {
        Self {
            write,
            compressed_size: 0,
            uncompressed_size: 0,
        }
    }

    pub fn append_file(&mut self, path: impl AsRef<Path>, file: File) -> Result<()> {
        let modified_at = file.metadata()
            .map(|m| m.modified().unwrap_or_else(|_| SystemTime::now()))
            .unwrap_or_else(|_| SystemTime::now());

        let header = Header::builder().path(&path).modification(modified_at).build();

        self.write_entry(header, file);

        // self.write.write_u64::<LittleEndian>(123)?;
        // self.write_entry_header();
        Ok(())
    }

    pub fn append<R: Read>(&mut self, path: impl AsRef<Path>, file: R) -> Result<()> {
        self.write.write_u64::<LittleEndian>(123)?;
        // self.write_entry_header();
        Ok(())
    }

    pub fn append_with_header(&mut self, h: Header, file: impl Read) -> Result<()> {
        Ok(())
    }

    fn write_entry(&mut self, mut header: Header, data: impl Read) -> Result<()> {
        self.write_entry_header(&mut header)?;
        self.write_entry_body(&mut header, data)?;

        self.entries.push(header);
        Ok(())
    }

    fn write_entry_header(&mut self, header: &mut Header) -> Result<()> {
        header.offset = self.position;

        self.write.write_u32::<LittleEndian>(0x04034b50)?; // magic number
        self.write.write_u16::<LittleEndian>(0xA)?; // version
        self.write.write_u16::<LittleEndian>(0b00000000_00001000 | ((header.path.is_ascii() as u16) << 11))?; // general purpose flag
        self.write.write_u16::<LittleEndian>(0)?; // compression method
        self.write.write_u16::<LittleEndian>(header.modification_time)?; // modification_time
        self.write.write_u16::<LittleEndian>(header.modification_date)?; // modification_date
        self.write.write_u32::<LittleEndian>(0)?; // crc-32
        self.write.write_u32::<LittleEndian>(0)?; // compressed size
        self.write.write_u32::<LittleEndian>(0)?; // uncompressed size
        self.write.write_u16::<LittleEndian>(header.path.as_bytes().len() as u16)?; // file name length
        self.write.write_u16::<LittleEndian>(0)?; // extra field length
        self.write.write_all(header.path.as_bytes())?; // path

        self.position += 30 + header.path.as_bytes().len() as u64;

        Ok(())
    }

    fn write_entry_body(& mut self, header: &mut Header, mut data: impl Read) -> Result<()> {
        let mut hash_reader = HashReader::new(data);
        let (uncompressed_size, compressed_size) = match header.compression {
            Compression::None => {
                let result = std::io::copy(&mut hash_reader, &mut self.write)?;
                (result, result)
            },
            Compression::Deflate(level) => {
                let mut encoder = DeflateEncoder::new(&mut self.write, flate2::Compression::new(level as u32));
                std::io::copy(&mut hash_reader, &mut encoder)?;
                encoder.flush()?;

                (encoder.total_in(), encoder.total_out())
            }
        };
        let crc32 = hash_reader.hash();

        self.position += compressed_size;

        // TODO: Прочитать где устанаваливается режим zip64, и решить нужно ли все файлы писать в zip64 т.к. мы не знаем наперед будут ли слишком большие файлы
        // writing data descriptor
        self.write.write_u32::<LittleEndian>(0x08074b50)?; // data descriptor signature
        self.write.write_u32::<LittleEndian>(crc32)?;

        if uncompressed_size < 0xFFFFFFFF || compressed_size < 0xFFFFFFFF {
            self.write.write_u32::<LittleEndian>(uncompressed_size as u32)?;
            self.write.write_u32::<LittleEndian>(compressed_size as u32)?;

            self.position += 8;
        } else {
            self.write.write_u64::<LittleEndian>(uncompressed_size)?;
            self.write.write_u64::<LittleEndian>(compressed_size)?;

            self.position += 16;
        }


        header.compressed_size = compressed_size;
        header.uncompressed_size = uncompressed_size;
        header.crc32 = crc32;

        Ok(())
    }
    
    fn write_central_directory(&mut self) -> Result<()> {
        let entries_count = self.entries.len();
        let mut central_directory_size = 0u64;
        for header in self.entries {
            self.write.write_u32::<LittleEndian>(0x02014b50); // signature
            self.write.write_u16::<LittleEndian>(0xA); // version made by
            self.write.write_u16::<LittleEndian>(0xA); // version to extract
            self.write.write_u16::<LittleEndian>(0b00000000_00001000); // general purpose bit flag
            self.write.write_u16::<LittleEndian>(0); // compression method
            self.write.write_u16::<LittleEndian>(header.modification_time); // last mod file time
            self.write.write_u16::<LittleEndian>(header.modification_date); // last mod file date
            self.write.write_u32::<LittleEndian>(header.crc32); // crc32
            self.write.write_u32::<LittleEndian>(header.compressed_size as u32); // compressed size
            self.write.write_u32::<LittleEndian>(header.uncompressed_size as u32); // uncompressed size
            self.write.write_u16::<LittleEndian>(header.path.len() as u16); // file name length
            self.write.write_u16::<LittleEndian>(0); // extra field length
            self.write.write_u16::<LittleEndian>(0); // file comment length
            self.write.write_u16::<LittleEndian>(0); // disk number start
            self.write.write_u16::<LittleEndian>(0); // internal file attributes
            self.write.write_u32::<LittleEndian>(0); // external file attributes
            self.write.write_u32::<LittleEndian>(header.offset as u32); // relative offset of local header
            self.write.write_all(header.path.as_bytes()); // file name

            central_directory_size += 46 + header.path.as_bytes().len();
        }

        self.write.write_u32::<LittleEndian>(0x06054b50);
        self.write.write_u16::<LittleEndian>(0); // number of this disk
        self.write.write_u16::<LittleEndian>(0); // disk where central directory starts
        self.write.write_u16::<LittleEndian>(entries_count as u16); // number of central directory records on this disk
        self.write.write_u16::<LittleEndian>(entries_count as u16); // number of central directory records total
        self.write.write_u32::<LittleEndian>(central_directory_size as u32); // size of the central directory
        self.write.write_u32::<LittleEndian>(self.position as u32); // offset of central directory
        self.write.write_u32::<LittleEndian>(0); // zip comment length

        Ok(())
    }

    fn write_data(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_data_entry(&mut self) -> Result<()> {
        Ok(())
    }
}
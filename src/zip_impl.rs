use byteorder::{LittleEndian, WriteBytesExt};
use std::cmp::min;
use std::io::Result;
use std::io::Write;

#[derive(Debug)]
pub struct Header<P: AsRef<str>> {
    pub(crate) compression_id: u16,
    pub(crate) path: P,
    pub(crate) modification_time: u16,
    pub(crate) modification_date: u16,

    pub(crate) compressed_size: u64,
    pub(crate) uncompressed_size: u64,
    pub(crate) crc32: u32,
    pub(crate) offset: u64,
}
impl<P: AsRef<str>> Header<P> {
    fn path_str(&self) -> &str {
        self.path.as_ref()
    }

    fn path_bytes(&self) -> &[u8] {
        self.path_str().as_bytes()
    }
}

pub struct ZipWriter<W: Write, P: AsRef<str>> {
    pub(crate) write: W,
    pub(crate) position: u64,
    pub(crate) entries: Vec<Header<P>>,
}

impl<W: Write, P: AsRef<str>> AsMut<ZipWriter<W, P>> for ZipWriter<W, P> {
    fn as_mut(&mut self) -> &mut ZipWriter<W, P> {
        self
    }
}

impl<W: Write, P: AsRef<str>> ZipWriter<W, P> {
    pub(crate) fn write_entry_header(&mut self, header: &mut Header<P>) -> Result<()> {
        header.offset = self.position;

        self.write.write_u32::<LittleEndian>(0x04034b50)?; // magic number
        self.write.write_u16::<LittleEndian>(0x2D)?; // version
        self.write.write_u16::<LittleEndian>(
            0b0000_0000_0000_1000 | ((!header.path_str().is_ascii() as u16) << 11),
        )?; // general purpose flag
        self.write
            .write_u16::<LittleEndian>(header.compression_id)?; // compression method
        self.write
            .write_u16::<LittleEndian>(header.modification_time)?; // modification_time
        self.write
            .write_u16::<LittleEndian>(header.modification_date)?; // modification_date
        self.write.write_u32::<LittleEndian>(0)?; // crc-32
        self.write.write_u32::<LittleEndian>(0xFFFFFFFF)?; // compressed size
        self.write.write_u32::<LittleEndian>(0xFFFFFFFF)?; // uncompressed size
        self.write
            .write_u16::<LittleEndian>(header.path_bytes().len() as u16)?; // file name length
        self.write.write_u16::<LittleEndian>(20)?; // extra field length
        self.write.write_all(header.path_bytes())?; // path

        self.write.write_u16::<LittleEndian>(0x0001)?; // header id (ZIP64)
        self.write.write_u16::<LittleEndian>(16)?;
        self.write.write_u64::<LittleEndian>(0)?;
        self.write.write_u64::<LittleEndian>(0)?;

        self.position += 30 + header.path_bytes().len() as u64 + 20;

        Ok(())
    }

    pub(crate) fn write_entry_data_descriptor(&mut self, header: &Header<P>) -> Result<()> {
        self.write.write_u32::<LittleEndian>(0x08074b50)?; // data descriptor signature
        self.write.write_u32::<LittleEndian>(header.crc32)?;

        if header.uncompressed_size < 0xFFFFFFFF && header.compressed_size < 0xFFFFFFFF {
            self.write
                .write_u32::<LittleEndian>(header.uncompressed_size as u32)?;
            self.write
                .write_u32::<LittleEndian>(header.compressed_size as u32)?;

            self.position += 16;
        } else {
            self.write
                .write_u64::<LittleEndian>(header.uncompressed_size)?;
            self.write
                .write_u64::<LittleEndian>(header.compressed_size)?;

            self.position += 24;
        }

        Ok(())
    }

    pub(crate) fn write_central_directory(mut self) -> Result<W> {
        let entries_count = self.entries.len() as u64;
        let mut central_directory_size = 0u64;
        let central_directory_offset = self.position;
        for header in self.entries {
            let overflow_fields = (header.uncompressed_size >= 0xFFFFFFFF
                || header.compressed_size >= 0xFFFFFFFF) as u16
                * 2
                + (header.offset >= 0xFFFFFFFF) as u16;

            self.write.write_u32::<LittleEndian>(0x02014b50)?; // signature
            self.write.write_u16::<LittleEndian>(0x2D)?; // version made by
            self.write.write_u16::<LittleEndian>(0x2D)?; // version to extract
            self.write.write_u16::<LittleEndian>(
                0b0000_0000_0000_1000 | ((!header.path_str().is_ascii() as u16) << 11),
            )?; // general purpose bit flag
            self.write
                .write_u16::<LittleEndian>(header.compression_id)?; // compression method
            self.write
                .write_u16::<LittleEndian>(header.modification_time)?; // last mod file time
            self.write
                .write_u16::<LittleEndian>(header.modification_date)?; // last mod file date
            self.write.write_u32::<LittleEndian>(header.crc32)?; // crc32
            self.write
                .write_u32::<LittleEndian>(min(header.compressed_size, 0xFFFFFFFF) as u32)?; // compressed size
            self.write
                .write_u32::<LittleEndian>(min(header.uncompressed_size, 0xFFFFFFFF) as u32)?; // uncompressed size
            self.write
                .write_u16::<LittleEndian>(header.path_bytes().len() as u16)?; // file name length
            self.write
                .write_u16::<LittleEndian>(if overflow_fields > 0 {
                    4 + 8 * overflow_fields
                } else {
                    0
                })?; // extra field length
            self.write.write_u16::<LittleEndian>(0)?; // file comment length
            self.write.write_u16::<LittleEndian>(0)?; // disk number start
            self.write.write_u16::<LittleEndian>(0)?; // internal file attributes
            self.write.write_u32::<LittleEndian>(0)?; // external file attributes
            self.write
                .write_u32::<LittleEndian>(min(header.offset, 0xFFFFFFFF) as u32)?; // relative offset of local header
            self.write.write_all(header.path_bytes())?; // file name
            if overflow_fields > 0 {
                self.write.write_u16::<LittleEndian>(0x0001)?; // header id (ZIP64)
                self.write.write_u16::<LittleEndian>(8 * overflow_fields)?;

                if header.uncompressed_size >= 0xFFFFFFFF || header.compressed_size >= 0xFFFFFFFF {
                    self.write
                        .write_u64::<LittleEndian>(header.uncompressed_size)?;
                    self.write
                        .write_u64::<LittleEndian>(header.compressed_size)?;
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
        self.write
            .write_u64::<LittleEndian>(central_directory_size)?;
        self.write
            .write_u64::<LittleEndian>(central_directory_offset)?; // offset of central directory

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
        self.write
            .write_u32::<LittleEndian>(min(central_directory_size, 0xFFFFFFFF) as u32)?; // size of the central directory
        self.write
            .write_u32::<LittleEndian>(min(central_directory_offset, 0xFFFFFFFF) as u32)?; // offset of central directory
        self.write.write_u16::<LittleEndian>(0)?; // zip comment length

        Ok(self.write)
    }
}

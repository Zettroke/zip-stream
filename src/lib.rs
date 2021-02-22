use bytes::BufMut;
use std::io::{Read, Cursor};
use std::cmp;

use crc32fast::Hasher;
use std::fmt::Formatter;

// struct LocalHeader {
//     pub signature: u32,
//     pub version: u16,
//     pub bit_flag: u16,
//     pub compression: u16,
//     pub modification_time: u16,
//     pub modification_date: u16,
//     pub crc32: u16,
//     pub compressed_size: u32,
//     pub uncompressed_size: u32,
//     pub file_name: String,
//     pub extra_field_length: u16
// }
//
// impl LocalHeader {
//     pub fn write_local_file_header<T: BufMut>(&self, buff: &mut T) {
//         buff.put_u32_le(0x04034b50);
//         buff.put_u16_le(self.version);
//         buff.put_u16_le(0b00000000_00001000);
//         buff.put_u16_le(0);
//         buff.put_u16_le(self.modification_time);
//         buff.put_u16_le(self.modification_date);
//         buff.put_u32_le(0);
//         buff.put_u32_le(0);
//         buff.put_u32_le(0);
//         buff.put_u16_le(self.file_name.len() as u16);
//         buff.put_u16_le(0);
//         buff.put_slice(self.file_name.as_bytes());
//     }
// }

#[derive(Debug)]
pub struct ZipEntry<R: Read> {
    name: String,
    reader: R,
    modification_time: u16,
    modification_date: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    crc32: u32,
    header_offset: u64,
}

impl<R: Read> ZipEntry<R> {
    pub(crate) fn header_len(&self) -> usize {
        30 + self.name.len()
    }

    pub(crate) fn write_local_file_header<B: BufMut>(&self, mut buff: B) {
        buff.put_u32_le(0x04034b50);
        buff.put_u16_le(0xA);
        buff.put_u16_le(0b00000000_00001000);
        buff.put_u16_le(0);
        buff.put_u16_le(self.modification_time);
        buff.put_u16_le(self.modification_date);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u16_le(self.name.len() as u16);
        buff.put_u16_le(0);
        buff.put_slice(self.name.as_bytes());
    }

    pub(crate) fn tail_header_len(&self) -> usize {
        12
    }

    pub(crate) fn write_tail_header<B: BufMut>(&self, mut buff: B) {
        buff.put_u32_le(self.crc32);
        buff.put_u32_le(self.compressed_size as u32);
        buff.put_u32_le(self.uncompressed_size as u32);
    }

    pub fn new<S: Into<String>>(name: S, data: R) -> ZipEntry<R> {
        ZipEntry {
            header_offset: 0,
            name: name.into(),
            uncompressed_size: 0,
            compressed_size: 0,
            modification_time: 0,
            modification_date: 0,
            crc32: 0,
            reader: data
        }
    }
}

pub struct ZipPacker<R, I = Vec<ZipEntry<R>>> where R: Read, I: IntoIterator<Item=ZipEntry<R>> {
    files: I
}

impl<R, I> ZipPacker<R, I> where R: Read, I: IntoIterator<Item=ZipEntry<R>> {
    pub fn with_file_iterator<II: IntoIterator<Item=ZipEntry<R>>>(iter: II) -> ZipPacker<R, II> {
        ZipPacker {
            files: iter,
        }
    }

    pub fn reader(self) -> ZipReader<R, I::IntoIter> {
        ZipReader {
            files_iter: self.files.into_iter(),
            state: ZipReaderState::Initial,
            remainder: Cursor::new(Vec::new()),
            central_directory: Cursor::new(Vec::new()),
            offset: 0,
            entries_count: 0
        }
    }
}

impl<R: Read> ZipPacker<R> {
    pub fn new() -> Self {
        Self {
            files: vec![]
        }
    }

    pub fn add_file(&mut self, entry: ZipEntry<R>) {
        self.files.push(entry);
    }
}

// #[derive(Debug)]
enum ZipReaderState<R: Read> {
    Initial,
    EntryHeader(ZipEntry<R>),
    EntryBody(ZipEntry<R>, Hasher),
    EntryTail(ZipEntry<R>),
    CentralDirectory,
}

impl<R: Read> std::fmt::Debug for ZipReaderState<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ZipReaderState::Initial => f.write_str("Initial"),
            ZipReaderState::EntryHeader(..) => f.write_str("EntryHeader"),
            ZipReaderState::EntryBody(..) => f.write_str("EntryBody"),
            ZipReaderState::EntryTail(..) => f.write_str("EntryTail"),
            ZipReaderState::CentralDirectory => f.write_str("CentralDirectory"),
        }
    }
}

pub struct ZipReader<R: Read, I: Iterator<Item=ZipEntry<R>>> {
    files_iter: I,

    state: ZipReaderState<R>,

    entries_count: u64,
    offset: u64,

    remainder: Cursor<Vec<u8>>,
    central_directory: Cursor<Vec<u8>>,
}

impl<R: Read, I: Iterator<Item=ZipEntry<R>>> ZipReader<R, I> {
    fn write_entry_header(&mut self, entry: &ZipEntry<R>, buff: &mut [u8]) -> usize {
        let n = cmp::min(entry.header_len(), buff.len());
        if buff.len() >= entry.header_len() {
            entry.write_local_file_header(buff);
        } else {
            entry.write_local_file_header(buff.chain_mut(self.remainder.get_mut()));
        }

        self.offset += entry.header_len() as u64;

        return n;
    }

    fn write_entry_body(&mut self, entry: &mut ZipEntry<R>, hasher: &mut Hasher, buff: &mut [u8]) -> std::io::Result<usize> {
        let res = entry.reader.read(buff);
        if let Ok(n) = res {
            entry.uncompressed_size += n;
            entry.compressed_size += n;
            self.offset += n as u64;
            if n != 0 {
                hasher.update(&buff[..n]);
            }
        }

        return res;
    }

    fn write_entry_tail(&mut self, entry: &ZipEntry<R>, buff: &mut [u8]) -> usize {
        let n = cmp::min(entry.tail_header_len(), buff.len());
        if buff.len() >= entry.tail_header_len() {
            entry.write_tail_header(buff);
        } else {
            entry.write_tail_header(buff.chain_mut(self.remainder.get_mut()));
        }

        self.offset += entry.tail_header_len() as u64;

        return n;
    }

    fn write_central_directory(&mut self, entry: &ZipEntry<R>) {
        let buff = self.central_directory.get_mut();

        buff.put_u32_le(0x02014b50); // signature
        buff.put_u16_le(0xA); // version made by
        buff.put_u16_le(0xA); // version to extract
        buff.put_u16_le(0b00000000_00001000); // general purpose bit flag
        buff.put_u16_le(0); // compression method
        buff.put_u16_le(entry.modification_time); // last mod file time
        buff.put_u16_le(entry.modification_date); // last mod file date
        buff.put_u32_le(entry.crc32); // crc32
        buff.put_u32_le(entry.compressed_size as u32); // compressed size
        buff.put_u32_le(entry.uncompressed_size as u32); // uncompressed size
        buff.put_u16_le(entry.name.len() as u16); // file name length
        buff.put_u16_le(0); // extra field length
        buff.put_u16_le(0); // file comment length
        buff.put_u16_le(0); // disk number start
        buff.put_u16_le(0); // internal file attributes
        buff.put_u32_le(0); // external file attributes
        buff.put_u32_le(entry.header_offset as u32); // relative offset of local header
        buff.put_slice(entry.name.as_bytes()); // file name
    }

    fn write_end_of_central_directory(&mut self) {
        let buff = self.central_directory.get_mut();

        let central_directory_size = buff.len() as u32;

        buff.put_u32_le(0x06054b50);
        buff.put_u16_le(0); // number of this disk
        buff.put_u16_le(0); // disk where central directory starts
        buff.put_u16_le(self.entries_count as u16); // number of central directory records on this disk
        buff.put_u16_le(self.entries_count as u16); // number of central directory records total
        buff.put_u32_le(central_directory_size); // size of the central directory
        buff.put_u32_le(self.offset as u32); // offset of central directory
        buff.put_u32_le(0); // zip comment length
    }
}

impl<R: Read, I: Iterator<Item=ZipEntry<R>>> Read for ZipReader<R, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // println!("state: {:?}", self.state);
        // if we have remaining bytes from previous read, we flushing them all
        if self.remainder.get_ref().len() > 0 {
            let n = self.remainder.read(buf);
            if self.remainder.position() as usize == self.remainder.get_ref().len() {
                self.remainder.get_mut().clear();
                self.remainder.set_position(0);
            }
            return n;
        }

        return match std::mem::replace(&mut self.state, ZipReaderState::Initial) {
            ZipReaderState::Initial => {
                let mut entry = self.files_iter.next().unwrap();
                entry.header_offset = self.offset;
                let n = self.write_entry_header(&entry, buf);

                self.state = ZipReaderState::EntryBody(entry, Hasher::new());

                Ok(n)
            }
            ZipReaderState::EntryHeader(entry) => {
                let n = self.write_entry_header(&entry, buf);

                self.state = ZipReaderState::EntryBody(entry, Hasher::new());

                Ok(n)
            }
            ZipReaderState::EntryBody(mut entry, mut hasher) => {
                let res = self.write_entry_body(&mut entry, &mut hasher, buf);


                if let Ok(n) = res {
                    if n == 0 {
                        entry.crc32 = hasher.finalize();
                        self.state = ZipReaderState::EntryTail(entry);

                        return self.read(buf);
                    }
                }
                self.state = ZipReaderState::EntryBody(entry, hasher);

                res
            }
            ZipReaderState::EntryTail(entry) => {
                let n = self.write_entry_tail(&entry, buf);

                self.entries_count += 1;

                match self.files_iter.next() {
                    Some(mut next_entry) => {
                        next_entry.header_offset = self.offset;
                        self.state = ZipReaderState::EntryHeader(next_entry);
                    }
                    None => {
                        self.state = ZipReaderState::CentralDirectory;
                    }
                }

                // central directory header
                self.write_central_directory(&entry);

                if let ZipReaderState::CentralDirectory = self.state {
                    self.write_end_of_central_directory();
                }

                Ok(n)
            },
            ZipReaderState::CentralDirectory => {
                let res = self.central_directory.read(buf);

                self.state = ZipReaderState::CentralDirectory;
                res
            }
        }
    }
}

// impl<T> Read for ZipPacker<T> where R: Read {
//     fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
//         self.fill_out_buff();
//         // self.buffer.advance()
//         Ok(0)
//     }
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

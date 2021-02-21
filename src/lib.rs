use bytes::{Bytes, BytesMut, BufMut};
use std::io::{Read, Cursor};
use bytes::buf::Limit;
use std::cmp;
use std::iter::Zip;

pub fn kappa() -> u32 {
    1337
}

struct LocalHeader {
    pub signature: u32,
    pub version: u16,
    pub bit_flag: u16,
    pub compression: u16,
    pub modification_time: u16,
    pub modification_date: u16,
    pub crc32: u16,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub file_name: String,
    pub extra_field_length: u16
}

impl LocalHeader {
    pub fn write_local_file_header<T: BufMut>(&self, buff: &mut T) {
        buff.put_u32_le(0x04034b50);
        buff.put_u16_le(self.version);
        buff.put_u16_le(0b00000000_00001000);
        buff.put_u16_le(0);
        buff.put_u16_le(self.modification_time);
        buff.put_u16_le(self.modification_date);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u16_le(self.file_name.len() as u16);
        buff.put_u16_le(0);
        buff.put_slice(self.file_name.as_bytes());
    }
}

struct ZipEntry<R: Read> {
    name: String,
    reader: R,
    compressed_size: usize,
    uncompressed_size: usize
}

impl<R: Read> ZipEntry<R> {

    pub fn header_len(&self) -> usize {
        30 + self.name.len()
    }

    pub fn write_local_file_header<B: BufMut>(&self, mut buff: B) {
        buff.put_u32_le(0x04034b50);
        buff.put_u16_le(0xA);
        buff.put_u16_le(0b00000000_00001000);
        buff.put_u16_le(0);
        buff.put_u16_le(/*self.modification_time*/ 0);
        buff.put_u16_le(/*self.modification_date*/ 0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u16_le(self.name.len() as u16);
        buff.put_u16_le(0);
        buff.put_slice(self.name.as_bytes());
    }

    pub fn tail_header_len(&self) -> usize {
        12
    }

    pub fn write_tail_header<B: BufMut>(&self, mut buff: B) {
        buff.put_u32_le(0 /* crc */);
        buff.put_u32_le(self.compressed_size as u32);
        buff.put_u32_le(self.uncompressed_size as u32);
    }
}

struct ZipPacker<R, I = Vec<ZipEntry<R>>> where R: Read, I: IntoIterator<Item=ZipEntry<R>> {
    files: I
}

impl<R, I> ZipPacker<R, I> where R: Read, I: IntoIterator<Item=ZipEntry<R>> {
    // pub fn new() -> Self {
    //     ZipPacker {
    //         buffer: BytesMut::with_capacity(64*1024),
    //         files: vec![]
    //     }
    // }

    // pub fn add_file<S: Into<String>>(&mut self, entry: ZipEntry<R>) {
    //     self.files.push(entry);
    // }

    pub fn with_file_iterator<II: IntoIterator<Item=ZipEntry<R>>>(iter: II) -> ZipPacker<R, II> {
        ZipPacker {
            files: iter,
        }
    }

    pub fn reader(self) -> ZipReader<R, I::IntoIter> {
        ZipReader {
            files_iter: self.files.into_iter(),
            state: ZipReaderState::Initial,
            remainder: Cursor::new(Vec::new())
        }
    }
}

impl<R: Read> ZipPacker<R> {
    pub fn new() -> Self {
        Self {
            files: vec![]
        }
    }

    pub fn add_file<S: Into<String>>(&mut self, entry: ZipEntry<R>) {
        self.files.push(entry);
    }
}

enum ZipReaderState<R: Read> {
    Initial,
    EntryHeader(ZipEntry<R>),
    EntryBody(ZipEntry<R>),
    EntryTail(ZipEntry<R>),
    CentralDirectory
}

struct ZipReader<R: Read, I: Iterator<Item=ZipEntry<R>>> {
    files_iter: I,

    state: ZipReaderState<R>,

    remainder: Cursor<Vec<u8>>
}

impl<R: Read, I: Iterator<Item=ZipEntry<R>>> ZipReader<R, I> {
    fn advance(&mut self) {

    }

    fn write_entry_header(&mut self, entry: &ZipEntry<R>, mut buff: &mut [u8]) -> usize {
        let mut n = cmp::min(entry.header_len(), buff.len());
        if buff.len() >= entry.header_len() {
            entry.write_local_file_header(buff);
        } else {
            entry.write_local_file_header(buff.chain_mut(self.remainder.get_mut()));
        }

        return n;
    }

    fn write_entry_tail(&mut self, entry: &ZipEntry<R>, mut buff: &mut [u8]) -> usize {
        let mut n = cmp::min(entry.tail_header_len(), buff.len());
        if buff.len() >= entry.tail_header_len() {
            entry.write_tail_header(buff);
        } else {
            entry.write_tail_header(buff.chain_mut(self.remainder.get_mut()));
        }

        return n;
    }
}

impl<R: Read, I: Iterator<Item=ZipEntry<R>>> Read for ZipReader<R, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // if we have remaining bytes from previous read, we flushing them all
        if self.remainder.get_ref().len() > 0 {
            let n = self.remainder.read(buf);
            if self.remainder.position() as usize == self.remainder.get_ref().len() {
                self.remainder.get_mut().clear();
                self.remainder.set_position(0);
            }
            return n;
        }

        match std::mem::replace(&mut self.state, ZipReaderState::Initial) {
            ZipReaderState::Initial => {
                let entry = self.files_iter.next().unwrap();
                let n = self.write_entry_header(&entry, buf);

                self.state = ZipReaderState::EntryBody(entry);

                return Ok(n);
            },
            ZipReaderState::EntryHeader(mut entry) => {
                let n = self.write_entry_header(&entry, buf);

                self.state = ZipReaderState::EntryBody(entry);

                return Ok(n);
            },
            ZipReaderState::EntryBody(mut entry) => {
                let res = entry.reader.read(buf);
                if let Ok(n) = res {
                    entry.uncompressed_size += n;
                    entry.compressed_size += n;
                    if n == 0 {
                        self.state = ZipReaderState::EntryTail(entry);
                        return Ok(0);
                    }
                }
                self.state = ZipReaderState::EntryBody(entry);
                return res;
            },
            ZipReaderState::EntryTail(mut entry) => {
                let n = self.write_entry_header(&entry, buf);

                match self.files_iter.next() {
                    Some(next_entry) => {
                        self.state = ZipReaderState::EntryHeader(next_entry);
                    },
                    None => {

                        self.state = ZipReaderState::CentralDirectory();
                    }
                }
                // directory headers

                return Ok(n);
            },
            _ => {}
        }
        Ok(0)
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

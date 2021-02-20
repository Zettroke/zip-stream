use bytes::{Bytes, BytesMut, BufMut};
use std::io::{Read, Cursor};
use bytes::buf::Limit;

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

struct ZipEntry<T: AsMut<dyn Read>> {
    name: String,
    reader: T
}

struct ZipPacker<T, I = Vec<ZipEntry<T>>> where T: AsMut<dyn Read>, I: IntoIterator<Item=ZipEntry<T>> {
    files: I
}

impl<T, I> ZipPacker<T, I> where T: AsMut<dyn Read>, I: IntoIterator<Item=ZipEntry<T>> {
    // pub fn new() -> Self {
    //     ZipPacker {
    //         buffer: BytesMut::with_capacity(64*1024),
    //         files: vec![]
    //     }
    // }

    // pub fn add_file<S: Into<String>>(&mut self, entry: ZipEntry<T>) {
    //     self.files.push(entry);
    // }

    pub fn with_file_iterator<II: IntoIterator<Item=ZipEntry<T>>>(iter: II) -> ZipPacker<T, II> {
        ZipPacker {
            files: iter,
        }
    }

    pub fn reader(self) -> ZipReader<T, I::IntoIter> {
        ZipReader {
            files_iter: self.files.into_iter(),
            state: ZipReaderState::Initial,
            remainder: Cursor::new(Vec::new())
        }
    }
}

impl<T: AsMut<dyn Read>> ZipPacker<T> {
    pub fn new() -> Self {
        Self {
            files: vec![]
        }
    }

    pub fn add_file<S: Into<String>>(&mut self, entry: ZipEntry<T>) {
        self.files.push(entry);
    }
}

enum ZipReaderState<T: AsMut<dyn Read>> {
    Initial,
    EntryHeader(ZipEntry<T>),
    EntryBody(ZipEntry<T>),
    EntryTail(ZipEntry<T>)
}

struct ZipReader<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> {
    files_iter: I,
    // current_entry: Option<ZipEntry<T>>,

    state: ZipReaderState<T>,

    remainder: Cursor<Vec<u8>>
}

impl<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> ZipReader<T, I> {
    fn advance(&mut self) {

    }

    fn write_entry_header(&mut self, entry: &ZipEntry<T>, mut buff: &mut [u8]) {
        if buff.len() >= 30 + entry.name.len() {
            Self::write_entry_header_impl(&entry, buff);
        } else {
            Self::write_entry_header_impl(&entry, buff.chain_mut(self.remainder.get_mut()));
        }
    }

    fn write_entry_header_impl<T: BufMut>(entry: &ZipEntry<T>, mut buff: T) {
        buff.put_u32_le(0x04034b50);
        buff.put_u16_le(0xA);
        buff.put_u16_le(0b00000000_00001000);
        buff.put_u16_le(0);
        buff.put_u16_le(/*self.modification_time*/ 0);
        buff.put_u16_le(/*self.modification_date*/ 0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u32_le(0);
        buff.put_u16_le(entry.name.len() as u16);
        buff.put_u16_le(0);
        buff.put_slice(entry.name.as_bytes());
    }
}

impl<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> Read for ZipReader<T, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // if we have remaining bytes from previous read, we flushing them all
        if self.remainder.get_ref().len() > 0 {
            return self.remainder.read(buf);
        }

        match &self.state {
            ZipReaderState::Initial => {
                let entry = self.files_iter.next().unwrap();
                self.write_entry_header(&entry, buf);

                self.state = ZipReaderState::EntryBody(entry);
            }
            _ => {}
        }
        Ok(0)
    }
}

// impl<T> Read for ZipPacker<T> where T: AsMut<dyn Read> {
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

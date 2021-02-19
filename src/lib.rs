use bytes::{Bytes, BytesMut, BufMut};
use std::io::Read;
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
        buff.put_u16_le(self.extra_field_length);
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
            current_entry: None,
            buff: BytesMut::with_capacity(64*1024)
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

struct ZipReader<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> {
    files_iter: I,
    current_entry: Option<ZipEntry<T>>,
    buff: Limit<BytesMut>
}

impl<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> ZipReader<T, I> {
    fn advance(&mut self) {

    }
}

impl<T: AsMut<dyn Read>, I: Iterator<Item=ZipEntry<T>>> Read for ZipReader<T, I> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // if buf.len() <= self.buff.
        let chain = buf.chain_mut(&mut self.buff);
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

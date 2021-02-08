use bytes::{Bytes, BytesMut, BufMut};
use std::io::Read;


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

struct ZipPacker<T> where T: AsMut<dyn Read> {
    buffer: BytesMut,
    files: Vec<(String, T)>
}

impl<T> ZipPacker<T> where T: AsMut<dyn Read> {
    pub fn new() -> Self {
        ZipPacker {
            buffer: BytesMut::with_capacity(64*1024),
            files: vec![]
        }
    }

    pub fn add_file<S: Into<String>>(&mut self, name: S, file: T) {
        self.files.push((name.into(), file));
        // self.buffer.put_u128_le()
    }

    fn fill_out_buff(&mut self) {


    }
}


impl<T> Read for ZipPacker<T> where T: AsMut<dyn Read> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.fill_out_buff();
        // self.buffer.advance()
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

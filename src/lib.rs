use bytes::{Bytes, BytesMut, BufMut};
use std::io::Read;


pub fn kappa() -> u32 {
    1337
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
        self.buffer.put_u128_le()
    }

    fn fill_out_buff(&mut self) {
        // self.buffer.put_u32_le()
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

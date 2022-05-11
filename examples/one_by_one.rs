// use zip_stream::ZipPacker;
// use std::fs::File;
// use std::io::{Read, Write};

use std::io::{Result, Write};



// trait Compressor: Write {
//     // fn write(&mut self, buff: &[u8]) -> Result<()>;
//
//     fn total_out(self) -> u64;
// }
//
// struct Stored<W: Write> {
//     inner: W,
//     out: u64
// }
//
// impl<W: Write> Write for Stored<W> {
//     fn write(&mut self, buf: &[u8]) -> Result<usize> {
//         let cnt = self.inner.write(buf)?;
//         self.out += cnt;
//
//         Ok(cnt)
//     }
//
//     fn flush(&mut self) -> Result<()> {
//         self.inner.flush()
//     }
// }
//
// impl<W: Write> Compressor for Stored<W> {
//
//     fn total_out(self) -> u64 {
//         self.out
//     }
// }

fn main() {
}
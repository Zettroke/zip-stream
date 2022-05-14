use std::io::{Result, Write};

pub mod deflate;
pub mod store;

pub use store::{Store, StoreConfig};

use crate::Header;
use crc32fast::Hasher;

pub trait CompressorConfig<W: WriterWrapper>
where
    Self: Sized,
{
    // TODO: wait for GAT stabilization and make generic over W
    type CompressorTarget: Compressor<Config = Self, Inner = W>;

    fn build(self, inner: <Self::CompressorTarget as Compressor>::Inner) -> Self::CompressorTarget {
        Self::CompressorTarget::new(self, inner)
    }
}

pub trait WriterWrapper: Write {
    type Inner;
    type Path: AsRef<str>;

    fn start_entry(&mut self, header: &mut Header<Self::Path>) -> Result<()>;
    fn end_entry(self, data: Header<Self::Path>) -> Result<Self::Inner>;
}

/// Marker trait to mark owned WriteWrapper, so we call allow creating writer for it
pub trait WriterWrapperOwned {}

pub trait Compressor: Write {
    type Inner: WriterWrapper;
    type Config;
    fn new(config: Self::Config, inner: Self::Inner) -> Self;
    fn compression_id() -> u16;
    fn finish(self) -> Result<(EntryData, Self::Inner)>;
}

#[derive(Debug)]
pub struct EntryData {
    pub uncompressed_size: u64,
    pub compressed_size: u64,
}

pub struct HashWriteWrapper<W: Write> {
    inner: W,
    hasher: Hasher,
}

impl<W: Write> Write for HashWriteWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.hasher.update(buf);
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.hasher.update(buf);
        self.inner.write_all(buf)
    }
}

impl<W: Write> HashWriteWrapper<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    pub fn finish(self) -> (u32, W) {
        (self.hasher.finalize(), self.inner)
    }
}

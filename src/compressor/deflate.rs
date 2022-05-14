use crate::compressor::{Compressor, CompressorConfig, EntryData, WriterWrapper};
use flate2::write::DeflateEncoder;
use std::io::Result;
use std::io::Write;

pub use flate2::Compression as DeflateConfig;

impl<W: WriterWrapper> CompressorConfig<W> for DeflateConfig {
    type CompressorTarget = Deflate<W>;
}

pub struct Deflate<W: WriterWrapper> {
    inner: DeflateEncoder<W>,
}

impl<W: WriterWrapper> Write for Deflate<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<W: WriterWrapper> Compressor for Deflate<W> {
    type Inner = W;
    type Config = DeflateConfig;

    fn new(config: Self::Config, inner: Self::Inner) -> Self {
        Self {
            inner: DeflateEncoder::new(inner, config),
        }
    }

    fn compression_id() -> u16 {
        8
    }

    fn finish(mut self) -> Result<(EntryData, Self::Inner)> {
        self.inner.try_finish()?;
        Ok((
            EntryData {
                uncompressed_size: self.inner.total_in(),
                compressed_size: self.inner.total_out(),
            },
            self.inner.finish()?,
        ))
    }
}

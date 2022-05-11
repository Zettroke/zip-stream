use std::io::{Write, Result};
use crate::compressor::{Compressor, EntryData, WriterWrapper, CompressorConfig};

pub struct StoreConfig;

impl<W: WriterWrapper> CompressorConfig<W> for StoreConfig {
    type CompressorTarget = Store<W>;
}


pub struct Store<W: WriterWrapper> {
    inner: W,
    out: u64,
}

impl<W: WriterWrapper> Write for Store<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let cnt = self.inner.write(buf)?;
        self.out += cnt as u64;

        Ok(cnt)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)?;

        self.out += buf.len() as u64;

        Ok(())
    }
}

impl<W: WriterWrapper> Compressor for Store<W> {
    type Inner = W;
    type Config = StoreConfig;
    fn new(_config: Self::Config, inner: W) -> Self {
        Store {
            inner,
            out: 0,
        }
    }

    fn compression_id() -> u16 {
        0
    }

    fn finish(self) -> Result<(EntryData, Self::Inner)> {
        Ok((
            EntryData {
                uncompressed_size: self.out,
                compressed_size: self.out,
            },
            self.inner
        ))
    }
}

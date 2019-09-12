use crc::crc32::{Digest, Hasher32, IEEE};
use getset::CopyGetters;
use std::{
    fs::File,
    io::{copy, sink, Read, Result},
    path::Path,
};

#[derive(CopyGetters)]
pub struct Reader<'io, IO>
where
    IO: Read,
{
    io: &'io mut IO,
    #[get_copy = "pub"]
    crc32: Option<u32>,
    have_read: usize,
    digest: Digest,
}

pub fn new_reader<'io, IO: Read + 'io>(io: &'io mut IO) -> Reader<IO> {
    Reader {
        io: io,
        crc32: None,
        have_read: 0,
        digest: Digest::new(IEEE),
    }
}

impl<'io, IO> Read for Reader<'io, IO>
where
    IO: Read + 'io,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // TODO: Think about async read
        let result = self.io.read(buf);
        if let Ok(have_read) = result {
            if buf.len() > 0 {
                if have_read > 0 {
                    self.have_read += have_read;
                    self.digest.write(buf.get(..have_read).unwrap())
                } else {
                    self.crc32 = Some(self.digest.sum32());
                }
            }
        }
        result
    }
}

pub fn from<IO: Read>(mut io: &mut IO) -> Result<u32> {
    let mut reader = new_reader(&mut io);
    copy(&mut reader, &mut sink())?;
    Ok(reader.crc32().unwrap())
}

pub fn from_bytes<S: AsRef<[u8]>>(buf: S) -> u32 {
    let mut digest = Digest::new(IEEE);
    digest.write(buf.as_ref());
    digest.sum32()
}

pub fn from_file<P: AsRef<Path>>(path: P) -> Result<u32> {
    from(&mut File::open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qiniu_test_utils::temp_file;
    use std::{
        boxed::Box,
        error::Error,
        io::{empty, Cursor},
        result::Result,
    };

    #[test]
    fn test_crc32_from_data() -> Result<(), Box<dyn Error>> {
        assert_eq!(from(&mut empty())?, 0);
        assert_eq!(from(&mut Cursor::new("crc32"))?, 2947273566);
        Ok(())
    }

    #[test]
    fn test_crc32_from_file() -> Result<(), Box<dyn Error>> {
        assert_eq!(from_file(temp_file::create_temp_file(1 << 20)?)?, 2584642182);
        assert_eq!(from_file(temp_file::create_temp_file(4 * (1 << 20))?)?, 3216722958);
        assert_eq!(from_file(temp_file::create_temp_file(5 * (1 << 20))?)?, 1123170717);
        assert_eq!(from_file(temp_file::create_temp_file(8 * (1 << 20))?)?, 267778539);
        assert_eq!(from_file(temp_file::create_temp_file(9 * (1 << 20))?)?, 962330351);
        Ok(())
    }
}

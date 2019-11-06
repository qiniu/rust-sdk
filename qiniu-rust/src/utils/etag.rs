use super::base64;
use crypto::{digest::Digest, sha1::Sha1};
use std::{
    fs::File,
    io::{copy, sink, Read, Result},
    option::Option,
    path::Path,
};
use tap::TapResultOps;

pub const BLOCK_SIZE: usize = 1 << 22;
pub const ETAG_SIZE: usize = 28;

pub struct Etag {
    buffer: Vec<u8>,
    sha1s: Vec<Vec<u8>>,
}

pub fn new() -> Etag {
    Etag {
        buffer: Vec::new(),
        sha1s: Vec::new(),
    }
}

impl Digest for Etag {
    fn input(&mut self, input: &[u8]) {
        self.buffer.extend_from_slice(input);
        let mut iter = self.buffer.chunks_exact(BLOCK_SIZE);
        for block in iter.by_ref() {
            self.sha1s.push(Self::sha1(block));
        }
        self.buffer = {
            let mut new_buffer = Vec::new();
            new_buffer.extend_from_slice(iter.remainder());
            new_buffer
        };
    }

    fn result(&mut self, mut out: &mut [u8]) {
        if !self.buffer.is_empty() {
            self.sha1s.push(Self::sha1(&self.buffer));
            self.buffer.clear();
        }
        self.encode_sha1s(&mut out);
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.sha1s.clear();
    }

    fn output_bits(&self) -> usize {
        ETAG_SIZE * 8
    }

    fn block_size(&self) -> usize {
        self.sha1s.len()
    }
}

impl Etag {
    fn sha1(bytes: &[u8]) -> Vec<u8> {
        let mut sha1 = Sha1::new();
        sha1.input(bytes);
        let mut result = vec![0; sha1.output_bytes()];
        sha1.result(&mut result);
        result
    }

    fn encode_sha1s(&mut self, mut result: &mut [u8]) {
        match self.sha1s.len() {
            0 => result.copy_from_slice(b"Fto5o-5ea0sNMlW_75VgGJCv2AcJ"),
            1 => {
                let mut buf = Vec::with_capacity(32);
                buf.push(0x16u8);
                buf.extend_from_slice(self.sha1s.first().unwrap());
                base64::urlsafe_slice(&buf, &mut result);
            }
            _ => {
                let mut buf = Vec::with_capacity(1024);
                for sha1 in self.sha1s.iter() {
                    buf.extend_from_slice(&sha1);
                }
                let sha1 = Self::sha1(&buf);
                buf.clear();
                buf.push(0x96u8);
                buf.extend_from_slice(&sha1);
                base64::urlsafe_slice(&buf, &mut result);
            }
        }
    }
}

pub struct Reader<IO>
where
    IO: Read,
{
    io: IO,
    etag: Option<String>,
    have_read: usize,
    digest: Etag,
}

pub fn new_reader<IO: Read>(io: IO) -> Reader<IO> {
    Reader {
        io,
        etag: None,
        have_read: 0,
        digest: new(),
    }
}

impl<IO> Read for Reader<IO>
where
    IO: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.io.read(buf).tap_ok(|&mut have_read| {
            if !buf.is_empty() {
                if have_read > 0 {
                    self.have_read += have_read;
                    self.digest.input(buf.get(..have_read).unwrap())
                } else {
                    let mut etag = vec![0; self.digest.output_bytes()];
                    self.digest.result(&mut etag);
                    self.etag = Some(String::from_utf8(etag).unwrap());
                }
            }
        })
    }
}

impl<IO> Reader<IO>
where
    IO: Read,
{
    pub fn etag(&self) -> Option<&str> {
        self.etag.as_ref().map(|s| s.as_str())
    }

    pub fn into_etag(self) -> Option<String> {
        self.etag
    }
}

pub fn from<IO: Read>(io: IO) -> Result<String> {
    let mut reader = new_reader(io);
    copy(&mut reader, &mut sink())?;
    Ok(reader.into_etag().unwrap())
}

pub fn from_bytes<S: AsRef<[u8]>>(buf: S) -> String {
    let mut etag_digest = new();
    etag_digest.input(buf.as_ref());
    let mut etag = vec![0; etag_digest.output_bytes()];
    etag_digest.result(&mut etag);
    String::from_utf8(etag).unwrap()
}

pub fn from_file<P: AsRef<Path>>(path: P) -> Result<String> {
    from(File::open(path)?)
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
    fn test_etag_from_data() -> Result<(), Box<dyn Error>> {
        assert_eq!(from(&mut empty())?, "Fto5o-5ea0sNMlW_75VgGJCv2AcJ",);
        assert_eq!(from(&mut Cursor::new("etag"))?, "FpLiADEaVoALPkdb8tJEJyRTXoe_");
        Ok(())
    }

    #[test]
    fn test_etag_from_file() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            from_file(temp_file::create_temp_file(1 << 20)?)?,
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(4 * (1 << 20))?)?,
            "FicHOveBNs5Kn9d74M3b9tI4D-8r",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(5 * (1 << 20))?)?,
            "lg-Eb5KFCuZn-cUfj_oS2PPOU9xy",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(8 * (1 << 20))?)?,
            "lkSKZOMToDp-EqLDVuT1pyjQssl-",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(9 * (1 << 20))?)?,
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
        Ok(())
    }
}

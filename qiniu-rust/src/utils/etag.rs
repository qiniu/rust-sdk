use super::base64;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::fs::OpenOptions;
use std::io::copy;
use std::io::sink;
use std::io::Read;
use std::io::Result;
use std::iter;
use std::option::Option;
use std::path::Path;

const BLOCK_SIZE: usize = 1 << 22;

pub struct Reader<'io, IO>
where
    IO: Read,
{
    io: &'io mut IO,
    etag: Option<String>,
    have_read: usize,
    buffer: Vec<u8>,
    sha1s: Vec<Vec<u8>>,
}

pub fn new_reader<'io, IO: Read + 'io>(io: &'io mut IO) -> Reader<IO> {
    Reader {
        io: io,
        etag: None,
        have_read: 0,
        buffer: Vec::new(),
        sha1s: Vec::new(),
    }
}

impl<'io, IO> Read for Reader<'io, IO>
where
    IO: Read + 'io,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let result = self.io.read(buf);

        if let Ok(have_read) = result {
            if buf.len() > 0 {
                if have_read > 0 {
                    self.have_read += have_read;
                    self.update_buffer(buf.get(..have_read).unwrap());
                } else {
                    self.calculate_etag();
                }
            }
        }

        result
    }
}

impl<'io, IO> Reader<'io, IO>
where
    IO: Read + 'io,
{
    fn update_buffer(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
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

    fn calculate_etag(&mut self) {
        if !self.buffer.is_empty() {
            self.sha1s.push(Self::sha1(&self.buffer));
        }
        self.buffer.clear();
        self.etag = Some(self.encode_sha1s());
    }

    fn sha1(bytes: &[u8]) -> Vec<u8> {
        let mut sha1 = Sha1::new();
        sha1.input(bytes);
        let mut result = iter::repeat(0)
            .take(sha1.output_bytes())
            .collect::<Vec<u8>>();
        sha1.result(&mut result);
        result
    }

    fn encode_sha1s(&mut self) -> String {
        match self.sha1s.len() {
            0 => "Fto5o-5ea0sNMlW_75VgGJCv2AcJ".to_owned(),
            1 => {
                let mut buf = Vec::with_capacity(32);
                buf.push(0x16u8);
                buf.extend_from_slice(self.sha1s.first().unwrap());
                base64::urlsafe(&buf)
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
                base64::urlsafe(&buf)
            }
        }
    }

    pub fn etag(&self) -> &Option<String> {
        &self.etag
    }
}

pub fn from<IO: Read>(mut io: &mut IO) -> Result<String> {
    let mut reader = new_reader(&mut io);
    copy(&mut reader, &mut sink())?;
    Ok(reader.etag().clone().unwrap())
}

pub fn from_file<P: AsRef<Path>>(path: P) -> Result<String> {
    from(&mut OpenOptions::new().read(true).open(path)?)
}

#[cfg(test)]
mod tests {
    use super::super::temp_file;
    use super::*;

    #[test]
    fn test_etag_from_data() {
        assert_eq!(
            from(&mut std::io::empty()).unwrap(),
            "Fto5o-5ea0sNMlW_75VgGJCv2AcJ",
        );
        assert_eq!(
            from(&mut stringreader::StringReader::new("etag")).unwrap(),
            "FpLiADEaVoALPkdb8tJEJyRTXoe_"
        );
    }

    #[test]
    fn test_etag_from_file() {
        assert_eq!(
            from_file(temp_file::create_temp_file(1 << 20).unwrap()).unwrap(),
            "Foyl8onxBLWeRLL5oItRJphv6i4b",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(4 * (1 << 20)).unwrap()).unwrap(),
            "FicHOveBNs5Kn9d74M3b9tI4D-8r",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(5 * (1 << 20)).unwrap()).unwrap(),
            "lg-Eb5KFCuZn-cUfj_oS2PPOU9xy",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(8 * (1 << 20)).unwrap()).unwrap(),
            "lkSKZOMToDp-EqLDVuT1pyjQssl-",
        );
        assert_eq!(
            from_file(temp_file::create_temp_file(9 * (1 << 20)).unwrap()).unwrap(),
            "ljgVjMtyMsOgIySv79U8Qz4TrUO4",
        );
    }
}

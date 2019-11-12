use std::io::Result;
use std::io::Write;
use tempfile::NamedTempFile;

const FAKE_DATA: [u8; 4096] = make_fake_data();

pub fn create_temp_file(size: usize) -> Result<NamedTempFile> {
    let mut temp = NamedTempFile::new()?;
    let mut rest = size;

    while rest > 0 {
        let to_write = rest.min(FAKE_DATA.len());
        let mut have_written = 0usize;
        while have_written < to_write {
            have_written += temp
                .as_file_mut()
                .write(&FAKE_DATA.get(have_written..to_write).unwrap())?;
        }
        rest -= to_write;
    }
    Ok(temp)
}

const fn make_fake_data() -> [u8; 4096] {
    let mut buf = [b'b'; 4096];
    buf[0] = b'A';
    buf[4094] = b'\r';
    buf[4095] = b'\n';
    buf
}

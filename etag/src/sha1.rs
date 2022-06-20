use sha1::{Digest, Sha1};

pub(super) fn sha1(bytes: &[u8]) -> Vec<u8> {
    let mut sha1 = Sha1::default();
    sha1.update(bytes);
    sha1.finalize().to_vec()
}

pub(super) fn hash_sha1s(sha1s: &[Vec<u8>]) -> Vec<u8> {
    match sha1s.len() {
        0 => vec![
            0x16, 0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0xd, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60, 0x18, 0x90, 0xaf,
            0xd8, 0x7, 0x9,
        ],
        1 => {
            let mut buf = Vec::with_capacity(32);
            buf.push(0x16u8);
            buf.extend_from_slice(sha1s.first().unwrap());
            buf
        }
        _ => {
            let mut buf = Vec::with_capacity(sha1s.iter().map(|s| s.len()).sum());
            for sha1 in sha1s.iter() {
                buf.extend_from_slice(sha1);
            }
            let sha1 = sha1(&buf);
            buf.clear();
            buf.push(0x96u8);
            buf.extend_from_slice(&sha1);
            buf
        }
    }
}

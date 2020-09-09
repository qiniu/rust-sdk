use super::super::http::{HeaderName, HeaderValue, StatusCode};
use std::str::from_utf8;

#[inline]
pub(crate) fn is_status_line(line: &[u8]) -> bool {
    line.starts_with(b"HTTP/")
}

#[inline]
pub(crate) fn is_ended_line(line: &[u8]) -> bool {
    line == b"\r\n"
}

#[inline]
pub(crate) fn parse_status_line(line: &[u8]) -> Option<StatusCode> {
    line.split(u8::is_ascii_whitespace)
        .skip(1)
        .find(|s| !s.is_empty())
        .and_then(|s| from_utf8(s).ok())
        .and_then(|s| s.parse::<StatusCode>().ok())
}

#[inline]
pub(crate) fn parse_header_line(line: &[u8]) -> Option<(HeaderName, HeaderValue)> {
    if let Ok(line) = from_utf8(line) {
        let mut iter = line
            .trim_matches(char::is_whitespace)
            .splitn(2, ':')
            .take(2)
            .map(|s| s.trim_matches(char::is_whitespace));
        if let (Some(header_name), Some(header_value)) = (iter.next(), iter.next()) {
            return Some((header_name.into(), header_value.into()));
        }
    }
    None
}

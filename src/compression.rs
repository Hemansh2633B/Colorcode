//! Compression abstraction for payload bytes.

use std::fmt;
use std::io::Cursor;
use std::str::FromStr;

use crate::types::ColorCodeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compression {
    None,
    Lz4,
    Zstd,
}

impl Compression {
    pub fn id(self) -> u8 {
        match self {
            Compression::None => 0,
            Compression::Lz4 => 1,
            Compression::Zstd => 2,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Compression::None),
            1 => Some(Compression::Lz4),
            2 => Some(Compression::Zstd),
            _ => None,
        }
    }
}

impl Default for Compression {
    fn default() -> Self {
        Compression::None
    }
}

impl fmt::Display for Compression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Compression::None => f.write_str("none"),
            Compression::Lz4 => f.write_str("lz4"),
            Compression::Zstd => f.write_str("zstd"),
        }
    }
}

impl FromStr for Compression {
    type Err = ColorCodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(Compression::None),
            "lz4" => Ok(Compression::Lz4),
            "zstd" | "zstandard" => Ok(Compression::Zstd),
            _ => Err(ColorCodeError::InvalidOption(format!(
                "unknown compression method '{value}'"
            ))),
        }
    }
}

pub fn compress(data: &[u8], method: Compression) -> Result<Vec<u8>, ColorCodeError> {
    match method {
        Compression::None => Ok(data.to_vec()),
        Compression::Lz4 => Ok(lz4_flex::block::compress_prepend_size(data)),
        Compression::Zstd => zstd::stream::encode_all(Cursor::new(data), 3)
            .map_err(|err| ColorCodeError::Compression(err.to_string())),
    }
}

pub fn decompress(data: &[u8], method: Compression) -> Result<Vec<u8>, ColorCodeError> {
    match method {
        Compression::None => Ok(data.to_vec()),
        Compression::Lz4 => lz4_flex::block::decompress_size_prepended(data)
            .map_err(|err| ColorCodeError::Compression(err.to_string())),
        Compression::Zstd => zstd::stream::decode_all(Cursor::new(data))
            .map_err(|err| ColorCodeError::Compression(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz4_roundtrip() {
        let data = b"aaaaabbbbbccccccddddddaaaaabbbbbcccccc";
        let encoded = compress(data, Compression::Lz4).unwrap();
        let decoded = decompress(&encoded, Compression::Lz4).unwrap();
        assert_eq!(decoded, data);
    }
}

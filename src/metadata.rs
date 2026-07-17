//! Fixed-size metadata frame.

use crate::base16::{pack_bytes_to_nibbles, unpack_nibbles_to_bytes};
use crate::compression::Compression;
use crate::types::{ColorCodeError, EccLevel, Version};

pub const MAGIC: [u8; 4] = *b"C16M";
pub const METADATA_BYTES: usize = 16;
pub const METADATA_NIBBLES: usize = METADATA_BYTES * 2;
pub const METADATA_COPIES: usize = 2;
pub const METADATA_TOTAL_NIBBLES: usize = METADATA_NIBBLES * METADATA_COPIES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    pub version: Version,
    pub flags: u8,
    pub payload_len: u32,
    pub compression: Compression,
    pub ecc_level: EccLevel,
    pub crc32: u32,
}

impl Metadata {
    pub fn new(
        payload_len: usize,
        compression: Compression,
        ecc_level: EccLevel,
        crc32: u32,
    ) -> Result<Self, ColorCodeError> {
        let payload_len = u32::try_from(payload_len).map_err(|_| {
            ColorCodeError::InvalidMetadata("payload length exceeds u32".to_string())
        })?;
        let flags = if compression == Compression::None {
            0
        } else {
            0b0000_0001
        };
        Ok(Self {
            version: Version,
            flags,
            payload_len,
            compression,
            ecc_level,
            crc32,
        })
    }

    pub fn to_bytes(self) -> [u8; METADATA_BYTES] {
        let mut bytes = [0u8; METADATA_BYTES];
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4] = self.version.id();
        bytes[5] = self.flags;
        bytes[6] = self.compression.id();
        bytes[7] = self.ecc_level.id();
        bytes[8..12].copy_from_slice(&self.payload_len.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.crc32.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: [u8; METADATA_BYTES]) -> Result<Self, ColorCodeError> {
        if bytes[..4] != MAGIC {
            return Err(ColorCodeError::InvalidMetadata(
                "magic number mismatch".to_string(),
            ));
        }
        let version_byte = bytes[4];
        if version_byte != 2 {
            return Err(ColorCodeError::InvalidMetadata(
                "unsupported version".to_string(),
            ));
        }
        let version = Version::current();
        let compression = Compression::from_id(bytes[6]).ok_or_else(|| {
            ColorCodeError::InvalidMetadata("unsupported compression method".to_string())
        })?;
        let ecc_level = EccLevel::from_id(bytes[7])
            .ok_or_else(|| ColorCodeError::InvalidMetadata("unsupported ECC level".to_string()))?;
        let payload_len = u32::from_be_bytes(bytes[8..12].try_into().expect("slice length"));
        let crc32 = u32::from_be_bytes(bytes[12..16].try_into().expect("slice length"));
        Ok(Self {
            version,
            flags: bytes[5],
            payload_len,
            compression,
            ecc_level,
            crc32,
        })
    }

    pub fn to_nibbles(self) -> [u8; METADATA_NIBBLES] {
        let bytes = self.to_bytes();
        let mut nibbles = [0u8; METADATA_NIBBLES];
        pack_bytes_to_nibbles(&bytes, &mut nibbles);
        nibbles
    }

    pub fn from_nibbles(nibbles: &[u8]) -> Result<Self, ColorCodeError> {
        if nibbles.len() != METADATA_NIBBLES {
            return Err(ColorCodeError::InvalidMetadata(format!(
                "expected {METADATA_NIBBLES} metadata nibbles, got {}",
                nibbles.len()
            )));
        }
        let mut bytes = [0u8; METADATA_BYTES];
        unpack_nibbles_to_bytes(nibbles, &mut bytes);
        Self::from_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_roundtrip() {
        let metadata = Metadata::new(123, Compression::Zstd, EccLevel::High, 0x1234_5678).unwrap();
        let decoded = Metadata::from_nibbles(&metadata.to_nibbles()).unwrap();
        assert_eq!(decoded, metadata);
    }
}

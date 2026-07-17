//! Encoder pipeline.

use crc32fast::Hasher;

use crate::base16::pack_bytes_to_nibbles;
use crate::compression::{self, Compression};
use crate::ecc;
use crate::layout;
use crate::metadata::Metadata;
use crate::types::{
    ColorCodeError, ColorMatrix, DEFAULT_MODULE_SIZE, DEFAULT_QUIET_ZONE, EccLevel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderOptions {
    pub ecc_level: EccLevel,
    pub compression: Compression,
    pub module_size: u32,
    pub quiet_zone: u32,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            ecc_level: EccLevel::Low,
            compression: Compression::None,
            module_size: DEFAULT_MODULE_SIZE,
            quiet_zone: DEFAULT_QUIET_ZONE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodedCode {
    pub matrix: ColorMatrix,
    pub metadata: Metadata,
    pub stored_payload_len: usize,
    pub ecc_nibbles: usize,
}

impl EncodedCode {
    pub fn render_png(&self, module_size: u32) -> image::RgbImage {
        self.matrix.render_png(module_size)
    }

    pub fn save_png<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        module_size: u32,
    ) -> Result<(), ColorCodeError> {
        self.matrix.save_png(path, module_size)
    }
}

#[derive(Debug, Clone)]
pub struct Encoder {
    options: EncoderOptions,
}

impl Encoder {
    pub fn new(options: EncoderOptions) -> Self {
        Self { options }
    }

    pub fn options(&self) -> EncoderOptions {
        self.options
    }

    pub fn encode(data: &[u8]) -> Result<EncodedCode, ColorCodeError> {
        Self::new(EncoderOptions::default()).encode_bytes(data)
    }

    pub fn encode_bytes(&self, data: &[u8]) -> Result<EncodedCode, ColorCodeError> {
        let compressed = compression::compress(data, self.options.compression)?;
        let mut hasher = Hasher::new();
        hasher.update(data);
        let crc32 = hasher.finalize();

        let data_nibble_count = compressed.len() * 2;
        let encoded_count = ecc::encoded_nibble_count(data_nibble_count, self.options.ecc_level);
        let capacity = layout::payload_capacity_nibbles();
        if encoded_count > capacity {
            return Err(ColorCodeError::PayloadTooLarge {
                needed: encoded_count,
                capacity,
            });
        }

        let metadata = Metadata::new(
            compressed.len(),
            self.options.compression,
            self.options.ecc_level,
            crc32,
        )?;

        let mut nibbles = vec![0u8; data_nibble_count];
        pack_bytes_to_nibbles(&compressed, &mut nibbles);
        let encoded_payload = ecc::encode(&nibbles, self.options.ecc_level)?;

        let mut matrix = ColorMatrix::new();
        layout::apply_function_patterns(&mut matrix);
        layout::write(&mut matrix, metadata, &encoded_payload)?;

        Ok(EncodedCode {
            matrix,
            metadata,
            stored_payload_len: compressed.len(),
            ecc_nibbles: encoded_payload.len(),
        })
    }
}

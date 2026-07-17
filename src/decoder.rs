//! Decoder pipeline.

use std::path::Path;

use crc32fast::Hasher;

use crate::base16::unpack_nibbles_to_bytes;
use crate::compression;
use crate::ecc;
use crate::layout;
use crate::metadata::Metadata;
use crate::types::{ColorCodeError, ColorMatrix};
use crate::vision::{self, SampledMatrix};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoderOptions {
    pub color_tolerance: f32,
}

impl Default for DecoderOptions {
    fn default() -> Self {
        Self {
            color_tolerance: 38.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPayload {
    pub data: Vec<u8>,
    pub metadata: Metadata,
    pub crc32_valid: bool,
    pub ecc_corrections: usize,
}

#[derive(Debug, Clone)]
pub struct Decoder {
    options: DecoderOptions,
}

impl Decoder {
    pub fn new(options: DecoderOptions) -> Self {
        Self { options }
    }

    pub fn decode<P: AsRef<Path>>(path: P) -> Result<DecodedPayload, ColorCodeError> {
        Self::new(DecoderOptions::default()).decode_path(path)
    }

    pub fn decode_path<P: AsRef<Path>>(&self, path: P) -> Result<DecodedPayload, ColorCodeError> {
        let image = image::open(path)?;
        let sampled = vision::sample_image(&image, self.options.color_tolerance)?;
        self.decode_sampled(&sampled)
    }

    pub fn decode_matrix(&self, matrix: &ColorMatrix) -> Result<DecodedPayload, ColorCodeError> {
        let sampled = SampledMatrix {
            matrix: matrix.clone(),
            erasures: vec![false; crate::types::TOTAL_MODULES],
            distances: vec![0.0; crate::types::TOTAL_MODULES],
        };
        self.decode_sampled(&sampled)
    }

    pub fn decode_sampled(
        &self,
        sampled: &SampledMatrix,
    ) -> Result<DecodedPayload, ColorCodeError> {
        let mut last_error = None;
        let mut oriented = Vec::with_capacity(4);
        for turns in 0..4 {
            let sample = sampled.rotated(turns);
            let score = layout::function_mismatch_score(&sample.matrix);
            oriented.push((score, sample));
        }
        oriented.sort_by_key(|(score, _)| *score);

        for (_, sample) in oriented {
            let candidates = layout::read_metadata_candidates(&sample.matrix);
            if candidates.is_empty() {
                continue;
            }

            for metadata in candidates {
                match self.decode_oriented(&sample, metadata) {
                    Ok(decoded) => return Ok(decoded),
                    Err(err) => last_error = Some(err),
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ColorCodeError::ImageDecode("no valid metadata candidates found".to_string())
        }))
    }

    fn decode_oriented(
        &self,
        sampled: &SampledMatrix,
        metadata: Metadata,
    ) -> Result<DecodedPayload, ColorCodeError> {
        let payload_nibbles = metadata.payload_len as usize * 2;
        let encoded_count = ecc::encoded_nibble_count(payload_nibbles, metadata.ecc_level);
        let (encoded, erasures) =
            layout::read_payload_with_erasures(&sampled.matrix, &sampled.erasures, encoded_count)?;
        let (data_nibbles, corrections) =
            ecc::decode_with_erasures(&encoded, metadata.ecc_level, payload_nibbles, &erasures)?;

        let mut stored_payload = vec![0u8; metadata.payload_len as usize];
        unpack_nibbles_to_bytes(&data_nibbles, &mut stored_payload);
        let data = compression::decompress(&stored_payload, metadata.compression)?;

        let mut hasher = Hasher::new();
        hasher.update(&data);
        let actual_crc = hasher.finalize();
        if actual_crc != metadata.crc32 {
            return Err(ColorCodeError::CrcMismatch {
                expected: metadata.crc32,
                actual: actual_crc,
            });
        }

        Ok(DecodedPayload {
            data,
            metadata,
            crc32_valid: true,
            ecc_corrections: corrections,
        })
    }
}

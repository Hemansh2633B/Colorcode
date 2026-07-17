//! ColorCode is a 16-color, nibble-addressed matrix code.
//!
//! The crate exposes a small API around the Version 2 format:
//! encode bytes into a 1280x1280 color matrix, render it losslessly, and decode
//! rendered images back into the original byte stream.

pub mod base16;
pub mod classifier;
pub mod compression;
pub mod decoder;
pub mod ecc;
pub mod encoder;
pub mod finder;
pub mod geometry;
pub mod homography;
pub mod layout;
pub mod metadata;
pub mod palette;
pub mod renderer;
pub mod timing;
pub mod types;
pub mod vision;

#[cfg(target_os = "android")]
pub mod android_jni;

pub use compression::Compression;
pub use decoder::{DecodedPayload, Decoder, DecoderOptions};
pub use encoder::{EncodedCode, Encoder, EncoderOptions};
pub use types::{
    ALL_VERSIONS, ColorCodeError, ColorMatrix, EccLevel, MATRIX_SIZE, MAX_MATRIX_SIZE, Version,
    DEFAULT_QUIET_ZONE, DEFAULT_MODULE_SIZE,
};

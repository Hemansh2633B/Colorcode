//! ColorCode is a 16-color, nibble-addressed matrix code.
//!
//! The crate exposes a small API around the Version 1 format:
//! encode bytes into a 32x32 color matrix, render it losslessly, and decode
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

pub use compression::Compression;
pub use decoder::{DecodedPayload, Decoder, DecoderOptions};
pub use encoder::{EncodedCode, Encoder, EncoderOptions};
pub use types::{ColorCodeError, ColorMatrix, EccLevel, Version};

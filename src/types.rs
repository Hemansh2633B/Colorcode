//! Core types and constants for the ColorCode format.

use std::fmt;
use std::str::FromStr;

/// The Version 2 matrix dimension: 1280x1280 modules (approx 500KB capacity).
pub const MATRIX_SIZE: usize = 1280;

/// Total number of modules in the matrix.
pub const TOTAL_MODULES: usize = MATRIX_SIZE * MATRIX_SIZE;

/// Maximum matrix size across all versions (for backwards compatibility).
pub const MAX_MATRIX_SIZE: usize = MATRIX_SIZE;

/// All supported versions (currently only V2).
pub const ALL_VERSIONS: &[Version] = &[Version];

/// Default quiet zone used by the renderer, measured in modules.
pub const DEFAULT_QUIET_ZONE: u32 = 4;

/// Default module size used by the CLI.
pub const DEFAULT_MODULE_SIZE: u32 = 16;

/// The single supported version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version;

impl Version {
    pub const fn current() -> Self {
        Version
    }

    #[inline]
    pub const fn id(self) -> u8 {
        2
    }

    #[inline]
    pub const fn size(self) -> usize {
        MATRIX_SIZE
    }

    /// Parse CLI text such as `v2` / `2` / `version 2`.
    pub fn parse(text: &str) -> Option<Self> {
        let normalized = text.trim().to_ascii_lowercase();
        let normalized = normalized
            .strip_prefix("version")
            .map(str::trim)
            .unwrap_or(&normalized);
        matches!(normalized, "v2" | "2" | "version").then_some(Version)
    }
}

impl Default for Version {
    fn default() -> Self {
        Version
    }
}

/// Error correction level with GF(16) block parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EccLevel {
    Low,
    Medium,
    High,
    Maximum,
}

impl EccLevel {
    pub fn id(self) -> u8 {
        match self {
            EccLevel::Low => 0,
            EccLevel::Medium => 1,
            EccLevel::High => 2,
            EccLevel::Maximum => 3,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(EccLevel::Low),
            1 => Some(EccLevel::Medium),
            2 => Some(EccLevel::High),
            3 => Some(EccLevel::Maximum),
            _ => None,
        }
    }

    /// Number of data symbols in each GF(16) block.
    pub fn data_shards(self) -> usize {
        match self {
            EccLevel::Low => 13,
            EccLevel::Medium => 11,
            EccLevel::High => 9,
            EccLevel::Maximum => 7,
        }
    }

    /// Number of parity symbols in each GF(16) block.
    pub fn parity_shards(self) -> usize {
        15 - self.data_shards()
    }

    pub fn block_len(self) -> usize {
        self.data_shards() + self.parity_shards()
    }

    /// Maximum unknown symbol errors corrected per block.
    pub fn max_symbol_errors(self) -> usize {
        self.parity_shards() / 2
    }
}

impl Default for EccLevel {
    fn default() -> Self {
        EccLevel::Low
    }
}

impl fmt::Display for EccLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EccLevel::Low => f.write_str("low"),
            EccLevel::Medium => f.write_str("medium"),
            EccLevel::High => f.write_str("high"),
            EccLevel::Maximum => f.write_str("maximum"),
        }
    }
}

impl FromStr for EccLevel {
    type Err = ColorCodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "low" => Ok(EccLevel::Low),
            "medium" | "med" => Ok(EccLevel::Medium),
            "high" => Ok(EccLevel::High),
            "maximum" | "max" => Ok(EccLevel::Maximum),
            _ => Err(ColorCodeError::InvalidOption(format!(
                "unknown ECC level '{value}'"
            ))),
        }
    }
}

/// The 1280×1280 color matrix, stored row-major.
#[derive(Clone, PartialEq, Eq)]
pub struct ColorMatrix {
    pub data: Vec<u8>,
    pub size: usize,
}

impl ColorMatrix {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; TOTAL_MODULES],
            size: MATRIX_SIZE,
        }
    }

    pub fn new_for(_version: Version) -> Self {
        Self::new()
    }

    /// Get a mutable reference to a module at (row, col).
    #[inline]
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut u8 {
        debug_assert!(row < self.size && col < self.size);
        &mut self.data[row * self.size + col]
    }

    /// Get a module value at (row, col).
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> u8 {
        debug_assert!(row < self.size && col < self.size);
        self.data[row * self.size + col]
    }

    /// Set a module value at (row, col).
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: u8) {
        debug_assert!(value < 16);
        *self.get_mut(row, col) = value;
    }

    pub fn rotated_clockwise(&self) -> Self {
        let mut rotated = Self::new();
        for row in 0..self.size {
            for col in 0..self.size {
                rotated.set(col, self.size - 1 - row, self.get(row, col));
            }
        }
        rotated
    }

    pub fn rotated(&self, turns_clockwise: u8) -> Self {
        let mut matrix = self.clone();
        for _ in 0..turns_clockwise % 4 {
            matrix = matrix.rotated_clockwise();
        }
        matrix
    }

    pub fn resize_to(&mut self, new_size: usize) {
        self.data.resize(new_size * new_size, 0);
        self.size = new_size;
    }

    pub fn iter(&self) -> impl Iterator<Item = &u8> {
        self.data.iter()
    }
}

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ColorMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorMatrix")
            .field("size", &MATRIX_SIZE)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ColorCodeError {
    #[error("payload is too large: needs {needed} nibbles, capacity is {capacity}")]
    PayloadTooLarge { needed: usize, capacity: usize },
    #[error("payload size {needed} bytes exceeds maximum supported capacity {capacity} bytes")]
    PayloadTooLargeBytes { needed: usize, capacity: usize },
    #[error("invalid metadata: {0}")]
    InvalidMetadata(String),
    #[error("invalid option: {0}")]
    InvalidOption(String),
    #[error("image could not be decoded: {0}")]
    ImageDecode(String),
    #[error("CRC32 mismatch: expected {expected:08x}, got {actual:08x}")]
    CrcMismatch { expected: u32, actual: u32 },
    #[error("ECC failed: {0}")]
    Ecc(String),
    #[error("compression failed: {0}")]
    Compression(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

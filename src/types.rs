//! Core types and constants for the ColorCode format.

use std::fmt;
use std::str::FromStr;

/// The Version 1 matrix dimension: 32x32 modules.
pub const MATRIX_SIZE: usize = 32;

/// Total number of modules in the matrix.
pub const TOTAL_MODULES: usize = MATRIX_SIZE * MATRIX_SIZE;

/// Raw nibble capacity before reserved areas and error correction.
pub const RAW_CAPACITY_NIBBLES: usize = TOTAL_MODULES;

/// Default quiet zone used by the renderer, measured in modules.
pub const DEFAULT_QUIET_ZONE: u32 = 4;

/// Default module size used by the CLI.
pub const DEFAULT_MODULE_SIZE: u32 = 16;

/// Version identifier. The enum is intentionally small today, but keeps the
/// public API open for larger matrices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Version {
    V1,
}

impl Version {
    #[inline]
    pub fn id(self) -> u8 {
        match self {
            Version::V1 => 1,
        }
    }

    #[inline]
    pub fn size(self) -> usize {
        match self {
            Version::V1 => MATRIX_SIZE,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Version::V1),
            _ => None,
        }
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

/// The Version 1 color matrix, stored row-major.
#[derive(Clone, PartialEq, Eq)]
pub struct ColorMatrix {
    pub data: [u8; TOTAL_MODULES],
}

impl ColorMatrix {
    pub fn new() -> Self {
        ColorMatrix {
            data: [0u8; TOTAL_MODULES],
        }
    }

    /// Get a mutable reference to a module at (row, col).
    #[inline]
    pub fn get_mut(&mut self, row: usize, col: usize) -> &mut u8 {
        debug_assert!(row < MATRIX_SIZE && col < MATRIX_SIZE);
        &mut self.data[row * MATRIX_SIZE + col]
    }

    /// Get a module value at (row, col).
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> u8 {
        debug_assert!(row < MATRIX_SIZE && col < MATRIX_SIZE);
        self.data[row * MATRIX_SIZE + col]
    }

    /// Set a module value at (row, col).
    #[inline]
    pub fn set(&mut self, row: usize, col: usize, value: u8) {
        debug_assert!(value < 16);
        *self.get_mut(row, col) = value;
    }

    pub fn rotated_clockwise(&self) -> Self {
        let mut rotated = Self::new();
        for row in 0..MATRIX_SIZE {
            for col in 0..MATRIX_SIZE {
                rotated.set(col, MATRIX_SIZE - 1 - row, self.get(row, col));
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

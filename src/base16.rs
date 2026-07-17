//! Base-16 (nibble) encoding: no conversion needed.
//!
//! Since each cell stores a 4-bit nibble directly, bytes map
//! trivially: each byte → 2 nibbles. This module provides framing
//! metadata so the decoder knows the original byte count.

/// Pack bytes into nibbles (4-bit values, 0-15).
///
/// Each byte produces two nibbles: high nibble first, then low nibble.
/// Zero allocation — caller provides the output buffer.
#[inline]
pub fn pack_bytes_to_nibbles(input: &[u8], output: &mut [u8]) {
    debug_assert!(output.len() >= input.len() * 2);
    for (i, byte) in input.iter().enumerate() {
        output[i * 2] = byte >> 4; // High nibble
        output[i * 2 + 1] = byte & 0x0F; // Low nibble
    }
}

/// Unpack nibbles back into bytes.
///
/// Reads pairs of nibbles (high, low) and reassembles into bytes.
/// Extra trailing nibbles (if odd count) are ignored.
#[inline]
pub fn unpack_nibbles_to_bytes(nibbles: &[u8], output: &mut [u8]) {
    let pair_count = (nibbles.len() / 2).min(output.len());
    for i in 0..pair_count {
        output[i] = (nibbles[i * 2] << 4) | (nibbles[i * 2 + 1] & 0x0F);
    }
}

/// Validate that all nibbles are in range 0-15.
pub fn validate_nibbles(nibbles: &[u8]) -> Result<(), NibbleError> {
    for (i, &n) in nibbles.iter().enumerate() {
        if n > 15 {
            return Err(NibbleError {
                position: i,
                value: n,
            });
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid nibble value {value} at position {position} (must be 0-15)")]
pub struct NibbleError {
    pub position: usize,
    pub value: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack_roundtrip() {
        let original = b"Hello World! 12345";
        let mut nibbles = vec![0u8; original.len() * 2];
        pack_bytes_to_nibbles(original, &mut nibbles);

        let mut decoded = vec![0u8; original.len()];
        unpack_nibbles_to_bytes(&nibbles, &mut decoded);
        assert_eq!(&decoded, original);
    }

    #[test]
    fn test_nibble_values() {
        let bytes = &[0x00, 0xFF, 0xAB, 0x12];
        let mut nibbles = vec![0u8; 8];
        pack_bytes_to_nibbles(bytes, &mut nibbles);
        assert_eq!(nibbles, [0, 0, 15, 15, 10, 11, 1, 2]);
    }

    #[test]
    fn test_validate_invalid() {
        assert!(validate_nibbles(&[0, 15, 16]).is_err());
        assert!(validate_nibbles(&[0, 15]).is_ok());
    }
}

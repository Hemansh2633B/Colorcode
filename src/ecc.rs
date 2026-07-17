//! Nibble-level Reed-Solomon style error correction over GF(16).
//!
//! Blocks use 15 symbols, which is the natural maximum codeword length over
//! GF(16). Encoding is systematic: data nibbles are followed by parity
//! nibbles. Decoding solves the syndrome equations over candidate error
//! locations, which handles both known erasures and unknown symbol errors.

use crate::types::{ColorCodeError, EccLevel};

const FIELD_SIZE: usize = 16;
const PRIMITIVE_POLY: u8 = 0b1_0011;
const PRIMITIVE: u8 = 2;

#[inline]
fn gf_add(a: u8, b: u8) -> u8 {
    a ^ b
}

#[inline]
fn gf_sub(a: u8, b: u8) -> u8 {
    a ^ b
}

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u8;
    while b != 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        b >>= 1;
        a <<= 1;
        if a & 0x10 != 0 {
            a ^= PRIMITIVE_POLY;
        }
    }
    result & 0x0f
}

fn gf_pow(base: u8, power: usize) -> u8 {
    let mut value = 1u8;
    for _ in 0..power {
        value = gf_mul(value, base);
    }
    value
}

fn gf_inv(value: u8) -> Option<u8> {
    if value == 0 {
        return None;
    }
    Some(gf_pow(value, FIELD_SIZE - 2))
}

fn parity_check_value(row: usize, col: usize) -> u8 {
    gf_pow(PRIMITIVE, (row + 1) * col)
}

/// Encode data nibbles and append parity nibbles in fixed-size blocks.
pub fn encode(data_nibbles: &[u8], level: EccLevel) -> Result<Vec<u8>, ColorCodeError> {
    if data_nibbles.iter().any(|&n| n > 0x0f) {
        return Err(ColorCodeError::Ecc(
            "input contains values outside the 4-bit range".to_string(),
        ));
    }

    let data_per_block = level.data_shards();
    let block_len = level.block_len();
    let blocks = data_nibbles.len().div_ceil(data_per_block);
    let mut output = Vec::with_capacity(blocks * block_len);

    for block in 0..blocks {
        let start = block * data_per_block;
        let mut codeword = vec![0u8; block_len];
        for i in 0..data_per_block {
            if let Some(&value) = data_nibbles.get(start + i) {
                codeword[i] = value;
            }
        }

        let rhs = data_syndrome_rhs(&codeword[..data_per_block], level);
        let parity = solve_parity(&rhs, level)
            .ok_or_else(|| ColorCodeError::Ecc("failed to solve parity equations".to_string()))?;
        codeword[data_per_block..].copy_from_slice(&parity);
        output.extend_from_slice(&codeword);
    }

    Ok(output)
}

/// Decode a stream of ECC-protected nibbles.
pub fn decode(
    encoded_nibbles: &[u8],
    level: EccLevel,
    original_data_count: usize,
) -> Result<(Vec<u8>, usize), ColorCodeError> {
    decode_with_erasures(encoded_nibbles, level, original_data_count, &[])
}

/// Decode with known erasure positions in the encoded stream.
pub fn decode_with_erasures(
    encoded_nibbles: &[u8],
    level: EccLevel,
    original_data_count: usize,
    erasure_positions: &[usize],
) -> Result<(Vec<u8>, usize), ColorCodeError> {
    let block_len = level.block_len();
    let data_per_block = level.data_shards();
    let parity = level.parity_shards();

    if encoded_nibbles.len() % block_len != 0 {
        return Err(ColorCodeError::Ecc(format!(
            "encoded length {} is not a multiple of block size {}",
            encoded_nibbles.len(),
            block_len
        )));
    }
    if encoded_nibbles.iter().any(|&n| n > 0x0f) {
        return Err(ColorCodeError::Ecc(
            "encoded stream contains values outside the 4-bit range".to_string(),
        ));
    }

    let blocks = encoded_nibbles.len() / block_len;
    let mut decoded = Vec::with_capacity(original_data_count);
    let mut corrections = 0usize;

    for block in 0..blocks {
        let start = block * block_len;
        let mut codeword = encoded_nibbles[start..start + block_len].to_vec();
        let mut local_erasures = erasure_positions
            .iter()
            .copied()
            .filter(|&pos| pos >= start && pos < start + block_len)
            .map(|pos| pos - start)
            .collect::<Vec<_>>();
        local_erasures.sort_unstable();
        local_erasures.dedup();

        if local_erasures.len() > parity {
            return Err(ColorCodeError::Ecc(format!(
                "block {block} has {} erasures, but only {parity} parity symbols",
                local_erasures.len()
            )));
        }

        for &pos in &local_erasures {
            codeword[pos] = 0;
        }

        let syndromes = syndrome(&codeword, level);
        if syndromes.iter().any(|&v| v != 0) {
            let candidate =
                find_error_solution(&syndromes, &local_erasures, level).ok_or_else(|| {
                    ColorCodeError::Ecc(format!("unrecoverable errors in block {block}"))
                })?;

            for (pos, magnitude) in candidate {
                let before = codeword[pos];
                codeword[pos] = gf_sub(codeword[pos], magnitude);
                if before != codeword[pos] {
                    corrections += 1;
                }
            }

            if syndrome(&codeword, level).iter().any(|&v| v != 0) {
                return Err(ColorCodeError::Ecc(format!(
                    "correction did not clear syndrome in block {block}"
                )));
            }
        }

        let data_start = block * data_per_block;
        let remaining = original_data_count.saturating_sub(data_start);
        let take = remaining.min(data_per_block);
        decoded.extend_from_slice(&codeword[..take]);
    }

    decoded.truncate(original_data_count);
    Ok((decoded, corrections))
}

/// Count encoded nibbles including padding and parity.
pub fn encoded_nibble_count(data_count: usize, level: EccLevel) -> usize {
    data_count.div_ceil(level.data_shards()) * level.block_len()
}

fn data_syndrome_rhs(data: &[u8], level: EccLevel) -> Vec<u8> {
    let parity = level.parity_shards();
    let mut rhs = vec![0u8; parity];
    for row in 0..parity {
        let mut sum = 0u8;
        for (col, &value) in data.iter().enumerate() {
            sum = gf_add(sum, gf_mul(parity_check_value(row, col), value));
        }
        rhs[row] = sum;
    }
    rhs
}

fn solve_parity(rhs: &[u8], level: EccLevel) -> Option<Vec<u8>> {
    let data = level.data_shards();
    let parity = level.parity_shards();
    let mut matrix = vec![vec![0u8; parity + 1]; parity];
    for row in 0..parity {
        for col in 0..parity {
            matrix[row][col] = parity_check_value(row, data + col);
        }
        matrix[row][parity] = rhs[row];
    }
    solve_linear_system(matrix, parity)
}

fn syndrome(codeword: &[u8], level: EccLevel) -> Vec<u8> {
    let parity = level.parity_shards();
    let mut values = vec![0u8; parity];
    for (row, value) in values.iter_mut().enumerate() {
        let mut sum = 0u8;
        for (col, &symbol) in codeword.iter().enumerate() {
            sum = gf_add(sum, gf_mul(parity_check_value(row, col), symbol));
        }
        *value = sum;
    }
    values
}

fn find_error_solution(
    syndrome: &[u8],
    erasures: &[usize],
    level: EccLevel,
) -> Option<Vec<(usize, u8)>> {
    let parity = level.parity_shards();
    let block_len = level.block_len();
    let max_unknown = (parity.saturating_sub(erasures.len())) / 2;

    for unknown_count in 0..=max_unknown {
        let mut current = Vec::with_capacity(unknown_count);
        if let Some(solution) = search_positions(
            0,
            unknown_count,
            block_len,
            erasures,
            &mut current,
            syndrome,
            level,
        ) {
            return Some(solution);
        }
    }

    None
}

fn search_positions(
    start: usize,
    remaining: usize,
    block_len: usize,
    erasures: &[usize],
    current: &mut Vec<usize>,
    syndrome: &[u8],
    level: EccLevel,
) -> Option<Vec<(usize, u8)>> {
    if remaining == 0 {
        let mut positions = erasures.to_vec();
        positions.extend_from_slice(current);
        positions.sort_unstable();
        positions.dedup();
        return solve_error_values(&positions, syndrome, level);
    }

    for pos in start..block_len {
        if erasures.binary_search(&pos).is_ok() {
            continue;
        }
        current.push(pos);
        if let Some(solution) = search_positions(
            pos + 1,
            remaining - 1,
            block_len,
            erasures,
            current,
            syndrome,
            level,
        ) {
            return Some(solution);
        }
        current.pop();
    }
    None
}

fn solve_error_values(
    positions: &[usize],
    syndrome: &[u8],
    level: EccLevel,
) -> Option<Vec<(usize, u8)>> {
    if positions.is_empty() {
        return if syndrome.iter().all(|&v| v == 0) {
            Some(Vec::new())
        } else {
            None
        };
    }

    let rows = level.parity_shards();
    let cols = positions.len();
    let mut matrix = vec![vec![0u8; cols + 1]; rows];
    for row in 0..rows {
        for (col, &position) in positions.iter().enumerate() {
            matrix[row][col] = parity_check_value(row, position);
        }
        matrix[row][cols] = syndrome[row];
    }

    let solution = solve_linear_system(matrix, cols)?;
    let values = positions
        .iter()
        .copied()
        .zip(solution)
        .filter(|&(_, value)| value != 0)
        .collect::<Vec<_>>();

    Some(values)
}

fn solve_linear_system(mut matrix: Vec<Vec<u8>>, unknowns: usize) -> Option<Vec<u8>> {
    let rows = matrix.len();
    let mut pivot_row = 0usize;
    let mut pivot_cols = Vec::with_capacity(unknowns);

    for col in 0..unknowns {
        let pivot = (pivot_row..rows).find(|&row| matrix[row][col] != 0);
        let Some(pivot) = pivot else {
            continue;
        };
        matrix.swap(pivot_row, pivot);
        let inv = gf_inv(matrix[pivot_row][col])?;
        for c in col..=unknowns {
            matrix[pivot_row][c] = gf_mul(matrix[pivot_row][c], inv);
        }

        for row in 0..rows {
            if row == pivot_row {
                continue;
            }
            let factor = matrix[row][col];
            if factor == 0 {
                continue;
            }
            for c in col..=unknowns {
                matrix[row][c] = gf_sub(matrix[row][c], gf_mul(factor, matrix[pivot_row][c]));
            }
        }

        pivot_cols.push(col);
        pivot_row += 1;
        if pivot_row == rows {
            break;
        }
    }

    for row in pivot_row..rows {
        let all_zero = (0..unknowns).all(|col| matrix[row][col] == 0);
        if all_zero && matrix[row][unknowns] != 0 {
            return None;
        }
    }

    if pivot_cols.len() < unknowns {
        return None;
    }

    let mut solution = vec![0u8; unknowns];
    for (row, &col) in pivot_cols.iter().enumerate() {
        solution[col] = matrix[row][unknowns];
    }
    Some(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_level_roundtrip() {
        let data = (0..40).map(|v| (v % 16) as u8).collect::<Vec<_>>();
        let encoded = encode(&data, EccLevel::Low).unwrap();
        let (decoded, corrections) = decode(&encoded, EccLevel::Low, data.len()).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(corrections, 0);
    }

    #[test]
    fn corrects_unknown_symbol_errors() {
        let data = (0..26).map(|v| (v % 16) as u8).collect::<Vec<_>>();
        let mut encoded = encode(&data, EccLevel::Medium).unwrap();
        encoded[2] ^= 0x07;
        encoded[10] ^= 0x05;
        let (decoded, corrections) = decode(&encoded, EccLevel::Medium, data.len()).unwrap();
        assert_eq!(decoded, data);
        assert_eq!(corrections, 2);
    }

    #[test]
    fn corrects_erasures() {
        let data = (0..18).map(|v| (v % 16) as u8).collect::<Vec<_>>();
        let mut encoded = encode(&data, EccLevel::High).unwrap();
        encoded[1] = 0;
        encoded[3] = 0;
        encoded[5] = 0;
        encoded[7] = 0;
        let erasures = [1usize, 3, 5, 7];
        let (decoded, corrections) =
            decode_with_erasures(&encoded, EccLevel::High, data.len(), &erasures).unwrap();
        assert_eq!(decoded, data);
        assert!(corrections >= 3);
    }

    #[test]
    fn encoded_count_uses_complete_blocks() {
        assert_eq!(encoded_nibble_count(13, EccLevel::Low), 15);
        assert_eq!(encoded_nibble_count(14, EccLevel::Low), 30);
    }
}

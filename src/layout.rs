//! Matrix placement engine.

use crate::finder;
use crate::metadata::{METADATA_COPIES, METADATA_NIBBLES, METADATA_TOTAL_NIBBLES, Metadata};
use crate::timing;
use crate::types::{ColorCodeError, ColorMatrix, EccLevel, MATRIX_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
}

pub fn apply_function_patterns(matrix: &mut ColorMatrix) {
    finder::apply(matrix);
    timing::apply(matrix);
}

pub fn is_reserved(row: usize, col: usize) -> bool {
    finder::is_reserved(row, col) || timing::is_reserved(row, col)
}

pub fn data_cells() -> Vec<Cell> {
    let mut cells = Vec::with_capacity(MATRIX_SIZE * MATRIX_SIZE);
    for row in 0..MATRIX_SIZE {
        for col in 0..MATRIX_SIZE {
            if !is_reserved(row, col) {
                cells.push(Cell { row, col });
            }
        }
    }
    cells
}

pub fn payload_cells() -> Vec<Cell> {
    let cells = data_cells();
    cells[METADATA_TOTAL_NIBBLES..].to_vec()
}

pub fn payload_capacity_nibbles() -> usize {
    data_cells().len() - METADATA_TOTAL_NIBBLES
}

pub fn payload_data_capacity_nibbles(level: EccLevel) -> usize {
    let full_blocks = payload_capacity_nibbles() / level.block_len();
    full_blocks * level.data_shards()
}

pub fn payload_data_capacity_bytes(level: EccLevel) -> usize {
    payload_data_capacity_nibbles(level) / 2
}

pub fn write(
    matrix: &mut ColorMatrix,
    metadata: Metadata,
    encoded_payload: &[u8],
) -> Result<(), ColorCodeError> {
    let cells = data_cells();
    let capacity = cells.len() - METADATA_TOTAL_NIBBLES;
    if encoded_payload.len() > capacity {
        return Err(ColorCodeError::PayloadTooLarge {
            needed: encoded_payload.len(),
            capacity,
        });
    }

    let metadata_nibbles = metadata.to_nibbles();
    for copy in 0..METADATA_COPIES {
        let offset = copy * METADATA_NIBBLES;
        for (idx, &value) in metadata_nibbles.iter().enumerate() {
            let cell = cells[offset + idx];
            matrix.set(cell.row, cell.col, value);
        }
    }

    for (idx, &value) in encoded_payload.iter().enumerate() {
        let cell = cells[METADATA_TOTAL_NIBBLES + idx];
        matrix.set(cell.row, cell.col, value);
    }

    for idx in METADATA_TOTAL_NIBBLES + encoded_payload.len()..cells.len() {
        let cell = cells[idx];
        let value = ((cell.row * 3 + cell.col * 5 + 7) & 0x0f) as u8;
        matrix.set(cell.row, cell.col, value);
    }

    Ok(())
}

pub fn read_metadata_candidates(matrix: &ColorMatrix) -> Vec<Metadata> {
    let cells = data_cells();
    let mut candidates = Vec::with_capacity(METADATA_COPIES);
    for copy in 0..METADATA_COPIES {
        let offset = copy * METADATA_NIBBLES;
        let mut nibbles = [0u8; METADATA_NIBBLES];
        for idx in 0..METADATA_NIBBLES {
            let cell = cells[offset + idx];
            nibbles[idx] = matrix.get(cell.row, cell.col);
        }
        if let Ok(metadata) = Metadata::from_nibbles(&nibbles) {
            if !candidates.contains(&metadata) {
                candidates.push(metadata);
            }
        }
    }
    candidates
}

pub fn read_payload(matrix: &ColorMatrix, count: usize) -> Result<Vec<u8>, ColorCodeError> {
    let cells = payload_cells();
    if count > cells.len() {
        return Err(ColorCodeError::PayloadTooLarge {
            needed: count,
            capacity: cells.len(),
        });
    }
    let mut nibbles = Vec::with_capacity(count);
    for cell in cells.iter().take(count) {
        nibbles.push(matrix.get(cell.row, cell.col));
    }
    Ok(nibbles)
}

pub fn read_payload_with_erasures(
    matrix: &ColorMatrix,
    erasures: &[bool],
    count: usize,
) -> Result<(Vec<u8>, Vec<usize>), ColorCodeError> {
    let cells = payload_cells();
    if count > cells.len() {
        return Err(ColorCodeError::PayloadTooLarge {
            needed: count,
            capacity: cells.len(),
        });
    }
    let mut nibbles = Vec::with_capacity(count);
    let mut erasure_positions = Vec::new();
    for (idx, cell) in cells.iter().take(count).enumerate() {
        nibbles.push(matrix.get(cell.row, cell.col));
        let matrix_idx = cell.row * MATRIX_SIZE + cell.col;
        if erasures.get(matrix_idx).copied().unwrap_or(false) {
            erasure_positions.push(idx);
        }
    }
    Ok((nibbles, erasure_positions))
}

pub fn function_mismatch_score(matrix: &ColorMatrix) -> usize {
    finder::validate(matrix) + timing::validate(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_is_positive() {
        assert!(payload_capacity_nibbles() > 1_000_000);
        assert!(payload_data_capacity_bytes(EccLevel::Low) > 500_000);
    }
}

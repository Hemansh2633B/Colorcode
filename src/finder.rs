//! Custom chromatic finder patterns.

use crate::types::{ColorMatrix, MATRIX_SIZE};

pub const FINDER_SIZE: usize = 5;

pub const FINDER_PATTERN: [[u8; FINDER_SIZE]; FINDER_SIZE] = [
    [0, 2, 5, 8, 0],
    [3, 1, 1, 1, 4],
    [6, 1, 15, 1, 9],
    [10, 1, 1, 1, 11],
    [0, 12, 13, 14, 0],
];

pub fn apply(matrix: &mut ColorMatrix) {
    for (row, col) in finder_origins() {
        for r in 0..FINDER_SIZE {
            for c in 0..FINDER_SIZE {
                matrix.set(row + r, col + c, FINDER_PATTERN[r][c]);
            }
        }
    }
}

pub fn validate(matrix: &ColorMatrix) -> usize {
    let mut mismatches = 0usize;
    for (row, col) in finder_origins() {
        for r in 0..FINDER_SIZE {
            for c in 0..FINDER_SIZE {
                if matrix.get(row + r, col + c) != FINDER_PATTERN[r][c] {
                    mismatches += 1;
                }
            }
        }
    }
    mismatches
}

pub fn is_reserved(row: usize, col: usize) -> bool {
    finder_origins().iter().any(|&(origin_row, origin_col)| {
        row >= origin_row
            && row < origin_row + FINDER_SIZE
            && col >= origin_col
            && col < origin_col + FINDER_SIZE
    })
}

pub fn finder_origins() -> [(usize, usize); 4] {
    let last = MATRIX_SIZE - FINDER_SIZE;
    [(0, 0), (0, last), (last, 0), (last, last)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_is_not_rotationally_symmetric() {
        let mut rotated = [[0u8; FINDER_SIZE]; FINDER_SIZE];
        for r in 0..FINDER_SIZE {
            for c in 0..FINDER_SIZE {
                rotated[c][FINDER_SIZE - 1 - r] = FINDER_PATTERN[r][c];
            }
        }
        assert_ne!(rotated, FINDER_PATTERN);
    }
}

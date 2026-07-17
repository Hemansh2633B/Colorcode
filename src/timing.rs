//! Custom chromatic timing pattern.

use crate::finder::FINDER_SIZE;
use crate::types::{ColorMatrix, MATRIX_SIZE};

pub const TIMING_ROW: usize = FINDER_SIZE;
pub const TIMING_COL: usize = FINDER_SIZE;
pub const TIMING_START: usize = FINDER_SIZE;
pub const TIMING_END: usize = MATRIX_SIZE - FINDER_SIZE;

pub fn apply(matrix: &mut ColorMatrix) {
    for col in TIMING_START..TIMING_END {
        matrix.set(TIMING_ROW, col, horizontal_value(col));
    }
    for row in TIMING_START..TIMING_END {
        matrix.set(row, TIMING_COL, vertical_value(row));
    }
}

pub fn validate(matrix: &ColorMatrix) -> usize {
    let mut mismatches = 0usize;
    for col in TIMING_START..TIMING_END {
        if matrix.get(TIMING_ROW, col) != horizontal_value(col) {
            mismatches += 1;
        }
    }
    for row in TIMING_START..TIMING_END {
        if matrix.get(row, TIMING_COL) != vertical_value(row) {
            mismatches += 1;
        }
    }
    mismatches
}

pub fn is_reserved(row: usize, col: usize) -> bool {
    (row == TIMING_ROW && (TIMING_START..TIMING_END).contains(&col))
        || (col == TIMING_COL && (TIMING_START..TIMING_END).contains(&row))
}

#[inline]
pub fn horizontal_value(col: usize) -> u8 {
    ((col * 5 + 3) & 0x0f) as u8
}

#[inline]
pub fn vertical_value(row: usize) -> u8 {
    ((row * 7 + 11) & 0x0f) as u8
}

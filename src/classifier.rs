//! Delta-E based color classifier.

use crate::palette::{Lab, WhiteBalance, delta_e, palette_lab, rgb_to_lab};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Classification {
    pub value: u8,
    pub distance: f32,
    pub accepted: bool,
}

#[derive(Debug, Clone)]
pub struct ColorClassifier {
    tolerance: f32,
    white_balance: WhiteBalance,
    palette: [Lab; 16],
}

impl ColorClassifier {
    pub fn new(tolerance: f32, white_balance: WhiteBalance) -> Self {
        Self {
            tolerance,
            white_balance,
            palette: palette_lab(),
        }
    }

    pub fn classify(&self, rgb: [u8; 3]) -> Classification {
        let balanced = self.white_balance.apply(rgb);
        let lab = rgb_to_lab(balanced);
        let mut best_value = 0u8;
        let mut best_distance = f32::INFINITY;

        for (value, &target) in self.palette.iter().enumerate() {
            let distance = delta_e(lab, target);
            if distance < best_distance {
                best_distance = distance;
                best_value = value as u8;
            }
        }

        Classification {
            value: best_value,
            distance: best_distance,
            accepted: best_distance <= self.tolerance,
        }
    }
}

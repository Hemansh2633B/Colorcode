//! Image loading, coarse grid detection, and module sampling.

use image::{DynamicImage, Pixel, RgbImage};

use crate::classifier::ColorClassifier;
use crate::geometry::{Point, Quad};
use crate::homography::Homography;
use crate::palette::estimate_white_balance;
use crate::types::{ColorCodeError, ColorMatrix, DEFAULT_QUIET_ZONE, MATRIX_SIZE, TOTAL_MODULES};

#[derive(Debug, Clone)]
pub struct SampledMatrix {
    pub matrix: ColorMatrix,
    pub erasures: Vec<bool>,
    pub distances: Vec<f32>,
}

impl SampledMatrix {
    pub fn rotated(&self, turns_clockwise: u8) -> Self {
        let mut sample = self.clone();
        for _ in 0..turns_clockwise % 4 {
            sample = sample.rotated_clockwise();
        }
        sample
    }

    fn rotated_clockwise(&self) -> Self {
        let mut matrix = ColorMatrix::new();
        let mut erasures = vec![false; TOTAL_MODULES];
        let mut distances = vec![0.0; TOTAL_MODULES];
        for row in 0..MATRIX_SIZE {
            for col in 0..MATRIX_SIZE {
                let src_idx = row * MATRIX_SIZE + col;
                let dst_row = col;
                let dst_col = MATRIX_SIZE - 1 - row;
                let dst_idx = dst_row * MATRIX_SIZE + dst_col;
                matrix.set(dst_row, dst_col, self.matrix.get(row, col));
                erasures[dst_idx] = self.erasures[src_idx];
                distances[dst_idx] = self.distances[src_idx];
            }
        }
        Self {
            matrix,
            erasures,
            distances,
        }
    }
}

pub fn sample_image(image: &DynamicImage, tolerance: f32) -> Result<SampledMatrix, ColorCodeError> {
    let rgb = image.to_rgb8();
    let white_balance = estimate_white_balance(rgb.pixels().map(|pixel| pixel.0));
    let classifier = ColorClassifier::new(tolerance, white_balance);
    if let Some(sample) = sample_exact_grid(&rgb, &classifier) {
        return Ok(sample);
    }

    let quad = detect_non_white_quad(&rgb).ok_or_else(|| {
        ColorCodeError::ImageDecode("could not locate a ColorCode matrix".to_string())
    })?;
    sample_quad(&rgb, quad, &classifier)
}

fn sample_exact_grid(image: &RgbImage, classifier: &ColorClassifier) -> Option<SampledMatrix> {
    let (width, height) = image.dimensions();
    if width != height {
        return None;
    }

    let quiet_modules = MATRIX_SIZE as u32 + DEFAULT_QUIET_ZONE * 2;
    let (module_size, quiet_zone) = if width % quiet_modules == 0 {
        (width / quiet_modules, DEFAULT_QUIET_ZONE)
    } else if width % MATRIX_SIZE as u32 == 0 {
        (width / MATRIX_SIZE as u32, 0)
    } else {
        return None;
    };

    if module_size == 0 {
        return None;
    }

    let mut matrix = ColorMatrix::new();
    let mut erasures = vec![false; TOTAL_MODULES];
    let mut distances = vec![0.0; TOTAL_MODULES];
    let origin = quiet_zone * module_size;
    let radius = (module_size / 4).max(1);

    for row in 0..MATRIX_SIZE {
        for col in 0..MATRIX_SIZE {
            let center_x = origin + col as u32 * module_size + module_size / 2;
            let center_y = origin + row as u32 * module_size + module_size / 2;
            let rgb = average_patch(image, center_x, center_y, radius);
            let classification = classifier.classify(rgb);
            let idx = row * MATRIX_SIZE + col;
            matrix.set(row, col, classification.value);
            erasures[idx] = !classification.accepted;
            distances[idx] = classification.distance;
        }
    }

    Some(SampledMatrix {
        matrix,
        erasures,
        distances,
    })
}

fn detect_non_white_quad(image: &RgbImage) -> Option<Quad> {
    let (width, height) = image.dimensions();
    let mut top_left = (f32::INFINITY, Point::new(0.0, 0.0));
    let mut top_right = (f32::NEG_INFINITY, Point::new(0.0, 0.0));
    let mut bottom_right = (f32::NEG_INFINITY, Point::new(0.0, 0.0));
    let mut bottom_left = (f32::INFINITY, Point::new(0.0, 0.0));
    let mut count = 0usize;

    for y in 0..height {
        for x in 0..width {
            let rgb = image.get_pixel(x, y).to_rgb().0;
            if is_background(rgb) {
                continue;
            }
            count += 1;
            let point = Point::new(x as f32, y as f32);
            let sum = point.x + point.y;
            let diff = point.x - point.y;
            if sum < top_left.0 {
                top_left = (sum, point);
            }
            if sum > bottom_right.0 {
                bottom_right = (sum, point);
            }
            if diff > top_right.0 {
                top_right = (diff, point);
            }
            if diff < bottom_left.0 {
                bottom_left = (diff, point);
            }
        }
    }

    if count < 64 {
        return None;
    }

    Some(Quad {
        top_left: top_left.1,
        top_right: top_right.1,
        bottom_right: bottom_right.1,
        bottom_left: bottom_left.1,
    })
}

fn sample_quad(
    image: &RgbImage,
    quad: Quad,
    classifier: &ColorClassifier,
) -> Result<SampledMatrix, ColorCodeError> {
    let homography = Homography::from_quad(quad);
    let mut matrix = ColorMatrix::new();
    let mut erasures = vec![false; TOTAL_MODULES];
    let mut distances = vec![0.0; TOTAL_MODULES];

    for row in 0..MATRIX_SIZE {
        for col in 0..MATRIX_SIZE {
            let u = (col as f32 + 0.5) / MATRIX_SIZE as f32;
            let v = (row as f32 + 0.5) / MATRIX_SIZE as f32;
            let point = homography.map_unit(u, v);
            let rgb = sample_nearest(image, point);
            let classification = classifier.classify(rgb);
            let idx = row * MATRIX_SIZE + col;
            matrix.set(row, col, classification.value);
            erasures[idx] = !classification.accepted;
            distances[idx] = classification.distance;
        }
    }

    Ok(SampledMatrix {
        matrix,
        erasures,
        distances,
    })
}

fn average_patch(image: &RgbImage, center_x: u32, center_y: u32, radius: u32) -> [u8; 3] {
    let (width, height) = image.dimensions();
    let min_x = center_x.saturating_sub(radius);
    let min_y = center_y.saturating_sub(radius);
    let max_x = (center_x + radius).min(width.saturating_sub(1));
    let max_y = (center_y + radius).min(height.saturating_sub(1));
    let mut sum = [0u32; 3];
    let mut count = 0u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let rgb = image.get_pixel(x, y).0;
            sum[0] += rgb[0] as u32;
            sum[1] += rgb[1] as u32;
            sum[2] += rgb[2] as u32;
            count += 1;
        }
    }

    [
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
    ]
}

fn sample_nearest(image: &RgbImage, point: Point) -> [u8; 3] {
    let (width, height) = image.dimensions();
    let x = point.x.round().clamp(0.0, width.saturating_sub(1) as f32) as u32;
    let y = point.y.round().clamp(0.0, height.saturating_sub(1) as f32) as u32;
    image.get_pixel(x, y).0
}

fn is_background(rgb: [u8; 3]) -> bool {
    let max = rgb[0].max(rgb[1]).max(rgb[2]);
    let min = rgb[0].min(rgb[1]).min(rgb[2]);
    max > 235 && min > 220 && max.saturating_sub(min) < 24
}

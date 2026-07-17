//! Image loading, coarse grid detection, and module sampling.

use image::{DynamicImage, Pixel, RgbImage};

use crate::classifier::ColorClassifier;
use crate::geometry::{Point, Quad};
use crate::homography::Homography;
use crate::palette::estimate_white_balance;
use crate::types::{ColorCodeError, ColorMatrix, DEFAULT_QUIET_ZONE, Version, ALL_VERSIONS};

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
        let s = self.matrix.size;
        let mut matrix = ColorMatrix::new();
        matrix.resize_to(s);
        let mut erasures = vec![false; s * s];
        let mut distances = vec![0.0; s * s];
        for row in 0..s {
            for col in 0..s {
                let src_idx = row * s + col;
                let dst_row = col;
                let dst_col = s - 1 - row;
                let dst_idx = dst_row * s + dst_col;
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

    // For each known version, try both with and without the default quiet zone,
    // and pick the one that divides the image exactly (lowest version first).
    let mut chosen: Option<(u32, usize, u32)> = None;
    for &version in ALL_VERSIONS {
        let matrix_size = version.size() as u32;
        for &quiet in &[DEFAULT_QUIET_ZONE, 0u32] {
            let total_modules = matrix_size + quiet * 2;
            if total_modules == 0 || width % total_modules != 0 {
                continue;
            }
            let module_size = width / total_modules;
            if module_size == 0 {
                continue;
            }
            // Prefer the smaller version when both fit.
            if chosen.is_none() || chosen.unwrap().1 > version.size() {
                chosen = Some((module_size, version.size(), quiet));
            }
        }
    }
    let (module_size, matrix_size, quiet_zone) = chosen?;

    let total_modules = matrix_size * matrix_size;
    let mut matrix = ColorMatrix::new();
            matrix.resize_to(matrix_size);
    let mut erasures = vec![false; total_modules];
    let mut distances = vec![0.0; total_modules];
    let origin = quiet_zone * module_size;
    // Sample radius is capped to a single pixel when the module is small so
    // neighboring modules are never mixed into the average.
    let radius = (module_size / 4).min(module_size.saturating_sub(1) / 2).max(0);

    for row in 0..matrix_size {
        for col in 0..matrix_size {
            let center_x = origin + col as u32 * module_size + module_size / 2;
            let center_y = origin + row as u32 * module_size + module_size / 2;
            let rgb = average_patch(image, center_x, center_y, radius);
            let classification = classifier.classify(rgb);
            let idx = row * matrix_size + col;
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

/// Sample a matrix from a detected quadrilateral region. Currently only
/// supports the current version since the homography sampler is calibrated for the
/// small grid used in the V2 PNGs. Larger-version images rely on
/// `sample_exact_grid`.
fn sample_quad(
    image: &RgbImage,
    quad: Quad,
    classifier: &ColorClassifier,
) -> Result<SampledMatrix, ColorCodeError> {
    let homography = Homography::from_quad(quad);
    let s = Version::current().size();
    let total = s * s;
    let mut matrix = ColorMatrix::new();
    matrix.resize_to(s);
    let mut erasures = vec![false; total];
    let mut distances = vec![0.0; total];

    for row in 0..s {
        for col in 0..s {
            let u = (col as f32 + 0.5) / s as f32;
            let v = (row as f32 + 0.5) / s as f32;
            let point = homography.map_unit(u, v);
            let rgb = sample_nearest(image, point);
            let classification = classifier.classify(rgb);
            let idx = row * s + col;
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

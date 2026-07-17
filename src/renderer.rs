//! Lossless RGB PNG rendering.

use std::path::Path;

use image::{Rgb, RgbImage};

use crate::palette::rgb_for_nibble;
use crate::types::{ColorCodeError, ColorMatrix, DEFAULT_QUIET_ZONE};
use crate::vision;

pub fn render_png(matrix: &ColorMatrix, module_size: u32, quiet_zone: u32) -> RgbImage {
    let modules = matrix.size as u32 + quiet_zone * 2;
    let size = modules * module_size;
    let mut image = RgbImage::from_pixel(size, size, Rgb([255, 255, 255]));

    for row in 0..matrix.size {
        for col in 0..matrix.size {
            let rgb = Rgb(rgb_for_nibble(matrix.get(row, col)));
            let start_x = (col as u32 + quiet_zone) * module_size;
            let start_y = (row as u32 + quiet_zone) * module_size;
            for y in start_y..start_y + module_size {
                for x in start_x..start_x + module_size {
                    image.put_pixel(x, y, rgb);
                }
            }
        }
    }

    image
}

pub fn save_png<P: AsRef<Path>>(
    matrix: &ColorMatrix,
    path: P,
    module_size: u32,
    quiet_zone: u32,
) -> Result<(), ColorCodeError> {
    render_png(matrix, module_size, quiet_zone).save(path)?;
    Ok(())
}

impl ColorMatrix {
    pub fn render_png(&self, module_size: u32) -> RgbImage {
        render_png(self, module_size, DEFAULT_QUIET_ZONE)
    }

    pub fn render_png_with_quiet_zone(&self, module_size: u32, quiet_zone: u32) -> RgbImage {
        render_png(self, module_size, quiet_zone)
    }

    pub fn save_png<P: AsRef<Path>>(
        &self,
        path: P,
        module_size: u32,
    ) -> Result<(), ColorCodeError> {
        save_png(self, path, module_size, DEFAULT_QUIET_ZONE)
    }

    pub fn save_png_with_quiet_zone<P: AsRef<Path>>(
        &self,
        path: P,
        module_size: u32,
        quiet_zone: u32,
    ) -> Result<(), ColorCodeError> {
        save_png(self, path, module_size, quiet_zone)
    }

    pub fn load_png<P: AsRef<Path>>(path: P) -> Result<Self, ColorCodeError> {
        let image = image::open(path)?;
        Ok(vision::sample_image(&image, 38.0)?.matrix)
    }
}

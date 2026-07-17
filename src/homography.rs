//! Lightweight quadrilateral sampling helpers.
//!
//! The decoder uses a bilinear quad mapper. The module boundary is
//! kept explicit so a full projective homography can be swapped in later
//! without changing decoder call sites.

use crate::geometry::{Point, Quad};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography {
    quad: Quad,
}

impl Homography {
    pub fn from_quad(quad: Quad) -> Self {
        Self { quad }
    }

    pub fn map_unit(self, u: f32, v: f32) -> Point {
        self.quad.interpolate(u, v)
    }
}

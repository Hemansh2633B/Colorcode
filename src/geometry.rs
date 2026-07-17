//! Small geometry primitives used by the image sampler.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quad {
    pub top_left: Point,
    pub top_right: Point,
    pub bottom_right: Point,
    pub bottom_left: Point,
}

impl Quad {
    pub fn square(size: f32) -> Self {
        Self {
            top_left: Point::new(0.0, 0.0),
            top_right: Point::new(size, 0.0),
            bottom_right: Point::new(size, size),
            bottom_left: Point::new(0.0, size),
        }
    }

    pub fn interpolate(self, u: f32, v: f32) -> Point {
        let top = lerp(self.top_left, self.top_right, u);
        let bottom = lerp(self.bottom_left, self.bottom_right, u);
        lerp(top, bottom, v)
    }
}

#[inline]
pub fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}

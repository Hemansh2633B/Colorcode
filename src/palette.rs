//! Fixed 16-color palette and perceptual color distance helpers.

/// The required ColorCode palette in nibble order.
pub const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0xff, 0xff, 0xff],
    [0xff, 0x00, 0x00],
    [0x00, 0xff, 0x00],
    [0x00, 0x00, 0xff],
    [0xff, 0xff, 0x00],
    [0x00, 0xff, 0xff],
    [0xff, 0x00, 0xff],
    [0xff, 0x80, 0x00],
    [0x80, 0x00, 0xff],
    [0x80, 0xff, 0x00],
    [0xff, 0x80, 0xc0],
    [0x8b, 0x45, 0x13],
    [0x80, 0x80, 0x80],
    [0x00, 0x00, 0x80],
    [0x00, 0x64, 0x00],
];

pub const HEX_PALETTE: [&str; 16] = [
    "#000000", "#FFFFFF", "#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF",
    "#FF8000", "#8000FF", "#80FF00", "#FF80C0", "#8B4513", "#808080", "#000080", "#006400",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
        }
    }
}

impl WhiteBalance {
    pub fn apply(self, rgb: [u8; 3]) -> [u8; 3] {
        [
            clamp_channel(rgb[0] as f32 * self.red),
            clamp_channel(rgb[1] as f32 * self.green),
            clamp_channel(rgb[2] as f32 * self.blue),
        ]
    }
}

#[inline]
pub fn rgb_for_nibble(value: u8) -> [u8; 3] {
    PALETTE[(value & 0x0f) as usize]
}

#[inline]
pub fn hex_for_nibble(value: u8) -> &'static str {
    HEX_PALETTE[(value & 0x0f) as usize]
}

pub fn palette_lab() -> [Lab; 16] {
    PALETTE.map(rgb_to_lab)
}

pub fn rgb_to_lab(rgb: [u8; 3]) -> Lab {
    let r = srgb_to_linear(rgb[0] as f32 / 255.0);
    let g = srgb_to_linear(rgb[1] as f32 / 255.0);
    let b = srgb_to_linear(rgb[2] as f32 / 255.0);

    let x = r * 0.412_456_4 + g * 0.357_576_1 + b * 0.180_437_5;
    let y = r * 0.212_672_9 + g * 0.715_152_2 + b * 0.072_175;
    let z = r * 0.019_333_9 + g * 0.119_192 + b * 0.950_304_1;

    let fx = lab_f(x / 0.950_47);
    let fy = lab_f(y);
    let fz = lab_f(z / 1.088_83);

    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

#[inline]
pub fn delta_e(a: Lab, b: Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

pub fn estimate_white_balance<I>(pixels: I) -> WhiteBalance
where
    I: IntoIterator<Item = [u8; 3]>,
{
    let mut candidates = pixels.into_iter().collect::<Vec<_>>();
    if candidates.is_empty() {
        return WhiteBalance::default();
    }

    candidates.sort_unstable_by_key(|rgb| {
        let luma = 2126u32 * rgb[0] as u32 + 7152u32 * rgb[1] as u32 + 722u32 * rgb[2] as u32;
        std::cmp::Reverse(luma)
    });

    let take = (candidates.len() / 20).clamp(16, candidates.len());
    let mut sum = [0f32; 3];
    let mut count = 0f32;
    for rgb in candidates.into_iter().take(take) {
        let max = rgb[0].max(rgb[1]).max(rgb[2]);
        let min = rgb[0].min(rgb[1]).min(rgb[2]);
        if max < 96 || max.saturating_sub(min) > 96 {
            continue;
        }
        sum[0] += rgb[0] as f32;
        sum[1] += rgb[1] as f32;
        sum[2] += rgb[2] as f32;
        count += 1.0;
    }

    if count < 1.0 {
        return WhiteBalance::default();
    }

    let avg = [sum[0] / count, sum[1] / count, sum[2] / count];
    let gray = (avg[0] + avg[1] + avg[2]) / 3.0;
    WhiteBalance {
        red: safe_gain(gray, avg[0]),
        green: safe_gain(gray, avg[1]),
        blue: safe_gain(gray, avg[2]),
    }
}

#[inline]
fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[inline]
fn lab_f(value: f32) -> f32 {
    if value > 0.008_856 {
        value.cbrt()
    } else {
        7.787 * value + 16.0 / 116.0
    }
}

#[inline]
fn clamp_channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[inline]
fn safe_gain(target: f32, source: f32) -> f32 {
    if source < 1.0 {
        1.0
    } else {
        (target / source).clamp(0.5, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_palette_values_match() {
        assert_eq!(PALETTE[0], [0, 0, 0]);
        assert_eq!(PALETTE[11], [255, 128, 192]);
        assert_eq!(PALETTE[15], [0, 100, 0]);
        assert_eq!(HEX_PALETTE[12], "#8B4513");
    }
}

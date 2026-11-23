//! A simple RGBA8 framebuffer. Transport-agnostic: the Thermalright adapter is
//! responsible for converting a `Frame` into the device's wire format after the
//! real protocol is validated.

/// An RGBA color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const BLACK: Rgba = Rgba(0, 0, 0, 255);
    pub const WHITE: Rgba = Rgba(255, 255, 255, 255);
    pub const GRAY: Rgba = Rgba(90, 90, 96, 255);

    /// Linear interpolation between two colors, `t` clamped to `[0,1]`.
    pub fn lerp(a: Rgba, b: Rgba, t: f64) -> Rgba {
        let t = t.clamp(0.0, 1.0);
        let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
        Rgba(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2), m(a.3, b.3))
    }
}

/// A heap-allocated RGBA framebuffer, row-major, 4 bytes per pixel.
#[derive(Debug, Clone)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Frame {
    /// Create a frame filled with `fill`. Returns `None` for zero dimensions or a
    /// size that would overflow (malformed frame dimensions are rejected —
    /// TEST-PLAN "Safety regression").
    pub fn new(width: u32, height: u32, fill: Rgba) -> Option<Frame> {
        if width == 0 || height == 0 {
            return None;
        }
        let count = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        let mut pixels = vec![0u8; count];
        for px in pixels.chunks_exact_mut(4) {
            px[0] = fill.0;
            px[1] = fill.1;
            px[2] = fill.2;
            px[3] = fill.3;
        }
        Some(Frame { width, height, pixels })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA bytes, `width * height * 4` long.
    pub fn bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Set one pixel. Out-of-bounds coordinates are ignored (never panic).
    pub fn put(&mut self, x: u32, y: u32, c: Rgba) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        self.pixels[i] = c.0;
        self.pixels[i + 1] = c.1;
        self.pixels[i + 2] = c.2;
        self.pixels[i + 3] = c.3;
    }

    pub fn get(&self, x: u32, y: u32) -> Option<Rgba> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y as usize * self.width as usize) + x as usize) * 4;
        Some(Rgba(self.pixels[i], self.pixels[i + 1], self.pixels[i + 2], self.pixels[i + 3]))
    }

    /// Fill an axis-aligned rectangle, clamped to the frame.
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, c: Rgba) {
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);
        for yy in y..y1 {
            for xx in x..x1 {
                self.put(xx, yy, c);
            }
        }
    }

    /// Draw a 1px rectangle border, clamped to the frame.
    pub fn stroke_rect(&mut self, x: u32, y: u32, w: u32, h: u32, c: Rgba) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(x, y, w, 1, c);
        self.fill_rect(x, y + h - 1, w, 1, c);
        self.fill_rect(x, y, 1, h, c);
        self.fill_rect(x + w - 1, y, 1, h, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dimensions() {
        assert!(Frame::new(0, 10, Rgba::BLACK).is_none());
        assert!(Frame::new(10, 0, Rgba::BLACK).is_none());
    }

    #[test]
    fn correct_buffer_size_and_fill() {
        let f = Frame::new(4, 3, Rgba::WHITE).unwrap();
        assert_eq!(f.bytes().len(), 4 * 3 * 4);
        assert_eq!(f.get(0, 0), Some(Rgba::WHITE));
        assert_eq!(f.get(3, 2), Some(Rgba::WHITE));
        assert_eq!(f.get(4, 0), None);
    }

    #[test]
    fn put_ignores_out_of_bounds() {
        let mut f = Frame::new(2, 2, Rgba::BLACK).unwrap();
        f.put(99, 99, Rgba::WHITE); // must not panic
        assert_eq!(f.get(0, 0), Some(Rgba::BLACK));
    }

    #[test]
    fn fill_rect_clamps() {
        let mut f = Frame::new(4, 4, Rgba::BLACK).unwrap();
        f.fill_rect(2, 2, 100, 100, Rgba::WHITE);
        assert_eq!(f.get(3, 3), Some(Rgba::WHITE));
        assert_eq!(f.get(1, 1), Some(Rgba::BLACK));
    }

    #[test]
    fn lerp_endpoints() {
        assert_eq!(Rgba::lerp(Rgba::BLACK, Rgba::WHITE, 0.0), Rgba::BLACK);
        assert_eq!(Rgba::lerp(Rgba::BLACK, Rgba::WHITE, 1.0), Rgba::WHITE);
    }
}

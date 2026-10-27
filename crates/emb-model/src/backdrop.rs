//! A bitmap under the design, for tracing.

use std::sync::Arc;

use emb_geom::{BBox, Point};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A bitmap image under the design. It is not stitched.
///
/// The image file bytes (PNG, JPEG or BMP) are kept as they are and saved in
/// the project file, so the project needs no other file. The bytes are
/// shared with `Arc`, so undo snapshots do not copy them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backdrop {
    /// File name of the image, for the user.
    pub name: String,
    /// The image file bytes, saved as base64.
    #[serde(serialize_with = "to_base64", deserialize_with = "from_base64")]
    pub data: Arc<Vec<u8>>,
    /// Image size in pixels.
    pub pixels: (u32, u32),
    /// Top-left corner in design coordinates (mm).
    pub origin: Point,
    /// Width in mm. The height follows from the pixel size.
    pub width_mm: f64,
    /// 0 is invisible, 1 is full strength.
    #[serde(default = "full")]
    pub opacity: f64,
    #[serde(default = "yes")]
    pub visible: bool,
    /// A locked backdrop cannot be moved on the canvas.
    #[serde(default = "yes")]
    pub locked: bool,
}

fn full() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

/// Default opacity of a new backdrop, so stitches and outlines stand out on it.
pub const DEFAULT_OPACITY: f64 = 0.5;
/// Smallest width that the app can set, in mm.
pub const MIN_WIDTH_MM: f64 = 1.0;

impl Backdrop {
    /// A new backdrop of `width_mm`, centered on `center`.
    pub fn new(name: impl Into<String>, data: Vec<u8>, pixels: (u32, u32), center: Point, width_mm: f64) -> Self {
        let mut b = Backdrop {
            name: name.into(),
            data: Arc::new(data),
            pixels,
            origin: Point::ZERO,
            width_mm: width_mm.max(MIN_WIDTH_MM),
            opacity: DEFAULT_OPACITY,
            visible: true,
            locked: true,
        };
        b.set_center(center);
        b
    }

    pub fn height_mm(&self) -> f64 {
        if self.pixels.0 == 0 {
            return self.width_mm;
        }
        self.width_mm * f64::from(self.pixels.1) / f64::from(self.pixels.0)
    }

    pub fn bbox(&self) -> BBox {
        BBox::new(self.origin, self.origin + Point::new(self.width_mm, self.height_mm()))
    }

    pub fn center(&self) -> Point {
        self.bbox().center()
    }

    pub fn set_center(&mut self, c: Point) {
        self.origin = c - Point::new(self.width_mm / 2.0, self.height_mm() / 2.0);
    }

    /// Changes the width and keeps the center.
    pub fn set_width(&mut self, w: f64) {
        let c = self.center();
        self.width_mm = w.max(MIN_WIDTH_MM);
        self.set_center(c);
    }
}

impl PartialEq for Backdrop {
    fn eq(&self, o: &Self) -> bool {
        self.name == o.name
            && (Arc::ptr_eq(&self.data, &o.data) || self.data == o.data)
            && self.pixels == o.pixels
            && self.origin == o.origin
            && self.width_mm == o.width_mm
            && self.opacity == o.opacity
            && self.visible == o.visible
            && self.locked == o.locked
    }
}

pub(crate) fn to_base64<S: Serializer>(data: &Arc<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
    use base64::Engine;
    s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(data.as_slice()))
}

pub(crate) fn from_base64<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<Vec<u8>>, D::Error> {
    use base64::Engine;
    let s = String::deserialize(d)?;
    base64::engine::general_purpose::STANDARD
        .decode(s.as_bytes())
        .map(Arc::new)
        .map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_follows_pixels_and_center_stays() {
        let mut b = Backdrop::new("logo.png", vec![1, 2, 3], (200, 100), Point::new(10.0, 20.0), 80.0);
        assert_eq!(b.height_mm(), 40.0);
        assert_eq!(b.origin, Point::new(-30.0, 0.0));
        b.set_width(40.0);
        assert_eq!(b.center(), Point::new(10.0, 20.0));
        assert_eq!(b.bbox().height(), 20.0);
    }

    #[test]
    fn serde_round_trip_with_base64() {
        let b = Backdrop::new("a.png", (0..=255).collect(), (16, 16), Point::ZERO, 50.0);
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("\"data\":\"AAECAwQF"));
        let back: Backdrop = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
        assert!(!Arc::ptr_eq(&back.data, &b.data));
    }
}

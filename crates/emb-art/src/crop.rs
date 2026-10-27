//! Cut a picture to a rectangle, an ellipse or a polygon. Pixels outside
//! the shape become transparent, and the picture becomes as small as the
//! box of the shape.

use emb_geom::region::ring_contains;
use emb_geom::{BBox, Point};
use image::{Rgba, RgbaImage};

/// A crop shape in pixel coordinates (x right, y down).
#[derive(Debug, Clone, PartialEq)]
pub enum CropShape {
    Rect(BBox),
    Ellipse(BBox),
    Polygon(Vec<Point>),
}

impl CropShape {
    fn bbox(&self) -> Option<BBox> {
        match self {
            CropShape::Rect(b) | CropShape::Ellipse(b) => Some(*b),
            CropShape::Polygon(p) => BBox::from_points(p.iter().copied()),
        }
    }

    /// Returns `true` if the middle of pixel (x, y) is inside the shape.
    fn contains(&self, x: u32, y: u32) -> bool {
        let p = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
        match self {
            CropShape::Rect(b) => b.contains(p),
            CropShape::Ellipse(b) => {
                let c = b.center();
                let (rx, ry) = (b.width() / 2.0, b.height() / 2.0);
                rx > 0.0 && ry > 0.0 && ((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2) <= 1.0
            }
            CropShape::Polygon(pts) => ring_contains(pts, p),
        }
    }
}

/// The cropped picture and the pixel position of its top-left corner in
/// the old picture. `None` if the shape has no pixels of the picture.
pub fn crop(img: &RgbaImage, shape: &CropShape) -> Option<(RgbaImage, (u32, u32))> {
    let (w, h) = img.dimensions();
    let b = shape.bbox()?;
    let x0 = b.min.x.floor().max(0.0) as u32;
    let y0 = b.min.y.floor().max(0.0) as u32;
    let x1 = (b.max.x.ceil().max(0.0) as u32).min(w);
    let y1 = (b.max.y.ceil().max(0.0) as u32).min(h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let mut out = RgbaImage::new(x1 - x0, y1 - y0);
    let mut any = false;
    for y in y0..y1 {
        for x in x0..x1 {
            if shape.contains(x, y) {
                out.put_pixel(x - x0, y - y0, *img.get_pixel(x, y));
                any = true;
            } else {
                out.put_pixel(x - x0, y - y0, Rgba([0, 0, 0, 0]));
            }
        }
    }
    any.then_some((out, (x0, y0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn rect_ellipse_and_polygon() {
        let img = RgbaImage::from_pixel(20, 10, Rgba([200, 0, 0, 255]));
        let (r, at) = crop(&img, &CropShape::Rect(BBox::new(p(2.0, 3.0), p(12.0, 8.0)))).unwrap();
        assert_eq!((r.dimensions(), at), ((10, 5), (2, 3)));
        assert!(r.pixels().all(|q| q[3] == 255));

        let (e, _) = crop(&img, &CropShape::Ellipse(BBox::new(p(0.0, 0.0), p(10.0, 10.0)))).unwrap();
        assert_eq!(e.get_pixel(0, 0)[3], 0, "the corner is outside the ellipse");
        assert_eq!(e.get_pixel(5, 5)[3], 255);

        let tri = CropShape::Polygon(vec![p(0.0, 0.0), p(10.0, 0.0), p(0.0, 10.0)]);
        let (t, _) = crop(&img, &tri).unwrap();
        assert_eq!(t.dimensions(), (10, 10));
        assert_eq!(t.get_pixel(1, 1)[3], 255);
        assert_eq!(t.get_pixel(8, 8)[3], 0);

        assert!(crop(&img, &CropShape::Rect(BBox::new(p(30.0, 30.0), p(40.0, 40.0)))).is_none());
    }
}

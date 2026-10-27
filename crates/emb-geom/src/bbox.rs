use serde::{Deserialize, Serialize};

use crate::Point;

/// An axis-aligned bounding box in mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BBox {
    pub min: Point,
    pub max: Point,
}

impl BBox {
    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// Returns a box of `width` × `height` with its center at `center`.
    pub fn from_center_size(center: Point, width: f64, height: f64) -> Self {
        let h = Point::new(width / 2.0, height / 2.0);
        Self::new(center - h, center + h)
    }

    /// Returns the smallest box that contains all points, or `None` if there are no points.
    pub fn from_points<I: IntoIterator<Item = Point>>(points: I) -> Option<Self> {
        let mut it = points.into_iter();
        let first = it.next()?;
        let mut b = BBox::new(first, first);
        for p in it {
            b.include(p);
        }
        Some(b)
    }

    pub fn include(&mut self, p: Point) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    pub fn union(self, o: BBox) -> BBox {
        let mut b = self;
        b.include(o.min);
        b.include(o.max);
        b
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> Point {
        self.min.midpoint(self.max)
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Returns `true` if the two boxes overlap or touch.
    pub fn intersects(&self, o: &BBox) -> bool {
        self.min.x <= o.max.x && o.min.x <= self.max.x && self.min.y <= o.max.y && o.min.y <= self.max.y
    }

    /// Returns `true` if `o` is fully inside this box.
    pub fn contains_box(&self, o: &BBox) -> bool {
        self.contains(o.min) && self.contains(o.max)
    }

    /// Returns the box made larger by `d` on each side. A negative `d` makes it smaller.
    pub fn expand(&self, d: f64) -> BBox {
        BBox::new(self.min - Point::new(d, d), self.max + Point::new(d, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_points_and_contains() {
        let b = BBox::from_points([Point::new(1.0, 2.0), Point::new(-1.0, 5.0)]).unwrap();
        assert_eq!(b.width(), 2.0);
        assert_eq!(b.height(), 3.0);
        assert!(b.contains(Point::new(0.0, 3.0)));
        assert!(!b.contains(Point::new(0.0, 6.0)));
        assert!(BBox::from_points(std::iter::empty()).is_none());
    }

    #[test]
    fn intersects() {
        let a = BBox::new(Point::ZERO, Point::new(2.0, 2.0));
        assert!(a.intersects(&BBox::new(Point::new(1.0, 1.0), Point::new(3.0, 3.0))));
        assert!(a.intersects(&BBox::new(Point::new(2.0, 0.0), Point::new(3.0, 1.0))));
        assert!(!a.intersects(&BBox::new(Point::new(2.1, 0.0), Point::new(3.0, 1.0))));
    }
}

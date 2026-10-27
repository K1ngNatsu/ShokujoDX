use core::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use serde::{Deserialize, Serialize};

/// A point or a vector in mm.
///
/// The serialized form is a two-item array `[x, y]` to keep project files small.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(from = "[f64; 2]", into = "[f64; 2]")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn dot(self, o: Point) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// Z component of the 3D cross product. Positive if `o` is clockwise
    /// from `self` on screen (Y axis down).
    pub fn cross(self, o: Point) -> f64 {
        self.x * o.y - self.y * o.x
    }

    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }

    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }

    pub fn distance(self, o: Point) -> f64 {
        (o - self).length()
    }

    pub fn lerp(self, o: Point, t: f64) -> Point {
        Point::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }

    pub fn midpoint(self, o: Point) -> Point {
        self.lerp(o, 0.5)
    }

    /// Returns the unit vector, or `None` if the length is zero.
    pub fn normalized(self) -> Option<Point> {
        let len = self.length();
        (len > 0.0).then(|| Point::new(self.x / len, self.y / len))
    }

    /// Returns the vector turned 90° (x, y) -> (-y, x).
    pub fn perp(self) -> Point {
        Point::new(-self.y, self.x)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    /// Distance from this point to the segment `a`-`b`.
    pub fn distance_to_segment(self, a: Point, b: Point) -> f64 {
        let ab = b - a;
        let len_sq = ab.length_sq();
        if len_sq == 0.0 {
            return self.distance(a);
        }
        let t = ((self - a).dot(ab) / len_sq).clamp(0.0, 1.0);
        self.distance(a + ab * t)
    }
}

impl From<[f64; 2]> for Point {
    fn from([x, y]: [f64; 2]) -> Self {
        Point::new(x, y)
    }
}

impl From<Point> for [f64; 2] {
    fn from(p: Point) -> Self {
        [p.x, p.y]
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, o: Point) {
        *self = *self + o;
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, o: Point) {
        *self = *self - o;
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    fn mul(self, s: f64) -> Point {
        Point::new(self.x * s, self.y * s)
    }
}

impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Point {
        Point::new(-self.x, -self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_array() {
        let p = Point::new(1.5, -2.0);
        // Round trip through the `[f64; 2]` form that serde uses.
        let a: [f64; 2] = p.into();
        assert_eq!(a, [1.5, -2.0]);
        assert_eq!(Point::from(a), p);
    }

    #[test]
    fn segment_distance() {
        let p = Point::new(1.0, 1.0);
        assert_eq!(p.distance_to_segment(Point::ZERO, Point::new(2.0, 0.0)), 1.0);
        assert_eq!(p.distance_to_segment(Point::ZERO, Point::ZERO), 2.0_f64.sqrt());
    }
}

use serde::{Deserialize, Serialize};

use crate::{Point, math};

/// A 2D affine transform.
///
/// It maps `(x, y)` to `(a*x + c*y + e, b*x + d*y + f)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(dx: f64, dy: f64) -> Self {
        Transform {
            e: dx,
            f: dy,
            ..Self::IDENTITY
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Self {
        Transform {
            a: sx,
            d: sy,
            ..Self::IDENTITY
        }
    }

    /// Rotation about the origin. A positive angle turns clockwise on screen
    /// (Y axis down). Multiples of 90° are exact.
    pub fn rotate_deg(deg: f64) -> Self {
        let (s, c) = math::sin_cos_deg(deg);
        Transform {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Returns the transform that applies `self` first, then `next`.
    pub fn then(self, next: Transform) -> Transform {
        let m = next;
        Transform {
            a: m.a * self.a + m.c * self.b,
            b: m.b * self.a + m.d * self.b,
            c: m.a * self.c + m.c * self.d,
            d: m.b * self.c + m.d * self.d,
            e: m.a * self.e + m.c * self.f + m.e,
            f: m.b * self.e + m.d * self.f + m.f,
        }
    }

    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Applies only the linear part (no translation). Use this for vectors.
    pub fn apply_vector(&self, v: Point) -> Point {
        Point::new(self.a * v.x + self.c * v.y, self.b * v.x + self.d * v.y)
    }

    /// Returns the inverse transform, or `None` if the transform is singular.
    pub fn inverse(&self) -> Option<Transform> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let inv = 1.0 / det;
        let a = self.d * inv;
        let b = -self.b * inv;
        let c = -self.c * inv;
        let d = self.a * inv;
        Some(Transform {
            a,
            b,
            c,
            d,
            e: -(a * self.e + c * self.f),
            f: -(b * self.e + d * self.f),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turn_is_exact() {
        let t = Transform::rotate_deg(90.0);
        // Clockwise on screen with Y down: +X goes to +Y.
        assert_eq!(t.apply(Point::new(1.0, 0.0)), Point::new(0.0, 1.0));
    }

    #[test]
    fn then_order() {
        let t = Transform::translate(1.0, 0.0).then(Transform::scale(2.0, 2.0));
        assert_eq!(t.apply(Point::ZERO), Point::new(2.0, 0.0));
    }

    #[test]
    fn inverse_round_trip() {
        let t = Transform::rotate_deg(33.0)
            .then(Transform::scale(2.0, 0.5))
            .then(Transform::translate(3.0, -4.0));
        let p = Point::new(7.0, 11.0);
        let q = t.inverse().unwrap().apply(t.apply(p));
        assert!(p.distance(q) < 1e-9);
    }
}

//! Lettering envelopes: the text bends into a shape (P4 Milestone 4,
//! ADR 0018).
//!
//! An envelope moves the top and the bottom edge of the text box. Each
//! point of a letter keeps its X position and its height as a part of the
//! box at that X: a point halfway up the box stays halfway between the
//! new edges. Unlike a baseline, an envelope bends the letters.

use emb_geom::{BBox, Path, PathCmd, Point, math};
use serde::{Deserialize, Serialize};

use crate::ObjectKind;

/// The shape of an envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeShape {
    #[default]
    None,
    /// The top edge arches up; the letters in the middle are taller.
    Bridge,
    /// The top edge dips; the letters in the middle are shorter.
    Valley,
    /// Both edges arch up: the text bows up.
    ArchOver,
    /// Both edges arch down: the text bows down.
    ArchUnder,
    /// The top arches up and the bottom down: the middle is taller.
    Bulge,
    /// The top dips and the bottom rises: the middle is shorter.
    Pinch,
    /// The text gets smaller to the right, about its middle line.
    PennantRight,
    /// The text gets smaller to the left.
    PennantLeft,
    /// The top rises to the right; the bottom stays straight.
    PerspectiveRight,
    /// The top rises to the left.
    PerspectiveLeft,
    /// The text is tallest in the middle, with straight edges.
    Diamond,
    /// Both edges go up and down once, together.
    Wave,
    /// The text fills an oval: taller in the middle, shorter at the ends.
    Oval,
}

impl EnvelopeShape {
    pub const ALL: [EnvelopeShape; 14] = [
        EnvelopeShape::None,
        EnvelopeShape::Bridge,
        EnvelopeShape::Valley,
        EnvelopeShape::ArchOver,
        EnvelopeShape::ArchUnder,
        EnvelopeShape::Bulge,
        EnvelopeShape::Pinch,
        EnvelopeShape::PennantRight,
        EnvelopeShape::PennantLeft,
        EnvelopeShape::PerspectiveRight,
        EnvelopeShape::PerspectiveLeft,
        EnvelopeShape::Diamond,
        EnvelopeShape::Wave,
        EnvelopeShape::Oval,
    ];

    pub fn name(self) -> &'static str {
        match self {
            EnvelopeShape::None => "None",
            EnvelopeShape::Bridge => "Bridge",
            EnvelopeShape::Valley => "Valley",
            EnvelopeShape::ArchOver => "Arch over",
            EnvelopeShape::ArchUnder => "Arch under",
            EnvelopeShape::Bulge => "Bulge",
            EnvelopeShape::Pinch => "Pinch",
            EnvelopeShape::PennantRight => "Pennant right",
            EnvelopeShape::PennantLeft => "Pennant left",
            EnvelopeShape::PerspectiveRight => "Perspective right",
            EnvelopeShape::PerspectiveLeft => "Perspective left",
            EnvelopeShape::Diamond => "Diamond",
            EnvelopeShape::Wave => "Wave",
            EnvelopeShape::Oval => "Oval",
        }
    }
}

/// An envelope: a shape and its amount.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub shape: EnvelopeShape,
    /// How far the edges move, as a part of the text height (0 to 1).
    pub amount: f64,
}

impl Default for Envelope {
    fn default() -> Self {
        Envelope {
            shape: EnvelopeShape::None,
            amount: 0.4,
        }
    }
}

/// The largest amount; more would turn the text inside out (pinch).
pub const MAX_AMOUNT: f64 = 0.9;

impl Envelope {
    pub fn is_none(&self) -> bool {
        self.shape == EnvelopeShape::None || self.amount == 0.0
    }

    /// How far the top edge moves up and the bottom edge moves down at `u`
    /// (0 at the left of the box, 1 at the right), as parts of the height.
    pub fn edges(&self, u: f64) -> (f64, f64) {
        let a = self.amount.clamp(0.0, MAX_AMOUNT);
        let arc = math::sin(std::f64::consts::PI * u);
        let tri = 1.0 - (2.0 * u - 1.0).abs();
        let wave = math::sin(2.0 * std::f64::consts::PI * u) / 2.0;
        match self.shape {
            EnvelopeShape::None => (0.0, 0.0),
            EnvelopeShape::Bridge => (a * arc, 0.0),
            EnvelopeShape::Valley => (-a * arc, 0.0),
            EnvelopeShape::ArchOver => (a * arc, -a * arc),
            EnvelopeShape::ArchUnder => (-a * arc, a * arc),
            EnvelopeShape::Bulge => (a * arc / 2.0, a * arc / 2.0),
            EnvelopeShape::Pinch => (-a * arc / 2.0, -a * arc / 2.0),
            EnvelopeShape::PennantRight => (-a * u / 2.0, -a * u / 2.0),
            EnvelopeShape::PennantLeft => (-a * (1.0 - u) / 2.0, -a * (1.0 - u) / 2.0),
            EnvelopeShape::PerspectiveRight => (a * u, 0.0),
            EnvelopeShape::PerspectiveLeft => (a * (1.0 - u), 0.0),
            EnvelopeShape::Diamond => (a * tri / 2.0, a * tri / 2.0),
            EnvelopeShape::Wave => (a * wave, -a * wave),
            EnvelopeShape::Oval => {
                let half = (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt() - 0.5;
                (a * half, a * half)
            }
        }
    }

    /// The new top and bottom of box `b` at `x`.
    fn top_bottom(&self, b: &BBox, x: f64) -> (f64, f64) {
        let h = b.height();
        let u = (x - b.min.x) / b.width().max(1e-9);
        let (up, down) = self.edges(u);
        (b.min.y - up * h, b.max.y + down * h)
    }

    /// Where point `p` of box `b` goes.
    pub fn warp(&self, b: &BBox, p: Point) -> Point {
        let h = b.height().max(1e-9);
        let (top, bottom) = self.top_bottom(b, p.x);
        let v = (p.y - b.min.y) / h;
        Point::new(p.x, top + v * (bottom - top))
    }

    /// The inverse of [`Envelope::warp`].
    pub fn unwarp(&self, b: &BBox, p: Point) -> Point {
        let (top, bottom) = self.top_bottom(b, p.x);
        let span = bottom - top;
        let v = if span.abs() < 1e-9 { 0.0 } else { (p.y - top) / span };
        Point::new(p.x, b.min.y + v * b.height())
    }

    /// Bends `kind` with box `b`. Its paths get points at most `step` mm
    /// apart first, so that straight lines can bend.
    pub fn warp_kind(&self, b: &BBox, kind: &mut ObjectKind) {
        let step = (b.width() / 80.0).max(0.25);
        for p in kind.paths_mut() {
            *p = densify(p, step);
        }
        map_points(kind, &|q| self.warp(b, q));
    }
}

/// The path as straight lines at most `step` apart (curves flattened).
pub fn densify(path: &Path, step: f64) -> Path {
    let mut out = Path::new();
    for pl in path.flatten(0.02) {
        let n = pl.points.len();
        let mut pts = Vec::with_capacity(n * 2);
        let segs = if pl.closed { n } else { n - 1 };
        pts.push(pl.points[0]);
        for i in 0..segs {
            let (a, c) = (pl.points[i], pl.points[(i + 1) % n]);
            let k = (a.distance(c) / step).ceil().max(1.0) as usize;
            for j in 1..=k {
                pts.push(a.lerp(c, j as f64 / k as f64));
            }
        }
        if pl.closed {
            pts.pop();
        }
        out = out.move_to(pts[0]);
        for q in &pts[1..] {
            out = out.line_to(*q);
        }
        if pl.closed {
            out = out.close();
        }
    }
    out
}

/// Moves every point of `kind` (path points, curve control points and the
/// rungs of a satin) with `f`.
pub fn map_points(kind: &mut ObjectKind, f: &dyn Fn(Point) -> Point) {
    for p in kind.paths_mut() {
        for c in &mut p.cmds {
            match c {
                PathCmd::MoveTo(a) | PathCmd::LineTo(a) => *a = f(*a),
                PathCmd::CubicTo(a, b, e) => {
                    *a = f(*a);
                    *b = f(*b);
                    *e = f(*e);
                }
                PathCmd::Close => {}
            }
        }
    }
    if let ObjectKind::Satin { geometry, .. } = kind {
        for (a, b) in &mut geometry.rungs {
            *a = f(*a);
            *b = f(*b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx() -> BBox {
        BBox::new(Point::new(0.0, -10.0), Point::new(100.0, 0.0))
    }

    #[test]
    fn bridge_raises_the_middle_top_only() {
        let e = Envelope {
            shape: EnvelopeShape::Bridge,
            amount: 0.5,
        };
        let b = bx();
        assert_eq!(e.warp(&b, Point::new(50.0, -10.0)), Point::new(50.0, -15.0));
        assert_eq!(e.warp(&b, Point::new(50.0, 0.0)), Point::new(50.0, 0.0));
        assert_eq!(e.warp(&b, Point::new(50.0, -5.0)), Point::new(50.0, -7.5));
        let end = e.warp(&b, Point::new(0.0, -10.0));
        assert!(end.distance(Point::new(0.0, -10.0)) < 1e-9);
    }

    #[test]
    fn every_shape_unwarps() {
        let b = bx();
        for shape in EnvelopeShape::ALL {
            let e = Envelope { shape, amount: 0.6 };
            for p in [Point::new(10.0, -3.0), Point::new(55.0, -9.0), Point::new(90.0, 2.0)] {
                let back = e.unwarp(&b, e.warp(&b, p));
                assert!(back.distance(p) < 1e-9, "{shape:?} {p:?} {back:?}");
            }
        }
        assert!(Envelope::default().is_none());
    }

    #[test]
    fn warp_kind_bends_straight_lines() {
        let e = Envelope {
            shape: EnvelopeShape::ArchOver,
            amount: 0.5,
        };
        let mut k = ObjectKind::Running {
            path: Path::new()
                .move_to(Point::new(0.0, -5.0))
                .line_to(Point::new(100.0, -5.0)),
            params: Default::default(),
        };
        e.warp_kind(&bx(), &mut k);
        let pts: Vec<Point> = k.paths().next().unwrap().points().collect();
        assert!(pts.len() > 50);
        let mid = pts.iter().find(|p| (p.x - 50.0).abs() < 1e-6).unwrap();
        assert!(
            (mid.y - -10.0).abs() < 1e-9,
            "the middle moved up by half the height: {mid:?}"
        );
        // Ten pieces on each long side, one on each short side.
        assert_eq!(densify(&Path::rect(bx()), 10.0).points().count(), 22);
    }
}

//! Cutwork needles (P11 Milestone 8, ADR 0051): lines that the cutting
//! needles of an embroidery machine cut.
//!
//! A cutwork kit has four needles with blades; each blade cuts in one
//! direction. A cut line is cut into pieces by direction, and each piece
//! goes to the needle whose blade is nearest to it (each needle has 45° of
//! directions). The pieces of one needle are one color block, so the
//! machine stops to change the needle (or to turn the cutting tool) at each
//! color change.
//!
//! The needle order and the colors of each brand are from the public
//! Ink/Stitch documentation ("Cutwork", <https://inkstitch.org/docs/cutwork/>,
//! updated 2022-12-30; Ink/Stitch is GPL-3.0, spec rule 4). How each
//! machine finds a cutting needle in its file is not public: a tester with
//! such a machine must check the files (the exit test).

use emb_geom::math::{atan2, to_degrees};
use emb_geom::{Path, Point, Polyline};

use crate::{Design, Functions, ObjectId, ObjectKind, Rgb, RunningParams, TrimMode};

/// The space between two needle points of a cut line, at the start, in mm.
pub const DEFAULT_PENETRATION_MM: f64 = 1.0;

/// A brand of cutwork kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutBrand {
    Bernina,
    Brother,
    Husqvarna,
    Janome,
}

impl CutBrand {
    pub const ALL: [CutBrand; 4] = [
        CutBrand::Bernina,
        CutBrand::Brother,
        CutBrand::Husqvarna,
        CutBrand::Janome,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CutBrand::Bernina => "Bernina",
            CutBrand::Brother => "Brother, Baby Lock",
            CutBrand::Husqvarna => "Husqvarna Viking, Pfaff",
            CutBrand::Janome => "Janome",
        }
    }

    /// The brand of the kit for a machine brand, if it is one of these.
    pub fn of_machine(brand: &str) -> Option<CutBrand> {
        let b = brand.to_lowercase();
        if b.contains("bernina") || b.contains("bernette") {
            Some(CutBrand::Bernina)
        } else if b.contains("brother") || b.contains("baby lock") || b.contains("babylock") {
            Some(CutBrand::Brother)
        } else if b.contains("husqvarna") || b.contains("viking") || b.contains("pfaff") {
            Some(CutBrand::Husqvarna)
        } else if b.contains("janome") {
            Some(CutBrand::Janome)
        } else {
            None
        }
    }

    /// The needles 1 to 4 of the kit.
    pub fn needles(self) -> [CutNeedle; 4] {
        let n = |angle_deg, color| CutNeedle { angle_deg, color };
        match self {
            CutBrand::Bernina => [
                n(90.0, Rgb(255, 255, 0)),
                n(45.0, Rgb(0, 255, 0)),
                n(0.0, Rgb(255, 0, 0)),
                n(135.0, Rgb(255, 0, 255)),
            ],
            CutBrand::Husqvarna => [
                n(45.0, Rgb(220, 30, 30)),
                n(0.0, Rgb(240, 210, 0)),
                n(135.0, Rgb(0, 160, 60)),
                n(90.0, Rgb(30, 80, 220)),
            ],
            CutBrand::Brother => [
                n(45.0, Rgb(30, 80, 220)),
                n(0.0, Rgb(130, 40, 160)),
                n(135.0, Rgb(0, 160, 60)),
                n(90.0, Rgb(240, 130, 0)),
            ],
            CutBrand::Janome => [
                n(0.0, Rgb(220, 30, 30)),
                n(45.0, Rgb(30, 80, 220)),
                n(90.0, Rgb(20, 20, 20)),
                n(135.0, Rgb(0, 160, 60)),
            ],
        }
    }
}

/// One cutting needle: the direction of its blade (0° level, 45° as "/",
/// 90° upright, 135° as "\", as the design shows on the screen) and the
/// color of its color block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CutNeedle {
    pub angle_deg: f64,
    pub color: Rgb,
}

impl CutNeedle {
    /// The sign of the blade direction.
    pub fn symbol(&self) -> &'static str {
        match self.angle_deg as i64 {
            0 => "—",
            45 => "/",
            90 => "|",
            _ => "\\",
        }
    }
}

/// The direction of a stitch from `a` to `b` as the screen shows it (Y is
/// down in the design), in [0°, 180°).
pub fn direction_deg(a: Point, b: Point) -> f64 {
    to_degrees(atan2(-(b.y - a.y), b.x - a.x)).rem_euclid(180.0)
}

/// The needle (index into `needles`) whose blade is nearest to `deg`.
pub fn needle_for(deg: f64, needles: &[CutNeedle]) -> usize {
    let off = |n: &CutNeedle| {
        let d = (deg - n.angle_deg).rem_euclid(180.0);
        d.min(180.0 - d)
    };
    (0..needles.len())
        .min_by(|&a, &b| off(&needles[a]).total_cmp(&off(&needles[b])).then(a.cmp(&b)))
        .unwrap_or(0)
}

/// The pieces of a cut line, each with its needle (index into `needles`),
/// in the order of the line.
pub fn cut_pieces(line: &Polyline, needles: &[CutNeedle]) -> Vec<(usize, Vec<Point>)> {
    let mut pts = line.points.clone();
    pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
    if line.closed && pts.len() > 2 {
        pts.push(pts[0]);
    }
    let mut out: Vec<(usize, Vec<Point>)> = Vec::new();
    for w in pts.windows(2) {
        let k = needle_for(direction_deg(w[0], w[1]), needles);
        match out.last_mut() {
            Some((n, piece)) if *n == k => piece.push(w[1]),
            _ => out.push((k, vec![w[0], w[1]])),
        }
    }
    out
}

/// The thread name of needle `i` (0-based) of a kit.
pub fn needle_name(i: usize, n: &CutNeedle) -> String {
    format!("Cutwork needle {} ({})", i + 1, n.symbol())
}

impl Design {
    /// Makes cut lines of the lines `ids` for the kit `brand`: one object
    /// for each needle that has pieces, in needle order, with a thread of
    /// its name and color, at the place of the first line. The lines go.
    /// Returns the new objects.
    pub fn make_cutwork(&mut self, ids: &[ObjectId], brand: CutBrand, penetration_mm: f64) -> Vec<ObjectId> {
        let needles = brand.needles();
        let mut pieces: Vec<Vec<Vec<Point>>> = vec![Vec::new(); needles.len()];
        let mut used = Vec::new();
        for id in ids {
            let Some(ObjectKind::Running { path, .. }) = self.object(*id).map(|o| &o.kind) else {
                continue;
            };
            for line in path.flatten(0.05) {
                for (k, piece) in cut_pieces(&line, &needles) {
                    pieces[k].push(piece);
                }
            }
            used.push(*id);
        }
        if used.is_empty() {
            return Vec::new();
        }
        let mut at = used.iter().filter_map(|id| self.index_of(*id)).min().unwrap_or(0);
        for id in &used {
            self.remove_object(*id);
        }
        let mut out = Vec::new();
        for (k, list) in pieces.into_iter().enumerate() {
            if list.is_empty() {
                continue;
            }
            let name = needle_name(k, &needles[k]);
            let thread = match self.threads.iter().find(|t| t.name == name) {
                Some(t) => t.id,
                None => self.add_thread(name.clone(), needles[k].color),
            };
            let parts: Vec<Path> = list
                .iter()
                .map(|p| p[1..].iter().fold(Path::new().move_to(p[0]), |a, q| a.line_to(*q)))
                .collect();
            let id = self.insert_object(
                at,
                thread,
                ObjectKind::Running {
                    path: Path::joined(&parts),
                    params: RunningParams {
                        stitch_length_mm: penetration_mm.max(0.3),
                        cut: true,
                        ..RunningParams::default()
                    },
                },
            );
            if let Some(o) = self.object_mut(id) {
                o.name = Some(name);
                // No lock stitches: the blades would cut there too.
                o.functions = Functions {
                    tie_in: false,
                    tie_off: false,
                    trim: TrimMode::Always,
                    ..Functions::default()
                };
            }
            at += 1;
            out.push(id);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_go_to_the_nearest_blade() {
        let needles = CutBrand::Bernina.needles();
        let p = Point::new;
        // Level, "/" (up to the right on the screen: Y is down), upright, "\".
        assert_eq!(direction_deg(p(0.0, 0.0), p(10.0, 0.0)), 0.0);
        assert!((direction_deg(p(0.0, 0.0), p(10.0, -10.0)) - 45.0).abs() < 1e-9);
        assert!((direction_deg(p(0.0, 0.0), p(0.0, -10.0)) - 90.0).abs() < 1e-9);
        assert!((direction_deg(p(0.0, 0.0), p(10.0, 10.0)) - 135.0).abs() < 1e-9);
        // A line and the same line backward have the same needle.
        assert_eq!(direction_deg(p(10.0, 0.0), p(0.0, 0.0)), 0.0);
        // Bernina: 1 |, 2 /, 3 —, 4 \.
        assert_eq!(needle_for(90.0, &needles), 0);
        assert_eq!(needle_for(50.0, &needles), 1);
        assert_eq!(needle_for(170.0, &needles), 2, "170° is 10° from level");
        assert_eq!(needle_for(10.0, &needles), 2);
        assert_eq!(needle_for(130.0, &needles), 3);
        assert_eq!(
            needles.iter().map(|n| n.symbol()).collect::<Vec<_>>(),
            ["|", "/", "—", "\\"]
        );
    }

    #[test]
    fn a_square_is_cut_by_two_needles_and_a_circle_by_four() {
        let needles = CutBrand::Brother.needles();
        let square = Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(20.0, 20.0)));
        let pieces = cut_pieces(&square.flatten(0.05)[0], &needles);
        let used: std::collections::BTreeSet<usize> = pieces.iter().map(|(k, _)| *k).collect();
        // Brother: 1 /, 2 —, 3 \, 4 |.
        assert_eq!(used.into_iter().collect::<Vec<_>>(), [1, 3]);
        assert_eq!(pieces.len(), 4, "the four sides");
        let circle = Path::ellipse(Point::ZERO, 20.0, 20.0);
        let pieces = cut_pieces(&circle.flatten(0.05)[0], &needles);
        assert_eq!(
            pieces
                .iter()
                .map(|(k, _)| *k)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        // Each piece is within 22.5° (and a little for the flattening) of its blade.
        for (k, piece) in &pieces {
            for w in piece.windows(2) {
                let d = (direction_deg(w[0], w[1]) - needles[*k].angle_deg).rem_euclid(180.0);
                assert!(d.min(180.0 - d) <= 22.5 + 1e-6);
            }
        }
    }

    #[test]
    fn make_cutwork_gives_one_object_for_each_needle_in_order() {
        let mut d = Design::new("cut");
        let t = d.add_thread("Black", Rgb(0, 0, 0));
        let circle = d.add_object(
            t,
            ObjectKind::Running {
                path: Path::ellipse(Point::ZERO, 15.0, 15.0),
                params: RunningParams::default(),
            },
        );
        let new = d.make_cutwork(&[circle], CutBrand::Bernina, 1.0);
        assert_eq!(new.len(), 4);
        // The line goes: only the four cut lines are left.
        assert_eq!(d.objects().len(), 4);
        assert!(
            d.objects()
                .iter()
                .all(|o| matches!(&o.kind, ObjectKind::Running { params, .. } if params.cut))
        );
        let names: Vec<String> = new
            .iter()
            .map(|id| d.thread(d.object(*id).unwrap().thread).unwrap().name.clone())
            .collect();
        assert_eq!(
            names,
            [
                "Cutwork needle 1 (|)",
                "Cutwork needle 2 (/)",
                "Cutwork needle 3 (—)",
                "Cutwork needle 4 (\\)"
            ]
        );
        let o = d.object(new[0]).unwrap();
        assert!(!o.functions.tie_in && !o.functions.tie_off);
        assert!(matches!(&o.kind, ObjectKind::Running { params, .. } if params.cut));
    }
}

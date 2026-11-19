//! The families of clipart: small designs of fills and lines, about 50 mm
//! large, with their middle at (0, 0). Y is down, as on the canvas.

use emb_geom::math::{cos, sin};
use emb_geom::{Path, Point};

use super::{Family, Make, PI, TAU, count, size};
use crate::{Design, FillParams, ObjectKind, Rgb, RunningParams};

/// The radius of a clipart item, in mm.
const R: f64 = 25.0;

const GOLD: (&str, Rgb) = ("Gold", Rgb(230, 170, 30));
const BROWN: (&str, Rgb) = ("Brown", Rgb(110, 70, 40));
const NAVY: (&str, Rgb) = ("Navy", Rgb(25, 40, 100));
const BLUE: (&str, Rgb) = ("Blue", Rgb(40, 140, 200));
const ICE: (&str, Rgb) = ("Ice blue", Rgb(150, 200, 235));
const RED: (&str, Rgb) = ("Red", Rgb(200, 30, 50));
const PINK: (&str, Rgb) = ("Pink", Rgb(235, 120, 160));
const YELLOW: (&str, Rgb) = ("Yellow", Rgb(250, 215, 50));
const ORANGE: (&str, Rgb) = ("Orange", Rgb(240, 140, 30));
const GREEN: (&str, Rgb) = ("Green", Rgb(40, 140, 70));
const DARK_GREEN: (&str, Rgb) = ("Dark green", Rgb(20, 80, 40));
const GRAY: (&str, Rgb) = ("Gray", Rgb(130, 135, 145));
const WHITE: (&str, Rgb) = ("White", Rgb(240, 240, 240));

/// A design that is being made.
struct Art(Design);

impl Art {
    fn new() -> Art {
        Art(Design::new(""))
    }

    fn fill_with(&mut self, color: (&str, Rgb), outer: &[Point], holes: &[Vec<Point>], angle_deg: f64) {
        let t = self.0.add_thread(color.0, color.1);
        self.0.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::polygon(outer),
                holes: holes.iter().map(|h| Path::polygon(h)).collect(),
                params: FillParams {
                    angle_deg,
                    ..FillParams::default()
                },
            },
        );
    }

    fn fill(&mut self, color: (&str, Rgb), outer: &[Point], angle_deg: f64) {
        self.fill_with(color, outer, &[], angle_deg);
    }

    /// A line of three runs: closed round `points`, or open.
    fn line(&mut self, color: (&str, Rgb), points: &[Point], closed: bool) {
        let path = if closed {
            Path::polygon(points)
        } else {
            points[1..]
                .iter()
                .fold(Path::new().move_to(points[0]), |p, q| p.line_to(*q))
        };
        let t = self.0.add_thread(color.0, color.1);
        self.0.add_object(
            t,
            ObjectKind::Running {
                path,
                params: RunningParams {
                    repeats: 3,
                    ..RunningParams::default()
                },
            },
        );
    }
}

/// A closed shape from its radius (in mm) at each angle; the angle 0 is up.
fn round(n: usize, radius: impl Fn(f64) -> f64) -> Vec<Point> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let r = radius(a);
            Point::new(r * sin(a), -r * cos(a))
        })
        .collect()
}

/// A shape with `n` corners out (radius `outer`) and `n` corners in
/// (radius `inner`), one corner up.
fn spikes(n: usize, outer: f64, inner: f64) -> Vec<Point> {
    (0..2 * n)
        .map(|k| {
            let a = PI * k as f64 / n as f64;
            let r = if k % 2 == 0 { outer } else { inner };
            Point::new(r * sin(a), -r * cos(a))
        })
        .collect()
}

fn circle(c: Point, rx: f64, ry: f64) -> Vec<Point> {
    (0..40)
        .map(|k| {
            let a = TAU * f64::from(k) / 40.0;
            Point::new(c.x + rx * cos(a), c.y + ry * sin(a))
        })
        .collect()
}

fn turned(p: &[Point], angle: f64, to: Point) -> Vec<Point> {
    let (s, c) = (sin(angle), cos(angle));
    p.iter()
        .map(|q| Point::new(to.x + q.x * c - q.y * s, to.y + q.x * s + q.y * c))
        .collect()
}

fn pts(p: &[(f64, f64)]) -> Vec<Point> {
    p.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

/// A heart about 2 high and 2 wide, its point at (0, 1).
fn heart() -> Vec<Point> {
    (0..48)
        .map(|k| {
            let a = TAU * f64::from(k) / 48.0;
            let x = 16.0 * sin(a) * sin(a) * sin(a);
            let y = 13.0 * cos(a) - 5.0 * cos(2.0 * a) - 2.0 * cos(3.0 * a) - cos(4.0 * a);
            Point::new(x / 16.0, -(y + 2.5) / 14.5)
        })
        .collect()
}

fn star(v: &[f64]) -> Design {
    let mut a = Art::new();
    let shape = spikes(v[0] as usize, R, R * v[1]);
    a.fill(GOLD, &shape, 0.0);
    a.line(BROWN, &shape, true);
    a.0
}

fn flower(v: &[f64]) -> Design {
    let (n, middle) = (v[0], v[1]);
    let mut a = Art::new();
    a.fill(PINK, &round(120, |t| R * (0.62 + 0.38 * cos(n * t))), 45.0);
    a.fill(YELLOW, &circle(Point::ZERO, R * middle, R * middle), 135.0);
    a.0
}

fn sun(v: &[f64]) -> Design {
    let (n, rays) = (v[0] as usize, v[1]);
    let mut a = Art::new();
    a.fill(YELLOW, &spikes(n, R, R * (1.0 - rays)), 0.0);
    let disc = R * (1.0 - rays) * 0.9;
    a.fill(ORANGE, &circle(Point::ZERO, disc, disc), 45.0);
    a.0
}

fn snowflake(v: &[f64]) -> Design {
    let (n, twigs) = (v[0] as usize, v[1] as usize);
    let mut a = Art::new();
    // One line: each arm out with its twigs, and back to the middle.
    let mut line = vec![Point::ZERO];
    for k in 0..n {
        let turn = TAU * k as f64 / n as f64;
        let at = |r: f64, side: f64| {
            let t = turn + side;
            Point::new(r * sin(t), -r * cos(t))
        };
        for t in 1..=twigs {
            let r = R * t as f64 / (twigs + 1) as f64;
            let tip = r + 0.22 * R;
            let spread = 0.2 * R / tip;
            line.extend([at(r, 0.0), at(tip, -spread), at(r, 0.0), at(tip, spread), at(r, 0.0)]);
        }
        line.extend([at(R, 0.0), Point::ZERO]);
    }
    a.line(ICE, &line, false);
    a.0
}

fn seal(v: &[f64]) -> Design {
    let (n, ring) = (v[0], v[1] >= 0.5);
    let mut a = Art::new();
    a.fill(RED, &round(160, |t| R * (0.93 + 0.07 * cos(n * t))), 45.0);
    if ring {
        a.line(GOLD, &circle(Point::ZERO, 0.72 * R, 0.72 * R), true);
    }
    a.0
}

fn polygon(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    let shape = round(n, |_| R);
    a.fill(BLUE, &shape, 45.0);
    a.line(NAVY, &shape, true);
    a.0
}

fn gear(v: &[f64]) -> Design {
    let (n, hole) = (v[0] as usize, v[1]);
    let mut a = Art::new();
    // Each tooth: two corners at the root and two at the tip.
    let mut shape = Vec::new();
    for k in 0..n {
        let step = TAU / n as f64;
        let at = |part: f64, r: f64| {
            let t = step * (k as f64 + part);
            Point::new(r * sin(t), -r * cos(t))
        };
        shape.extend([at(0.0, 0.78 * R), at(0.12, R), at(0.38, R), at(0.5, 0.78 * R)]);
    }
    a.fill_with(GRAY, &shape, &[circle(Point::ZERO, hole * R, hole * R)], 45.0);
    a.0
}

fn burst(v: &[f64]) -> Design {
    let mut a = Art::new();
    a.fill(RED, &spikes(v[0] as usize, R, R * (1.0 - v[1])), 0.0);
    a.0
}

fn shield(v: &[f64]) -> Design {
    let point = v[0];
    let mut a = Art::new();
    // A flat top, straight sides, and a curve from each side to the point.
    let (w, top, side) = (0.8 * R, -R, R * (1.0 - 2.0 * point));
    let curve = |x: f64, t: f64| {
        // From (x, side) to the point (0, R); it starts straight down.
        let bend = side + 0.65 * (R - side);
        Point::new(
            (1.0 - t) * (1.0 - t) * x + 2.0 * t * (1.0 - t) * x,
            (1.0 - t) * (1.0 - t) * side + 2.0 * t * (1.0 - t) * bend + t * t * R,
        )
    };
    let mut shape = pts(&[(-w, top), (w, top)]);
    shape.extend((0..=12).map(|k| curve(w, f64::from(k) / 12.0)));
    shape.extend((0..12).rev().map(|k| curve(-w, f64::from(k) / 12.0)));
    a.fill(NAVY, &shape, 45.0);
    a.line(GOLD, &shape, true);
    a.0
}

fn ribbon(v: &[f64]) -> Design {
    let (half, notch) = (v[0] / 2.0, v[1]);
    let h = 9.0;
    let mut a = Art::new();
    let cut = notch * 2.0 * h;
    a.fill(
        NAVY,
        &pts(&[
            (-half, -h),
            (half, -h),
            (half - cut, 0.0),
            (half, h),
            (-half, h),
            (-half + cut, 0.0),
        ]),
        90.0,
    );
    let inner = half - cut - 5.0;
    a.line(
        WHITE,
        &pts(&[
            (-inner, -h + 3.0),
            (inner, -h + 3.0),
            (inner, h - 3.0),
            (-inner, h - 3.0),
        ]),
        true,
    );
    a.0
}

fn plus(v: &[f64]) -> Design {
    let w = v[0] * R;
    let mut a = Art::new();
    let shape = pts(&[
        (-w, -R),
        (w, -R),
        (w, -w),
        (R, -w),
        (R, w),
        (w, w),
        (w, R),
        (-w, R),
        (-w, w),
        (-R, w),
        (-R, -w),
        (-w, -w),
    ]);
    a.fill(RED, &shape, 45.0);
    a.line(WHITE, &shape, true);
    a.0
}

fn moon(v: &[f64]) -> Design {
    let inner = v[0];
    let mut a = Art::new();
    // The outer edge is the left half of a circle; the inner edge goes
    // `inner` of the way to it.
    let mut shape: Vec<Point> = (0..=32)
        .map(|k| {
            let t = PI * f64::from(k) / 32.0;
            Point::new(0.5 * R - R * sin(t), -R * cos(t))
        })
        .collect();
    shape.extend((1..32).map(|k| {
        let t = PI * (1.0 - f64::from(k) / 32.0);
        Point::new(0.5 * R - inner * R * sin(t), -R * cos(t))
    }));
    a.fill(YELLOW, &shape, 45.0);
    a.0
}

fn cloud(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    // A flat base, and round bumps over it: the middle bumps are the
    // highest.
    let (half, base) = (1.1 * R, 0.55 * R);
    let w = 2.0 * half / n as f64;
    let mut shape = pts(&[(half, base), (-half, base)]);
    for k in 0..n {
        let c = -half + w * (k as f64 + 0.5);
        let middle = base - 0.3 * R - 0.55 * R * sin(PI * (k as f64 + 0.5) / n as f64);
        for i in 0..=12 {
            let t = PI * f64::from(i) / 12.0;
            shape.push(Point::new(c - 0.5 * w * cos(t), middle - 0.4 * R * sin(t)));
        }
    }
    a.fill(ICE, &shape, 0.0);
    a.line(GRAY, &shape, true);
    a.0
}

fn drop(v: &[f64]) -> Design {
    let w = v[0];
    let mut a = Art::new();
    // A point at the top, round at the bottom.
    let shape: Vec<Point> = (0..64)
        .map(|k| {
            let t = TAU * f64::from(k) / 64.0;
            Point::new(R * w * sin(t) * sin(t / 2.0) * sin(t / 2.0) / 0.65, -R * cos(t))
        })
        .collect();
    a.fill(BLUE, &shape, 45.0);
    a.0
}

fn clover(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    // Each leaf is a heart with its point at the middle.
    let leaf: Vec<Point> = heart()
        .into_iter()
        .map(|p| Point::new(p.x * 0.42 * R, (p.y - 1.0) * 0.42 * R))
        .collect();
    for k in 0..n {
        let angle = TAU * k as f64 / n as f64;
        a.fill(
            GREEN,
            &turned(&leaf, angle, Point::new(0.0, -0.1 * R)),
            45.0 + 90.0 * k as f64,
        );
    }
    a.line(
        DARK_GREEN,
        &pts(&[(0.0, -0.1 * R), (0.05 * R, 0.5 * R), (0.25 * R, R)]),
        false,
    );
    a.0
}

fn paw(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    a.fill(BROWN, &circle(Point::new(0.0, 0.35 * R), 0.55 * R, 0.45 * R), 0.0);
    // The toes on an arc over the pad.
    for k in 0..n {
        let t = PI * (0.18 + 0.64 * k as f64 / (n - 1) as f64);
        let c = Point::new(-0.75 * R * cos(t), 0.2 * R - 0.85 * R * sin(t));
        a.fill(BROWN, &circle(c, 0.2 * R, 0.26 * R), 0.0);
    }
    a.0
}

fn crown(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    let (w, base, low) = (R, 0.6 * R, -0.05 * R);
    let mut shape = pts(&[(w, base), (-w, base)]);
    // The points of the crown: the middle ones are the highest.
    for k in 0..n {
        let x = -w + 2.0 * w * k as f64 / (n - 1) as f64;
        let high = 0.55 * R + 0.3 * R * sin(PI * k as f64 / (n - 1) as f64);
        if k > 0 {
            shape.push(Point::new(x - w / (n - 1) as f64, low));
        }
        shape.push(Point::new(x, low - high));
    }
    a.fill(GOLD, &shape, 0.0);
    a.line(BROWN, &pts(&[(-w, base - 0.22 * R), (w, base - 0.22 * R)]), false);
    a.0
}

fn tree(v: &[f64]) -> Design {
    let n = v[0] as usize;
    let mut a = Art::new();
    a.fill(
        BROWN,
        &pts(&[(-0.12 * R, 0.6 * R), (0.12 * R, 0.6 * R), (0.12 * R, R), (-0.12 * R, R)]),
        0.0,
    );
    // The tiers from the bottom to the top: each one is smaller and laps
    // over the one below it.
    let high = 1.6 * R / n as f64;
    for k in 0..n {
        let base = 0.6 * R - high * k as f64;
        let w = R * (0.85 - 0.5 * k as f64 / n as f64);
        a.fill(GREEN, &pts(&[(-w, base), (w, base), (0.0, base - 1.5 * high)]), 0.0);
    }
    a.0
}

fn rosette(v: &[f64]) -> Design {
    let n = v[0];
    let mut a = Art::new();
    a.line(NAVY, &round(240, |t| R * (0.55 + 0.45 * cos(n * t))), true);
    a.0
}

/// The families of clipart, in menu order.
pub(super) static FAMILIES: &[Family] = &[
    Family {
        name: "Star",
        params: &[count("Points", 4, 8, 5), size("Inner part", 0.3, 0.7, 0.4)],
        make: Make::Clipart(star),
        presets: &[
            ("Six-point star", &[6.0, 0.5]),
            ("Eight-point star", &[8.0, 0.6]),
            ("Compass star", &[4.0, 0.3]),
        ],
        keywords: "star award shape",
    },
    Family {
        name: "Flower",
        params: &[count("Petals", 4, 8, 5), size("Middle", 0.2, 0.5, 0.35)],
        make: Make::Clipart(flower),
        presets: &[("Flower", &[5.0, 0.35]), ("Daisy", &[8.0, 0.3])],
        keywords: "flower blossom plant nature",
    },
    Family {
        name: "Sun",
        params: &[count("Rays", 8, 16, 12), size("Rays length", 0.2, 0.5, 0.35)],
        make: Make::Clipart(sun),
        presets: &[("Sun", &[12.0, 0.35]), ("Bright sun", &[16.0, 0.45])],
        keywords: "sun weather summer",
    },
    Family {
        name: "Snowflake",
        params: &[count("Arms", 5, 8, 6), count("Twigs", 0, 3, 2)],
        make: Make::Clipart(snowflake),
        presets: &[("Snowflake", &[6.0, 2.0]), ("Snow crystal", &[8.0, 1.0])],
        keywords: "snowflake snow winter christmas",
    },
    Family {
        name: "Seal",
        params: &[count("Scallops", 8, 20, 16), count("Ring", 0, 1, 1)],
        make: Make::Clipart(seal),
        presets: &[("Seal", &[16.0, 1.0]), ("Scalloped badge", &[10.0, 0.0])],
        keywords: "seal badge award rosette",
    },
    Family {
        name: "Polygon",
        params: &[count("Sides", 3, 8, 6)],
        make: Make::Clipart(polygon),
        presets: &[
            ("Triangle", &[3.0]),
            ("Pentagon", &[5.0]),
            ("Hexagon", &[6.0]),
            ("Octagon", &[8.0]),
        ],
        keywords: "polygon shape",
    },
    Family {
        name: "Gear",
        params: &[count("Teeth", 6, 14, 10), size("Hole", 0.2, 0.5, 0.35)],
        make: Make::Clipart(gear),
        presets: &[("Gear", &[10.0, 0.35]), ("Cog", &[8.0, 0.25])],
        keywords: "gear cog wheel machine",
    },
    Family {
        name: "Burst",
        params: &[count("Points", 8, 24, 16), size("Depth", 0.1, 0.4, 0.25)],
        make: Make::Clipart(burst),
        presets: &[("Burst", &[16.0, 0.25]), ("Starburst", &[24.0, 0.35])],
        keywords: "burst splash label",
    },
    Family {
        name: "Shield",
        params: &[size("Point", 0.2, 0.7, 0.4)],
        make: Make::Clipart(shield),
        presets: &[("Shield", &[0.4]), ("Pointed shield", &[0.6])],
        keywords: "shield crest badge",
    },
    Family {
        name: "Ribbon",
        params: &[size("Length (mm)", 60.0, 120.0, 90.0), size("Notch", 0.1, 0.4, 0.2)],
        make: Make::Clipart(ribbon),
        presets: &[("Ribbon", &[90.0, 0.2])],
        keywords: "ribbon banner scroll name",
    },
    Family {
        name: "Plus",
        params: &[size("Arm width", 0.15, 0.6, 0.35)],
        make: Make::Clipart(plus),
        presets: &[("Plus", &[0.35]), ("Cross", &[0.22])],
        keywords: "plus cross first aid",
    },
    Family {
        name: "Moon",
        params: &[size("Inner edge", 0.2, 0.85, 0.5)],
        make: Make::Clipart(moon),
        presets: &[("Moon", &[0.5]), ("Crescent", &[0.75])],
        keywords: "moon night crescent",
    },
    Family {
        name: "Cloud",
        params: &[count("Bumps", 3, 5, 4)],
        make: Make::Clipart(cloud),
        presets: &[("Cloud", &[4.0])],
        keywords: "cloud weather sky",
    },
    Family {
        name: "Drop",
        params: &[size("Width", 0.5, 1.0, 0.75)],
        make: Make::Clipart(drop),
        presets: &[("Drop", &[0.75])],
        keywords: "drop water rain",
    },
    Family {
        name: "Clover",
        params: &[count("Leaves", 3, 4, 3)],
        make: Make::Clipart(clover),
        presets: &[("Clover", &[3.0]), ("Four-leaf clover", &[4.0])],
        keywords: "clover shamrock luck plant",
    },
    Family {
        name: "Paw",
        params: &[count("Toes", 3, 5, 4)],
        make: Make::Clipart(paw),
        presets: &[("Paw", &[4.0])],
        keywords: "paw dog cat pet animal",
    },
    Family {
        name: "Crown",
        params: &[count("Points", 3, 5, 3)],
        make: Make::Clipart(crown),
        presets: &[("Crown", &[3.0]), ("Tall crown", &[5.0])],
        keywords: "crown king queen royal",
    },
    Family {
        name: "Tree",
        params: &[count("Tiers", 2, 4, 3)],
        make: Make::Clipart(tree),
        presets: &[("Tree", &[3.0]), ("Tall tree", &[4.0])],
        keywords: "tree fir christmas forest nature",
    },
    Family {
        name: "Rosette",
        params: &[count("Loops", 3, 12, 8)],
        make: Make::Clipart(rosette),
        presets: &[("Rosette", &[8.0]), ("Knot", &[5.0])],
        keywords: "rosette ornament line",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{Item, Kind, families_of};
    use emb_geom::BBox;

    #[test]
    fn each_clipart_item_is_a_small_design() {
        for f in families_of(Kind::Clipart) {
            assert!(!f.keywords.is_empty(), "{f:?}");
            let lo: Vec<f64> = f.params.iter().map(|p| p.min).collect();
            let hi: Vec<f64> = f.params.iter().map(|p| p.max).collect();
            for v in [f.defaults(), lo, hi] {
                let Item::Clipart(d) = f.item("x", &v) else { panic!() };
                assert!(!d.objects().is_empty(), "{f:?} {v:?}");
                assert_eq!(d.notes, f.keywords);
                let b = d
                    .objects()
                    .iter()
                    .filter_map(|o| o.kind.control_bbox())
                    .reduce(BBox::union)
                    .unwrap();
                // About 50 mm large (a ribbon is longer), round the middle.
                assert!(b.width().max(b.height()) >= 35.0, "{f:?} {v:?}: {b:?}");
                assert!(b.width() <= 125.0 && b.height() <= 70.0, "{f:?} {v:?}: {b:?}");
                assert!(
                    b.center().x.abs() < 6.0 && b.center().y.abs() < 12.0,
                    "{f:?} {v:?}: {b:?}"
                );
                for o in d.objects() {
                    assert!(d.thread(o.thread).is_some(), "{f:?} {v:?}");
                }
            }
        }
    }
}

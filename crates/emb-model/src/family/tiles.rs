//! The families of program-split patterns and of carving stamps
//! (`crate::split`).
//!
//! A split pattern is a tile of rows; each row has the places of the
//! needle points as parts (0 to 1) of the width of the tile. The first
//! parameter of each family is the width of the tile in mm.
//!
//! A stamp is a set of lines in the unit box, from -0.5 to 0.5.

use emb_geom::Point;
use emb_geom::math::{cos, sin};

use super::{Family, Make, PI, Param, TAU, count, size};
use crate::SplitPattern;

/// The rows of a tile for each mm of its width: at the row spacing of a
/// fill (0.4 mm) the tile is about 1.6 times as high as it is wide.
const ROWS_PER_MM: f64 = 4.0;

/// The longest stitch between two needle points of a row, in mm at scale
/// 1: a longer stitch in a fill is loose.
const MAX_STITCH_MM: f64 = 6.0;

fn rows_of(width_mm: f64) -> usize {
    ((width_mm * ROWS_PER_MM).round() as usize).clamp(8, 60)
}

/// A tile from the rule `f(row, t)`, with t from 0 to 1 down the tile. A
/// row with no point gets one point, at a place that is different in each
/// row, so the rows with no picture make no line.
fn tile(width_mm: f64, rows: usize, f: impl Fn(usize, f64) -> Vec<f64>) -> SplitPattern {
    SplitPattern {
        name: String::new(),
        width_mm,
        rows: (0..rows)
            .map(|r| {
                let mut v: Vec<f64> = f(r, r as f64 / rows as f64)
                    .into_iter()
                    .filter(|x| x.is_finite())
                    .map(|x| x.rem_euclid(1.0))
                    .collect();
                v.sort_by(f64::total_cmp);
                v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
                // 0 and a value just below 1 are the same place.
                if v.len() > 1 && v[v.len() - 1] > 1.0 - 1e-6 {
                    v.pop();
                }
                if v.is_empty() {
                    v.push((r as f64 * 0.382).fract());
                }
                // A stitch from one needle point to the next is not longer
                // than MAX_STITCH_MM: a longer one gets points between, a
                // little to the left or to the right in each row, so these
                // points make no line.
                let mut more = Vec::new();
                for k in 0..v.len() {
                    let (a, b) = (v[k], if k + 1 < v.len() { v[k + 1] } else { v[0] + 1.0 });
                    let parts = ((b - a) * width_mm / MAX_STITCH_MM).ceil() as usize;
                    let shift = ((r as f64 * 0.618).fract() - 0.5) * 0.3 / parts.max(1) as f64;
                    for j in 1..parts {
                        more.push((a + (b - a) * (j as f64 / parts as f64 + shift)).rem_euclid(1.0));
                    }
                }
                if !more.is_empty() {
                    v.extend(more);
                    v.sort_by(f64::total_cmp);
                    v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
                }
                v
            })
            .collect(),
    }
}

/// Points every `1 / n` of the tile: a row of short stitches, a line.
fn dense(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64).collect()
}

/// A tile with the outline of a closed shape in it. `shape`: the points of
/// the shape in the unit box (-0.5 to 0.5); `part`: the size of the shape
/// as a part of the tile. Each row has needle points where it crosses the
/// outline.
fn shape_tile(width_mm: f64, part: f64, shape: &[Point]) -> SplitPattern {
    // A square tile at the row spacing of a fill (0.4 mm), so a circle
    // is round.
    let rows = ((width_mm / 0.4).round() as usize).max(10);
    tile(width_mm, rows, |r, _| {
        // The row as Y in the unit box of the shape.
        let y = ((r as f64 + 0.5) / rows as f64 - 0.5) / part;
        let mut hits = Vec::new();
        for k in 0..shape.len() {
            let (a, b) = (shape[k], shape[(k + 1) % shape.len()]);
            if (a.y > y) != (b.y > y) {
                let x = a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
                hits.push(0.5 + x * part);
            }
        }
        hits
    })
}

/// A closed shape from its radius at each angle.
fn round_shape(n: usize, radius: impl Fn(f64) -> f64) -> Vec<Point> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let r = radius(a);
            Point::new(r * cos(a), r * sin(a))
        })
        .collect()
}

/// A star with `n` points: the outer radius 0.5, the inner radius
/// `0.5 * inner`, one point up.
fn star_shape(n: usize, inner: f64) -> Vec<Point> {
    (0..2 * n)
        .map(|k| {
            let a = -PI / 2.0 + PI * k as f64 / n as f64;
            let r = if k % 2 == 0 { 0.5 } else { 0.5 * inner };
            Point::new(r * cos(a), r * sin(a))
        })
        .collect()
}

/// A regular polygon with `n` sides in the circle of radius `r`, one corner up.
fn polygon_shape(n: usize, r: f64) -> Vec<Point> {
    (0..n)
        .map(|k| {
            let a = -PI / 2.0 + TAU * k as f64 / n as f64;
            Point::new(r * cos(a), r * sin(a))
        })
        .collect()
}

/// A heart in the unit box, its point down.
fn heart_shape(n: usize) -> Vec<Point> {
    (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let x = 16.0 * sin(a) * sin(a) * sin(a);
            let y = 13.0 * cos(a) - 5.0 * cos(2.0 * a) - 2.0 * cos(3.0 * a) - cos(4.0 * a);
            Point::new(x / 34.0, -y / 34.0 - 0.05)
        })
        .collect()
}

/// A flower with `n` petals; `depth` is how deep the gaps between them go.
fn flower_shape(n: f64, depth: f64) -> Vec<Point> {
    // One petal points up.
    round_shape(96, |a| 0.5 * (1.0 - 0.5 * depth * (1.0 - cos(n * (a + PI / 2.0)))))
}

fn closed(mut p: Vec<Point>) -> Vec<Point> {
    if let Some(first) = p.first().copied() {
        p.push(first);
    }
    p
}

// The split patterns.

fn bricks(v: &[f64]) -> SplitPattern {
    let rows = rows_of(v[0]);
    let (half, offset) = (rows / 2, v[1]);
    tile(v[0], rows, |r, _| {
        if r == 0 || r == half {
            dense(8)
        } else if r < half {
            vec![0.0]
        } else {
            vec![offset]
        }
    })
}

fn diamonds(v: &[f64]) -> SplitPattern {
    let n = v[1];
    tile(v[0], rows_of(v[0]), |_, t| {
        let d = 0.5 / n * (1.0 - (2.0 * t - 1.0).abs());
        (0..n as usize)
            .flat_map(|k| {
                let c = (k as f64 + 0.5) / n;
                [c - d, c + d]
            })
            .collect()
    })
}

fn waves(v: &[f64]) -> SplitPattern {
    let (n, a) = (v[1], v[2]);
    tile(v[0], rows_of(v[0]), |_, t| {
        (0..n as usize)
            .map(|k| (k as f64 + 0.5 + a * sin(TAU * t)) / n)
            .collect()
    })
}

fn zigzags(v: &[f64]) -> SplitPattern {
    let n = v[1];
    tile(v[0], rows_of(v[0]), |_, t| {
        (0..n as usize)
            .map(|k| (k as f64 + 0.999 * (1.0 - (2.0 * t - 1.0).abs())) / n)
            .collect()
    })
}

fn herringbone(v: &[f64]) -> SplitPattern {
    let n = v[1];
    tile(v[0], rows_of(v[0]), |_, t| {
        (0..n as usize)
            .flat_map(|k| [(k as f64 + 0.5 * t) / n, (k as f64 + 1.0 - 0.5 * t) / n])
            .collect()
    })
}

fn checkers(v: &[f64]) -> SplitPattern {
    let (rows, n) = (rows_of(v[0]), v[1] as usize);
    let cell = 0.5 / n as f64;
    tile(v[0], rows, |r, _| {
        let odd = usize::from(r >= rows / 2);
        (0..n)
            .flat_map(|k| {
                // A square of short stitches, and the edge of the square
                // next to it.
                let x0 = (2 * k + odd) as f64 * cell;
                (0..=4).map(move |i| x0 + cell * f64::from(i) / 4.0)
            })
            .collect()
    })
}

fn scales(v: &[f64]) -> SplitPattern {
    let n = v[1];
    tile(v[0], rows_of(v[0]), |_, t| {
        let half = 0.5 / n * (1.0 - t * t).max(0.0).sqrt();
        (0..n as usize)
            .flat_map(|k| {
                let c = (k as f64 + 0.5) / n;
                [c - half, c + half]
            })
            .collect()
    })
}

fn basket(v: &[f64]) -> SplitPattern {
    let (rows, n) = (rows_of(v[0]), v[1]);
    tile(v[0], rows, |r, _| {
        let shift = if r < rows / 2 { 0.0 } else { 0.25 };
        (0..2 * n as usize).map(|k| (k as f64 * 0.5 + shift) / n).collect()
    })
}

fn grid(v: &[f64]) -> SplitPattern {
    let n = v[1] as usize;
    tile(v[0], rows_of(v[0]), |r, _| if r == 0 { dense(6 * n) } else { dense(n) })
}

fn stripes(v: &[f64]) -> SplitPattern {
    let n = v[1] as usize;
    tile(v[0], 10, |_, _| dense(n))
}

fn diagonals(v: &[f64]) -> SplitPattern {
    let (n, back) = (v[1], v[2] >= 0.5);
    tile(v[0], rows_of(v[0]), |_, t| {
        let shift = if back { -t } else { t };
        (0..n as usize).map(|k| (k as f64 + shift) / n).collect()
    })
}

fn lattice(v: &[f64]) -> SplitPattern {
    let n = v[1];
    tile(v[0], rows_of(v[0]), |_, t| {
        (0..n as usize)
            .flat_map(|k| [(k as f64 + t) / n, (k as f64 - t) / n])
            .collect()
    })
}

fn stairs(v: &[f64]) -> SplitPattern {
    let (n, steps) = (v[1], v[2]);
    tile(v[0], rows_of(v[0]), |_, t| {
        let shift = (t * steps).floor() / steps;
        (0..n as usize).map(|k| (k as f64 + shift) / n).collect()
    })
}

fn circles(v: &[f64]) -> SplitPattern {
    shape_tile(v[0], v[1], &round_shape(64, |_| 0.5))
}

fn polygons(v: &[f64]) -> SplitPattern {
    shape_tile(v[0], v[2], &polygon_shape(v[1] as usize, 0.5))
}

fn stars(v: &[f64]) -> SplitPattern {
    shape_tile(v[0], v[2], &star_shape(v[1] as usize, 0.45))
}

fn hearts(v: &[f64]) -> SplitPattern {
    shape_tile(v[0], v[1], &heart_shape(64))
}

fn flowers(v: &[f64]) -> SplitPattern {
    shape_tile(v[0], v[2], &flower_shape(v[1], 0.6))
}

const WIDTH: Param = size("Tile width (mm)", 2.0, 10.0, 4.0);
const LINES: Param = count("Lines", 1, 3, 1);
const PART: Param = size("Size", 0.4, 0.95, 0.8);

/// The families of split patterns, in menu order.
pub(super) static SPLITS: &[Family] = &[
    Family {
        name: "Bricks",
        params: &[WIDTH, size("Offset", 0.0, 0.5, 0.5)],
        make: Make::Split(bricks),
        presets: &[("Long brick", &[8.0, 0.5]), ("Third brick", &[6.0, 0.33])],
        keywords: "",
    },
    Family {
        name: "Diamonds",
        params: &[WIDTH, LINES],
        make: Make::Split(diamonds),
        presets: &[("Small diamonds", &[4.0, 2.0]), ("Large diamond", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Waves",
        params: &[WIDTH, LINES, size("Height", 0.1, 0.45, 0.35)],
        make: Make::Split(waves),
        presets: &[
            ("Double wave", &[6.0, 2.0, 0.3]),
            ("Gentle wave", &[5.0, 1.0, 0.15]),
            ("Triple wave", &[8.0, 3.0, 0.25]),
        ],
        keywords: "",
    },
    Family {
        name: "Zigzags",
        params: &[WIDTH, LINES],
        make: Make::Split(zigzags),
        presets: &[("Double zigzag", &[6.0, 2.0]), ("Wide zigzag", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Herringbones",
        params: &[WIDTH, count("Lines", 1, 2, 1)],
        make: Make::Split(herringbone),
        presets: &[("Fine herringbone", &[4.0, 2.0]), ("Wide herringbone", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Checkers",
        params: &[WIDTH, count("Squares", 1, 3, 1)],
        make: Make::Split(checkers),
        presets: &[("Small checker", &[4.0, 2.0]), ("Large checker", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Fish scales",
        params: &[WIDTH, count("Scales", 1, 2, 1)],
        make: Make::Split(scales),
        presets: &[("Small scales", &[4.0, 2.0]), ("Large scales", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Baskets",
        params: &[WIDTH, count("Bands", 1, 3, 1)],
        make: Make::Split(basket),
        presets: &[("Fine basket", &[6.0, 2.0]), ("Wide basket", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Grids",
        params: &[WIDTH, LINES],
        make: Make::Split(grid),
        presets: &[("Fine grid", &[4.0, 2.0]), ("Wide grid", &[8.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Stripes",
        params: &[WIDTH, count("Lines", 1, 4, 1)],
        make: Make::Split(stripes),
        presets: &[("Stripes", &[4.0, 1.0]), ("Pinstripes", &[4.0, 2.0])],
        keywords: "",
    },
    Family {
        name: "Diagonals",
        params: &[WIDTH, LINES, count("To the left", 0, 1, 0)],
        make: Make::Split(diagonals),
        presets: &[
            ("Diagonal", &[4.0, 1.0, 0.0]),
            ("Back diagonal", &[4.0, 1.0, 1.0]),
            ("Fine diagonal", &[4.0, 2.0, 0.0]),
        ],
        keywords: "",
    },
    Family {
        name: "Lattices",
        params: &[WIDTH, count("Lines", 1, 2, 1)],
        make: Make::Split(lattice),
        presets: &[("Lattice", &[5.0, 1.0]), ("Fine lattice", &[6.0, 2.0])],
        keywords: "",
    },
    Family {
        name: "Stairs",
        params: &[WIDTH, count("Lines", 1, 2, 1), count("Steps", 2, 6, 4)],
        make: Make::Split(stairs),
        presets: &[("Stairs", &[5.0, 1.0, 4.0]), ("Fine stairs", &[5.0, 2.0, 3.0])],
        keywords: "",
    },
    Family {
        name: "Circles",
        params: &[WIDTH, PART],
        make: Make::Split(circles),
        presets: &[("Circles", &[5.0, 0.7]), ("Dots", &[3.0, 0.5])],
        keywords: "",
    },
    Family {
        name: "Polygons",
        params: &[WIDTH, count("Sides", 3, 8, 6), PART],
        make: Make::Split(polygons),
        presets: &[
            ("Hexagons", &[5.0, 6.0, 0.8]),
            ("Triangles", &[5.0, 3.0, 0.8]),
            ("Octagons", &[6.0, 8.0, 0.8]),
        ],
        keywords: "",
    },
    Family {
        name: "Stars",
        params: &[WIDTH, count("Points", 4, 8, 5), PART],
        make: Make::Split(stars),
        presets: &[("Stars", &[6.0, 5.0, 0.85]), ("Six-point stars", &[6.0, 6.0, 0.85])],
        keywords: "",
    },
    Family {
        name: "Hearts",
        params: &[WIDTH, PART],
        make: Make::Split(hearts),
        presets: &[("Hearts", &[6.0, 0.8])],
        keywords: "",
    },
    Family {
        name: "Flowers",
        params: &[WIDTH, count("Petals", 3, 8, 5), PART],
        make: Make::Split(flowers),
        presets: &[("Flowers", &[6.0, 5.0, 0.85]), ("Daisies", &[7.0, 8.0, 0.85])],
        keywords: "",
    },
];

// The stamps.

type Lines = Vec<Vec<Point>>;

fn line(a: (f64, f64), b: (f64, f64)) -> Vec<Point> {
    vec![Point::new(a.0, a.1), Point::new(b.0, b.1)]
}

fn scaled(p: Vec<Point>, s: f64) -> Vec<Point> {
    p.into_iter().map(|q| Point::new(q.x * s, q.y * s)).collect()
}

fn rings(v: &[f64]) -> Lines {
    let n = v[0] as usize;
    (1..=n)
        .map(|k| closed(round_shape(48, |_| 0.5 * k as f64 / n as f64)))
        .collect()
}

fn star(v: &[f64]) -> Lines {
    vec![closed(star_shape(v[0] as usize, v[1]))]
}

fn polygon(v: &[f64]) -> Lines {
    let (n, nested) = (v[0] as usize, v[1] as usize);
    (0..nested)
        .map(|k| closed(polygon_shape(n, 0.5 * (nested - k) as f64 / nested as f64)))
        .collect()
}

fn rose(v: &[f64]) -> Lines {
    vec![closed(flower_shape(v[0], v[1]))]
}

fn spiral(v: &[f64]) -> Lines {
    let turns = v[0];
    let n = (24.0 * turns) as usize;
    vec![
        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                let (a, r) = (TAU * turns * t, 0.5 * t);
                Point::new(r * cos(a), r * sin(a))
            })
            .collect(),
    ]
}

fn wave_lines(v: &[f64]) -> Lines {
    let (n, waves, h) = (v[0] as usize, v[1], v[2]);
    (0..n)
        .map(|k| {
            // The lines are in the box with their waves.
            let y0 = (0.5 - h) * (2.0 * (k as f64 + 0.5) / n as f64 - 1.0);
            (0..=32)
                .map(|i| {
                    let t = f64::from(i) / 32.0;
                    Point::new(t - 0.5, y0 + h * sin(TAU * waves * t))
                })
                .collect()
        })
        .collect()
}

fn sunburst(v: &[f64]) -> Lines {
    let (n, inner) = (v[0] as usize, v[1]);
    let mut lines: Lines = (0..n)
        .map(|k| {
            let a = TAU * k as f64 / n as f64;
            let (c, s) = (cos(a), sin(a));
            vec![
                Point::new(0.5 * inner * c, 0.5 * inner * s),
                Point::new(0.5 * c, 0.5 * s),
            ]
        })
        .collect();
    lines.push(closed(round_shape(32, |_| 0.5 * inner)));
    lines
}

fn hatch(v: &[f64]) -> Lines {
    let (n, crossed) = (v[0] as usize, v[1] >= 0.5);
    let mut lines = Lines::new();
    for k in 0..n {
        let at = (k as f64 + 0.5) / n as f64 - 0.5;
        lines.push(line((-0.5, at), (0.5, at)));
        if crossed {
            lines.push(line((at, -0.5), (at, 0.5)));
        }
    }
    lines
}

fn snowflake(v: &[f64]) -> Lines {
    let (n, twigs) = (v[0] as usize, v[1] as usize);
    let mut lines = Lines::new();
    for k in 0..n {
        let a = -PI / 2.0 + TAU * k as f64 / n as f64;
        let at = |r: f64, turn: f64| Point::new(r * cos(a + turn), r * sin(a + turn));
        lines.push(vec![Point::new(0.0, 0.0), at(0.5, 0.0)]);
        for t in 1..=twigs {
            // A twig on each side of the arm.
            let r = 0.5 * t as f64 / (twigs + 1) as f64;
            let tip = r + 0.12;
            let spread = 0.1 / tip;
            lines.push(vec![at(tip, -spread), at(r, 0.0), at(tip, spread)]);
        }
    }
    lines
}

fn squares(v: &[f64]) -> Lines {
    let (nested, turned) = (v[0] as usize, v[1] >= 0.5);
    (0..nested)
        .map(|k| {
            let r = 0.5 * (nested - k) as f64 / nested as f64;
            let corners = if turned {
                vec![(0.0, -r), (r, 0.0), (0.0, r), (-r, 0.0)]
            } else {
                vec![(-r, -r), (r, -r), (r, r), (-r, r)]
            };
            closed(corners.into_iter().map(|(x, y)| Point::new(x, y)).collect())
        })
        .collect()
}

fn heart(v: &[f64]) -> Lines {
    let nested = v[0] as usize;
    (0..nested)
        .map(|k| {
            let s = (nested - k) as f64 / nested as f64;
            // A smaller heart stays at the middle of the larger one.
            closed(scaled(heart_shape(48), s))
        })
        .collect()
}

fn leaf(v: &[f64]) -> Lines {
    let veins = v[0] as usize;
    let edge = |x: f64| 0.24 * sin(PI * (x + 0.5));
    let mut lines = vec![
        closed(
            (0..40)
                .map(|k| {
                    let a = TAU * f64::from(k) / 40.0;
                    Point::new(0.5 * cos(a), 0.24 * sin(a) * sin(a).abs().sqrt())
                })
                .collect(),
        ),
        line((-0.5, 0.0), (0.5, 0.0)),
    ];
    for k in 1..=veins {
        let x = k as f64 / (veins + 1) as f64 - 0.5;
        let tip = 0.6 * edge(x + 0.1);
        lines.push(vec![
            Point::new(x + 0.1, -tip),
            Point::new(x, 0.0),
            Point::new(x + 0.1, tip),
        ]);
    }
    lines
}

fn clover(v: &[f64]) -> Lines {
    let n = v[0] as usize;
    let r = 0.5 / (1.0 + 1.0 / sin(PI / n as f64)).min(2.2);
    (0..n)
        .map(|k| {
            let a = -PI / 2.0 + TAU * k as f64 / n as f64;
            let (cx, cy) = ((0.5 - r) * cos(a), (0.5 - r) * sin(a));
            closed(
                round_shape(32, |_| r)
                    .into_iter()
                    .map(|p| Point::new(p.x + cx, p.y + cy))
                    .collect(),
            )
        })
        .collect()
}

fn moon(v: &[f64]) -> Lines {
    // The outer edge is a circle; the inner edge is an arc through its
    // two ends that goes `part` of the way to the other side.
    let part = v[0];
    let mut p: Vec<Point> = (0..=24)
        .map(|k| {
            let a = PI / 2.0 + PI * f64::from(k) / 24.0;
            Point::new(0.5 * cos(a), 0.5 * sin(a))
        })
        .collect();
    p.extend((1..24).map(|k| {
        let a = -PI / 2.0 + PI * f64::from(k) / 24.0;
        Point::new(-0.5 * part * cos(a), 0.5 * sin(a))
    }));
    vec![closed(p)]
}

fn drop(v: &[f64]) -> Lines {
    let w = v[0];
    vec![closed(
        (0..48)
            .map(|k| {
                let a = TAU * f64::from(k) / 48.0;
                // A point at the top, round at the bottom.
                Point::new(0.5 * w * sin(a) * sin(a / 2.0) * sin(a / 2.0) / 0.65, -0.5 * cos(a))
            })
            .collect(),
    )]
}

/// The families of stamps, in menu order.
pub(super) static STAMPS: &[Family] = &[
    Family {
        name: "Rings",
        params: &[count("Rings", 1, 4, 2)],
        make: Make::Stamp(rings),
        presets: &[("Rings", &[2.0]), ("Target", &[4.0])],
        keywords: "",
    },
    Family {
        name: "Star",
        params: &[count("Points", 4, 10, 5), size("Inner part", 0.3, 0.7, 0.4)],
        make: Make::Stamp(star),
        presets: &[
            ("Six-point star", &[6.0, 0.5]),
            ("Eight-point star", &[8.0, 0.6]),
            ("Compass", &[4.0, 0.3]),
        ],
        keywords: "",
    },
    Family {
        name: "Polygon",
        params: &[count("Sides", 3, 8, 6), count("Nested", 1, 3, 1)],
        make: Make::Stamp(polygon),
        presets: &[
            ("Triangle", &[3.0, 1.0]),
            ("Hexagon", &[6.0, 1.0]),
            ("Nested hexagons", &[6.0, 3.0]),
            ("Octagon", &[8.0, 1.0]),
        ],
        keywords: "",
    },
    Family {
        name: "Rose",
        params: &[count("Petals", 3, 8, 5), size("Depth", 0.2, 0.8, 0.5)],
        make: Make::Stamp(rose),
        presets: &[
            ("Daisy", &[8.0, 0.6]),
            ("Clover rose", &[3.0, 0.7]),
            ("Soft flower", &[6.0, 0.3]),
        ],
        keywords: "",
    },
    Family {
        name: "Spiral",
        params: &[size("Turns", 1.0, 4.0, 2.5)],
        make: Make::Stamp(spiral),
        presets: &[("Spiral", &[3.0]), ("Tight spiral", &[4.0])],
        keywords: "",
    },
    Family {
        name: "Wave lines",
        params: &[
            count("Lines", 1, 5, 3),
            count("Waves", 1, 4, 2),
            size("Height", 0.02, 0.15, 0.08),
        ],
        make: Make::Stamp(wave_lines),
        presets: &[("Ripples", &[5.0, 2.0, 0.05]), ("Big waves", &[2.0, 1.0, 0.12])],
        keywords: "",
    },
    Family {
        name: "Sunburst",
        params: &[count("Rays", 6, 16, 12), size("Middle", 0.1, 0.5, 0.3)],
        make: Make::Stamp(sunburst),
        presets: &[("Sunburst", &[12.0, 0.3]), ("Rays", &[8.0, 0.15])],
        keywords: "",
    },
    Family {
        name: "Hatch",
        params: &[count("Lines", 2, 6, 4), count("Crossed", 0, 1, 0)],
        make: Make::Stamp(hatch),
        presets: &[("Lines", &[4.0, 0.0]), ("Crosshatch", &[4.0, 1.0])],
        keywords: "",
    },
    Family {
        name: "Snowflake",
        params: &[count("Arms", 4, 8, 6), count("Twigs", 0, 2, 2)],
        make: Make::Stamp(snowflake),
        presets: &[("Snowflake", &[6.0, 2.0]), ("Asterisk", &[8.0, 0.0])],
        keywords: "",
    },
    Family {
        name: "Squares",
        params: &[count("Nested", 1, 4, 1), count("Turned", 0, 1, 0)],
        make: Make::Stamp(squares),
        presets: &[
            ("Square", &[1.0, 0.0]),
            ("Nested squares", &[3.0, 0.0]),
            ("Nested diamonds", &[3.0, 1.0]),
        ],
        keywords: "",
    },
    Family {
        name: "Hearts",
        params: &[count("Nested", 1, 3, 2)],
        make: Make::Stamp(heart),
        presets: &[("Double heart", &[2.0])],
        keywords: "",
    },
    Family {
        name: "Leaf",
        params: &[count("Veins", 0, 4, 3)],
        make: Make::Stamp(leaf),
        presets: &[("Veined leaf", &[3.0])],
        keywords: "",
    },
    Family {
        name: "Clover",
        params: &[count("Leaves", 3, 5, 3)],
        make: Make::Stamp(clover),
        presets: &[("Clover", &[3.0]), ("Four-leaf clover", &[4.0])],
        keywords: "",
    },
    Family {
        name: "Moon",
        params: &[size("Inner edge", 0.2, 0.9, 0.5)],
        make: Make::Stamp(moon),
        presets: &[("Moon", &[0.6])],
        keywords: "",
    },
    Family {
        name: "Drop",
        params: &[size("Width", 0.5, 1.0, 0.8)],
        make: Make::Stamp(drop),
        presets: &[("Drop", &[0.7])],
        keywords: "",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{Item, Kind, families_of};

    fn corners(f: &super::Family) -> Vec<Vec<f64>> {
        vec![
            f.defaults(),
            f.params.iter().map(|p| p.min).collect(),
            f.params.iter().map(|p| p.max).collect(),
        ]
    }

    #[test]
    fn each_split_pattern_is_well_formed() {
        for f in families_of(Kind::Split) {
            for v in corners(f) {
                let Item::Split(p) = f.item("x", &v) else { panic!() };
                assert!((p.width_mm - v[0]).abs() < 1e-9 && p.rows.len() >= 8, "{f:?} {v:?}");
                let mut points = 0;
                for r in &p.rows {
                    assert!(!r.is_empty(), "{f:?} {v:?}");
                    assert!(r.iter().all(|x| (0.0..1.0).contains(x)), "{f:?} {v:?}: {r:?}");
                    assert!(r.windows(2).all(|w| w[0] < w[1]), "{f:?} {v:?}: {r:?}");
                    points += r.len();
                }
                // No stitch of a row is longer than 6 mm (and a little,
                // for the points that the rule put between).
                for r in &p.rows {
                    for k in 0..r.len() {
                        let next = if k + 1 < r.len() { r[k + 1] } else { r[0] + 1.0 };
                        assert!((next - r[k]) * p.width_mm <= 6.0 * 1.2, "{f:?} {v:?}: {r:?}");
                    }
                }
                // A picture, not a line of points in each row and not a
                // dense field: the needle points are at most 0.3 mm apart
                // on the average only in a pattern of short stitches.
                assert!(points <= p.rows.len() * 26, "{f:?} {v:?}: {points} points");
            }
        }
    }

    #[test]
    fn a_shape_tile_has_the_outline_of_its_shape() {
        let Item::Split(p) = families_of(Kind::Split)
            .into_iter()
            .find(|f| f.name == "Circles")
            .unwrap()
            .item("x", &[5.0, 0.8])
        else {
            panic!()
        };
        // A row through the middle of the circle crosses it 0.8 apart; the
        // first and the last row do not cross it.
        let mid = &p.rows[p.rows.len() / 2];
        assert_eq!(mid.len(), 2);
        assert!((mid[1] - mid[0] - 0.8).abs() < 0.02, "{mid:?}");
        assert_eq!(p.rows[0].len(), 1);
        assert_ne!(p.rows[0], p.rows[1], "the rows with no picture make no line");
    }

    #[test]
    fn each_stamp_is_in_the_unit_box() {
        for f in families_of(Kind::Stamp) {
            for v in corners(f) {
                let Item::Stamp(s) = f.item("x", &v) else { panic!() };
                assert!(!s.lines.is_empty(), "{f:?} {v:?}");
                let (mut w, mut h) = (0.0f64, 0.0f64);
                for l in &s.lines {
                    assert!(l.len() >= 2, "{f:?} {v:?}");
                    for p in l {
                        assert!(p.x.abs() <= 0.5 + 1e-9 && p.y.abs() <= 0.5 + 1e-9, "{f:?} {v:?}: {p:?}");
                        w = w.max(p.x.abs());
                        h = h.max(p.y.abs());
                    }
                }
                // The stamp uses the box.
                assert!(w.max(h) > 0.4, "{f:?} {v:?}: {w} {h}");
            }
        }
    }
}

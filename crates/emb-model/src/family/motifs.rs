//! The families of motifs. A motif is one line of needle points from
//! (0, 0) to (1, 0); Y is across the line, and a negative Y is above it.
//! A family with a count makes that many cells along the line.

use emb_geom::math::{cos, sin};

use super::{Family, Make, PI, Param, TAU, count, curve, size};

type Points = Vec<(f64, f64)>;

/// The points of `n` cells from (0, 0) to (1, 0): `cell(x0, w)` gives the
/// points of the cell that starts at `x0` and is `w` long, after its first
/// point (x0, 0) and with its last point (x0 + w, 0).
fn cells(n: f64, cell: impl Fn(f64, f64) -> Points) -> Points {
    let n = n.max(1.0) as usize;
    let w = 1.0 / n as f64;
    let mut out = vec![(0.0, 0.0)];
    for k in 0..n {
        out.extend(cell(k as f64 * w, w));
    }
    out
}

/// A shape on a stem: from (0, 0) along the line to (0.5, 0), the points of
/// the shape, back to (0.5, 0), and on to (1, 0).
fn on_stem(shape: Points) -> Points {
    let mut out = vec![(0.0, 0.0), (0.5, 0.0)];
    out.extend(shape);
    out.extend([(0.5, 0.0), (1.0, 0.0)]);
    out
}

fn waves(v: &[f64]) -> Points {
    let (n, h) = (v[0], v[1]);
    curve((12.0 * n) as usize, |t| (t, -h * sin(TAU * n * t)))
}

fn zigzag(v: &[f64]) -> Points {
    let (h, both) = (v[1], v[2] >= 0.5);
    cells(v[0], |x, w| {
        if both {
            vec![(x + 0.25 * w, -h), (x + 0.75 * w, h), (x + w, 0.0)]
        } else {
            vec![(x + 0.5 * w, -h), (x + w, 0.0)]
        }
    })
}

fn scallops(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        curve(10, |t| (x + w * (0.5 - 0.5 * cos(PI * t)), -h * sin(PI * t)))
            .into_iter()
            .skip(1)
            .collect()
    })
}

fn loops(v: &[f64]) -> Points {
    let (n, s) = (v[0], v[1]);
    curve((16.0 * n) as usize, |t| {
        // The line goes back at the top of each loop.
        (t + s / n * sin(TAU * n * t), -s / n * (1.0 - cos(TAU * n * t)))
    })
}

fn steps(v: &[f64]) -> Points {
    let (h, top) = (v[1], v[2]);
    cells(v[0], |x, w| {
        let a = 0.5 * w * (1.0 - top);
        vec![
            (x + a, 0.0),
            (x + a, -h),
            (x + w - a, -h),
            (x + w - a, 0.0),
            (x + w, 0.0),
        ]
    })
}

fn diamonds(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        vec![
            (x + 0.5 * w, -h),
            (x + w, 0.0),
            (x + 0.5 * w, h),
            (x, 0.0),
            (x + w, 0.0),
        ]
    })
}

fn triangles(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        vec![(x + w, 0.0), (x + 0.5 * w, -h), (x, 0.0), (x + w, 0.0)]
    })
}

fn beads(v: &[f64]) -> Points {
    let (part, h) = (v[1], v[2]);
    cells(v[0], |x, w| {
        let (c, r) = (x + 0.5 * w, 0.5 * w * part);
        let mut p = curve(16, |t| (c - r * cos(TAU * t), -h * sin(TAU * t)));
        p.push((x + w, 0.0));
        p
    })
}

fn star(v: &[f64]) -> Points {
    let (n, inner, r) = (v[0] as usize, v[1], v[2]);
    on_stem(
        (0..=2 * n)
            .map(|k| {
                let a = PI / 2.0 + PI * k as f64 / n as f64;
                let rad = if k % 2 == 0 { r } else { r * inner };
                (0.5 + rad * cos(a), -r + rad * sin(a))
            })
            .collect(),
    )
}

fn flower(v: &[f64]) -> Points {
    let (n, r) = (v[0], v[1]);
    // The stem goes to the middle of the flower; each petal is a loop
    // from the middle and back. One petal points up.
    let mut p = vec![(0.5, -r)];
    p.extend(curve((10.0 * n) as usize, |t| {
        let a = TAU * t;
        let rad = r * sin(0.5 * n * a).abs();
        let turned = a - PI / n - PI / 2.0;
        (0.5 + rad * cos(turned), -r + rad * sin(turned))
    }));
    p.push((0.5, -r));
    on_stem(p)
}

fn leaf(v: &[f64]) -> Points {
    let (h, veins) = (v[0], v[1] as usize);
    // The upper edge, the lower edge back, and the middle vein with its
    // side veins.
    let mut p = curve(10, |t| (t, -h * sin(PI * t)));
    p.extend(curve(10, |t| (1.0 - t, h * sin(PI * t))).into_iter().skip(1));
    for k in 1..=veins {
        let x = k as f64 / (veins + 1) as f64;
        let tip = 0.7 * h * sin(PI * (x + 0.08));
        p.extend([(x, 0.0), (x + 0.08, -tip), (x, 0.0), (x + 0.08, tip), (x, 0.0)]);
    }
    p.push((1.0, 0.0));
    p
}

fn key(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        vec![
            (x, -h),
            (x + 0.7 * w, -h),
            (x + 0.7 * w, -0.4 * h),
            (x + 0.35 * w, -0.4 * h),
            (x + 0.7 * w, -0.4 * h),
            (x + 0.7 * w, 0.0),
            (x + w, 0.0),
        ]
    })
}

fn arches(v: &[f64]) -> Points {
    let (h, point) = (v[1], v[2]);
    cells(v[0], |x, w| {
        curve(12, |t| {
            // 0 at the two feet of the arch, 1 at its top.
            let s = 1.0 - (2.0 * t - 1.0).abs();
            let round = (1.0 - (1.0 - s) * (1.0 - s)).max(0.0).sqrt();
            (x + w * t, -h * ((1.0 - point) * round + point * s))
        })
        .into_iter()
        .skip(1)
        .collect()
    })
}

fn spiral(v: &[f64]) -> Points {
    let (turns, r) = (v[0], v[1]);
    // From the bottom of the spiral in to its middle.
    on_stem(curve((16.0 * turns) as usize, |t| {
        let (a, rad) = (PI / 2.0 + TAU * turns * t, r * (1.0 - t));
        (0.5 + rad * cos(a), -r + rad * sin(a))
    }))
}

fn feather(v: &[f64]) -> Points {
    let (n, h, slant) = (v[0] as usize, v[1], v[2]);
    let w = 1.0 / n as f64;
    let mut p = vec![(0.0, 0.0)];
    for k in 0..n {
        let x = (k as f64 + 0.3) * w;
        let side = if k % 2 == 0 { -h } else { h };
        p.extend([(x, 0.0), (x + slant * w, side), (x, 0.0)]);
    }
    p.push((1.0, 0.0));
    p
}

fn crosses(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        let (a, b) = (x + 0.15 * w, x + 0.85 * w);
        vec![(a, 0.0), (a, -h), (b, h), (b, -h), (a, h), (a, 0.0), (x + w, 0.0)]
    })
}

fn hearts(v: &[f64]) -> Points {
    let s = v[1];
    cells(v[0], |x, w| {
        // The point of the heart is on the line, in the middle of the cell.
        let mut p = vec![(x + 0.5 * w, 0.0)];
        p.extend(curve(24, |t| {
            let a = TAU * (t + 0.5);
            let hx = 16.0 * sin(a) * sin(a) * sin(a);
            let hy = 13.0 * cos(a) - 5.0 * cos(2.0 * a) - 2.0 * cos(3.0 * a) - cos(4.0 * a);
            (x + 0.5 * w + s * w * hx / 17.0, -s * w * (hy + 17.0) / 17.0)
        }));
        p.extend([(x + 0.5 * w, 0.0), (x + w, 0.0)]);
        p
    })
}

fn comb(v: &[f64]) -> Points {
    let (n, h, both) = (v[0] as usize, v[1], v[2] >= 0.5);
    let w = 1.0 / n as f64;
    let mut p = vec![(0.0, 0.0)];
    for k in 0..n {
        let x = (k as f64 + 0.5) * w;
        p.extend([(x, 0.0), (x, -h), (x, 0.0)]);
        if both {
            p.extend([(x, h), (x, 0.0)]);
        }
    }
    p.push((1.0, 0.0));
    p
}

fn vine(v: &[f64]) -> Points {
    let (n, s) = (v[0] as usize, v[1]);
    let w = 1.0 / n as f64;
    let mut p = vec![(0.0, 0.0)];
    for k in 0..n {
        let x = (k as f64 + 0.3) * w;
        let up = if k % 2 == 0 { -1.0 } else { 1.0 };
        // A small leaf: out along one edge and back along the other.
        let tip = (x + 0.6 * w, up * s);
        p.extend([
            (x, 0.0),
            (x + 0.1 * w, up * 0.6 * s),
            tip,
            (x + 0.5 * w, up * 0.3 * s),
            (x, 0.0),
        ]);
    }
    p.push((1.0, 0.0));
    p
}

fn fan(v: &[f64]) -> Points {
    let (n, r) = (v[0] as usize, v[1]);
    let mut p = Vec::new();
    for k in 0..n {
        let a = PI * k as f64 / (n - 1) as f64;
        p.extend([(0.5 - r * cos(a), -r * sin(a)), (0.5, 0.0)]);
    }
    // The edge of the fan, from its right end back over the top.
    p.extend(curve(12, |t| (0.5 + r * cos(PI * t), -r * sin(PI * t))));
    on_stem(p)
}

fn asterisk(v: &[f64]) -> Points {
    let (n, r) = (v[0] as usize, v[1]);
    let mut p = vec![(0.5, -r)];
    for k in 0..n {
        let a = PI / 2.0 + TAU * k as f64 / n as f64;
        p.extend([(0.5 + r * cos(a), -r + r * sin(a)), (0.5, -r)]);
    }
    on_stem(p)
}

fn boxes(v: &[f64]) -> Points {
    let part = v[1];
    cells(v[0], |x, w| {
        let (c, a) = (x + 0.5 * w, 0.5 * w * part);
        vec![
            (c - a, 0.0),
            (c - a, -a),
            (c + a, -a),
            (c + a, a),
            (c - a, a),
            (c - a, 0.0),
            (x + w, 0.0),
        ]
    })
}

fn polygon(v: &[f64]) -> Points {
    let (n, r) = (v[0] as usize, v[1]);
    on_stem(
        (0..=n)
            .map(|k| {
                let a = PI / 2.0 + TAU * k as f64 / n as f64;
                (0.5 + r * cos(a), -r + r * sin(a))
            })
            .collect(),
    )
}

fn arrows(v: &[f64]) -> Points {
    let h = v[1];
    cells(v[0], |x, w| {
        let (tip, back) = (x + 0.6 * w, x + 0.35 * w);
        vec![(tip, 0.0), (back, -h), (tip, 0.0), (back, h), (tip, 0.0), (x + w, 0.0)]
    })
}

fn stairs(v: &[f64]) -> Points {
    let (n, h) = (v[0] as usize, v[1]);
    // `n` steps up to the middle and `n` steps down.
    let (dx, dy) = (0.5 / n as f64, h / n as f64);
    let mut p = vec![(0.0, 0.0)];
    for k in 1..=n {
        let (x, y) = (dx * k as f64, -dy * k as f64);
        p.extend([(x - dx, y), (x, y)]);
    }
    for k in 1..=n {
        let (x, y) = (0.5 + dx * k as f64, -h + dy * k as f64);
        p.extend([(x, y - dy), (x, y)]);
    }
    p
}

const COUNT: Param = count("Count", 1, 4, 1);
const HEIGHT: Param = size("Height", 0.1, 0.6, 0.35);
const BOTH: Param = count("Both sides", 0, 1, 0);

/// The families of motifs, in menu order.
pub(super) static FAMILIES: &[Family] = &[
    Family {
        name: "Waves",
        params: &[COUNT, HEIGHT],
        make: Make::Motif(waves),
        presets: &[
            ("Double wave", &[2.0, 0.3]),
            ("Low wave", &[1.0, 0.15]),
            ("Ripples", &[3.0, 0.2]),
        ],
        keywords: "",
    },
    Family {
        name: "Zigzag",
        params: &[COUNT, HEIGHT, BOTH],
        make: Make::Motif(zigzag),
        presets: &[
            ("Zigzag", &[2.0, 0.3, 1.0]),
            ("Sharp zigzag", &[3.0, 0.5, 1.0]),
            ("Triple peak", &[3.0, 0.4, 0.0]),
        ],
        keywords: "",
    },
    Family {
        name: "Scallops",
        params: &[COUNT, HEIGHT],
        make: Make::Motif(scallops),
        presets: &[("Double scallop", &[2.0, 0.3]), ("Small scallops", &[3.0, 0.2])],
        keywords: "",
    },
    Family {
        name: "Loops",
        params: &[count("Count", 1, 3, 1), size("Size", 0.2, 0.45, 0.3)],
        make: Make::Motif(loops),
        presets: &[("Double loop", &[2.0, 0.35]), ("Big loop", &[1.0, 0.42])],
        keywords: "",
    },
    Family {
        name: "Steps",
        params: &[count("Count", 1, 3, 1), HEIGHT, size("Top", 0.2, 0.8, 0.5)],
        make: Make::Motif(steps),
        presets: &[("Battlement", &[2.0, 0.3, 0.5]), ("Wide step", &[1.0, 0.25, 0.7])],
        keywords: "",
    },
    Family {
        name: "Diamonds",
        params: &[count("Count", 1, 3, 1), HEIGHT],
        make: Make::Motif(diamonds),
        presets: &[("Double diamond", &[2.0, 0.3]), ("Slim diamond", &[1.0, 0.18])],
        keywords: "",
    },
    Family {
        name: "Triangles",
        params: &[count("Count", 1, 3, 1), HEIGHT],
        make: Make::Motif(triangles),
        presets: &[("Double triangle", &[2.0, 0.4]), ("Low triangle", &[1.0, 0.25])],
        keywords: "",
    },
    Family {
        name: "Beads",
        params: &[
            count("Count", 1, 4, 1),
            size("Width", 0.3, 0.9, 0.6),
            size("Height", 0.05, 0.4, 0.2),
        ],
        make: Make::Motif(beads),
        presets: &[
            ("Double bead", &[2.0, 0.7, 0.2]),
            ("Pearl", &[1.0, 0.5, 0.25]),
            ("Pearls", &[3.0, 0.8, 0.14]),
        ],
        keywords: "",
    },
    Family {
        name: "Star",
        params: &[
            count("Points", 4, 8, 5),
            size("Inner part", 0.3, 0.7, 0.4),
            size("Size", 0.15, 0.45, 0.35),
        ],
        make: Make::Motif(star),
        presets: &[
            ("Six-point star", &[6.0, 0.5, 0.35]),
            ("Eight-point star", &[8.0, 0.6, 0.35]),
            ("Small star", &[5.0, 0.4, 0.22]),
        ],
        keywords: "",
    },
    Family {
        name: "Flower",
        params: &[count("Petals", 3, 8, 5), size("Size", 0.15, 0.4, 0.3)],
        make: Make::Motif(flower),
        presets: &[
            ("Daisy", &[8.0, 0.3]),
            ("Clover", &[3.0, 0.28]),
            ("Four-petal flower", &[4.0, 0.3]),
        ],
        keywords: "",
    },
    Family {
        name: "Leaf",
        params: &[size("Width", 0.1, 0.45, 0.3), count("Veins", 0, 3, 0)],
        make: Make::Motif(leaf),
        presets: &[("Slim leaf", &[0.15, 0.0]), ("Veined leaf", &[0.3, 2.0])],
        keywords: "",
    },
    Family {
        name: "Key",
        params: &[count("Count", 1, 3, 1), HEIGHT],
        make: Make::Motif(key),
        presets: &[("Key", &[1.0, 0.4]), ("Double key", &[2.0, 0.4])],
        keywords: "",
    },
    Family {
        name: "Arches",
        params: &[count("Count", 1, 3, 1), HEIGHT, size("Point", 0.0, 1.0, 0.0)],
        make: Make::Motif(arches),
        presets: &[
            ("Arch", &[1.0, 0.45, 0.0]),
            ("Pointed arch", &[1.0, 0.5, 0.8]),
            ("Arcade", &[2.0, 0.35, 0.3]),
        ],
        keywords: "",
    },
    Family {
        name: "Spiral",
        params: &[size("Turns", 1.0, 3.0, 1.5), size("Size", 0.15, 0.4, 0.3)],
        make: Make::Motif(spiral),
        presets: &[("Spiral", &[1.5, 0.3]), ("Curl", &[1.0, 0.22])],
        keywords: "",
    },
    Family {
        name: "Feather",
        params: &[count("Branches", 2, 6, 4), HEIGHT, size("Slant", 0.0, 0.8, 0.5)],
        make: Make::Motif(feather),
        presets: &[("Feather", &[4.0, 0.3, 0.5]), ("Thorns", &[6.0, 0.2, 0.2])],
        keywords: "",
    },
    Family {
        name: "Crosses",
        params: &[count("Count", 1, 3, 1), HEIGHT],
        make: Make::Motif(crosses),
        presets: &[("Cross stitch", &[2.0, 0.25]), ("Big cross", &[1.0, 0.35])],
        keywords: "",
    },
    Family {
        name: "Hearts",
        params: &[count("Count", 1, 2, 1), size("Size", 0.15, 0.4, 0.3)],
        make: Make::Motif(hearts),
        presets: &[("Double heart", &[2.0, 0.35]), ("Small heart", &[1.0, 0.2])],
        keywords: "",
    },
    Family {
        name: "Comb",
        params: &[count("Teeth", 2, 8, 4), HEIGHT, BOTH],
        make: Make::Motif(comb),
        presets: &[("Comb", &[4.0, 0.3, 0.0]), ("Ladder", &[5.0, 0.25, 1.0])],
        keywords: "",
    },
    Family {
        name: "Vine",
        params: &[count("Leaves", 2, 6, 3), size("Size", 0.15, 0.45, 0.3)],
        make: Make::Motif(vine),
        presets: &[("Vine", &[3.0, 0.3]), ("Fern", &[5.0, 0.25])],
        keywords: "",
    },
    Family {
        name: "Fan",
        params: &[count("Rays", 3, 7, 5), size("Size", 0.2, 0.5, 0.4)],
        make: Make::Motif(fan),
        presets: &[("Fan", &[5.0, 0.4]), ("Shell", &[7.0, 0.35])],
        keywords: "",
    },
    Family {
        name: "Asterisk",
        params: &[count("Arms", 3, 8, 6), size("Size", 0.1, 0.35, 0.25)],
        make: Make::Motif(asterisk),
        presets: &[("Asterisk", &[6.0, 0.25]), ("Snowflake", &[8.0, 0.3])],
        keywords: "",
    },
    Family {
        name: "Boxes",
        params: &[count("Count", 1, 3, 1), size("Size", 0.3, 0.9, 0.6)],
        make: Make::Motif(boxes),
        presets: &[("Square", &[1.0, 0.6]), ("Squares", &[2.0, 0.7])],
        keywords: "",
    },
    Family {
        name: "Polygon",
        params: &[count("Sides", 3, 8, 6), size("Size", 0.15, 0.4, 0.3)],
        make: Make::Motif(polygon),
        presets: &[
            ("Hexagon", &[6.0, 0.3]),
            ("Pentagon", &[5.0, 0.3]),
            ("Octagon", &[8.0, 0.3]),
        ],
        keywords: "",
    },
    Family {
        name: "Arrows",
        params: &[count("Count", 1, 3, 1), HEIGHT],
        make: Make::Motif(arrows),
        presets: &[("Double chevron", &[2.0, 0.3])],
        keywords: "",
    },
    Family {
        name: "Stairs",
        params: &[count("Steps", 2, 5, 3), HEIGHT],
        make: Make::Motif(stairs),
        presets: &[("Stairs", &[3.0, 0.45]), ("Pyramid", &[4.0, 0.5])],
        keywords: "",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{Item, Kind, families_of};

    #[test]
    fn each_motif_goes_from_the_first_to_the_second_reference_point() {
        for f in families_of(Kind::Motif) {
            let lo: Vec<f64> = f.params.iter().map(|p| p.min).collect();
            let hi: Vec<f64> = f.params.iter().map(|p| p.max).collect();
            for v in [f.defaults(), lo, hi] {
                let Item::Motif(m) = f.item("x", &v) else { panic!() };
                let (a, b) = (m.points[0], m.points[m.points.len() - 1]);
                assert!(a.distance(emb_geom::Point::new(0.0, 0.0)) < 1e-9, "{f:?} {v:?}: {a:?}");
                assert!(b.distance(emb_geom::Point::new(1.0, 0.0)) < 1e-9, "{f:?} {v:?}: {b:?}");
                assert!(m.points.len() >= 3, "{f:?} {v:?}");
                for p in &m.points {
                    assert!(p.x.is_finite() && p.y.is_finite(), "{f:?} {v:?}");
                    // In the length of the repeat (a loop can go a little
                    // back), and not higher than the motif is long.
                    assert!((-0.2..=1.2).contains(&p.x) && p.y.abs() <= 1.0, "{f:?} {v:?}: {p:?}");
                }
                // The built-in motifs have this check too.
                assert!(m.points.iter().all(|p| p.x > -0.1 && p.x < 1.1), "{f:?} {v:?}");
            }
        }
    }
}

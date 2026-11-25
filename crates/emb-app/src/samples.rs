//! The start designs of the tutorials (P19 Part 2, ADR 0162). They are made
//! in code, as the corpus designs are, so a tutorial opens the same design
//! on each computer, and nothing goes in the folders of the user (ADR
//! 0080). A help topic opens one with a link `[text](sample:<name>)`.
//!
//! A start design can have a backdrop picture that is drawn here too (a
//! logo to trace, a picture for Picture to Design). Its shapes are made
//! from the same points that the step file of the tutorial clicks
//! (`docs/help/steps`), so the stitches of the steps go on the picture.

use emb_geom::{Join, Path, Point};
use emb_model::{
    Backdrop, BorderPlace, ConstructionMark, Design, FillParams, MarkShape, ObjectKind, OffsetMake, OffsetSpec, Rgb,
    SatinGeometry, SatinParams,
};

use crate::app::AppState;
use crate::i18n::tr;

/// The start designs: the name in a `sample:` link, and the name of the
/// design (shown in its tab, translated).
pub const SAMPLES: &[(&str, &str)] = &[
    ("first-steps", "First design"),
    ("first-badge", "Badge"),
    ("logo-backdrop", "Leaf logo"),
    ("sew-well", "Leaf to sew well"),
    ("columns", "Columns"),
    ("sew-order", "Dots"),
    ("patch-picture", "Patch picture"),
    ("lettering-start", "Name"),
    ("towel", "Towel"),
    ("applique-heart", "Heart"),
    ("hooping-banner", "Banner"),
];

/// The prefix of a link to a start design in a help topic.
pub const LINK: &str = "sample:";

/// The two corners of the circle of the first design, in mm: the start
/// design marks them with construction points.
pub const FIRST_CORNERS: [Point; 2] = [Point { x: -30.0, y: -30.0 }, Point { x: 30.0, y: 30.0 }];

/// A point that a step clicks: x and y in mm, and a curve point (`true`,
/// a right click) or a corner (`false`, a left click).
type Click = (f64, f64, bool);

/// The leaf of the leaf logo: its outline, its hole, the two sides of its
/// stem and its vein (the clicks of `tutorial-logo-by-hand`).
pub const LEAF: &[Click] = &[
    (-25.0, 25.0, false),
    (-28.0, -5.0, true),
    (-5.0, -25.0, true),
    (28.0, -28.0, false),
    (25.0, 5.0, true),
    (5.0, 25.0, true),
];
pub const LEAF_HOLE: &[Click] = &[
    (-8.0, -10.0, true),
    (-4.0, -6.0, true),
    (-8.0, -2.0, true),
    (-12.0, -6.0, true),
];
pub const STEM_A: &[Click] = &[(-23.5, 26.5, false), (-30.0, 32.0, true), (-36.0, 39.5, false)];
pub const STEM_B: &[Click] = &[(-26.5, 23.5, false), (-33.0, 29.0, true), (-39.5, 36.5, false)];
pub const VEIN: &[Click] = &[(-18.0, 18.0, false), (0.0, -1.0, true), (16.0, -16.0, false)];

/// The shapes of `tutorial-columns`: the pairs of an arch (side A, side B),
/// the middle line of an S, the outline of a crescent and its angle lines,
/// and a wave (a vector line).
pub const ARCH: &[[f64; 4]] = &[
    [-50.0, 0.0, -44.0, 0.0],
    [-42.7, -17.7, -38.4, -13.4],
    [-25.0, -25.0, -25.0, -19.0],
    [-7.3, -17.7, -11.6, -13.4],
    [0.0, 0.0, -6.0, 0.0],
];
pub const S_LINE: &[Click] = &[
    (20.0, -30.0, false),
    (32.0, -22.0, true),
    (20.0, -12.0, true),
    (8.0, -2.0, true),
    (20.0, 6.0, false),
];
pub const CRESCENT: &[Click] = &[
    (-45.0, 15.0, false),
    (-25.0, 40.0, true),
    (-5.0, 15.0, false),
    (-25.0, 28.0, true),
];
pub const WAVE: &[Click] = &[
    (15.0, 30.0, false),
    (25.0, 22.0, true),
    (35.0, 38.0, true),
    (45.0, 30.0, false),
];

/// The heart of `tutorial-applique` (a fill).
pub const HEART: &[Click] = &[
    (0.0, -12.0, false),
    (12.0, -24.0, true),
    (26.0, -12.0, true),
    (0.0, 22.0, false),
    (-26.0, -12.0, true),
    (-12.0, -24.0, true),
];

/// Where the name of `tutorial-lettering` starts (a construction point).
pub const NAME_START: Point = Point { x: -35.0, y: 10.0 };

/// The width of the satin column of Column C (the default of the settings).
const COLUMN_C_MM: f64 = 3.0;

fn path_of(points: &[Click], closed: bool) -> Path {
    let pts: Vec<(Point, bool)> = points.iter().map(|&(x, y, c)| (Point::new(x, y), c)).collect();
    Path::through_points(&pts, closed)
}

/// The border that **Add Border** puts round the selection: a line with a
/// satin border, as the Offsets window that `Cmd::AddBorder` opens makes
/// it with the values of a new session. The badge of the second tutorial
/// is the result of the first tutorial (the step files check its size).
pub fn add_border_spec() -> OffsetSpec {
    OffsetSpec {
        distance_mm: 1.0,
        count: 1,
        spacing_mm: OffsetSpec::default().spacing_mm,
        corners: Join::Round,
        make: OffsetMake::Border {
            width_mm: 3.0,
            place: BorderPlace::Outside,
        },
    }
}

/// A picture of flat shapes on white (a backdrop of a start design): `ppm`
/// pixels for each mm, its top left corner at `origin` (mm).
struct Drawing {
    img: image::RgbaImage,
    origin: Point,
    ppm: f64,
}

impl Drawing {
    fn new(min: Point, max: Point, ppm: f64) -> Self {
        let w = ((max.x - min.x) * ppm).round() as u32;
        let h = ((max.y - min.y) * ppm).round() as u32;
        Drawing {
            img: image::RgbaImage::from_pixel(w, h, image::Rgba([255, 255, 255, 255])),
            origin: min,
            ppm,
        }
    }

    /// The first pixel column (or row) whose middle is at `mm` or after.
    fn pixel(&self, mm: f64, origin: f64, len: u32) -> u32 {
        ((mm - origin) * self.ppm - 0.5).ceil().clamp(0.0, f64::from(len)) as u32
    }

    /// Fills the closed `paths` (a path in another path is a hole).
    fn fill(&mut self, paths: &[Path], c: Rgb) {
        let edges: Vec<(Point, Point)> = paths
            .iter()
            .flat_map(|p| p.flatten(0.02))
            .flat_map(|l| {
                let n = l.points.len();
                (0..n).map(move |i| (l.points[i], l.points[(i + 1) % n]))
            })
            .collect();
        let (w, h) = self.img.dimensions();
        for y in 0..h {
            let py = self.origin.y + (f64::from(y) + 0.5) / self.ppm;
            let mut xs: Vec<f64> = edges
                .iter()
                .filter(|(a, b)| (a.y <= py) != (b.y <= py))
                .map(|(a, b)| a.x + (py - a.y) * (b.x - a.x) / (b.y - a.y))
                .collect();
            xs.sort_by(f64::total_cmp);
            for [a, b] in xs.as_chunks::<2>().0 {
                let (x0, x1) = (self.pixel(*a, self.origin.x, w), self.pixel(*b, self.origin.x, w));
                for x in x0..x1 {
                    self.img.put_pixel(x, y, image::Rgba([c.0, c.1, c.2, 255]));
                }
            }
        }
    }

    /// Draws `path` as a line `width_mm` wide.
    fn stroke(&mut self, path: &Path, width_mm: f64, c: Rgb) {
        let r = width_mm / 2.0;
        let (w, h) = self.img.dimensions();
        for line in path.flatten(0.02) {
            for seg in line.points.windows(2) {
                let (a, b) = (seg[0], seg[1]);
                let (x0, x1) = (
                    self.pixel(a.x.min(b.x) - r, self.origin.x, w),
                    self.pixel(a.x.max(b.x) + r, self.origin.x, w),
                );
                let (y0, y1) = (
                    self.pixel(a.y.min(b.y) - r, self.origin.y, h),
                    self.pixel(a.y.max(b.y) + r, self.origin.y, h),
                );
                let ab = b - a;
                let len2 = ab.x * ab.x + ab.y * ab.y;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let p = Point::new(
                            self.origin.x + (f64::from(x) + 0.5) / self.ppm,
                            self.origin.y + (f64::from(y) + 0.5) / self.ppm,
                        );
                        let ap = p - a;
                        let t = if len2 > 0.0 {
                            ((ap.x * ab.x + ap.y * ab.y) / len2).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let d = p - (a + ab * t);
                        if d.x * d.x + d.y * d.y <= r * r {
                            self.img.put_pixel(x, y, image::Rgba([c.0, c.1, c.2, 255]));
                        }
                    }
                }
            }
        }
    }

    /// The picture as a backdrop of the design, at its place.
    fn backdrop(&self, name: &str) -> Option<Backdrop> {
        let png = emb_art::encode_png(&self.img).ok()?;
        let (w, h) = self.img.dimensions();
        let size = Point::new(f64::from(w), f64::from(h)) * (1.0 / self.ppm);
        Some(Backdrop::new(name, png, (w, h), self.origin + size * 0.5, size.x))
    }
}

const GREEN: Rgb = Rgb(60, 150, 60);
const BROWN: Rgb = Rgb(120, 80, 40);
const DARK_GREEN: Rgb = Rgb(20, 80, 30);

/// The leaf logo as a picture.
fn leaf_picture() -> Option<Backdrop> {
    let mut p = Drawing::new(Point::new(-45.0, -40.0), Point::new(40.0, 45.0), 8.0);
    let stem: Vec<Click> = STEM_A.iter().chain(STEM_B.iter().rev()).copied().collect();
    p.fill(&[path_of(&stem, true)], BROWN);
    p.fill(&[path_of(LEAF, true), path_of(LEAF_HOLE, true)], GREEN);
    p.stroke(&path_of(VEIN, false), 1.0, DARK_GREEN);
    p.backdrop("leaf.png")
}

/// The shapes of the columns tutorial as a picture.
fn columns_picture() -> Option<Backdrop> {
    let purple = Rgb(120, 60, 150);
    let mut p = Drawing::new(Point::new(-55.0, -35.0), Point::new(50.0, 45.0), 8.0);
    let side = |i: usize| -> Vec<Click> { ARCH.iter().map(|q| (q[i], q[i + 1], true)).collect() };
    let mut arch = side(0);
    arch.extend(side(2).into_iter().rev());
    p.fill(&[path_of(&arch, true)], purple);
    p.stroke(&path_of(S_LINE, false), COLUMN_C_MM, purple);
    p.fill(&[path_of(CRESCENT, true)], purple);
    p.backdrop("columns.png")
}

/// A logo of two colors for Picture to Design: a navy circle with a yellow
/// star.
fn patch_picture() -> Option<Backdrop> {
    let mut p = Drawing::new(Point::new(-35.0, -35.0), Point::new(35.0, 35.0), 10.0);
    p.fill(&[Path::ellipse(Point::ZERO, 30.0, 30.0)], Rgb(25, 35, 90));
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 18.0 } else { 7.5 };
            let a = std::f64::consts::PI * (f64::from(i) / 5.0 - 0.5);
            Point::new(r * a.cos(), r * a.sin())
        })
        .collect();
    p.fill(&[Path::polygon(&star)], Rgb(240, 190, 30));
    p.backdrop("patch.png")
}

fn fill(outer: Path, holes: Vec<Path>) -> ObjectKind {
    ObjectKind::Fill {
        outer,
        holes,
        params: FillParams::default(),
    }
}

/// The start design `name`, or `None` if there is no such design.
pub fn design(name: &str) -> Option<Design> {
    let (_, title) = SAMPLES.iter().find(|(n, _)| *n == name)?;
    let mut d = Design::new(*title);
    match name {
        // An empty design with the two corners of the circle to draw.
        "first-steps" => {
            d.add_thread("Gold", Rgb(230, 170, 40));
            d.add_thread("Navy", Rgb(25, 35, 90));
            for at in FIRST_CORNERS {
                d.marks.push(ConstructionMark::new(MarkShape::Point { at }));
            }
        }
        // The badge of the first tutorial: a gold circle with a navy border.
        "first-badge" => {
            let gold = d.add_thread("Gold", Rgb(230, 170, 40));
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            let [a, b] = FIRST_CORNERS;
            let circle = Path::ellipse(a.midpoint(b), (b.x - a.x) / 2.0, (b.y - a.y) / 2.0);
            let fill = d.add_object(gold, fill(circle, Vec::new()));
            let border = d.add_offsets(&[fill], &add_border_spec()).ok()?;
            for id in border {
                if let Some(o) = d.object_mut(id) {
                    o.thread = navy;
                }
            }
        }
        // The picture of the leaf logo to trace, and its three threads.
        "logo-backdrop" => {
            d.add_thread("Green", GREEN);
            d.add_thread("Brown", BROWN);
            d.add_thread("Dark green", DARK_GREEN);
            d.backdrop = leaf_picture();
        }
        // The leaf logo made, with a fill that is too dense.
        "sew-well" => {
            let green = d.add_thread("Green", GREEN);
            let brown = d.add_thread("Brown", BROWN);
            let dark = d.add_thread("Dark green", DARK_GREEN);
            d.add_object(
                green,
                ObjectKind::Fill {
                    outer: path_of(LEAF, true),
                    holes: vec![path_of(LEAF_HOLE, true)],
                    params: FillParams {
                        spacing_mm: 0.2,
                        ..FillParams::default()
                    },
                },
            );
            d.add_object(
                brown,
                ObjectKind::Satin {
                    geometry: SatinGeometry {
                        rail_a: path_of(STEM_A, false),
                        rail_b: path_of(STEM_B, false),
                        rungs: vec![],
                    },
                    params: SatinParams::default(),
                },
            );
            d.add_object(
                dark,
                ObjectKind::Running {
                    path: path_of(VEIN, false),
                    params: Default::default(),
                },
            );
        }
        // The shapes to sew as columns, and a wave to convert.
        "columns" => {
            let purple = d.add_thread("Purple", Rgb(120, 60, 150));
            d.backdrop = columns_picture();
            d.add_object(
                purple,
                ObjectKind::Vector {
                    path: path_of(WAVE, false),
                    style: Default::default(),
                },
            );
        }
        // Six dots of two colors in a bad sew order.
        "sew-order" => {
            let red = d.add_thread("Red", Rgb(200, 40, 40));
            let blue = d.add_thread("Blue", Rgb(40, 80, 200));
            for (i, y) in [-15.0, 15.0].into_iter().enumerate() {
                for (j, x) in [-30.0, 0.0, 30.0].into_iter().enumerate() {
                    let t = if (i + j) % 2 == 0 { red } else { blue };
                    d.add_object(t, fill(Path::ellipse(Point::new(x, y), 10.0, 10.0), Vec::new()));
                }
            }
        }
        // A logo picture for Picture to Design.
        "patch-picture" => d.backdrop = patch_picture(),
        // Where the name starts.
        "lettering-start" => {
            d.add_thread("Navy", Rgb(25, 35, 90));
            d.marks.push(ConstructionMark::new(MarkShape::Point { at: NAME_START }));
        }
        "towel" => {
            d.add_thread("Navy", Rgb(25, 35, 90));
        }
        // A heart (a fill) to make an appliqué of.
        "applique-heart" => {
            let red = d.add_thread("Red", Rgb(200, 40, 40));
            d.add_object(red, fill(path_of(HEART, true), Vec::new()));
        }
        // Five dots in a row, 380 mm wide: larger than a hoop.
        "hooping-banner" => {
            let blue = d.add_thread("Blue", Rgb(40, 80, 200));
            for k in -2..=2 {
                let at = Point::new(80.0 * f64::from(k), 0.0);
                d.add_object(blue, fill(Path::ellipse(at, 30.0, 30.0), Vec::new()));
            }
        }
        _ => return None,
    }
    Some(d)
}

impl AppState {
    /// Opens the start design `name` in a new tab, on the machine and in
    /// the hoop of the design of now (as a new design).
    pub fn open_sample(&mut self, name: &str) {
        let Some(mut d) = design(name) else {
            self.message = crate::i18n::fill("No start design {}.", &[&name]);
            return;
        };
        d.name = tr(&d.name).to_string();
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        let title = d.name.clone();
        self.start_design(d);
        self.message = crate::i18n::fill("Start design \"{}\"", &[&title]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each start design is made, and has what its tutorial needs.
    #[test]
    fn each_start_design_is_made() {
        for (name, _) in SAMPLES {
            assert!(design(name).is_some(), "{name}");
        }
        let d = |name| design(name).unwrap();
        assert_eq!(d("first-steps").marks.len(), 2);
        let badge = d("first-badge");
        let kinds: Vec<&str> = badge
            .objects()
            .iter()
            .map(|o| match o.kind {
                ObjectKind::Fill { .. } => "fill",
                ObjectKind::Running { ref params, .. } if params.border.is_some() => "satin border",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["fill", "satin border"]);
        assert_ne!(badge.objects()[0].thread, badge.objects()[1].thread);
        for name in ["logo-backdrop", "columns", "patch-picture"] {
            let b = d(name).backdrop.unwrap_or_else(|| panic!("{name} has a picture"));
            assert!(b.pixels.0 > 500 && b.pixels.1 > 500, "{name}: {:?}", b.pixels);
        }
        assert_eq!(d("sew-well").objects().len(), 3);
        assert_eq!(d("sew-order").sewn_color_blocks().len(), 6);
        assert_eq!(d("hooping-banner").extents().unwrap().width().round(), 380.0);
        assert!(design("no such design").is_none());
    }

    /// The pictures have the colors of their shapes at the points that the
    /// steps click: the leaf is green, its hole is white.
    #[test]
    fn the_pictures_show_the_shapes() {
        let b = leaf_picture().unwrap();
        let img = image::load_from_memory(&b.data).unwrap().to_rgba8();
        let at = |p: Point| {
            let q = (p - b.origin) * (f64::from(b.pixels.0) / b.width_mm);
            img.get_pixel(q.x as u32, q.y as u32).0
        };
        assert_eq!(at(Point::new(10.0, 5.0)), [GREEN.0, GREEN.1, GREEN.2, 255]);
        assert_eq!(at(Point::new(-8.0, -6.0)), [255, 255, 255, 255], "the hole");
        assert_eq!(
            at(Point::new(-31.0, 32.0)),
            [BROWN.0, BROWN.1, BROWN.2, 255],
            "the stem"
        );
        assert_eq!(at(Point::new(35.0, 35.0)), [255, 255, 255, 255]);
    }

    /// A start design opens in a new tab, with the machine of the design
    /// before.
    #[test]
    fn a_start_design_opens_in_a_new_tab() {
        let mut h = crate::test_util::Harness::new();
        let machine = h.st.doc.design().machine.clone();
        h.st.open_sample("first-badge");
        assert_eq!(h.st.doc.design().objects().len(), 2);
        assert_eq!(h.st.doc.design().machine, machine);
        assert_eq!(h.st.doc.design().name, "Badge");
    }
}

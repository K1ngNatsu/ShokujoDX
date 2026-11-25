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
    ("team-shirt", "Team shirt"),
    ("photo-apple", "Apple photo"),
    ("vector-logo", "Cloud logo"),
    ("customer-dst", "LEAF"),
    ("decor-shapes", "Circle and line"),
    ("placket", "Placket"),
    ("quilt-heart", "Quilt heart"),
    ("cap-front", "Cap front"),
    ("cutwork-circle", "Cutwork circle"),
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

/// The name and the number on the back of the team shirt of
/// `tutorial-team-names`: the text, the font, the height (mm) and the
/// point of the baseline (the middle of the text).
pub const TEAM_TEXTS: [(&str, &str, f64, Point); 2] = [
    ("NAME", "Block Sans", 12.0, Point { x: 0.0, y: -12.0 }),
    ("00", "Varsity", 25.0, Point { x: 0.0, y: 22.0 }),
];

/// The cloud logo of `tutorial-vector-logo`, as vector artwork after an
/// import: three circles and a base for the cloud (center x, y and radius,
/// and two corners), and the sun.
pub const CLOUD_CIRCLES: [(f64, f64, f64); 3] = [(-18.0, 4.0, 12.0), (0.0, -4.0, 16.0), (17.0, 5.0, 11.0)];
pub const CLOUD_BASE: [Point; 2] = [Point { x: -30.0, y: 4.0 }, Point { x: 28.0, y: 16.0 }];
pub const SUN: (f64, f64, f64) = (26.0, -22.0, 12.0);

/// The circle (a fill) and the line under it of `tutorial-borders`.
pub const DECOR_CIRCLE: (Point, f64) = (Point { x: 0.0, y: -10.0 }, 30.0);
pub const DECOR_LINE: [Point; 2] = [Point { x: -35.0, y: 40.0 }, Point { x: 35.0, y: 40.0 }];

/// The line of the buttonholes of `tutorial-buttonholes`: the middle of
/// the placket of a shirt, 80 mm.
pub const PLACKET: [Point; 2] = [Point { x: 0.0, y: -40.0 }, Point { x: 0.0, y: 40.0 }];

/// The wave line of `tutorial-quilt`: the pattern of the quilt block. Its
/// ends are level, so the copies of the block join.
pub const QUILT_WAVE: [Point; 5] = [
    Point { x: -20.0, y: 85.0 },
    Point { x: -10.0, y: 80.0 },
    Point { x: 0.0, y: 85.0 },
    Point { x: 10.0, y: 80.0 },
    Point { x: 20.0, y: 85.0 },
];

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

/// A photo for Photo stitch: an apple lit from the top left, with its
/// shadow, a stem and a leaf, on a light background. It has the light and
/// the dark of a photo, not flat areas of a few colors.
fn photo_picture() -> Option<Backdrop> {
    let mut p = Drawing::new(Point::new(-40.0, -40.0), Point::new(40.0, 40.0), 8.0);
    let (w, h) = p.img.dimensions();
    // The light: from the top left and the front (a unit vector).
    let light = {
        let (x, y, z) = (-0.5_f64, -0.6, 0.62);
        let n = (x * x + y * y + z * z).sqrt();
        (x / n, y / n, z / n)
    };
    for row in 0..h {
        for col in 0..w {
            let q = Point::new(
                p.origin.x + (f64::from(col) + 0.5) / p.ppm,
                p.origin.y + (f64::from(row) + 0.5) / p.ppm,
            );
            let mut c = [236.0, 233.0, 226.0];
            // The shadow on the table, soft at its edge.
            let (sx, sy) = ((q.x - 7.0) / 30.0, (q.y - 25.0) / 8.0);
            let shade = 1.0 - (sx * sx + sy * sy);
            if shade > 0.0 {
                for v in &mut c {
                    *v *= 1.0 - 0.45 * shade.min(0.6);
                }
            }
            // The apple: a ball, darker where it turns from the light.
            let (u, v) = (q.x / 26.0, (q.y + 2.0) / 24.0);
            let r2 = u * u + v * v;
            if r2 < 1.0 {
                let z = (1.0 - r2).sqrt();
                let lambert = (u * light.0 + v * light.1 + z * light.2).max(0.0);
                // A small highlight (the light seen in the skin).
                let mut spec = (lambert * 1.02 - 0.02).max(0.0);
                for _ in 0..5 {
                    spec *= spec;
                }
                let base = [196.0, 32.0, 38.0];
                for (k, v) in c.iter_mut().enumerate() {
                    *v = base[k] * (0.22 + 0.78 * lambert) + 255.0 * 0.8 * spec;
                }
            }
            let px = c.map(|v| v.round().clamp(0.0, 255.0) as u8);
            p.img.put_pixel(col, row, image::Rgba([px[0], px[1], px[2], 255]));
        }
    }
    let stem = Path::new().move_to(Point::new(0.5, -22.0)).cubic_to(
        Point::new(0.5, -27.0),
        Point::new(2.0, -31.0),
        Point::new(4.5, -33.0),
    );
    p.stroke(&stem, 2.2, Rgb(90, 60, 30));
    let leaf = Path::new()
        .move_to(Point::new(3.0, -29.0))
        .cubic_to(Point::new(8.0, -36.0), Point::new(16.0, -35.0), Point::new(20.0, -31.0))
        .cubic_to(Point::new(14.0, -26.0), Point::new(8.0, -26.0), Point::new(3.0, -29.0))
        .close();
    p.fill(&[leaf], Rgb(70, 140, 50));
    p.backdrop("apple.png")
}

/// The leaf logo made by hand (the result of `tutorial-logo-by-hand`): the
/// leaf as a fill with `fill` and its hole, the stem as a satin column and
/// the vein as a running stitch, in three threads.
fn leaf_design(fill: FillParams) -> Design {
    let mut d = Design::new("");
    let green = d.add_thread("Green", GREEN);
    let brown = d.add_thread("Brown", BROWN);
    let dark = d.add_thread("Dark green", DARK_GREEN);
    d.add_object(
        green,
        ObjectKind::Fill {
            outer: path_of(LEAF, true),
            holes: vec![path_of(LEAF_HOLE, true)],
            params: fill,
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
    d
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
        // The leaf logo, sewn and written as a DST file, then read again: a
        // file from a customer (stitches only, no thread colors).
        "customer-dst" => {
            let mut leaf = leaf_design(FillParams::default());
            leaf.name = "LEAF".into();
            let plan = emb_stitch::build_plan(
                &leaf,
                &emb_stitch::PlanOptions::default(),
                &mut emb_stitch::StitchCache::default(),
            );
            let bytes = emb_formats::write_machine("dst", &plan.stitches, "LEAF", "")?;
            let read = emb_formats::read_machine("dst", &bytes).ok()?;
            let mut file = emb_model::design_from_stitches(&read, title);
            // On a white shirt: the first color of the file is black.
            file.background = Some(Rgb(245, 245, 240));
            std::mem::swap(&mut d, &mut file);
        }
        // The leaf logo made, with a fill that is too dense.
        "sew-well" => {
            let mut leaf = leaf_design(FillParams {
                spacing_mm: 0.2,
                ..FillParams::default()
            });
            leaf.name = d.name.clone();
            d = leaf;
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
        // A name and a number for the back of a team shirt.
        "team-shirt" => {
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            let red = d.add_thread("Red", Rgb(200, 30, 45));
            for ((text, family, height, at), t) in TEAM_TEXTS.into_iter().zip([navy, red]) {
                let font = emb_letter::bundled::embroidery(family)?;
                let mut l = emb_model::Lettering::new(text, emb_letter::LetterSource::font_ref(&font), height, at);
                l.justify = emb_model::Justify::Center;
                emb_letter::relayout(&mut l, &font);
                d.add_object(t, ObjectKind::Lettering(Box::new(l)));
            }
        }
        // A photo for Photo stitch.
        "photo-apple" => d.backdrop = photo_picture(),
        // A circle to put a border round, and a line for a motif run.
        "decor-shapes" => {
            let gold = d.add_thread("Gold", Rgb(230, 170, 40));
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            let (c, r) = DECOR_CIRCLE;
            d.add_object(gold, fill(Path::ellipse(c, r, r), Vec::new()));
            let [a, b] = DECOR_LINE;
            d.add_object(
                navy,
                ObjectKind::Running {
                    path: Path::new().move_to(a).line_to(b),
                    params: Default::default(),
                },
            );
        }
        // A heart to quilt round, and a wave line for a quilt block.
        "quilt-heart" => {
            let red = d.add_thread("Red", Rgb(200, 40, 40));
            d.add_object(red, fill(path_of(HEART, true), Vec::new()));
            d.add_object(
                red,
                ObjectKind::Running {
                    path: QUILT_WAVE[1..]
                        .iter()
                        .fold(Path::new().move_to(QUILT_WAVE[0]), |p, q| p.line_to(*q)),
                    params: Default::default(),
                },
            );
        }
        // The front of a cap: a frame line (flat, it sews first) and
        // lettering to make puff.
        "cap-front" => {
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            let red = d.add_thread("Red", Rgb(200, 30, 45));
            d.add_object(
                navy,
                ObjectKind::Running {
                    path: Path::rounded_rect(
                        emb_geom::BBox::new(Point::new(-45.0, -20.0), Point::new(45.0, 20.0)),
                        6.0,
                    ),
                    params: Default::default(),
                },
            );
            let font = emb_letter::bundled::embroidery("Block Sans")?;
            let mut l = emb_model::Lettering::new(
                "CAP",
                emb_letter::LetterSource::font_ref(&font),
                20.0,
                Point::new(0.0, 10.0),
            );
            l.justify = emb_model::Justify::Center;
            emb_letter::relayout(&mut l, &font);
            d.add_object(red, ObjectKind::Lettering(Box::new(l)));
        }
        // A circle line: the edge of a hole to cut.
        "cutwork-circle" => {
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            d.add_object(
                navy,
                ObjectKind::Running {
                    path: Path::ellipse(Point::ZERO, 10.0, 10.0),
                    params: Default::default(),
                },
            );
        }
        // The middle line of a placket, for a string of buttonholes.
        "placket" => {
            let navy = d.add_thread("Navy", Rgb(25, 35, 90));
            let [a, b] = PLACKET;
            d.add_object(
                navy,
                ObjectKind::Running {
                    path: Path::new().move_to(a).line_to(b),
                    params: Default::default(),
                },
            );
        }
        // A logo as vector artwork (as Import as Vectors makes it): a cloud
        // of four shapes that overlap, and a sun.
        "vector-logo" => {
            let (blue, yellow) = (Rgb(70, 140, 210), Rgb(245, 190, 30));
            let sky = d.add_thread("Sky blue", blue);
            let sun = d.add_thread("Yellow", yellow);
            let vector = |path: Path, color: Rgb| ObjectKind::Vector {
                path,
                style: emb_model::VectorStyle {
                    fill: Some(color),
                    ..Default::default()
                },
            };
            for (x, y, r) in CLOUD_CIRCLES {
                d.add_object(sky, vector(Path::ellipse(Point::new(x, y), r, r), blue));
            }
            let [a, b] = CLOUD_BASE;
            d.add_object(sky, vector(Path::rounded_rect(emb_geom::BBox::new(a, b), 4.0), blue));
            let (x, y, r) = SUN;
            d.add_object(sun, vector(Path::ellipse(Point::new(x, y), r, r), yellow));
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
        for name in ["logo-backdrop", "columns", "patch-picture", "photo-apple"] {
            let b = d(name).backdrop.unwrap_or_else(|| panic!("{name} has a picture"));
            assert!(b.pixels.0 > 500 && b.pixels.1 > 500, "{name}: {:?}", b.pixels);
        }
        assert_eq!(d("sew-well").objects().len(), 3);
        assert_eq!(d("sew-order").sewn_color_blocks().len(), 6);
        assert_eq!(d("hooping-banner").extents().unwrap().width().round(), 380.0);
        assert_eq!(d("team-shirt").objects().len(), 2);
        assert_eq!(d("vector-logo").objects().len(), 5);
        assert_eq!(d("decor-shapes").objects().len(), 2);
        assert_eq!(d("placket").objects().len(), 1);
        assert_eq!(d("quilt-heart").objects().len(), 2);
        assert_eq!(d("cap-front").objects().len(), 2);
        assert_eq!(d("cutwork-circle").objects().len(), 1);
        let file = d("customer-dst");
        assert!(
            file.objects().len() >= 3
                && file
                    .objects()
                    .iter()
                    .all(|o| matches!(o.kind, ObjectKind::Stitches { .. })),
            "a stitch object for each color"
        );
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

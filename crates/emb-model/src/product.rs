//! Product backdrops (P7 Milestone 5): a product under the design on the
//! canvas, to see the design on a T-shirt, a cap, a towel and so on. The
//! product does not sew.
//!
//! The products are drawn in code at a real size (an adult size M for the
//! garments), so the design shows at its true size on them. Each product
//! has the usual places for a design ([`Placement`]). The user can also put
//! a picture of their own product under the design, with its real width.
//!
//! Product coordinates are mm, Y down. For garments the origin is the
//! middle of the neck line at the height of the neck points; for the other
//! products, the middle of their top edge (the brim line of a cap).

use std::sync::Arc;

use emb_geom::{BBox, Path, Point};
use serde::{Deserialize, Serialize};

use crate::{Design, Rgb};

/// The products that are drawn in code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductKind {
    TShirt,
    TShirtBack,
    Polo,
    Sweatshirt,
    Cap,
    Tote,
    Towel,
    Cushion,
    PoloBack,
    SweatshirtBack,
    LongSleeve,
    LongSleeveBack,
    Hoodie,
    HoodieBack,
    Apron,
    Beanie,
    Messenger,
}

/// A side of a product: a garment has a front and a back (P16 Milestone 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    #[default]
    Front,
    Back,
}

/// How a part of a product is colored: the product color, darker, or
/// lighter, or the trim color (P16 Milestone 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    Body,
    Dark,
    Light,
    /// A trim part: the part `group` of [`ProductKind::trims`] in the trim
    /// color (darker with `dark`). It shows only when the product has a
    /// trim color for it; it lies on the parts in the product color.
    Trim {
        group: u8,
        dark: bool,
    },
}

/// One part of the drawing of a product, in product coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum ProductPart {
    /// A filled area (closed path), drawn in order.
    Area(Path, Shade),
    /// A line (a seam or an edge) of this width in mm, drawn darker.
    Line(Path, f64),
    /// A fold of the cloth along the path: no color, only light and shade.
    /// `width` is the width in mm; `height` (mm) is positive for a ridge
    /// and negative for a valley.
    Fold { path: Path, width: f64, height: f64 },
}

/// A usual place for a design on a product: the design center goes to
/// `at` (product coordinates); `max` is the usual largest design there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub name: &'static str,
    pub at: Point,
    pub max: (f64, f64),
}

/// The product under the design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductView {
    pub kind: ProductKind,
    pub color: Rgb,
    /// Where the product origin is, in view coordinates (after the design rotation).
    pub origin: Point,
    #[serde(default = "yes")]
    pub visible: bool,
    /// A picture of the user's own product, in place of the drawn product.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture: Option<ProductPicture>,
    /// Other designs on the product (P16 Milestone 2), for example a name
    /// on a sleeve with the open design at the left chest.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extras: Vec<ProductExtra>,
    /// The saved picture shows the front and the back of a garment next
    /// to each other (P16 Milestone 3).
    #[serde(default, skip_serializing_if = "is_false")]
    pub both_sides: bool,
    /// A second color for the trim parts (P16 Milestone 6), for example
    /// the collar of a polo or the front of a cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim: Option<Trim>,
}

/// The trim color of a product, and the trim parts that do not take it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trim {
    pub color: Rgb,
    /// The names (of [`ProductKind::trims`]) of the trim parts in the
    /// product color. The names stay when the product changes, so a part
    /// of the same name on the new product stays off too.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub off: Vec<String>,
}

/// The usual colors of blank garments, with generic names (P16 Milestone 6).
pub const SWATCHES: [(&str, Rgb); 18] = [
    ("White", Rgb(250, 250, 250)),
    ("Natural", Rgb(238, 230, 211)),
    ("Ash", Rgb(214, 214, 210)),
    ("Heather gray", Rgb(170, 170, 172)),
    ("Charcoal", Rgb(70, 72, 76)),
    ("Black", Rgb(24, 24, 26)),
    ("Navy", Rgb(30, 38, 66)),
    ("Royal blue", Rgb(32, 72, 160)),
    ("Light blue", Rgb(150, 190, 225)),
    ("Kelly green", Rgb(20, 135, 75)),
    ("Forest green", Rgb(32, 70, 48)),
    ("Military green", Rgb(92, 96, 64)),
    ("Red", Rgb(200, 30, 40)),
    ("Maroon", Rgb(100, 28, 40)),
    ("Orange", Rgb(240, 110, 30)),
    ("Gold", Rgb(245, 180, 30)),
    ("Light pink", Rgb(244, 196, 208)),
    ("Purple", Rgb(80, 45, 125)),
];

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_front(s: &Side) -> bool {
    *s == Side::Front
}

fn yes() -> bool {
    true
}

/// Another design on the product. It shows on the canvas and in the
/// picture on the product; it does not sew and does not change the open
/// design. The project keeps a copy of the design, so it shows also when
/// its file is not there.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductExtra {
    /// The name of the file of the design.
    pub name: String,
    /// The center of the design, in product coordinates.
    pub at: Point,
    #[serde(default = "yes")]
    pub visible: bool,
    /// The side of the product that the design is on.
    #[serde(default, skip_serializing_if = "is_front")]
    pub side: Side,
    /// A number from the contents of the design: the same design has the
    /// same key, so the app makes its stitches once.
    pub key: u64,
    /// The design (with no product, backdrop or picture data of its own).
    pub design: Design,
}

impl PartialEq for ProductExtra {
    fn eq(&self, o: &Self) -> bool {
        self.name == o.name && self.at == o.at && self.visible == o.visible && self.side == o.side && self.key == o.key
    }
}

impl ProductExtra {
    /// `design` (from the file `name`) at `at` (product coordinates).
    pub fn new(name: &str, mut design: Design, at: Point) -> Self {
        design.product = None;
        design.backdrop = None;
        let text = crate::project::to_string(&design);
        ProductExtra {
            name: name.to_string(),
            at,
            visible: true,
            side: Side::Front,
            key: fnv(text.as_bytes()),
            design,
        }
    }

    /// The size of the design (mm), if it has stitch objects.
    pub fn size(&self) -> Option<(f64, f64)> {
        self.design.view_extents().map(|b| (b.width(), b.height()))
    }

    /// The center of the design in its own view coordinates. The stitches
    /// move by `at - center` (and the product origin) onto the product.
    pub fn center(&self) -> Point {
        self.design
            .view_extents()
            .map_or(self.design.hoop.center, |b| b.center())
    }
}

/// The FNV-1a hash of `bytes` (the same on each run and each computer).
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// A picture of the user's own product. Its top left corner is at the
/// origin of the [`ProductView`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPicture {
    pub name: String,
    /// The image file bytes (PNG, JPEG or BMP), saved as base64.
    #[serde(
        serialize_with = "crate::backdrop::to_base64",
        deserialize_with = "crate::backdrop::from_base64"
    )]
    pub data: Arc<Vec<u8>>,
    pub pixels: (u32, u32),
    /// The real width of the product in the picture, in mm.
    pub width_mm: f64,
}

impl PartialEq for ProductPicture {
    fn eq(&self, o: &Self) -> bool {
        self.name == o.name
            && (Arc::ptr_eq(&self.data, &o.data) || self.data == o.data)
            && self.pixels == o.pixels
            && self.width_mm == o.width_mm
    }
}

impl PartialEq for ProductView {
    fn eq(&self, o: &Self) -> bool {
        self.kind == o.kind
            && self.color == o.color
            && self.origin == o.origin
            && self.visible == o.visible
            && self.picture == o.picture
            && self.extras == o.extras
            && self.both_sides == o.both_sides
            && self.trim == o.trim
    }
}

impl ProductPicture {
    pub fn height_mm(&self) -> f64 {
        if self.pixels.0 == 0 {
            return self.width_mm;
        }
        self.width_mm * f64::from(self.pixels.1) / f64::from(self.pixels.0)
    }
}

/// The color of a shade of a product of `color`. On dark products the
/// details are lighter, so they show.
pub fn shade(color: Rgb, s: Shade) -> Rgb {
    let luma = 0.299 * f64::from(color.0) + 0.587 * f64::from(color.1) + 0.114 * f64::from(color.2);
    let scale = |f: f64| {
        let c = |v: u8| (f64::from(v) * f).round().clamp(0.0, 255.0) as u8;
        Rgb(c(color.0), c(color.1), c(color.2))
    };
    let lift = |t: f64| {
        let c = |v: u8| (f64::from(v) + (255.0 - f64::from(v)) * t).round() as u8;
        Rgb(c(color.0), c(color.1), c(color.2))
    };
    match s {
        Shade::Body | Shade::Trim { dark: false, .. } => color,
        Shade::Trim { dark: true, .. } => shade(color, Shade::Dark),
        Shade::Dark if luma < 70.0 => lift(0.22),
        Shade::Dark => scale(0.72),
        Shade::Light => lift(0.25),
    }
}

/// A segment of the right half of a symmetric outline.
#[derive(Clone, Copy)]
enum Seg {
    L(f64, f64),
    C(f64, f64, f64, f64, f64, f64),
}

fn end(s: Seg) -> Point {
    match s {
        Seg::L(x, y) | Seg::C(_, _, _, _, x, y) => Point::new(x, y),
    }
}

/// A closed outline that is the same on both sides of x = 0: from `start`
/// (on the axis) along the right half `right` to a point on the axis, then
/// back along the mirror of the right half.
fn symmetric(start: (f64, f64), right: &[Seg]) -> Path {
    let p = |x: f64, y: f64| Point::new(x, y);
    let m = |q: Point| Point::new(-q.x, q.y);
    let mut path = Path::new().move_to(p(start.0, start.1));
    for s in right {
        path = match *s {
            Seg::L(x, y) => path.line_to(p(x, y)),
            Seg::C(a, b, c, d, x, y) => path.cubic_to(p(a, b), p(c, d), p(x, y)),
        };
    }
    let ends: Vec<Point> = std::iter::once(p(start.0, start.1))
        .chain(right.iter().map(|s| end(*s)))
        .collect();
    for i in (0..right.len()).rev() {
        path = match right[i] {
            Seg::L(..) => path.line_to(m(ends[i])),
            Seg::C(a, b, c, d, _, _) => path.cubic_to(m(p(c, d)), m(p(a, b)), m(ends[i])),
        };
    }
    path.close()
}

fn line(points: &[(f64, f64)]) -> Path {
    let mut path = Path::new().move_to(Point::new(points[0].0, points[0].1));
    for &(x, y) in &points[1..] {
        path = path.line_to(Point::new(x, y));
    }
    path
}

/// A line and its mirror at x = 0.
fn both(points: &[(f64, f64)]) -> [Path; 2] {
    let m: Vec<(f64, f64)> = points.iter().map(|&(x, y)| (-x, y)).collect();
    [line(points), line(&m)]
}

/// The neck rib line of a garment: a curve `d` mm below the neck line
/// from (±w, 0) through (0, depth).
fn neck_line(w: f64, depth: f64) -> Path {
    let p = Point::new;
    Path::new()
        .move_to(p(-w, 0.0))
        .cubic_to(p(-w, depth * 0.6), p(-w * 0.55, depth), p(0.0, depth))
        .cubic_to(p(w * 0.55, depth), p(w, depth * 0.6), p(w, 0.0))
}

/// A fold along a smooth curve through `points`.
fn fold(points: &[(f64, f64)], width: f64, height: f64) -> ProductPart {
    let pts: Vec<(Point, bool)> = points.iter().map(|&(x, y)| (Point::new(x, y), true)).collect();
    ProductPart::Fold {
        path: Path::through_points(&pts, false),
        width,
        height,
    }
}

/// A fold and its mirror at x = 0.
fn folds_both(points: &[(f64, f64)], width: f64, height: f64) -> [ProductPart; 2] {
    let m: Vec<(f64, f64)> = points.iter().map(|&(x, y)| (-x, y)).collect();
    [fold(points, width, height), fold(&m, width, height)]
}

/// The folds of a short-sleeve garment (T-shirt, polo): drape from the
/// arms, folds of the sleeves, and soft folds at the sides and the hem.
/// They stay out of the usual places of the large designs.
fn shirt_folds() -> Vec<ProductPart> {
    let mut v = Vec::new();
    v.extend(folds_both(
        &[(256.0, 268.0), (222.0, 345.0), (192.0, 445.0)],
        22.0,
        -1.6,
    ));
    v.extend(folds_both(&[(250.0, 295.0), (226.0, 375.0), (210.0, 455.0)], 12.0, 0.9));
    // Along the sleeve, and a soft ridge above the sleeve hem.
    v.extend(folds_both(
        &[(252.0, 100.0), (300.0, 148.0), (352.0, 186.0)],
        16.0,
        -1.0,
    ));
    v.extend(folds_both(&[(282.0, 232.0), (318.0, 206.0), (356.0, 182.0)], 10.0, 0.7));
    v.extend(folds_both(
        &[(245.0, 470.0), (240.0, 580.0), (248.0, 680.0)],
        24.0,
        -1.0,
    ));
    v.push(fold(&[(-210.0, 575.0), (-90.0, 598.0), (40.0, 588.0)], 30.0, 1.2));
    v.push(fold(&[(70.0, 630.0), (160.0, 618.0), (225.0, 600.0)], 22.0, -1.0));
    v.push(fold(&[(-150.0, 650.0), (-40.0, 660.0)], 16.0, -0.7));
    v
}

/// A trim part `group` (see [`ProductKind::trims`]) of the shape `path`.
/// It can go a little past the edge of the product: the drawing cuts it
/// at the edge.
fn trim(path: Path, group: u8) -> ProductPart {
    ProductPart::Area(path, Shade::Trim { group, dark: false })
}

/// The rib band round a neck: from the neck line (±`out_w`, depth
/// `out_depth`) in to the neck edge (±`w`, depth `neck`).
fn neck_band(w: f64, neck: f64, out_w: f64, out_depth: f64) -> Path {
    let p = Point::new;
    neck_line(out_w, out_depth)
        .line_to(p(w, 0.0))
        .cubic_to(p(w, neck * 0.5), p(w * 0.53, neck), p(0.0, neck))
        .cubic_to(p(-w * 0.53, neck), p(-w, neck * 0.5), p(-w, 0.0))
        .close()
}

/// A band of a sleeve (a hem, a cuff) as two trim parts: the quadrangle
/// `quad` (a hem line and two points past the opening) and its mirror.
fn sleeve_bands(quad: [(f64, f64); 4], group: u8) -> [ProductPart; 2] {
    both(&quad).map(|p| trim(p.close(), group))
}

/// The area that shows the inside of the back neck through a front neck.
fn inside_neck(w: f64, front: f64, back: f64) -> Path {
    let p = Point::new;
    Path::new()
        .move_to(p(-w, 0.0))
        .cubic_to(p(-w, front * 0.5), p(-w * 0.53, front), p(0.0, front))
        .cubic_to(p(w * 0.53, front), p(w, front * 0.5), p(w, 0.0))
        .cubic_to(p(w * 0.5, back * 0.6), p(w * 0.3, back), p(0.0, back))
        .cubic_to(p(-w * 0.3, back), p(-w * 0.5, back * 0.6), p(-w, 0.0))
        .close()
}

fn circle(x: f64, y: f64, r: f64) -> Path {
    Path::ellipse(Point::new(x, y), r, r)
}

/// The short-sleeve garment body (T-shirt, polo) with a front neck of
/// depth `neck` (a back has a shallow neck). Adult M laid flat: neck 190,
/// shoulders 460, chest 524, sleeves of 216 mm at 30° with an opening of
/// 175 mm, length 725 mm from the neck point.
fn short_sleeves(neck: f64) -> Path {
    use Seg::*;
    let w = 95.0;
    symmetric(
        (0.0, neck),
        &[
            C(w * 0.53, neck, w, neck * 0.5, w, 0.0),
            L(SHOULDER.0, SHOULDER.1),
            L(419.0, 153.0),
            L(331.0, 305.0),
            L(UNDERARM.0, UNDERARM.1),
            C(259.0, 400.0, 257.0, 560.0, 264.0, 715.0),
            C(180.0, 722.0, 90.0, 725.0, 0.0, 725.0),
        ],
    )
}

/// The shoulder point and the underarm point of a short-sleeve garment.
const SHOULDER: (f64, f64) = (230.0, 48.0);
const UNDERARM: (f64, f64) = (262.0, 265.0);

/// The lines of a short-sleeve garment: the sleeve hems, the armhole
/// seams and the hem. `rib` is the line width.
fn short_sleeve_lines(rib: f64) -> Vec<ProductPart> {
    // The sleeve hems are trim part 1 (T-shirt: sleeve hems, polo: cuffs).
    let mut v = Vec::from(sleeve_bands(
        [(400.0, 136.0), (307.0, 297.0), (345.0, 318.0), (440.0, 150.0)],
        1,
    ));
    // 25 mm in from the sleeve opening, a little long: the lines stop at
    // the edge of the product.
    for p in both(&[(400.0, 136.0), (307.0, 297.0)]) {
        v.push(ProductPart::Line(p, rib));
    }
    for p in armholes(SHOULDER, UNDERARM) {
        v.push(ProductPart::Line(p, 1.5));
    }
    v.push(ProductPart::Line(line(&[(-280.0, 690.0), (280.0, 690.0)]), rib));
    v
}

/// The armhole seams from the shoulder point `s` to the underarm point `u`:
/// curves that bend out to the sleeve.
fn armholes(s: (f64, f64), u: (f64, f64)) -> [Path; 2] {
    let p = Point::new;
    let seam = |m: f64| {
        Path::new().move_to(p(m * s.0, s.1)).cubic_to(
            p(m * (s.0 + 14.0), s.1 + (u.1 - s.1) * 0.35),
            p(m * (u.0 - 6.0), s.1 + (u.1 - s.1) * 0.75),
            p(m * u.0, u.1),
        )
    };
    [seam(1.0), seam(-1.0)]
}

/// The point at `k` (0 to 1) of the way from `a` to `z`.
fn at(a: (f64, f64), z: (f64, f64), k: f64) -> (f64, f64) {
    (a.0 + (z.0 - a.0) * k, a.1 + (z.1 - a.1) * k)
}

/// Creases across a long sleeve and their mirrors: the sleeve has the
/// shoulder point `s`, the cuff points `t` (outside) and `b` (inside) and
/// the underarm point `u`; each crease is at `k` of the sleeve length.
fn sleeve_creases(
    s: (f64, f64),
    t: (f64, f64),
    b: (f64, f64),
    u: (f64, f64),
    creases: &[(f64, f64)],
) -> Vec<ProductPart> {
    let mut v = Vec::new();
    for &(k, h) in creases {
        let (top, under) = (at(s, t, k), at(u, b, k));
        v.extend(folds_both(&[at(under, top, 0.08), at(top, under, 0.08)], 12.0, h));
    }
    v
}

/// A long-sleeve fleece top with rib cuffs and a waist rib (sweatshirt,
/// hoodie), adult M laid flat: the sleeves go down at 48°, 140 mm wide at
/// the arm and 100 mm at the rib cuff; the waist rib pulls the hem in. A
/// hoodie has a hood in place of the neck rib.
fn fleece_top(back: bool, hood: bool) -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let rib = 2.0;
    let neck = if back { 22.0 } else { 90.0 };
    let (s, t, b, u) = ((240.0, 55.0), (635.0, 493.0), (561.0, 560.0), (272.0, 300.0));
    let body = symmetric(
        (0.0, neck),
        &[
            C(50.0, neck, 95.0, neck * 0.5, 95.0, 0.0),
            L(s.0, s.1),
            L(t.0, t.1),
            L(b.0, b.1),
            L(u.0, u.1),
            L(266.0, 660.0),
            L(254.0, 722.0),
            C(170.0, 725.0, 90.0, 726.0, 0.0, 726.0),
        ],
    );
    let mut v = vec![Area(body, Shade::Body)];
    match (back, hood) {
        (false, false) => {
            v.push(Area(inside_neck(95.0, 90.0, 22.0), Shade::Dark));
            v.push(trim(neck_band(95.0, 90.0, 117.0, 112.0), 0));
            v.push(Line(neck_line(117.0, 112.0), rib));
        }
        (true, false) => {
            v.push(trim(neck_band(95.0, 22.0, 117.0, 44.0), 0));
            v.push(Line(neck_line(117.0, 44.0), rib));
        }
        (false, true) => v.extend(hood_front()),
        (true, true) => v.extend(hood_back()),
    }
    // The cuff ribs (60 mm) and the waist rib are trim part 1.
    v.extend(sleeve_bands(
        [(599.0, 445.0), (517.0, 518.0), (575.0, 590.0), (665.0, 505.0)],
        1,
    ));
    v.push(trim(
        Path::rect(BBox::new(Point::new(-300.0, 660.0), Point::new(300.0, 740.0))),
        1,
    ));
    // The cuff ribs, the armhole seams and the waist rib.
    for p in both(&[(599.0, 445.0), (517.0, 518.0)]) {
        v.push(Line(p, rib));
    }
    for p in armholes(s, u) {
        v.push(Line(p, 1.5));
    }
    v.push(Line(line(&[(-280.0, 660.0), (280.0, 660.0)]), rib));
    // A pocket across the front of a hoodie.
    if hood && !back {
        let pocket = symmetric(
            (0.0, 430.0),
            &[L(140.0, 430.0), L(205.0, 525.0), L(205.0, 630.0), L(0.0, 630.0)],
        );
        v.push(Area(pocket.clone(), Shade::Body));
        v.push(Line(pocket, rib));
        // The openings of the pocket have a shadow.
        v.extend(folds_both(
            &[(146.0, 440.0), (172.0, 478.0), (198.0, 516.0)],
            10.0,
            -1.4,
        ));
        v.push(fold(&[(-180.0, 600.0), (0.0, 612.0), (180.0, 600.0)], 18.0, -0.7));
    }
    // Drape from the arms, creases across the sleeves at the elbows and
    // above the cuffs, and soft folds above the waist rib.
    v.extend(folds_both(
        &[(266.0, 310.0), (226.0, 385.0), (198.0, 470.0)],
        22.0,
        -1.6,
    ));
    v.extend(folds_both(
        &[(245.0, 480.0), (242.0, 560.0), (250.0, 640.0)],
        22.0,
        -1.0,
    ));
    v.extend(sleeve_creases(
        s,
        t,
        b,
        u,
        &[(0.42, -1.2), (0.48, 1.0), (0.54, -1.0), (0.8, -0.9), (0.85, 0.8)],
    ));
    v.push(fold(&[(-200.0, 615.0), (-80.0, 632.0), (60.0, 625.0)], 24.0, 1.0));
    v.push(fold(&[(90.0, 640.0), (200.0, 630.0)], 16.0, -0.8));
    v
}

/// The hood of a hoodie seen from the front: it lies behind the neck and
/// shows its lining, and its drawstrings hang down.
fn hood_front() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let p = Point::new;
    // The hood behind the neck, and in the neck opening.
    let hood = Path::new()
        .move_to(p(-120.0, 9.5))
        .cubic_to(p(-132.0, -45.0), p(-80.0, -88.0), p(0.0, -88.0))
        .cubic_to(p(80.0, -88.0), p(132.0, -45.0), p(120.0, 9.5))
        .line_to(p(95.0, 0.0))
        .cubic_to(p(95.0, 45.0), p(50.0, 90.0), p(0.0, 90.0))
        .cubic_to(p(-50.0, 90.0), p(-95.0, 45.0), p(-95.0, 0.0))
        .close();
    let lining = symmetric(
        (0.0, -62.0),
        &[
            C(55.0, -62.0, 88.0, -30.0, 82.0, 20.0),
            C(72.0, 50.0, 26.0, 74.0, 0.0, 82.0),
        ],
    );
    let mut v = vec![
        Area(hood.clone(), Shade::Body),
        Line(hood, 1.5),
        Area(lining.clone(), Shade::Dark),
        Area(lining.clone(), Shade::Trim { group: 0, dark: true }),
        Line(lining, 2.0),
    ];
    // The hood edge rolls a little.
    v.push(fold(
        &[(-86.0, 10.0), (-70.0, -45.0), (0.0, -68.0), (70.0, -45.0), (86.0, 10.0)],
        14.0,
        1.4,
    ));
    // The drawstrings, with their ends.
    for m in [-1.0, 1.0] {
        let string =
            Path::new()
                .move_to(p(m * 22.0, 76.0))
                .cubic_to(p(m * 24.0, 130.0), p(m * 30.0, 190.0), p(m * 28.0, 245.0));
        v.push(Line(string, 3.0));
        v.push(Area(
            Path::rect(BBox::new(p(m * 28.0 - 2.5, 240.0), p(m * 28.0 + 2.5, 258.0))),
            Shade::Dark,
        ));
        v.push(Area(circle(m * 22.0, 76.0, 3.5), Shade::Dark));
    }
    v
}

/// The hood of a hoodie seen from the back: it stands behind the head, as
/// at the front, and its lower edge is sewn to the back neck. The back
/// stays free for a back print.
fn hood_back() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    let p = Point::new;
    let hood = Path::new()
        .move_to(p(-120.0, 9.5))
        .cubic_to(p(-132.0, -45.0), p(-80.0, -88.0), p(0.0, -88.0))
        .cubic_to(p(80.0, -88.0), p(132.0, -45.0), p(120.0, 9.5))
        .cubic_to(p(100.0, 26.0), p(52.0, 32.0), p(0.0, 32.0))
        .cubic_to(p(-52.0, 32.0), p(-100.0, 26.0), p(-120.0, 9.5))
        .close();
    let neck_seam = Path::new()
        .move_to(p(-120.0, 9.5))
        .cubic_to(p(-100.0, 26.0), p(-52.0, 32.0), p(0.0, 32.0))
        .cubic_to(p(52.0, 32.0), p(100.0, 26.0), p(120.0, 9.5));
    vec![
        Area(hood.clone(), Shade::Body),
        Line(hood, 1.5),
        Line(neck_seam, 2.0),
        // The seam down the middle of the hood.
        Line(line(&[(0.0, -86.0), (0.0, 31.0)]), 1.5),
        // The two halves of the hood are round, and the hood throws a
        // soft shadow on the back under its edge.
        fold(&[(-52.0, -66.0), (-66.0, -25.0), (-60.0, 18.0)], 48.0, 1.8),
        fold(&[(52.0, -66.0), (66.0, -25.0), (60.0, 18.0)], 48.0, 1.8),
        fold(&[(-110.0, 22.0), (0.0, 40.0), (110.0, 22.0)], 10.0, -1.2),
    ]
}

/// A long-sleeve T-shirt (front or back), adult M laid flat: the body of
/// the T-shirt, sleeves at 48° that narrow to 90 mm, and a hem at the cuff.
fn long_tee(back: bool) -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let rib = 2.0;
    let neck = if back { 22.0 } else { 95.0 };
    let w = 95.0;
    let (s, t, b, u) = (SHOULDER, (631.0, 494.0), (564.0, 554.0), UNDERARM);
    let body = symmetric(
        (0.0, neck),
        &[
            C(w * 0.53, neck, w, neck * 0.5, w, 0.0),
            L(s.0, s.1),
            L(t.0, t.1),
            L(b.0, b.1),
            L(u.0, u.1),
            C(259.0, 400.0, 257.0, 560.0, 264.0, 715.0),
            C(180.0, 722.0, 90.0, 725.0, 0.0, 725.0),
        ],
    );
    let mut v = vec![Area(body, Shade::Body)];
    if back {
        v.push(trim(neck_band(95.0, 22.0, 113.0, 42.0), 0));
        v.push(Line(neck_line(113.0, 42.0), rib));
    } else {
        v.push(Area(inside_neck(95.0, 95.0, 22.0), Shade::Dark));
        v.push(trim(neck_band(95.0, 95.0, 113.0, 113.0), 0));
        v.push(Line(neck_line(113.0, 113.0), rib));
    }
    v.extend(sleeve_bands(
        [(618.0, 471.0), (543.0, 539.0), (580.0, 575.0), (650.0, 505.0)],
        1,
    ));
    // The cuff hems (25 mm), the armhole seams and the hem.
    for p in both(&[(618.0, 471.0), (543.0, 539.0)]) {
        v.push(Line(p, rib));
    }
    for p in armholes(s, u) {
        v.push(Line(p, 1.5));
    }
    v.push(Line(line(&[(-280.0, 690.0), (280.0, 690.0)]), rib));
    v.extend(folds_both(
        &[(256.0, 268.0), (222.0, 345.0), (192.0, 445.0)],
        22.0,
        -1.6,
    ));
    v.extend(folds_both(
        &[(245.0, 470.0), (240.0, 580.0), (248.0, 680.0)],
        24.0,
        -1.0,
    ));
    v.extend(sleeve_creases(
        s,
        t,
        b,
        u,
        &[(0.4, -1.0), (0.46, 0.9), (0.52, -1.0), (0.84, -0.8), (0.89, 0.7)],
    ));
    v.push(fold(&[(-210.0, 575.0), (-90.0, 598.0), (40.0, 588.0)], 30.0, 1.2));
    v.push(fold(&[(70.0, 630.0), (160.0, 618.0), (225.0, 600.0)], 22.0, -1.0));
    v
}

/// A six-panel cap from the front, a little from above: the crown with the
/// two front panels and the side seams up to the button, the eyelets, and
/// the curved brim with its rows of stitches. The origin is the middle of
/// the line where the brim meets the crown.
fn cap() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let p = Point::new;
    let crown = symmetric(
        (0.0, -128.0),
        &[
            C(55.0, -128.0, 98.0, -102.0, 104.0, -48.0),
            C(106.0, -28.0, 106.0, -10.0, 104.0, 10.0),
            L(0.0, 10.0),
        ],
    );
    let brim = symmetric(
        (0.0, 4.0),
        &[
            C(50.0, 4.0, 90.0, 0.0, 102.0, -2.0),
            C(111.0, 2.0, 115.0, 10.0, 113.0, 20.0),
            C(104.0, 44.0, 64.0, 58.0, 0.0, 60.0),
        ],
    );
    // The two front panels between the side seams (trim part 0).
    let front = Path::new()
        .move_to(p(0.0, -126.0))
        .cubic_to(p(42.0, -118.0), p(68.0, -72.0), p(74.0, 3.0))
        .line_to(p(74.0, 12.0))
        .line_to(p(-74.0, 12.0))
        .line_to(p(-74.0, 3.0))
        .cubic_to(p(-68.0, -72.0), p(-42.0, -118.0), p(0.0, -126.0))
        .close();
    let mut v = vec![
        Area(crown, Shade::Body),
        trim(front, 0),
        // The front seam and the side seams from the button to the brim.
        Line(line(&[(0.0, -126.0), (0.0, 4.0)]), 1.5),
    ];
    for m in [-1.0, 1.0] {
        let seam =
            Path::new()
                .move_to(p(0.0, -126.0))
                .cubic_to(p(m * 42.0, -118.0), p(m * 68.0, -72.0), p(m * 74.0, 3.0));
        v.push(Line(seam, 1.5));
        v.push(Area(circle(m * 40.0, -100.0, 3.5), Shade::Dark));
        // The panels are a little round between the seams.
        v.push(fold(
            &[(m * 26.0, -100.0), (m * 36.0, -55.0), (m * 38.0, -10.0)],
            40.0,
            1.2,
        ));
        v.push(fold(
            &[(m * 88.0, -80.0), (m * 94.0, -35.0), (m * 94.0, -5.0)],
            24.0,
            1.0,
        ));
    }
    v.push(Area(brim.clone(), Shade::Body));
    v.push(trim(brim.clone(), 1));
    v.push(Line(brim, 1.5));
    // Rows of stitches along the front edge of the brim: the edge moved in
    // by 6 to 24 mm, square to it.
    let edge = |t: f64| {
        let (p0, p1, p2, p3) = ((113.0, 20.0), (104.0, 44.0), (64.0, 58.0), (0.0, 60.0));
        let u = 1.0 - t;
        let pt =
            |a: f64, b: f64, c: f64, d: f64| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
        let dt = |a: f64, b: f64, c: f64, d: f64| 3.0 * (u * u * (b - a) + 2.0 * u * t * (c - b) + t * t * (d - c));
        let (x, y) = (pt(p0.0, p1.0, p2.0, p3.0), pt(p0.1, p1.1, p2.1, p3.1));
        let (dx, dy) = (dt(p0.0, p1.0, p2.0, p3.0), dt(p0.1, p1.1, p2.1, p3.1));
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        // The edge goes to the left: inside is to its right (up).
        ((x, y), (dy / len, -dx / len))
    };
    for d in [6.0, 12.0, 18.0, 24.0] {
        // From a little in from the tip (where the edge turns) to the middle.
        let half: Vec<(f64, f64)> = (0..=16)
            .map(|i| {
                let ((x, y), (nx, ny)) = edge(0.18 + 0.82 * f64::from(i) / 16.0);
                (x - nx * d, y - ny * d)
            })
            .collect();
        // From the left tip to the middle, then to the right tip.
        let mut row: Vec<(f64, f64)> = half.iter().map(|&(x, y)| (-x, y)).collect();
        row.extend(half.iter().rev().skip(1));
        if row.len() > 1 {
            v.push(Line(line(&row), 0.8));
        }
    }
    // The brim is curved: lighter in the middle.
    v.push(fold(&[(-95.0, 22.0), (0.0, 34.0), (95.0, 22.0)], 30.0, 2.0));
    v.push(Area(circle(0.0, -127.0, 8.0), Shade::Dark));
    v.push(trim(circle(0.0, -127.0, 8.0), 2));
    v
}

/// A bib apron: the bib with its neck strap, the skirt with the ties at
/// the waist and a pocket in two halves. The origin is the middle of the
/// top edge of the bib.
fn apron() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let p = Point::new;
    let rib = 2.0;
    let body = symmetric(
        (0.0, 0.0),
        &[
            L(140.0, 0.0),
            C(145.0, 90.0, 190.0, 190.0, 300.0, 250.0),
            L(300.0, 860.0),
            L(0.0, 860.0),
        ],
    );
    let strap = Path::new()
        .move_to(p(-122.0, 0.0))
        .cubic_to(p(-122.0, -235.0), p(122.0, -235.0), p(122.0, 0.0));
    let pocket = Path::rect(BBox::new(p(-230.0, 470.0), p(230.0, 650.0)));
    let mut v = vec![
        Area(body, Shade::Body),
        Line(strap, 22.0),
        Line(line(&[(-150.0, 18.0), (150.0, 18.0)]), rib),
        Line(line(&[(-310.0, 262.0), (310.0, 262.0)]), rib),
        Line(line(&[(-310.0, 840.0), (310.0, 840.0)]), rib),
        Area(pocket.clone(), Shade::Body),
        trim(pocket.clone(), 0),
        Line(pocket, rib),
        Line(line(&[(0.0, 470.0), (0.0, 650.0)]), rib),
    ];
    for m in [-1.0, 1.0] {
        let tie = Path::new().move_to(p(m * 296.0, 262.0)).cubic_to(
            p(m * 360.0, 270.0),
            p(m * 400.0, 300.0),
            p(m * 410.0, 420.0),
        );
        v.push(Line(tie, 20.0));
        // Soft folds down the skirt.
        v.push(fold(
            &[(m * 215.0, 300.0), (m * 222.0, 560.0), (m * 210.0, 830.0)],
            40.0,
            1.6,
        ));
        v.push(fold(
            &[(m * 120.0, 680.0), (m * 128.0, 760.0), (m * 118.0, 835.0)],
            26.0,
            -1.2,
        ));
    }
    // The pocket bulges a little.
    v.push(fold(&[(-200.0, 645.0), (0.0, 655.0), (200.0, 645.0)], 14.0, -1.0));
    v
}

/// A knitted beanie with a cuff, from the front. The origin is the middle
/// of its top.
fn beanie() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let body = symmetric(
        (0.0, 0.0),
        &[C(75.0, 0.0, 112.0, 45.0, 112.0, 120.0), L(115.0, 240.0), L(0.0, 240.0)],
    );
    let mut v = vec![
        Area(body, Shade::Body),
        trim(
            Path::rect(BBox::new(Point::new(-130.0, 160.0), Point::new(130.0, 250.0))),
            0,
        ),
        Line(line(&[(-120.0, 160.0), (120.0, 160.0)]), 2.5),
    ];
    // The cuff folds over: a soft roll at its top.
    v.push(fold(&[(-112.0, 167.0), (0.0, 167.0), (112.0, 167.0)], 12.0, 2.0));
    // The knit ribs, on the cuff and up to the crown.
    for i in -14i32..=14 {
        let x = f64::from(i) * 8.0;
        v.push(fold(&[(x, 172.0), (x, 238.0)], 4.0, 0.35));
        if i.abs() <= 13 {
            v.push(fold(&[(x * 0.4, 14.0), (x * 0.85, 70.0), (x, 154.0)], 4.0, 0.3));
        }
    }
    v
}

/// A messenger bag from the front: the body, the flap with a row of
/// stitches, the strap and two closing straps with buckles. The origin is
/// the middle of the top edge of the body.
fn messenger() -> Vec<ProductPart> {
    use ProductPart::{Area, Line};
    use Seg::*;
    let p = Point::new;
    let body = Path::rect(BBox::new(p(-180.0, 0.0), p(180.0, 300.0)));
    let flap = symmetric(
        (0.0, -5.0),
        &[
            L(185.0, -5.0),
            L(185.0, 175.0),
            C(185.0, 198.0, 172.0, 208.0, 150.0, 208.0),
            L(0.0, 208.0),
        ],
    );
    let stitches = symmetric(
        (0.0, 2.0),
        &[
            L(177.0, 2.0),
            L(177.0, 174.0),
            C(177.0, 192.0, 168.0, 200.0, 150.0, 200.0),
            L(0.0, 200.0),
        ],
    );
    let strap = Path::new()
        .move_to(p(-165.0, 0.0))
        .cubic_to(p(-165.0, -320.0), p(165.0, -320.0), p(165.0, 0.0));
    let mut v = vec![
        Area(body, Shade::Body),
        Line(strap, 32.0),
        // The bottom of the bag sags a little.
        fold(&[(-160.0, 285.0), (0.0, 292.0), (160.0, 285.0)], 20.0, -1.0),
        Area(flap.clone(), Shade::Body),
        trim(flap.clone(), 0),
        Line(flap, 2.0),
        Line(stitches, 0.8),
        // The flap throws a shadow on the bag under its edge.
        fold(&[(-178.0, 214.0), (0.0, 214.0), (178.0, 214.0)], 10.0, -1.6),
    ];
    for m in [-1.0, 1.0] {
        let band = Path::rect(BBox::new(p(m * 135.0 - 10.0, 150.0), p(m * 135.0 + 10.0, 250.0)));
        v.push(Area(band.clone(), Shade::Body));
        v.push(trim(band.clone(), 1));
        v.push(Line(band, 1.5));
        v.push(Area(
            Path::rect(BBox::new(p(m * 135.0 - 13.0, 195.0), p(m * 135.0 + 13.0, 220.0))),
            Shade::Dark,
        ));
    }
    v
}

impl ProductKind {
    /// Each side of each product.
    pub const ALL: [ProductKind; 17] = [
        ProductKind::TShirt,
        ProductKind::TShirtBack,
        ProductKind::LongSleeve,
        ProductKind::LongSleeveBack,
        ProductKind::Polo,
        ProductKind::PoloBack,
        ProductKind::Sweatshirt,
        ProductKind::SweatshirtBack,
        ProductKind::Hoodie,
        ProductKind::HoodieBack,
        ProductKind::Cap,
        ProductKind::Beanie,
        ProductKind::Apron,
        ProductKind::Tote,
        ProductKind::Messenger,
        ProductKind::Towel,
        ProductKind::Cushion,
    ];

    /// The products, each at its front: the list that the user chooses from.
    pub const PRODUCTS: [ProductKind; 12] = [
        ProductKind::TShirt,
        ProductKind::LongSleeve,
        ProductKind::Polo,
        ProductKind::Sweatshirt,
        ProductKind::Hoodie,
        ProductKind::Cap,
        ProductKind::Beanie,
        ProductKind::Apron,
        ProductKind::Tote,
        ProductKind::Messenger,
        ProductKind::Towel,
        ProductKind::Cushion,
    ];

    /// The name of this side of the product.
    pub fn name(self) -> &'static str {
        match self {
            ProductKind::TShirt => "T-shirt, front",
            ProductKind::TShirtBack => "T-shirt, back",
            ProductKind::Polo => "Polo shirt",
            ProductKind::PoloBack => "Polo shirt, back",
            ProductKind::Sweatshirt => "Sweatshirt",
            ProductKind::SweatshirtBack => "Sweatshirt, back",
            ProductKind::LongSleeve => "Long-sleeve T-shirt",
            ProductKind::LongSleeveBack => "Long-sleeve T-shirt, back",
            ProductKind::Hoodie => "Hoodie",
            ProductKind::HoodieBack => "Hoodie, back",
            ProductKind::Apron => "Apron",
            ProductKind::Beanie => "Beanie",
            ProductKind::Messenger => "Messenger bag",
            ProductKind::Cap => "Cap",
            ProductKind::Tote => "Tote bag",
            ProductKind::Towel => "Bath towel",
            ProductKind::Cushion => "Cushion",
        }
    }

    /// The name of the product (both sides).
    pub fn product_name(self) -> &'static str {
        match self.front() {
            ProductKind::TShirt => "T-shirt",
            k => k.name(),
        }
    }

    /// The names of the trim parts of the product (the same on both
    /// sides): `Shade::Trim { group, .. }` is the part `group`. The trim
    /// color goes on them (P16 Milestone 6).
    pub fn trims(self) -> &'static [&'static str] {
        match self.front() {
            ProductKind::TShirt => &["Neck rib", "Sleeve hems"],
            ProductKind::LongSleeve => &["Neck rib", "Cuffs"],
            ProductKind::Polo => &["Collar", "Sleeve cuffs"],
            ProductKind::Sweatshirt => &["Neck rib", "Cuffs and waist rib"],
            ProductKind::Hoodie => &["Hood lining", "Cuffs and waist rib"],
            ProductKind::Cap => &["Front panels", "Brim", "Button"],
            ProductKind::Beanie => &["Cuff"],
            ProductKind::Apron => &["Pocket"],
            ProductKind::Messenger => &["Flap", "Closing straps"],
            ProductKind::Towel => &["Border"],
            _ => &[],
        }
    }

    /// The side of the product that this kind shows.
    pub fn side(self) -> Side {
        match self {
            ProductKind::TShirtBack
            | ProductKind::PoloBack
            | ProductKind::SweatshirtBack
            | ProductKind::LongSleeveBack
            | ProductKind::HoodieBack => Side::Back,
            _ => Side::Front,
        }
    }

    /// The other side of the product, if it has two.
    pub fn turned(self) -> Option<ProductKind> {
        Some(match self {
            ProductKind::TShirt => ProductKind::TShirtBack,
            ProductKind::TShirtBack => ProductKind::TShirt,
            ProductKind::Polo => ProductKind::PoloBack,
            ProductKind::PoloBack => ProductKind::Polo,
            ProductKind::Sweatshirt => ProductKind::SweatshirtBack,
            ProductKind::SweatshirtBack => ProductKind::Sweatshirt,
            ProductKind::LongSleeve => ProductKind::LongSleeveBack,
            ProductKind::LongSleeveBack => ProductKind::LongSleeve,
            ProductKind::Hoodie => ProductKind::HoodieBack,
            ProductKind::HoodieBack => ProductKind::Hoodie,
            _ => return None,
        })
    }

    /// The front of the product.
    pub fn front(self) -> ProductKind {
        match self.side() {
            Side::Front => self,
            Side::Back => self.turned().unwrap_or(self),
        }
    }

    /// The side `side` of the product (the same kind if it has one side).
    pub fn on(self, side: Side) -> ProductKind {
        match self.turned() {
            Some(t) if self.side() != side => t,
            _ => self,
        }
    }

    /// The drawing of the product: the first part is the outline (the body).
    pub fn parts(self) -> Vec<ProductPart> {
        use ProductPart::{Area, Line};
        let rib = 2.0;
        match self {
            ProductKind::TShirt => {
                let mut v = vec![
                    Area(short_sleeves(95.0), Shade::Body),
                    Area(inside_neck(95.0, 95.0, 22.0), Shade::Dark),
                    trim(neck_band(95.0, 95.0, 113.0, 113.0), 0),
                    Line(neck_line(113.0, 113.0), rib),
                ];
                v.extend(short_sleeve_lines(rib));
                v.extend(shirt_folds());
                v
            }
            ProductKind::TShirtBack => {
                let mut v = vec![
                    Area(short_sleeves(22.0), Shade::Body),
                    trim(neck_band(95.0, 22.0, 113.0, 42.0), 0),
                    Line(neck_line(113.0, 42.0), rib),
                ];
                v.extend(short_sleeve_lines(rib));
                v.extend(shirt_folds());
                v
            }
            ProductKind::Polo => {
                // A collar point lies on the chest: its fold goes round the
                // neck from the shoulder to the top of the placket, and its
                // outer edge goes back to the shoulder.
                let wing = |s: f64| {
                    let p = |x: f64, y: f64| Point::new(s * x, y);
                    Path::new()
                        .move_to(p(96.0, -4.0))
                        .cubic_to(p(72.0, 14.0), p(28.0, 58.0), p(6.0, 94.0))
                        .line_to(p(74.0, 132.0))
                        .cubic_to(p(96.0, 92.0), p(114.0, 44.0), p(121.0, 10.0))
                        .cubic_to(p(114.0, 3.0), p(105.0, -2.0), p(96.0, -4.0))
                        .close()
                };
                // The collar stands round the back of the neck.
                let p = Point::new;
                let back = Path::new()
                    .move_to(p(-121.0, 10.0))
                    .cubic_to(p(-80.0, -14.0), p(80.0, -14.0), p(121.0, 10.0))
                    .line_to(p(96.0, -4.0))
                    .cubic_to(p(55.0, 34.0), p(-55.0, 34.0), p(-96.0, -4.0))
                    .close();
                let placket = Path::rect(BBox::new(p(-16.0, 90.0), p(16.0, 250.0)));
                let mut v = vec![
                    Area(short_sleeves(95.0), Shade::Body),
                    Area(back.clone(), Shade::Body),
                    trim(back.clone(), 0),
                    Line(back, 1.5),
                    Area(inside_neck(95.0, 95.0, 22.0), Shade::Dark),
                    Area(placket.clone(), Shade::Body),
                    Line(placket, rib),
                    Area(wing(1.0), Shade::Body),
                    Area(wing(-1.0), Shade::Body),
                    trim(wing(1.0), 0),
                    trim(wing(-1.0), 0),
                    Line(wing(1.0), 1.5),
                    Line(wing(-1.0), 1.5),
                ];
                v.extend(short_sleeve_lines(rib));
                for y in [125.0, 175.0, 225.0] {
                    v.push(Area(circle(0.0, y, 5.0), Shade::Dark));
                }
                // The collar points roll a little over the chest.
                v.extend(folds_both(&[(104.0, 8.0), (72.0, 60.0), (44.0, 104.0)], 30.0, 1.4));
                v.extend(shirt_folds());
                v
            }
            ProductKind::PoloBack => {
                // From the back, the collar is a band round the neck.
                let p = Point::new;
                let collar = Path::new()
                    .move_to(p(-121.0, 10.0))
                    .cubic_to(p(-80.0, -14.0), p(80.0, -14.0), p(121.0, 10.0))
                    .cubic_to(p(70.0, 44.0), p(-70.0, 44.0), p(-121.0, 10.0))
                    .close();
                let mut v = vec![
                    Area(short_sleeves(22.0), Shade::Body),
                    Area(collar.clone(), Shade::Body),
                    trim(collar.clone(), 0),
                    Line(collar, rib),
                ];
                v.extend(short_sleeve_lines(rib));
                // The collar rolls over, and its edge has a shadow under it.
                v.push(fold(
                    &[(-112.0, 6.0), (-60.0, 12.0), (0.0, 14.0), (60.0, 12.0), (112.0, 6.0)],
                    20.0,
                    1.6,
                ));
                v.push(fold(
                    &[(-108.0, 20.0), (-60.0, 36.0), (0.0, 40.0), (60.0, 36.0), (108.0, 20.0)],
                    10.0,
                    -1.6,
                ));
                v.extend(shirt_folds());
                v
            }
            ProductKind::Sweatshirt => fleece_top(false, false),
            ProductKind::SweatshirtBack => fleece_top(true, false),
            ProductKind::Hoodie => fleece_top(false, true),
            ProductKind::HoodieBack => fleece_top(true, true),
            ProductKind::LongSleeve => long_tee(false),
            ProductKind::LongSleeveBack => long_tee(true),
            ProductKind::Apron => apron(),
            ProductKind::Beanie => beanie(),
            ProductKind::Messenger => messenger(),
            ProductKind::Cap => cap(),
            ProductKind::Tote => {
                let body = Path::rect(BBox::new(Point::new(-190.0, 0.0), Point::new(190.0, 420.0)));
                let handle = Path::new().move_to(Point::new(-100.0, 0.0)).cubic_to(
                    Point::new(-100.0, -190.0),
                    Point::new(100.0, -190.0),
                    Point::new(100.0, 0.0),
                );
                let mut v = vec![
                    Area(body, Shade::Body),
                    Line(handle, 25.0),
                    Line(line(&[(-190.0, 30.0), (190.0, 30.0)]), rib),
                ];
                // The handles pull the top; the bottom sags a little.
                v.extend(folds_both(&[(100.0, 34.0), (122.0, 62.0), (150.0, 84.0)], 14.0, -0.9));
                v.push(fold(&[(-170.0, 395.0), (0.0, 405.0), (170.0, 395.0)], 24.0, -1.0));
                v
            }
            ProductKind::Towel => {
                let body = Path::rect(BBox::new(Point::new(-250.0, 0.0), Point::new(250.0, 900.0)));
                let border = Path::rect(BBox::new(Point::new(-250.0, 760.0), Point::new(250.0, 830.0)));
                let mut v = vec![
                    Area(body, Shade::Body),
                    Area(border.clone(), Shade::Light),
                    trim(border, 0),
                    Line(line(&[(-250.0, 760.0), (250.0, 760.0)]), rib),
                    Line(line(&[(-250.0, 830.0), (250.0, 830.0)]), rib),
                    Line(line(&[(-250.0, 885.0), (250.0, 885.0)]), rib),
                ];
                // A towel on a rail hangs in soft waves at its sides.
                v.extend(folds_both(&[(228.0, 40.0), (224.0, 400.0), (230.0, 740.0)], 34.0, 1.8));
                v.extend(folds_both(&[(170.0, 60.0), (175.0, 300.0), (168.0, 560.0)], 30.0, -1.2));
                v
            }
            ProductKind::Cushion => {
                let body = Path::rect(BBox::new(Point::new(-225.0, 0.0), Point::new(225.0, 450.0)));
                let mut v = vec![Area(body.clone(), Shade::Body), Line(body, 6.0)];
                // The corners pinch where the filling pulls the cover.
                for (sx, sy) in [(1.0, 0.0), (-1.0, 0.0), (1.0, 450.0), (-1.0, 450.0)] {
                    let dy = if sy > 0.0 { -1.0 } else { 1.0 };
                    v.push(fold(
                        &[
                            (sx * 222.0, sy + dy * 3.0),
                            (sx * 200.0, sy + dy * 25.0),
                            (sx * 178.0, sy + dy * 50.0),
                        ],
                        12.0,
                        -1.4,
                    ));
                }
                v
            }
        }
    }

    /// The usual places for a design on the product.
    pub fn placements(self) -> &'static [Placement] {
        const fn pl(name: &'static str, x: f64, y: f64, w: f64, h: f64) -> Placement {
            Placement {
                name,
                at: Point { x, y },
                max: (w, h),
            }
        }
        // Left chest is on the left of the wearer: the right of the picture.
        const SHIRT: &[Placement] = &[
            pl("Left chest", 110.0, 190.0, 100.0, 100.0),
            pl("Center chest", 0.0, 190.0, 250.0, 150.0),
            pl("Full front", 0.0, 330.0, 300.0, 350.0),
            pl("Sleeve", 310.0, 190.0, 40.0, 40.0),
        ];
        const BACK: &[Placement] = &[
            pl("Below the collar", 0.0, 80.0, 100.0, 50.0),
            pl("Upper back", 0.0, 170.0, 300.0, 120.0),
            pl("Full back", 0.0, 330.0, 330.0, 380.0),
        ];
        const POLO_BACK: &[Placement] = &[
            pl("Below the collar", 0.0, 85.0, 100.0, 50.0),
            pl("Upper back", 0.0, 175.0, 300.0, 120.0),
            pl("Full back", 0.0, 330.0, 330.0, 380.0),
        ];
        const POLO: &[Placement] = &[
            pl("Left chest", 115.0, 200.0, 100.0, 90.0),
            pl("Right chest", -115.0, 200.0, 100.0, 90.0),
            pl("Sleeve", 310.0, 190.0, 40.0, 40.0),
        ];
        const SWEAT: &[Placement] = &[
            pl("Left chest", 115.0, 200.0, 100.0, 100.0),
            pl("Center chest", 0.0, 200.0, 280.0, 160.0),
            pl("Full front", 0.0, 350.0, 320.0, 350.0),
            pl("Sleeve", 325.0, 250.0, 40.0, 40.0),
        ];
        const CAP: &[Placement] = &[pl("Front", 0.0, -60.0, 110.0, 55.0)];
        const LONG: &[Placement] = &[
            pl("Left chest", 110.0, 190.0, 100.0, 100.0),
            pl("Center chest", 0.0, 190.0, 250.0, 150.0),
            pl("Full front", 0.0, 330.0, 300.0, 350.0),
            pl("Upper sleeve", 305.0, 215.0, 40.0, 40.0),
            pl("Lower sleeve", 490.0, 413.0, 50.0, 50.0),
        ];
        const HOODIE: &[Placement] = &[
            pl("Left chest", 115.0, 200.0, 100.0, 100.0),
            pl("Center chest", 0.0, 250.0, 250.0, 140.0),
            pl("Sleeve", 325.0, 250.0, 40.0, 40.0),
        ];
        const HOODIE_BACK: &[Placement] = &[
            pl("Below the hood", 0.0, 90.0, 100.0, 50.0),
            pl("Upper back", 0.0, 180.0, 300.0, 120.0),
            pl("Full back", 0.0, 340.0, 330.0, 380.0),
        ];
        const APRON: &[Placement] = &[
            pl("Bib", 0.0, 130.0, 200.0, 110.0),
            pl("Center", 0.0, 370.0, 300.0, 160.0),
            pl("Pocket", 115.0, 560.0, 140.0, 120.0),
        ];
        const BEANIE: &[Placement] = &[
            pl("Cuff", 0.0, 200.0, 110.0, 50.0),
            pl("Above the cuff", 0.0, 105.0, 90.0, 60.0),
        ];
        const MESSENGER: &[Placement] = &[pl("Flap", 0.0, 95.0, 240.0, 110.0)];
        const TOTE: &[Placement] = &[pl("Center", 0.0, 230.0, 280.0, 280.0)];
        const TOWEL: &[Placement] = &[
            pl("On the border", 0.0, 795.0, 420.0, 60.0),
            pl("Above the border", 0.0, 690.0, 300.0, 110.0),
        ];
        const CUSHION: &[Placement] = &[pl("Center", 0.0, 225.0, 320.0, 320.0)];
        match self {
            ProductKind::TShirt => SHIRT,
            ProductKind::TShirtBack | ProductKind::SweatshirtBack | ProductKind::LongSleeveBack => BACK,
            ProductKind::LongSleeve => LONG,
            ProductKind::Hoodie => HOODIE,
            ProductKind::HoodieBack => HOODIE_BACK,
            ProductKind::Apron => APRON,
            ProductKind::Beanie => BEANIE,
            ProductKind::Messenger => MESSENGER,
            ProductKind::PoloBack => POLO_BACK,
            ProductKind::Polo => POLO,
            ProductKind::Sweatshirt => SWEAT,
            ProductKind::Cap => CAP,
            ProductKind::Tote => TOTE,
            ProductKind::Towel => TOWEL,
            ProductKind::Cushion => CUSHION,
        }
    }

    /// Where a point of a flat design (product coordinates) shows on the
    /// product. A cap curves the design round its crown: it is a little
    /// narrower at its sides, and its sides are a little lower. The other
    /// products are flat.
    pub fn warp(self, p: Point) -> Point {
        if self != ProductKind::Cap {
            return p;
        }
        const R: f64 = 115.0;
        let x = p.x.clamp(-R, R);
        let dip = R - (R * R - x * x).sqrt();
        Point::new(R * emb_geom::math::sin(x / R), p.y + 0.12 * dip)
    }

    /// The box of the drawing, in product coordinates.
    pub fn bbox(self) -> BBox {
        self.parts()
            .iter()
            .filter_map(|p| match p {
                // A trim part can go past the edge (the drawing cuts it).
                ProductPart::Area(_, Shade::Trim { .. }) | ProductPart::Fold { .. } => None,
                ProductPart::Area(path, _) | ProductPart::Line(path, _) => path.control_bbox(),
            })
            .reduce(BBox::union)
            .unwrap_or_else(|| BBox::new(Point::ZERO, Point::ZERO))
    }
}

impl ProductView {
    pub fn new(kind: ProductKind, color: Rgb) -> Self {
        ProductView {
            kind,
            color,
            origin: Point::ZERO,
            visible: true,
            picture: None,
            extras: Vec::new(),
            both_sides: false,
            trim: None,
        }
    }

    /// The color of a part of the shade `s`, or `None` for a trim part
    /// that shows not (no trim color, or the part is off).
    pub fn part_color(&self, s: Shade) -> Option<Rgb> {
        match s {
            Shade::Trim { group, .. } => {
                let t = self.trim.as_ref()?;
                let name = self.kind.trims().get(usize::from(group))?;
                (!t.off.iter().any(|o| o == name)).then(|| shade(t.color, s))
            }
            _ => Some(shade(self.color, s)),
        }
    }

    /// True if the trim part `name` of the product has the trim color.
    pub fn trim_on(&self, name: &str) -> bool {
        self.trim.as_ref().is_some_and(|t| !t.off.iter().any(|o| o == name))
    }

    /// Gives the trim part `name` the trim color (`on`) or the product color.
    pub fn set_trim_on(&mut self, name: &str, on: bool) {
        if let Some(t) = self.trim.as_mut() {
            t.off.retain(|o| o != name);
            if !on {
                t.off.push(name.to_string());
            }
        }
    }

    /// The side of the product that shows (and that the open design is on).
    pub fn side(&self) -> Side {
        if self.picture.is_some() {
            Side::Front
        } else {
            self.kind.side()
        }
    }

    /// The placement of the other design `e` (on its side), if any.
    pub fn placement_of_extra(&self, e: &ProductExtra) -> Option<&'static Placement> {
        if self.picture.is_some() {
            return None;
        }
        self.kind
            .on(e.side)
            .placements()
            .iter()
            .find(|p| p.at.distance(e.at) < 1.0)
    }

    /// Shows the other side of a garment. The open design goes to the
    /// placement of the same name on that side, else to a free place.
    pub fn turn(&mut self, design: &Design) {
        let Some(other) = self.kind.turned().filter(|_| self.picture.is_none()) else {
            return;
        };
        let name = self.placement_of(design).map(|p| p.name);
        self.kind = other;
        let at = name
            .and_then(|n| other.placements().iter().find(|p| p.name == n))
            .map_or_else(|| self.free_place(design), |p| p.at);
        self.put_design_at(design, at);
    }

    /// The placement at `at` (product coordinates, within 1 mm), if any.
    pub fn placement_at(&self, at: Point) -> Option<&'static Placement> {
        if self.picture.is_some() {
            return None;
        }
        self.kind.placements().iter().find(|p| p.at.distance(at) < 1.0)
    }

    /// A place for one more design on the side that shows: the first
    /// placement that has no design (the open design or an extra), else the
    /// middle of the product.
    pub fn free_place(&self, design: &Design) -> Point {
        self.free_place_on(self.side(), design)
    }

    /// As [`ProductView::free_place`], on the side `side`.
    pub fn free_place_on(&self, side: Side, design: &Design) -> Point {
        if self.picture.is_some() {
            return self.bbox().center() - self.origin;
        }
        let kind = self.kind.on(side);
        let main = if side == self.side() {
            self.placement_of(design).map(|p| p.name)
        } else {
            None
        };
        let free = kind
            .placements()
            .iter()
            .find(|p| Some(p.name) != main && !self.extras.iter().any(|e| e.side == side && e.at.distance(p.at) < 1.0));
        free.map_or_else(|| kind.bbox().center(), |p| p.at)
    }

    /// Puts the other design `i` on the side `side`: at the placement of the
    /// same name, else at a free place.
    pub fn set_extra_side(&mut self, i: usize, side: Side, design: &Design) {
        let Some(e) = self.extras.get(i) else { return };
        if e.side == side {
            return;
        }
        let name = self.placement_of_extra(e).map(|p| p.name);
        let at = name
            .and_then(|n| self.kind.on(side).placements().iter().find(|p| p.name == n))
            .map(|p| p.at)
            .filter(|at| !self.extras.iter().any(|o| o.side == side && o.at.distance(*at) < 1.0))
            .unwrap_or_else(|| self.free_place_on(side, design));
        let e = &mut self.extras[i];
        e.side = side;
        e.at = at;
    }

    /// Changes the kind of the product. Each extra at a placement goes to
    /// the placement of the same name on its side of the new kind, if it
    /// has one.
    pub fn change_kind(&mut self, kind: ProductKind) {
        for e in &mut self.extras {
            let name = self
                .kind
                .on(e.side)
                .placements()
                .iter()
                .find(|p| p.at.distance(e.at) < 1.0)
                .map(|p| p.name);
            if let Some(p) = name.and_then(|n| kind.on(e.side).placements().iter().find(|p| p.name == n)) {
                e.at = p.at;
            }
        }
        self.kind = kind;
    }

    /// The box of the product on the canvas (view coordinates).
    pub fn bbox(&self) -> BBox {
        match &self.picture {
            Some(p) => BBox::new(self.origin, self.origin + Point::new(p.width_mm, p.height_mm())),
            None => {
                let b = self.kind.bbox();
                BBox::new(b.min + self.origin, b.max + self.origin)
            }
        }
    }

    /// Moves the product so that the place `at` (product coordinates) is
    /// under the center of the design.
    pub fn put_design_at(&mut self, design: &Design, at: Point) {
        let c = design.view_extents().map_or(design.hoop.center, |b| b.center());
        self.origin = c - at;
    }

    /// The placement that the design center is on (within 1 mm), if any.
    pub fn placement_of(&self, design: &Design) -> Option<&'static Placement> {
        if self.picture.is_some() {
            return None;
        }
        let c = design.view_extents()?.center();
        self.kind
            .placements()
            .iter()
            .find(|p| (p.at + self.origin).distance(c) < 1.0)
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::region::ring_contains;

    use super::*;
    use crate::{ObjectKind, ThreadId};

    fn outline(kind: ProductKind) -> Vec<Point> {
        match &kind.parts()[0] {
            ProductPart::Area(p, Shade::Body) => p.flatten(0.2).into_iter().flat_map(|pl| pl.points).collect(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn each_trim_part_is_drawn_and_a_trim_color_is_saved() {
        for kind in ProductKind::ALL {
            let groups = |k: ProductKind| -> Vec<u8> {
                k.parts()
                    .iter()
                    .filter_map(|p| match p {
                        ProductPart::Area(_, Shade::Trim { group, .. }) => Some(*group),
                        _ => None,
                    })
                    .collect()
            };
            let here = groups(kind);
            assert!(here.iter().all(|&g| usize::from(g) < kind.trims().len()), "{kind:?}");
            // Each name has a part on one side of the product at least.
            let both: Vec<u8> = [kind, kind.turned().unwrap_or(kind)]
                .into_iter()
                .flat_map(groups)
                .collect();
            for g in 0..kind.trims().len() {
                assert!(both.contains(&(g as u8)), "{kind:?}: no part {}", kind.trims()[g]);
            }
        }
        let mut v = ProductView::new(ProductKind::Cap, Rgb(30, 38, 66));
        let front = Shade::Trim { group: 0, dark: false };
        assert_eq!(v.part_color(front), None);
        v.trim = Some(Trim {
            color: Rgb(250, 250, 250),
            off: Vec::new(),
        });
        v.set_trim_on("Brim", false);
        assert_eq!(v.part_color(front), Some(Rgb(250, 250, 250)));
        assert_eq!(v.part_color(Shade::Trim { group: 1, dark: false }), None);
        assert_eq!(v.part_color(Shade::Body), Some(Rgb(30, 38, 66)));
        let mut d = Design::new("t");
        d.product = Some(v.clone());
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert_eq!(back.product, Some(v));
    }

    #[test]
    fn each_placement_and_its_largest_design_is_on_the_product() {
        for kind in ProductKind::ALL {
            let ring = outline(kind);
            assert!(!kind.placements().is_empty(), "{kind:?}");
            for p in kind.placements() {
                let (hw, hh) = (p.max.0 / 2.0, p.max.1 / 2.0);
                for q in [
                    p.at,
                    p.at + Point::new(-hw, -hh),
                    p.at + Point::new(hw, -hh),
                    p.at + Point::new(hw, hh),
                    p.at + Point::new(-hw, hh),
                ] {
                    assert!(ring_contains(&ring, q), "{kind:?} {}: {q:?}", p.name);
                }
            }
        }
    }

    #[test]
    fn garments_are_about_the_size_of_an_adult_m() {
        let b = ProductKind::TShirt.bbox();
        // Sleeve to sleeve and the length from the neck point.
        assert!((b.width() - 838.0).abs() < 1.0, "{b:?}");
        assert!((b.max.y - 725.0).abs() < 1.0);
        // The outline of a symmetric product is symmetric.
        assert!((b.min.x + b.max.x).abs() < 1e-9);
    }

    #[test]
    fn put_design_at_a_placement() {
        let mut d = Design::new("t");
        d.add_object(
            ThreadId(0),
            ObjectKind::Fill {
                outer: Path::rect(BBox::from_center_size(Point::new(30.0, 10.0), 60.0, 40.0)),
                holes: Vec::new(),
                params: Default::default(),
            },
        );
        let mut v = ProductView::new(ProductKind::TShirt, Rgb(255, 255, 255));
        let left_chest = ProductKind::TShirt.placements()[0];
        v.put_design_at(&d, left_chest.at);
        assert_eq!(v.origin + left_chest.at, Point::new(30.0, 10.0));
        assert_eq!(v.placement_of(&d).map(|p| p.name), Some("Left chest"));
        // The product box moves with the origin.
        assert_eq!(v.bbox().min, ProductKind::TShirt.bbox().min + v.origin);
    }

    fn square(x: f64, y: f64, side: f64) -> Design {
        let mut d = Design::new("logo");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::from_center_size(Point::new(x, y), side, side)),
                holes: Vec::new(),
                params: Default::default(),
            },
        );
        d
    }

    #[test]
    fn extras_go_to_free_places_and_keep_them_on_another_kind() {
        let main = square(30.0, 10.0, 60.0);
        let mut v = ProductView::new(ProductKind::Polo, Rgb(255, 255, 255));
        v.put_design_at(&main, ProductKind::Polo.placements()[0].at);
        // The left chest has the open design: the next place is the right chest.
        let at = v.free_place(&main);
        assert_eq!(v.placement_at(at).map(|p| p.name), Some("Right chest"));
        v.extras
            .push(ProductExtra::new("name.dst", square(500.0, 500.0, 30.0), at));
        let at = v.free_place(&main);
        assert_eq!(v.placement_at(at).map(|p| p.name), Some("Sleeve"));
        v.extras
            .push(ProductExtra::new("sleeve.dst", square(0.0, 0.0, 30.0), at));
        // The extra is centered on its place, whatever its own position.
        let e = &v.extras[0];
        assert_eq!(e.center(), Point::new(500.0, 500.0));
        assert_eq!(e.size(), Some((30.0, 30.0)));
        // A T-shirt has a sleeve too, but no right chest.
        let right = v.extras[0].at;
        v.change_kind(ProductKind::TShirt);
        assert_eq!(v.placement_at(v.extras[1].at).map(|p| p.name), Some("Sleeve"));
        assert_eq!(v.extras[0].at, right);
    }

    #[test]
    fn a_garment_has_two_sides_and_turns() {
        for kind in ProductKind::ALL {
            assert_eq!(kind.on(Side::Front).side(), Side::Front, "{kind:?}");
            match kind.turned() {
                Some(t) => {
                    assert_eq!(t.turned(), Some(kind));
                    assert_ne!(t.side(), kind.side());
                    assert_eq!(kind.on(Side::Back).side(), Side::Back);
                }
                None => assert_eq!(kind.on(Side::Back), kind),
            }
        }
        assert!(ProductKind::PRODUCTS.iter().all(|k| k.side() == Side::Front));
        let main = square(30.0, 10.0, 60.0);
        let mut v = ProductView::new(ProductKind::Polo, Rgb(255, 255, 255));
        v.put_design_at(&main, ProductKind::Polo.placements()[0].at);
        v.turn(&main);
        assert_eq!(v.kind, ProductKind::PoloBack);
        assert_eq!(v.placement_of(&main).map(|p| p.name), Some("Below the collar"));
        // An extra goes to the other side at a free place there.
        v.extras
            .push(ProductExtra::new("print.dst", square(0.0, 0.0, 30.0), Point::ZERO));
        v.set_extra_side(0, Side::Back, &main);
        assert_eq!(v.placement_of_extra(&v.extras[0]).map(|p| p.name), Some("Upper back"));
        v.set_extra_side(0, Side::Front, &main);
        assert_eq!(v.placement_of_extra(&v.extras[0]).map(|p| p.name), Some("Left chest"));
        // A project of an earlier version with a T-shirt back opens.
        let mut d = square(0.0, 0.0, 40.0);
        d.product = Some(ProductView::new(ProductKind::TShirtBack, Rgb(0, 0, 0)));
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert_eq!(back.product.unwrap().side(), Side::Back);
    }

    #[test]
    fn an_extra_is_saved_with_the_project_and_keeps_its_key() {
        let mut d = square(0.0, 0.0, 40.0);
        let mut v = ProductView::new(ProductKind::TShirt, Rgb(255, 255, 255));
        let mut logo = square(5.0, 5.0, 20.0);
        logo.product = Some(ProductView::new(ProductKind::Cap, Rgb(0, 0, 0)));
        let e = ProductExtra::new("logo.embproj", logo, Point::new(310.0, 190.0));
        assert!(e.design.product.is_none(), "the copy has no product of its own");
        assert_eq!(
            e.key,
            ProductExtra::new("other name", e.design.clone(), Point::ZERO).key
        );
        v.extras.push(e);
        d.product = Some(v);
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert_eq!(back.product, d.product);
        assert_eq!(back.product.unwrap().extras[0].size(), Some((20.0, 20.0)));
        // A project of an earlier version (no extras) opens.
        let old = crate::project::to_string(&square(0.0, 0.0, 40.0));
        assert!(crate::project::from_str(&old).is_ok());
    }

    #[test]
    fn a_cap_curves_the_design_and_the_rest_are_flat() {
        let c = ProductKind::Cap;
        assert_eq!(c.warp(Point::new(0.0, -60.0)), Point::new(0.0, -60.0));
        let side = c.warp(Point::new(55.0, -60.0));
        // A little narrower and a little lower at the side.
        assert!(side.x < 54.0 && side.x > 50.0, "{side:?}");
        assert!(side.y > -60.0 && side.y < -57.0, "{side:?}");
        for kind in ProductKind::ALL.into_iter().filter(|k| *k != c) {
            assert_eq!(kind.warp(Point::new(55.0, 30.0)), Point::new(55.0, 30.0));
        }
    }

    #[test]
    fn details_show_on_dark_and_light_products() {
        assert_eq!(shade(Rgb(200, 100, 50), Shade::Dark), Rgb(144, 72, 36));
        let black = shade(Rgb(10, 10, 10), Shade::Dark);
        assert!(black.0 > 50, "{black:?}");
    }
}

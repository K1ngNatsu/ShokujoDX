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
}

/// How a part of a product is colored: the product color, darker, or lighter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shade {
    Body,
    Dark,
    Light,
}

/// One part of the drawing of a product, in product coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum ProductPart {
    /// A filled area (closed path), drawn in order.
    Area(Path, Shade),
    /// A line (a seam or an edge) of this width in mm, drawn darker.
    Line(Path, f64),
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
}

fn yes() -> bool {
    true
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
        Shade::Body => color,
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
/// depth `neck` (a back has a shallow neck).
fn short_sleeves(neck: f64) -> Path {
    use Seg::*;
    let w = 95.0;
    symmetric(
        (0.0, neck),
        &[
            C(w * 0.53, neck, w, neck * 0.5, w, 0.0),
            L(235.0, 35.0),
            L(345.0, 205.0),
            L(285.0, 262.0),
            L(262.0, 225.0),
            L(270.0, 715.0),
            C(180.0, 722.0, 90.0, 725.0, 0.0, 725.0),
        ],
    )
}

impl ProductKind {
    pub const ALL: [ProductKind; 8] = [
        ProductKind::TShirt,
        ProductKind::TShirtBack,
        ProductKind::Polo,
        ProductKind::Sweatshirt,
        ProductKind::Cap,
        ProductKind::Tote,
        ProductKind::Towel,
        ProductKind::Cushion,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ProductKind::TShirt => "T-shirt, front",
            ProductKind::TShirtBack => "T-shirt, back",
            ProductKind::Polo => "Polo shirt",
            ProductKind::Sweatshirt => "Sweatshirt",
            ProductKind::Cap => "Cap",
            ProductKind::Tote => "Tote bag",
            ProductKind::Towel => "Bath towel",
            ProductKind::Cushion => "Cushion",
        }
    }

    /// The drawing of the product: the first part is the outline (the body).
    pub fn parts(self) -> Vec<ProductPart> {
        use ProductPart::{Area, Line};
        use Seg::*;
        let rib = 2.0;
        match self {
            ProductKind::TShirt => {
                let [s1, s2] = both(&[(333.0, 191.0), (273.0, 248.0)]);
                vec![
                    Area(short_sleeves(95.0), Shade::Body),
                    Area(inside_neck(95.0, 95.0, 22.0), Shade::Dark),
                    Line(neck_line(113.0, 113.0), rib),
                    Line(s1, rib),
                    Line(s2, rib),
                    Line(line(&[(-269.0, 690.0), (269.0, 690.0)]), rib),
                ]
            }
            ProductKind::TShirtBack => {
                let [s1, s2] = both(&[(333.0, 191.0), (273.0, 248.0)]);
                vec![
                    Area(short_sleeves(22.0), Shade::Body),
                    Line(neck_line(95.0, 40.0), rib),
                    Line(s1, rib),
                    Line(s2, rib),
                    Line(line(&[(-269.0, 690.0), (269.0, 690.0)]), rib),
                ]
            }
            ProductKind::Polo => {
                let [s1, s2] = both(&[(333.0, 191.0), (273.0, 248.0)]);
                let wing = |s: f64| {
                    let p = |x: f64, y: f64| Point::new(s * x, y);
                    Path::new()
                        .move_to(p(8.0, 100.0))
                        .line_to(p(98.0, -6.0))
                        .line_to(p(122.0, 6.0))
                        .line_to(p(72.0, 142.0))
                        .close()
                };
                let mut v = vec![
                    Area(short_sleeves(95.0), Shade::Body),
                    Area(inside_neck(95.0, 95.0, 22.0), Shade::Dark),
                    Line(
                        Path::rect(BBox::new(Point::new(-16.0, 95.0), Point::new(16.0, 250.0))),
                        rib,
                    ),
                    Area(wing(1.0), Shade::Light),
                    Area(wing(-1.0), Shade::Light),
                    Line(wing(1.0), rib),
                    Line(wing(-1.0), rib),
                    Line(s1, rib),
                    Line(s2, rib),
                    Line(line(&[(-269.0, 690.0), (269.0, 690.0)]), rib),
                ];
                for y in [130.0, 180.0, 230.0] {
                    v.push(Area(circle(0.0, y, 5.0), Shade::Dark));
                }
                v
            }
            ProductKind::Sweatshirt => {
                let body = symmetric(
                    (0.0, 90.0),
                    &[
                        C(50.0, 90.0, 95.0, 45.0, 95.0, 0.0),
                        L(240.0, 40.0),
                        L(430.0, 590.0),
                        L(370.0, 615.0),
                        L(262.0, 240.0),
                        L(268.0, 720.0),
                        C(180.0, 724.0, 90.0, 725.0, 0.0, 725.0),
                    ],
                );
                let [c1, c2] = both(&[(410.0, 532.0), (352.0, 557.0)]);
                vec![
                    Area(body, Shade::Body),
                    Area(inside_neck(95.0, 90.0, 22.0), Shade::Dark),
                    Line(neck_line(115.0, 112.0), rib),
                    Line(c1, rib),
                    Line(c2, rib),
                    Line(line(&[(-266.0, 660.0), (266.0, 660.0)]), rib),
                ]
            }
            ProductKind::Cap => {
                let crown = symmetric((0.0, -130.0), &[C(70.0, -130.0, 115.0, -70.0, 115.0, 0.0), L(0.0, 0.0)]);
                let brim = symmetric(
                    (0.0, -8.0),
                    &[
                        C(60.0, -8.0, 118.0, -6.0, 135.0, 8.0),
                        C(110.0, 42.0, 50.0, 52.0, 0.0, 52.0),
                    ],
                );
                vec![
                    Area(crown.clone(), Shade::Body),
                    Area(brim, Shade::Dark),
                    Line(line(&[(0.0, -130.0), (0.0, -4.0)]), 1.5),
                    Line(
                        Path::new().move_to(Point::new(-112.0, -4.0)).cubic_to(
                            Point::new(-60.0, -10.0),
                            Point::new(60.0, -10.0),
                            Point::new(112.0, -4.0),
                        ),
                        rib,
                    ),
                    Area(circle(0.0, -128.0, 7.0), Shade::Dark),
                ]
            }
            ProductKind::Tote => {
                let body = Path::rect(BBox::new(Point::new(-190.0, 0.0), Point::new(190.0, 420.0)));
                let handle = Path::new().move_to(Point::new(-100.0, 0.0)).cubic_to(
                    Point::new(-100.0, -190.0),
                    Point::new(100.0, -190.0),
                    Point::new(100.0, 0.0),
                );
                vec![
                    Area(body, Shade::Body),
                    Line(handle, 25.0),
                    Line(line(&[(-190.0, 30.0), (190.0, 30.0)]), rib),
                ]
            }
            ProductKind::Towel => {
                let body = Path::rect(BBox::new(Point::new(-250.0, 0.0), Point::new(250.0, 900.0)));
                let border = Path::rect(BBox::new(Point::new(-250.0, 760.0), Point::new(250.0, 830.0)));
                vec![
                    Area(body, Shade::Body),
                    Area(border, Shade::Light),
                    Line(line(&[(-250.0, 760.0), (250.0, 760.0)]), rib),
                    Line(line(&[(-250.0, 830.0), (250.0, 830.0)]), rib),
                    Line(line(&[(-250.0, 885.0), (250.0, 885.0)]), rib),
                ]
            }
            ProductKind::Cushion => {
                let body = Path::rect(BBox::new(Point::new(-225.0, 0.0), Point::new(225.0, 450.0)));
                vec![Area(body.clone(), Shade::Body), Line(body, 6.0)]
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
        ];
        const BACK: &[Placement] = &[
            pl("Below the collar", 0.0, 80.0, 100.0, 50.0),
            pl("Upper back", 0.0, 170.0, 300.0, 120.0),
            pl("Full back", 0.0, 330.0, 330.0, 380.0),
        ];
        const POLO: &[Placement] = &[
            pl("Left chest", 115.0, 200.0, 100.0, 90.0),
            pl("Right chest", -115.0, 200.0, 100.0, 90.0),
            pl("Sleeve", 295.0, 200.0, 40.0, 40.0),
        ];
        const SWEAT: &[Placement] = &[
            pl("Left chest", 115.0, 200.0, 100.0, 100.0),
            pl("Center chest", 0.0, 200.0, 280.0, 160.0),
            pl("Full front", 0.0, 350.0, 320.0, 350.0),
        ];
        const CAP: &[Placement] = &[pl("Front", 0.0, -55.0, 110.0, 55.0)];
        const TOTE: &[Placement] = &[pl("Center", 0.0, 230.0, 280.0, 280.0)];
        const TOWEL: &[Placement] = &[
            pl("On the border", 0.0, 795.0, 420.0, 60.0),
            pl("Above the border", 0.0, 690.0, 300.0, 110.0),
        ];
        const CUSHION: &[Placement] = &[pl("Center", 0.0, 225.0, 320.0, 320.0)];
        match self {
            ProductKind::TShirt => SHIRT,
            ProductKind::TShirtBack => BACK,
            ProductKind::Polo => POLO,
            ProductKind::Sweatshirt => SWEAT,
            ProductKind::Cap => CAP,
            ProductKind::Tote => TOTE,
            ProductKind::Towel => TOWEL,
            ProductKind::Cushion => CUSHION,
        }
    }

    /// The box of the drawing, in product coordinates.
    pub fn bbox(self) -> BBox {
        self.parts()
            .iter()
            .filter_map(|p| match p {
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
        }
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
        assert!((b.width() - 690.0).abs() < 1.0, "{b:?}");
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

    #[test]
    fn details_show_on_dark_and_light_products() {
        assert_eq!(shade(Rgb(200, 100, 50), Shade::Dark), Rgb(144, 72, 36));
        let black = shade(Rgb(10, 10, 10), Shade::Dark);
        assert!(black.0 > 50, "{black:?}");
    }
}

//! The product under the design (P7 Milestone 5): the drawn products as
//! filled areas, and the picture of the design on the product (the
//! virtual decoration picture).

use emb_geom::region::{self, Join};
use emb_geom::{BBox, Point};
use emb_model::{ProductPart, ProductView, Rgb, Shade, product::shade};

use crate::Segment;
use crate::raster::{Areas, Look, Picture, picture_at, stitches_onto_look};

const FLATTEN_MM: f64 = 0.2;

/// The areas of a drawn product in view coordinates, in drawing order:
/// rings (even-odd) and a color. Lines become thin areas.
pub fn areas(view: &ProductView) -> Vec<(Vec<Vec<Point>>, Rgb)> {
    let o = view.origin;
    let mv = |pts: Vec<Point>| pts.into_iter().map(|p| p + o).collect::<Vec<_>>();
    view.kind
        .parts()
        .into_iter()
        .map(|part| match part {
            ProductPart::Area(path, s) => (
                path.flatten(FLATTEN_MM).into_iter().map(|pl| mv(pl.points)).collect(),
                shade(view.color, s),
            ),
            ProductPart::Line(path, width) => {
                let band = region::offset_lines(&path.flatten(FLATTEN_MM), width / 2.0, Join::Round);
                let rings = band
                    .into_iter()
                    .flat_map(|r| std::iter::once(r.outer).chain(r.holes))
                    .map(mv)
                    .collect();
                (rings, shade(view.color, Shade::Dark))
            }
        })
        .collect()
}

/// The picture of the drawn product alone, and the area (view mm) that it shows.
pub fn product_picture(view: &ProductView, px_per_mm: f64) -> (Picture, BBox) {
    let b = view.bbox().expand(2.0);
    (picture_at(&[], &areas(view), b, px_per_mm, 0.0, None), b)
}

/// The picture of the design on the product: the product (drawn, or the
/// picture of the user's product decoded as `photo`), then the stitches
/// (view coordinates) with the thread look. The picture shows the product
/// and the design with 10 mm round them; the background is white.
pub fn decoration(
    view: &ProductView,
    photo: Option<&image::RgbaImage>,
    segments: &[Segment],
    px_per_mm: f64,
    thread_mm: f64,
    look: &Look,
) -> Picture {
    let stitches = BBox::from_points(segments.iter().flat_map(|s| [s.a, s.b]));
    let b = [Some(view.bbox()), stitches]
        .into_iter()
        .flatten()
        .reduce(BBox::union)
        .unwrap_or_else(|| view.bbox())
        .expand(10.0);
    let white = Some(Rgb(255, 255, 255));
    let mut pic = match (&view.picture, photo) {
        (Some(p), Some(img)) => {
            let mut pic = picture_at(&[], &[] as &Areas, b, px_per_mm, 0.0, white);
            paint_photo(&mut pic, b, px_per_mm, img, view.origin, p.width_mm, p.height_mm());
            pic
        }
        _ => picture_at(&[], &areas(view), b, px_per_mm, 0.0, white),
    };
    stitches_onto_look(&mut pic, b, segments, px_per_mm, thread_mm, look.lighting.as_ref());
    pic
}

/// Draws `img` over `pic` (which shows `b`) with its top left corner at
/// `at` and a size of `w` × `h` mm. Transparent pixels of `img` let the
/// white show.
fn paint_photo(pic: &mut Picture, b: BBox, px_per_mm: f64, img: &image::RgbaImage, at: Point, w: f64, h: f64) {
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 || w <= 0.0 || h <= 0.0 {
        return;
    }
    for py in 0..pic.height {
        let y = b.min.y + (f64::from(py) + 0.5) / px_per_mm;
        let v = (y - at.y) / h;
        if !(0.0..1.0).contains(&v) {
            continue;
        }
        let sy = ((v * f64::from(ih)) as u32).min(ih - 1);
        for px in 0..pic.width {
            let x = b.min.x + (f64::from(px) + 0.5) / px_per_mm;
            let u = (x - at.x) / w;
            if !(0.0..1.0).contains(&u) {
                continue;
            }
            let sx = ((u * f64::from(iw)) as u32).min(iw - 1);
            let s = img.get_pixel(sx, sy).0;
            let a = f64::from(s[3]) / 255.0;
            let i = (py as usize * pic.width as usize + px as usize) * 4;
            for (dst, src) in pic.rgba[i..i + 3].iter_mut().zip(s) {
                *dst = (f64::from(src) * a + f64::from(*dst) * (1.0 - a)).round() as u8;
            }
            pic.rgba[i + 3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_model::ProductKind;

    use super::*;
    use crate::SegmentKind;

    fn pixel(p: &Picture, b: BBox, px_per_mm: f64, at: Point) -> [u8; 4] {
        let x = ((at.x - b.min.x) * px_per_mm) as usize;
        let y = ((at.y - b.min.y) * px_per_mm) as usize;
        let i = (y * p.width as usize + x) * 4;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2], p.rgba[i + 3]]
    }

    #[test]
    fn the_product_shows_its_color_and_the_inside_of_the_neck() {
        let mut v = ProductView::new(ProductKind::TShirt, Rgb(200, 30, 40));
        v.origin = Point::new(10.0, -100.0);
        let (pic, b) = product_picture(&v, 1.0);
        // The chest is the product color; the inside of the back neck is darker.
        assert_eq!(
            pixel(&pic, b, 1.0, v.origin + Point::new(0.0, 300.0)),
            [200, 30, 40, 255]
        );
        let neck = pixel(&pic, b, 1.0, v.origin + Point::new(0.0, 60.0));
        assert!(neck[0] < 200 && neck[3] == 255, "{neck:?}");
        // Outside the shirt is clear.
        assert_eq!(pixel(&pic, b, 1.0, v.origin + Point::new(320.0, 600.0))[3], 0);
    }

    #[test]
    fn the_decoration_has_the_stitches_on_the_product() {
        let v = ProductView::new(ProductKind::Tote, Rgb(240, 230, 200));
        let seg = Segment {
            a: Point::new(-20.0, 230.0),
            b: Point::new(20.0, 230.0),
            color: Rgb(0, 0, 200),
            kind: SegmentKind::Stitch,
        };
        let pic = decoration(&v, None, &[seg], 2.0, 0.6, &Look::default());
        let b = v.bbox().expand(10.0);
        assert_eq!(pixel(&pic, b, 2.0, Point::new(0.0, 230.0))[2], 200);
        assert_eq!(pixel(&pic, b, 2.0, Point::new(0.0, 300.0)), [240, 230, 200, 255]);
        // White round the product.
        assert_eq!(pixel(&pic, b, 2.0, Point::new(-199.0, 400.0)), [255, 255, 255, 255]);
    }

    #[test]
    fn the_decoration_has_the_3d_look() {
        let v = ProductView::new(ProductKind::Tote, Rgb(240, 230, 200));
        let seg = Segment {
            a: Point::new(-20.0, 230.0),
            b: Point::new(20.0, 230.0),
            color: Rgb(0, 0, 200),
            kind: SegmentKind::Stitch,
        };
        let flat = decoration(&v, None, &[seg], 8.0, 1.0, &Look::default());
        let lit = decoration(&v, None, &[seg], 8.0, 1.0, &Look::three_d(None));
        let b = v.bbox().expand(10.0);
        // The top edge of the thread is lighter than the bottom edge (light from the top left).
        let top = pixel(&lit, b, 8.0, Point::new(0.0, 229.75))[2];
        let bottom = pixel(&lit, b, 8.0, Point::new(0.0, 230.25))[2];
        let flat_top = pixel(&flat, b, 8.0, Point::new(0.0, 229.75))[2];
        let flat_bottom = pixel(&flat, b, 8.0, Point::new(0.0, 230.25))[2];
        assert!(
            i32::from(top) - i32::from(bottom) > (i32::from(flat_top) - i32::from(flat_bottom)).abs() + 20,
            "{top} {bottom} {flat_top} {flat_bottom}"
        );
    }

    #[test]
    fn a_photo_of_the_product_is_under_the_stitches() {
        let img = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 120, 0, 255]));
        let mut v = ProductView::new(ProductKind::Tote, Rgb(0, 0, 0));
        v.picture = Some(emb_model::ProductPicture {
            name: "bag.png".into(),
            data: std::sync::Arc::new(Vec::new()),
            pixels: (4, 2),
            width_mm: 200.0,
        });
        let pic = decoration(&v, Some(&img), &[], 1.0, 0.4, &Look::default());
        let b = v.bbox().expand(10.0);
        assert_eq!(pixel(&pic, b, 1.0, Point::new(100.0, 50.0)), [0, 120, 0, 255]);
        assert_eq!(v.bbox().height(), 100.0);
    }
}

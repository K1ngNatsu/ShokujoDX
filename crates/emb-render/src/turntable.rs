//! A turntable view of the product (P16 Milestone 8, ADR 0108): the
//! product turns in steps, so the customer sees it from more sides.
//!
//! The front and the back of a drawn product (with their designs, as in
//! the saved picture) become a soft round body: the outline is "blown up"
//! (the square root of the solution of a Poisson equation in the outline,
//! so a part is round across its own width, and a sleeve is thinner than
//! the body). The front of the body has the flat coordinates of the
//! canvas, so the design is exactly on it. The hem of a garment is open (a
//! tube, not a closed bag). A cap has its own shape: a round crown and a
//! brim that comes forward.
//!
//! A view at an angle turns the body round the vertical axis through the
//! middle of the product and draws it from the front with no perspective,
//! one row at a time with a depth buffer: each row of the body is a closed
//! curve (the front and the back), so the view has no holes.

use emb_geom::{BBox, Point, math};
use emb_model::{ProductKind, ProductView, Rgb};

use crate::raster::{Picture, picture_at};

/// The views of a turn: one each 15°.
pub const STEPS: usize = 24;
/// The size of the cells of the Poisson equation (mm).
const CELL_MM: f64 = 4.0;
/// A view is made at this many times its size, then made smaller (smooth
/// edges).
const SUPERSAMPLE: u32 = 2;

/// The depth of a part over its width: the half depth of a round part
/// (mm) is this times the square root of twice the solution in it.
fn depth(kind: ProductKind) -> f64 {
    match kind {
        ProductKind::Cap => 1.25,
        ProductKind::Beanie => 1.1,
        ProductKind::Towel => 0.05,
        ProductKind::Apron => 0.1,
        ProductKind::Tote => 0.18,
        ProductKind::Messenger => 0.35,
        ProductKind::Cushion => 0.4,
        _ => 0.5,
    }
}

/// True if the bottom edge of the product is open (a garment or a hat is
/// a tube there, not a closed bag).
fn open_bottom(kind: ProductKind) -> bool {
    !matches!(
        kind,
        ProductKind::Towel | ProductKind::Apron | ProductKind::Tote | ProductKind::Messenger | ProductKind::Cushion
    )
}

/// A side of the product: its picture (clear round the product, see
/// `product::turn_side`) and the area (view mm) that it shows.
#[derive(Clone, Copy)]
pub struct Side<'a> {
    pub picture: &'a Picture,
    pub area: BBox,
}

impl Side<'_> {
    /// The color at the view point `p`, if the picture shows it.
    fn color_at(&self, p: Point, px_per_mm: f64) -> Option<[u8; 3]> {
        let x = ((p.x - self.area.min.x) * px_per_mm).floor();
        let y = ((p.y - self.area.min.y) * px_per_mm).floor();
        if x < 0.0 || y < 0.0 || x >= f64::from(self.picture.width) || y >= f64::from(self.picture.height) {
            return None;
        }
        let i = (y as usize * self.picture.width as usize + x as usize) * 4;
        let c = &self.picture.rgba[i..i + 4];
        (c[3] > 200).then(|| [c[0], c[1], c[2]])
    }
}

/// One closed part of the body: the cells that it covers, the depth (mm,
/// toward the viewer) of its front and back surfaces, and their colors.
struct Layer {
    on: Vec<bool>,
    front_z: Vec<f32>,
    back_z: Vec<f32>,
    front: Vec<[u8; 3]>,
    back: Vec<[u8; 3]>,
}

/// The body of a product, made once; its views at each angle come from it.
pub struct Body {
    px_per_mm: f64,
    /// The area of the cells, in product coordinates.
    area: BBox,
    width: usize,
    height: usize,
    layers: Vec<Layer>,
    /// The largest distance of a point of the body from the turn axis (mm).
    radius: f64,
}

/// The cells (`width` × `height` over `area`, product coordinates) inside
/// the drawn product `kind`, its mirror (a back, seen from behind) with
/// `mirror`.
fn mask(kind: ProductKind, area: BBox, px_per_mm: f64, mirror: bool) -> Vec<bool> {
    let v = ProductView::new(kind, Rgb(128, 128, 128));
    let areas: Vec<(Vec<Vec<Point>>, Rgb)> = crate::product::areas(&v)
        .into_iter()
        .map(|(rings, c)| {
            let rings = rings
                .into_iter()
                .map(|r| {
                    r.into_iter()
                        .map(|p| if mirror { Point::new(-p.x, p.y) } else { p })
                        .collect()
                })
                .collect();
            (rings, c)
        })
        .collect();
    let pic = picture_at(&[], &areas, area, px_per_mm, 0.0, None);
    pic.rgba.as_chunks::<4>().0.iter().map(|p| p[3] > 127).collect()
}

/// The cells inside `path` (product coordinates).
fn path_mask(path: &emb_geom::Path, area: BBox, px_per_mm: f64) -> Vec<bool> {
    let rings: Vec<Vec<Point>> = path.flatten(0.2).into_iter().map(|pl| pl.points).collect();
    let pic = picture_at(&[], &[(rings, Rgb(0, 0, 0))], area, px_per_mm, 0.0, None);
    pic.rgba.as_chunks::<4>().0.iter().map(|p| p[3] > 127).collect()
}

/// The height (mm) of the blown-up shape of the cells `on` (`w` × `h`,
/// `px_per_mm`): `ratio` times the square root of twice the solution `u`
/// of ∇²u = −1 in the cells (u = 0 out of them). With `open`, the bottom
/// edge does not pull the shape in.
fn inflate(on: &[bool], w: usize, h: usize, px_per_mm: f64, ratio: f64, open: bool) -> Vec<f32> {
    let s = ((CELL_MM * px_per_mm).round() as usize).max(1);
    let cell = s as f64 / px_per_mm;
    let (cw, ch0) = (w.div_ceil(s), h.div_ceil(s));
    // The open bottom: the cells go on down for as far as the shape is wide.
    let extra = if open { ch0 } else { 0 };
    let ch = ch0 + extra;
    let mut inside = vec![false; cw * ch];
    for cy in 0..ch0 {
        for cx in 0..cw {
            let (x, y) = ((cx * s + s / 2).min(w - 1), (cy * s + s / 2).min(h - 1));
            inside[cy * cw + cx] = on[y * w + x];
        }
    }
    if open {
        // The columns that end at the bottom of the shape (the hem, not a
        // sleeve that ends higher).
        let last: Vec<Option<usize>> = (0..cw)
            .map(|cx| (0..ch0).rev().find(|&cy| inside[cy * cw + cx]))
            .collect();
        if let Some(bottom) = last.iter().flatten().max().copied() {
            let near = ((40.0 / cell).ceil() as usize).max(1);
            for (cx, l) in last.iter().enumerate() {
                if let Some(l) = l
                    && l + near >= bottom
                {
                    for cy in l + 1..ch {
                        inside[cy * cw + cx] = true;
                    }
                }
            }
        }
    }
    let mut u = vec![0f64; cw * ch];
    let c2 = cell * cell;
    let omega = 1.9;
    for _ in 0..(2 * cw.max(ch) + 60) {
        for cy in 0..ch {
            for cx in 0..cw {
                let i = cy * cw + cx;
                if !inside[i] {
                    continue;
                }
                let at = |x: usize, y: usize| if inside[y * cw + x] { u[y * cw + x] } else { 0.0 };
                let mut sum = 0.0;
                sum += if cx > 0 { at(cx - 1, cy) } else { 0.0 };
                sum += if cx + 1 < cw { at(cx + 1, cy) } else { 0.0 };
                sum += if cy > 0 { at(cx, cy - 1) } else { 0.0 };
                // The last row of an open shape goes on as it is.
                sum += if cy + 1 < ch { at(cx, cy + 1) } else { u[i] };
                let new = (sum + c2) / 4.0;
                u[i] += omega * (new - u[i]);
            }
        }
    }
    // The fine cells, from the four nearest coarse cells.
    let height = |cx: f64, cy: f64| {
        let x0 = cx.floor().clamp(0.0, (cw - 1) as f64);
        let y0 = cy.floor().clamp(0.0, (ch - 1) as f64);
        let (x1, y1) = ((x0 + 1.0).min((cw - 1) as f64), (y0 + 1.0).min((ch - 1) as f64));
        let (tx, ty) = ((cx - x0).clamp(0.0, 1.0), (cy - y0).clamp(0.0, 1.0));
        let g = |x: f64, y: f64| u[y as usize * cw + x as usize];
        let top = g(x0, y0) * (1.0 - tx) + g(x1, y0) * tx;
        let bottom = g(x0, y1) * (1.0 - tx) + g(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    };
    let mut out = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            if on[y * w + x] {
                let v = height((x as f64 + 0.5) / s as f64 - 0.5, (y as f64 + 0.5) / s as f64 - 0.5);
                out[y * w + x] = (ratio * (2.0 * v.max(0.0)).sqrt()) as f32;
            }
        }
    }
    out
}

impl Body {
    /// The body of the product `kind` (the front) at `origin` (view mm):
    /// `front` and `back` are the pictures of its two sides (the back as
    /// seen from behind; for a product with one side, the product with no
    /// design). The views have `px_per_mm` pixels for each mm.
    pub fn new(kind: ProductKind, origin: Point, front: Side, back: Side, px_per_mm: f64) -> Body {
        let back_kind = kind.turned().unwrap_or(kind);
        let fine = px_per_mm * f64::from(SUPERSAMPLE);
        let b = kind.bbox();
        let mirrored = back_kind.bbox();
        let mirrored = BBox::new(
            Point::new(-mirrored.max.x, mirrored.min.y),
            Point::new(-mirrored.min.x, mirrored.max.y),
        );
        let area = b.union(mirrored).expand(4.0);
        let front_on = mask(kind, area, fine, false);
        let back_on = mask(back_kind, area, fine, true);
        let width = ((area.width() * fine).ceil() as usize).clamp(1, 16_384);
        let height = ((area.height() * fine).ceil() as usize).clamp(1, 16_384);
        let at = |i: usize| {
            let (x, y) = (i % width, i / width);
            Point::new(
                area.min.x + (x as f64 + 0.5) / fine,
                area.min.y + (y as f64 + 0.5) / fine,
            )
        };
        let grey = [200, 200, 200];
        let front_color = |p: Point| front.color_at(p + origin, fine);
        let back_color = |p: Point| back.color_at(Point::new(-p.x, p.y) + origin, fine);
        let layers = if kind == ProductKind::Cap {
            cap_layers(area, width, height, fine, &front_color, &back_color, &at)
        } else {
            let on: Vec<bool> = front_on.iter().zip(&back_on).map(|(a, b)| *a || *b).collect();
            let h = inflate(&on, width, height, fine, depth(kind), open_bottom(kind));
            let n = width * height;
            let (mut fc, mut bc) = (vec![None; n], vec![None; n]);
            for i in 0..n {
                if !on[i] {
                    continue;
                }
                let p = at(i);
                let (f, k) = (front_color(p), back_color(p));
                fc[i] = if front_on[i] { f.or(k) } else { k.or(f) };
                bc[i] = if back_on[i] { k.or(f) } else { f.or(k) };
            }
            let (fc, bc) = (
                fill_colors(fc, &on, width, height, grey),
                fill_colors(bc, &on, width, height, grey),
            );
            vec![Layer {
                front_z: h.clone(),
                back_z: h.iter().map(|v| -v).collect(),
                on,
                front: fc,
                back: bc,
            }]
        };
        let mut radius: f64 = 1.0;
        for l in &layers {
            for i in 0..width * height {
                if l.on[i] {
                    let x = at(i).x;
                    for z in [l.front_z[i], l.back_z[i]] {
                        radius = radius.max(math::hypot(x, f64::from(z)));
                    }
                }
            }
        }
        Body {
            px_per_mm: fine,
            area,
            width,
            height,
            layers,
            radius: radius + 2.0,
        }
    }

    /// The view of the body turned by `angle` degrees round the vertical
    /// axis (positive: the front turns to the right of the view, and the
    /// left side of the product comes to the front), on white.
    pub fn view(&self, angle: f64) -> Picture {
        let (sin, cos) = (math::sin(angle.to_radians()), math::cos(angle.to_radians()));
        let px = self.px_per_mm;
        let fw = ((2.0 * self.radius * px).ceil() as usize).max(1);
        let fh = self.height;
        let mut z_buf = vec![f32::NEG_INFINITY; fw * fh];
        let mut rgb = vec![[255f32; 3]; fw * fh];
        let x_of = |i: usize| self.area.min.x + (i as f64 + 0.5) / px;
        // A studio light from the top left in front of the product; the
        // product turns under it. A surface that faces the viewer is as
        // bright as in its flat picture.
        let (lx, ly, lz): (f64, f64, f64) = (-0.35, -0.45, 1.0);
        let l_len: f64 = (lx * lx + ly * ly + lz * lz).sqrt();
        let (lx, ly, lz) = (lx / l_len, ly / l_len, lz / l_len);
        let light = |d: f64| 0.45 + 0.6 * d.max(0.0);
        let facing = light(lz);
        for l in &self.layers {
            for y in 0..self.height {
                let row = y * self.width;
                let mut x = 0;
                while x < self.width {
                    if !l.on[row + x] {
                        x += 1;
                        continue;
                    }
                    let start = x;
                    while x < self.width && l.on[row + x] {
                        x += 1;
                    }
                    let end = x - 1;
                    // The samples of the closed curve of this row: the front
                    // from left to right, then the back from right to left.
                    let mut samples: Vec<(f64, f64, [u8; 3], f64)> = Vec::with_capacity(2 * (end - start + 1));
                    for (back, range) in [
                        (false, (start..=end).collect::<Vec<_>>()),
                        (true, (start..=end).rev().collect()),
                    ] {
                        let zs = if back { &l.back_z } else { &l.front_z };
                        for i in range {
                            let z = f64::from(zs[row + i]);
                            // The slope across and down, for the normal.
                            let zx = {
                                let (a, b) = (i.saturating_sub(1).max(start), (i + 1).min(end));
                                if b > a {
                                    (f64::from(zs[row + b]) - f64::from(zs[row + a])) * px / (b - a) as f64
                                } else {
                                    0.0
                                }
                            };
                            let zy = {
                                let up = if y > 0 && l.on[row - self.width + i] { y - 1 } else { y };
                                let down = if y + 1 < self.height && l.on[row + self.width + i] {
                                    y + 1
                                } else {
                                    y
                                };
                                if down > up {
                                    (f64::from(zs[down * self.width + i]) - f64::from(zs[up * self.width + i])) * px
                                        / (down - up) as f64
                                } else {
                                    0.0
                                }
                            };
                            // The outward normal: (−zx, −zy, 1) at the front,
                            // (zx, zy, −1) at the back.
                            let s = if back { -1.0 } else { 1.0 };
                            let (nx, ny, nz) = (-s * zx, -s * zy, s);
                            let len = (nx * nx + ny * ny + nz * nz).sqrt();
                            let (nx, ny, nz) = (nx / len, ny / len, nz / len);
                            // The normal turned with the body.
                            let (tx, tz) = (nx * cos + nz * sin, -nx * sin + nz * cos);
                            let f = (light(tx * lx + ny * ly + tz * lz) / facing).min(1.1);
                            let c = if back { l.back[row + i] } else { l.front[row + i] };
                            let xm = x_of(i);
                            samples.push((xm * cos + z * sin, -xm * sin + z * cos, c, f));
                        }
                    }
                    let n = samples.len();
                    for k in 0..n {
                        let (a, b) = (samples[k], samples[(k + 1) % n]);
                        let ta = (a.0 + self.radius) * px - 0.5;
                        let tb = (b.0 + self.radius) * px - 0.5;
                        let (lo, hi) = (ta.min(tb).ceil().max(0.0), ta.max(tb).floor().min((fw - 1) as f64));
                        if lo > hi {
                            continue;
                        }
                        for t in lo as usize..=hi as usize {
                            let s = if (tb - ta).abs() < 1e-9 {
                                0.0
                            } else {
                                (t as f64 - ta) / (tb - ta)
                            };
                            let z = a.1 + (b.1 - a.1) * s;
                            let i = y * fw + t;
                            if z as f32 > z_buf[i] {
                                z_buf[i] = z as f32;
                                let f = a.3 + (b.3 - a.3) * s;
                                for (c, out) in rgb[i].iter_mut().enumerate() {
                                    let v = f64::from(a.2[c]) + (f64::from(b.2[c]) - f64::from(a.2[c])) * s;
                                    *out = (v * f) as f32;
                                }
                            }
                        }
                    }
                }
            }
        }
        // Smaller by the supersample: the mean of each square of pixels.
        let ss = SUPERSAMPLE as usize;
        let (w, h) = (fw.div_ceil(ss), fh.div_ceil(ss));
        let mut rgba = vec![255u8; w * h * 4];
        for oy in 0..h {
            for ox in 0..w {
                let mut sum = [0f32; 3];
                let mut count = 0.0;
                for y in oy * ss..((oy + 1) * ss).min(fh) {
                    for x in ox * ss..((ox + 1) * ss).min(fw) {
                        for c in 0..3 {
                            sum[c] += rgb[y * fw + x][c];
                        }
                        count += 1.0;
                    }
                }
                let o = (oy * w + ox) * 4;
                for c in 0..3 {
                    rgba[o + c] = (sum[c] / count).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        Picture {
            width: w as u32,
            height: h as u32,
            rgba,
        }
    }

    /// The area (product coordinates, mm) of the front view: the views have
    /// the turn axis (x = 0) in their middle.
    pub fn view_area(&self) -> BBox {
        BBox::new(
            Point::new(-self.radius, self.area.min.y),
            Point::new(self.radius, self.area.max.y),
        )
    }
}

/// The colors of the cells `on`: a cell with no color (the edge of a
/// picture) takes the color of the nearest cell in its row or column.
fn fill_colors(colors: Vec<Option<[u8; 3]>>, on: &[bool], w: usize, h: usize, default: [u8; 3]) -> Vec<[u8; 3]> {
    const REACH: usize = 8;
    let mut out = vec![default; w * h];
    for i in 0..w * h {
        if !on[i] {
            continue;
        }
        if let Some(c) = colors[i] {
            out[i] = c;
            continue;
        }
        let (x, y) = (i % w, i / w);
        let found = (1..=REACH).find_map(|d| {
            let mut near = Vec::with_capacity(4);
            if x >= d {
                near.push(i - d);
            }
            if x + d < w {
                near.push(i + d);
            }
            if y >= d {
                near.push(i - d * w);
            }
            if y + d < h {
                near.push(i + d * w);
            }
            near.into_iter().find_map(|j| colors[j])
        });
        out[i] = found.unwrap_or(default);
    }
    out
}

/// The layers of a cap: the round crown (open at the bottom), and the
/// brim, which comes forward from the bottom of the crown.
fn cap_layers(
    area: BBox,
    width: usize,
    height: usize,
    px_per_mm: f64,
    front_color: &dyn Fn(Point) -> Option<[u8; 3]>,
    back_color: &dyn Fn(Point) -> Option<[u8; 3]>,
    at: &dyn Fn(usize) -> Point,
) -> Vec<Layer> {
    let (crown_path, brim_path) = emb_model::product::cap_crown_and_brim();
    let crown_on = path_mask(&crown_path, area, px_per_mm);
    let brim_on = path_mask(&brim_path, area, px_per_mm);
    let crown_h = inflate(&crown_on, width, height, px_per_mm, depth(ProductKind::Cap), true);
    let n = width * height;
    // The crown under the brim has the color of the crown above it.
    let (mut cf, mut cb) = (vec![None; n], vec![None; n]);
    for x in 0..width {
        let (mut last_f, mut last_b) = (None, None);
        for y in 0..height {
            let i = y * width + x;
            if !crown_on[i] {
                continue;
            }
            if !brim_on[i] {
                last_f = front_color(at(i)).or(last_f);
                last_b = back_color(at(i)).or(last_b);
            }
            cf[i] = last_f;
            cb[i] = last_b;
        }
    }
    let grey = [200, 200, 200];
    let (cf, cb) = (
        fill_colors(cf, &crown_on, width, height, grey),
        fill_colors(cb, &crown_on, width, height, grey),
    );
    // The brim starts at the front of the crown just above its bottom edge
    // and comes forward as it goes down in the drawing.
    let join_row = ((7.0 - area.min.y) * px_per_mm).floor().clamp(0.0, (height - 1) as f64) as usize;
    let mut brim_z = vec![0f32; n];
    let mut brim_c = vec![None; n];
    for i in 0..n {
        if !brim_on[i] {
            continue;
        }
        let p = at(i);
        let x = i % width;
        let base = f64::from(crown_h[join_row * width + x]);
        brim_z[i] = (base + 1.3 * (p.y - 4.0).max(0.0)) as f32;
        brim_c[i] = front_color(p);
    }
    let brim_f = fill_colors(brim_c, &brim_on, width, height, grey);
    let brim_b = brim_f.iter().map(|c| c.map(|v| (f64::from(v) * 0.55) as u8)).collect();
    vec![
        Layer {
            back_z: crown_h.iter().map(|v| -v).collect(),
            front_z: crown_h,
            on: crown_on,
            front: cf,
            back: cb,
        },
        Layer {
            back_z: brim_z.iter().map(|v| v - 3.0).collect(),
            front_z: brim_z,
            on: brim_on,
            front: brim_f,
            back: brim_b,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat picture of `kind` in `color` at `px_per_mm`, and its area.
    fn side(kind: ProductKind, color: Rgb, px_per_mm: f64) -> (Picture, BBox) {
        let v = ProductView::new(kind, color);
        let b = v.bbox().expand(10.0);
        let areas = crate::product::areas(&v);
        (picture_at(&[], &areas, b, px_per_mm, 0.0, Some(Rgb(255, 255, 255))), b)
    }

    fn px(p: &Picture, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * p.width + x) * 4) as usize;
        [p.rgba[i], p.rgba[i + 1], p.rgba[i + 2]]
    }

    /// The pixels of the row through `y` (product mm) that are not white.
    fn covered(body: &Body, pic: &Picture, y: f64) -> Vec<u32> {
        let a = body.view_area();
        let row = ((y - a.min.y) / a.height() * f64::from(pic.height)) as u32;
        (0..pic.width).filter(|&x| px(pic, x, row) != [255, 255, 255]).collect()
    }

    #[test]
    fn the_front_shows_at_0_the_back_at_180_and_the_side_is_narrow() {
        let k = 0.5;
        let (front, fb) = side(ProductKind::TShirt, Rgb(200, 30, 40), k * 2.0);
        let (back, bb) = side(ProductKind::TShirtBack, Rgb(30, 40, 200), k * 2.0);
        let body = Body::new(
            ProductKind::TShirt,
            Point::ZERO,
            Side {
                picture: &front,
                area: fb,
            },
            Side {
                picture: &back,
                area: bb,
            },
            k,
        );
        let a = body.view_area();
        let center = |pic: &Picture| {
            let x = ((0.0 - a.min.x) / a.width() * f64::from(pic.width)) as u32;
            let y = ((400.0 - a.min.y) / a.height() * f64::from(pic.height)) as u32;
            px(pic, x, y)
        };
        let at0 = body.view(0.0);
        let at180 = body.view(180.0);
        let c0 = center(&at0);
        let c180 = center(&at180);
        assert!(c0[0] > 150 && c0[2] < 80, "{c0:?}");
        assert!(c180[2] > 150 && c180[0] < 80, "{c180:?}");
        // From the side, the body is as wide as it is deep: much narrower
        // than from the front, and with no holes in a row.
        let wide = covered(&body, &at0, 400.0);
        let side = covered(&body, &body.view(90.0), 400.0);
        assert!(side.len() * 3 < wide.len() * 2, "{} {}", side.len(), wide.len());
        assert!(side.len() > 40, "{}", side.len());
        for row in [&wide, &side] {
            assert_eq!(row.last().unwrap() - row[0] + 1, row.len() as u32, "a hole in the row");
        }
        // The front at 0° is as wide as the flat T-shirt (538 mm at the chest).
        let w = wide.len() as f64 / (f64::from(at0.width) / a.width());
        assert!((w - 530.0).abs() < 15.0, "{w}");
    }

    #[test]
    fn the_brim_of_a_cap_comes_forward() {
        let k = 0.5;
        let (front, fb) = side(ProductKind::Cap, Rgb(30, 38, 66), k * 2.0);
        let s = Side {
            picture: &front,
            area: fb,
        };
        let body = Body::new(ProductKind::Cap, Point::ZERO, s, s, k);
        // From the side, the row of the brim reaches farther than the crown.
        let side = body.view(90.0);
        let brim = covered(&body, &side, 30.0);
        let crown = covered(&body, &side, -60.0);
        assert!(!brim.is_empty() && !crown.is_empty());
        assert!(brim.last().unwrap() > crown.last().unwrap(), "{brim:?} {crown:?}");
    }
}

//! Icons of the ribbon.
//!
//! Most icons are glyphs of the Phosphor icon font (MIT license, crate
//! `egui-phosphor`). The tools that make stitches have their own drawings,
//! because they show the stitch type: dashes for running stitches, zigzags
//! for satin, hatch lines for fills. See ADR 0010.

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2, pos2, vec2};

/// The name of the program for the desktop of Linux (the `.desktop` file of
/// the packages has this name, so the window gets the icon of the program).
pub const APP_ID: &str = "shokujodx";

/// The icon of the program (`data/icon`, made by `tools/make-icon.py`).
pub const APP_ICON_PNG: &[u8] = include_bytes!("../../../data/icon/icon-256.png");

/// The icon of the window and the task bar.
pub fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory_with_format(APP_ICON_PNG, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    Some(egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    })
}

/// An icon: a font glyph or one of our drawings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Glyph(&'static str),
    Drawn(Drawing),
}

/// Our own drawings, for the tools that make stitches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drawing {
    Run,
    Manual,
    Freehand,
    Shape,
    ComplexFill,
    FreeShape,
    Rectangle,
    RoundRect,
    Ellipse,
    Star,
    Column,
    ColumnB,
    ColumnC,
    Ring,
    Turning,
    Applique,
    AddHole,
    RemoveHoles,
    FillHoles,
}

/// Adds the icon font as a fallback of the normal text font, so a glyph
/// works in any label.
/// The fonts of the program, with a font of the computer for Japanese (and
/// Chinese) characters if it has one: for the Japanese language, and for
/// names of designs, threads and files.
pub fn install(ctx: &egui::Context) {
    install_with(ctx, true);
}

/// The fonts of the program; with `cjk`, a font of the computer with
/// Japanese (and Chinese) characters comes after the others, for the text
/// that they do not have (P10 Milestone 7: the Japanese language). Returns
/// `false` if `cjk` is asked and the computer has no such font.
pub fn install_with(ctx: &egui::Context, cjk: bool) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    let mut found = !cjk;
    if cjk && let Some((bytes, index)) = cjk_font() {
        fonts.font_data.insert(
            "cjk".into(),
            std::sync::Arc::new(egui::FontData {
                font: std::borrow::Cow::Borrowed(bytes),
                index,
                tweak: Default::default(),
            }),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push("cjk".into());
        }
        found = true;
    }
    ctx.set_fonts(fonts);
    found
}

/// Fonts with Japanese characters that the systems have, in order: the
/// file and the face in it.
const CJK_FONTS: &[(&str, u32)] = &[
    // Windows (Yu Gothic since Windows 8.1, Meiryo since Vista).
    ("%WINDIR%/Fonts/YuGothM.ttc", 0),
    ("%WINDIR%/Fonts/YuGothR.ttc", 0),
    ("%WINDIR%/Fonts/meiryo.ttc", 0),
    ("%WINDIR%/Fonts/msgothic.ttc", 0),
    // macOS.
    ("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0),
    ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
    ("/Library/Fonts/Arial Unicode.ttf", 0),
    // Linux (Noto Sans CJK in most desktop systems).
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf", 0),
    ("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", 0),
];

/// The first font of [`CJK_FONTS`] that the computer has, read one time.
fn cjk_font() -> Option<(&'static [u8], u32)> {
    static FONT: std::sync::OnceLock<Option<(&'static [u8], u32)>> = std::sync::OnceLock::new();
    *FONT.get_or_init(|| {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:/Windows".into());
        CJK_FONTS.iter().find_map(|(path, index)| {
            let path = path.replace("%WINDIR%", &windir);
            let bytes = std::fs::read(path).ok()?;
            let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
            Some((bytes, *index))
        })
    })
}

/// Paints the icon to fill `rect` (a square). `bg` is the color under the
/// icon, for the small badges.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32, bg: Color32) {
    match icon {
        Icon::Glyph(g) => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                g,
                FontId::proportional(rect.height()),
                color,
            );
        }
        Icon::Drawn(d) => draw(painter, rect, d, color, bg),
    }
}

fn draw(painter: &Painter, rect: Rect, d: Drawing, color: Color32, bg: Color32) {
    let s = rect.width().min(rect.height());
    let o = rect.center() - Vec2::splat(s / 2.0);
    let at = |x: f32, y: f32| o + vec2(x, y) * s;
    let w = (s / 14.0).max(1.0);
    let line = Stroke::new(w, color);
    let thin = Stroke::new((w * 0.6).max(0.8), color);
    let gap = s * 0.17;
    match d {
        Drawing::Run => dashed(painter, &wave(at), line, s * 0.11),
        Drawing::Manual => {
            let pts = [(0.1, 0.75), (0.32, 0.3), (0.52, 0.68), (0.72, 0.25), (0.9, 0.55)].map(|(x, y)| at(x, y));
            painter.add(egui::Shape::line(pts.to_vec(), thin));
            for p in pts {
                painter.circle_filled(p, w * 1.2, color);
            }
        }
        Drawing::Freehand => {
            let pts: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let t = i as f32 / 40.0;
                    let a = t * std::f32::consts::TAU * 1.5;
                    at(0.12 + 0.76 * t + 0.08 * a.sin(), 0.5 - 0.25 * a.cos() * (1.0 - t * 0.4))
                })
                .collect();
            dashed(painter, &pts, line, s * 0.1);
        }
        Drawing::Shape => fill(
            painter,
            &[[(0.12, 0.72), (0.2, 0.22), (0.62, 0.1), (0.9, 0.45), (0.68, 0.9)]
                .map(|(x, y)| at(x, y))
                .to_vec()],
            line,
            thin,
            gap,
        ),
        Drawing::ComplexFill => fill(
            painter,
            &[
                [(0.1, 0.2), (0.9, 0.12), (0.88, 0.88), (0.14, 0.8)]
                    .map(|(x, y)| at(x, y))
                    .to_vec(),
                ellipse(at(0.5, 0.5), s * 0.17, s * 0.14),
            ],
            line,
            thin,
            gap,
        ),
        Drawing::FreeShape => {
            let ring: Vec<Pos2> = (0..36)
                .map(|i| {
                    let a = i as f32 / 36.0 * std::f32::consts::TAU;
                    let r = s * (0.36 + 0.07 * (3.0 * a).sin());
                    at(0.5, 0.5) + vec2(a.cos(), a.sin()) * r
                })
                .collect();
            fill(painter, &[ring], line, thin, gap);
        }
        Drawing::Rectangle => fill(
            painter,
            &[rect_ring(at(0.1, 0.22), at(0.9, 0.78), 0.0)],
            line,
            thin,
            gap,
        ),
        Drawing::RoundRect => fill(
            painter,
            &[rect_ring(at(0.1, 0.22), at(0.9, 0.78), s * 0.16)],
            line,
            thin,
            gap,
        ),
        Drawing::Ellipse => fill(painter, &[ellipse(at(0.5, 0.5), s * 0.42, s * 0.32)], line, thin, gap),
        Drawing::Star => {
            let ring: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = (i as f32 * 36.0 - 90.0).to_radians();
                    let r = if i % 2 == 0 { s * 0.46 } else { s * 0.2 };
                    at(0.5, 0.54) + vec2(a.cos(), a.sin()) * r
                })
                .collect();
            fill(painter, &[ring], line, thin, gap * 0.85);
        }
        Drawing::Column | Drawing::ColumnB => {
            let (a, b) = band(at, s);
            satin(painter, &a, &b, thin);
            let rail = if d == Drawing::ColumnB { line } else { thin };
            painter.add(egui::Shape::line(a.clone(), rail));
            painter.add(egui::Shape::line(b.clone(), rail));
            if d == Drawing::ColumnB {
                for p in [a[0], *a.last().unwrap(), b[0], *b.last().unwrap()] {
                    painter.circle_filled(p, w * 1.1, color);
                }
            }
        }
        Drawing::ColumnC => {
            let (a, b) = band(at, s);
            satin(painter, &a, &b, line);
            let mid: Vec<Pos2> = a.iter().zip(&b).map(|(p, q)| p.lerp(*q, 0.5)).collect();
            dashed(painter, &mid, thin, s * 0.06);
        }
        Drawing::Ring => {
            let a = ellipse(at(0.5, 0.5), s * 0.44, s * 0.44);
            let b = ellipse(at(0.5, 0.5), s * 0.24, s * 0.24);
            let mut pts = Vec::new();
            for i in 0..=a.len() {
                pts.push(if i % 2 == 0 { a[i % a.len()] } else { b[i % b.len()] });
            }
            painter.add(egui::Shape::line(pts, thin));
        }
        Drawing::Turning => {
            let (a, b) = band(at, s);
            let mut outline = a.clone();
            outline.extend(b.iter().rev());
            painter.add(egui::Shape::closed_line(outline, line));
            for (p, q) in a.iter().zip(&b).skip(1).take(a.len() - 2) {
                painter.line_segment([*p, *q], thin);
            }
        }
        Drawing::Applique => {
            // The fabric patch, the dashed placement line and the satin border.
            let outer = rect_ring(at(0.12, 0.16), at(0.88, 0.84), s * 0.14);
            painter.add(egui::Shape::convex_polygon(
                outer.clone(),
                color.gamma_multiply(0.25),
                Stroke::NONE,
            ));
            painter.add(egui::Shape::closed_line(outer, Stroke::new(w * 2.2, color)));
            let inner = rect_ring(at(0.3, 0.34), at(0.7, 0.66), s * 0.06);
            dashed(painter, &closed(&inner), thin, s * 0.07);
        }
        Drawing::AddHole | Drawing::RemoveHoles | Drawing::FillHoles => {
            let outer = rect_ring(at(0.06, 0.14), at(0.8, 0.82), s * 0.08);
            let hole = ellipse(at(0.43, 0.48), s * 0.17, s * 0.17);
            match d {
                Drawing::AddHole => fill(painter, &[outer, hole], line, thin, gap),
                Drawing::RemoveHoles => {
                    fill(painter, &[outer], line, thin, gap);
                    dashed(painter, &closed(&hole), line, s * 0.07);
                }
                _ => {
                    painter.add(egui::Shape::closed_line(outer, line));
                    fill(painter, &[hole], line, thin, gap * 0.7);
                }
            }
            badge(painter, at(0.8, 0.8), s * 0.17, d != Drawing::RemoveHoles, line, bg);
        }
    }
}

/// A plus or minus sign in a circle, over what is under it.
fn badge(painter: &Painter, c: Pos2, r: f32, plus: bool, stroke: Stroke, bg: Color32) {
    painter.circle(c, r, bg, stroke);
    let d = r * 0.55;
    painter.line_segment([c - vec2(d, 0.0), c + vec2(d, 0.0)], stroke);
    if plus {
        painter.line_segment([c - vec2(0.0, d), c + vec2(0.0, d)], stroke);
    }
}

/// An S-shaped line from the lower left to the upper right.
fn wave(at: impl Fn(f32, f32) -> Pos2) -> Vec<Pos2> {
    (0..=32)
        .map(|i| {
            let t = i as f32 / 32.0;
            at(
                0.08 + 0.84 * t,
                0.62 - 0.26 * t + 0.18 * (t * std::f32::consts::TAU).sin(),
            )
        })
        .collect()
}

/// The two sides of a curved column from the lower left to the upper right.
fn band(at: impl Fn(f32, f32) -> Pos2, s: f32) -> (Vec<Pos2>, Vec<Pos2>) {
    let n = 11;
    let mid: Vec<Pos2> = (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            at(0.12 + 0.76 * t, 0.8 - 0.6 * t + 0.16 * (t * std::f32::consts::PI).sin())
        })
        .collect();
    let half = s * 0.13;
    let side = |sign: f32| -> Vec<Pos2> {
        (0..n)
            .map(|i| {
                let d = mid[(i + 1).min(n - 1)] - mid[i.saturating_sub(1)];
                let normal = vec2(-d.y, d.x).normalized();
                mid[i] + normal * half * sign
            })
            .collect()
    };
    (side(1.0), side(-1.0))
}

/// Satin stitches: a zigzag from side to side.
fn satin(painter: &Painter, a: &[Pos2], b: &[Pos2], stroke: Stroke) {
    let pts: Vec<Pos2> = (0..a.len()).map(|i| if i % 2 == 0 { a[i] } else { b[i] }).collect();
    painter.add(egui::Shape::line(pts, stroke));
}

fn ellipse(c: Pos2, rx: f32, ry: f32) -> Vec<Pos2> {
    (0..32)
        .map(|i| {
            let a = i as f32 / 32.0 * std::f32::consts::TAU;
            c + vec2(rx * a.cos(), ry * a.sin())
        })
        .collect()
}

/// A rectangle with round corners of radius `r` (0: sharp corners).
fn rect_ring(min: Pos2, max: Pos2, r: f32) -> Vec<Pos2> {
    if r <= 0.0 {
        return vec![min, pos2(max.x, min.y), max, pos2(min.x, max.y)];
    }
    let corners = [
        (pos2(max.x - r, min.y + r), -90.0),
        (pos2(max.x - r, max.y - r), 0.0),
        (pos2(min.x + r, max.y - r), 90.0),
        (pos2(min.x + r, min.y + r), 180.0),
    ];
    corners
        .iter()
        .flat_map(|&(c, start)| {
            (0..=4).map(move |k| {
                let a = (start + 22.5 * k as f32).to_radians();
                c + vec2(a.cos(), a.sin()) * r
            })
        })
        .collect()
}

fn closed(ring: &[Pos2]) -> Vec<Pos2> {
    let mut v = ring.to_vec();
    v.push(ring[0]);
    v
}

/// Running stitches: dashes of length `dash` along the line, with gaps of
/// half that length.
fn dashed(painter: &Painter, pts: &[Pos2], stroke: Stroke, dash: f32) {
    let (on, off) = (dash, dash * 0.5);
    let mut left = on;
    let mut drawing = true;
    let mut seg = vec![pts[0]];
    for w in pts.windows(2) {
        let (mut p, q) = (w[0], w[1]);
        let mut len = p.distance(q);
        while len > 0.0 {
            let step = left.min(len);
            let next = p + (q - p).normalized() * step;
            if drawing {
                seg.push(next);
            }
            left -= step;
            len -= step;
            p = next;
            if left <= 0.0 {
                if drawing {
                    painter.add(egui::Shape::line(std::mem::take(&mut seg), stroke));
                } else {
                    seg = vec![p];
                }
                drawing = !drawing;
                left = if drawing { on } else { off };
            }
        }
    }
    if drawing && seg.len() > 1 {
        painter.add(egui::Shape::line(seg, stroke));
    }
}

/// A fill: the outlines, and hatch lines at 45° inside them (even-odd, so a
/// second ring is a hole).
fn fill(painter: &Painter, rings: &[Vec<Pos2>], outline: Stroke, hatch: Stroke, gap: f32) {
    // Hatch lines are x + y = c. Along a line, v = x − y gives the position.
    let us = || rings.iter().flatten().map(|p| p.x + p.y);
    let (lo, hi) = (us().fold(f32::MAX, f32::min), us().fold(f32::MIN, f32::max));
    let mut c = lo + gap * 0.5;
    while c < hi {
        let mut vs = Vec::new();
        for ring in rings {
            for i in 0..ring.len() {
                let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
                let (up, uq) = (p.x + p.y, q.x + q.y);
                if (up - c) * (uq - c) < 0.0 {
                    let t = (c - up) / (uq - up);
                    vs.push((p.x - p.y) + t * ((q.x - q.y) - (p.x - p.y)));
                }
            }
        }
        vs.sort_by(f32::total_cmp);
        for pair in vs.as_chunks::<2>().0 {
            let ends = pair.map(|v| pos2((c + v) / 2.0, (c - v) / 2.0));
            painter.line_segment(ends, hatch);
        }
        c += gap;
    }
    for ring in rings {
        painter.add(egui::Shape::closed_line(ring.clone(), outline));
    }
}

#[cfg(test)]
mod cjk_tests {
    use super::*;

    #[test]
    fn japanese_text_has_glyphs_when_the_computer_has_a_font() {
        let ctx = egui::Context::default();
        if !install_with(&ctx, true) {
            // No Japanese font on this computer (a minimal Linux): nothing to test.
            return;
        }
        let mut out = ctx.run_ui(Default::default(), |_| {});
        out.textures_delta.clear();
        let font = egui::FontId::proportional(14.0);
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&font, "刺繍デザインを保存")));
        // The icons and the Latin text stay.
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&font, "Save")));
    }
}

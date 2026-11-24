//! Mitre corners (ADR 0086) stay in the footprint of the letters: no
//! stitch of mitred letters is more than [`TOL_MM`] out of the area of the
//! same letters with laps. That area is the stitches of the laps with the
//! notches between two lapped columns filled (narrower than two times
//! [`CLOSE_MM`]): a mitre corner can fill them, as the letter does.

use emb_cli::{Input, stitches};
use emb_geom::Point;
use emb_model::{Design, Justify, Lettering, ObjectKind, Rgb, StitchKind};

const TOL_MM: f64 = 0.3;
const CLOSE_MM: f64 = 0.8;
const CELL_MM: f64 = 0.1;
const STEP_MM: f64 = 0.1;

const TEXT: &str = "EFL7ZTHU";

fn design(font: &dyn emb_letter::LetterSource, mitre: bool) -> Design {
    let mut d = Design::new("");
    let t = d.add_thread("Navy", Rgb(25, 40, 100));
    let mut l = Lettering::new(TEXT, font.font_ref(), 20.0, Point::new(0.0, 10.0));
    l.justify = Justify::Center;
    l.letter_spacing_mm = 3.0;
    l.mitre_corners = mitre;
    emb_letter::relayout(&mut l, font);
    d.add_object(t, ObjectKind::Lettering(Box::new(l)));
    d
}

fn part_count(d: &Design) -> usize {
    match &d.objects()[0].kind {
        ObjectKind::Lettering(l) => l.parts().count(),
        _ => unreachable!(),
    }
}

/// Points along each stitch (from one needle point to the next), not more
/// than [`STEP_MM`] apart.
fn thread_points(d: Design) -> Vec<Point> {
    let (s, _) = stitches(&Input::Design(Box::new(d)), None).unwrap();
    let mut out = Vec::new();
    for w in s.records.windows(2) {
        if w[0].kind != StitchKind::Stitch || w[1].kind != StitchKind::Stitch {
            continue;
        }
        let (p, q) = (w[0].pos, w[1].pos);
        let n = (p.distance(q) / STEP_MM).ceil().max(1.0) as usize;
        out.extend((0..=n).map(|k| p.lerp(q, k as f64 / n as f64)));
    }
    out
}

/// A grid of cells, each covered or not.
struct Grid {
    cols: usize,
    rows: usize,
    on: Vec<bool>,
}

impl Grid {
    /// Each cell within `r` cells of a covered cell becomes covered.
    fn dilate(&self, r: i64) -> Grid {
        let mut on = vec![false; self.on.len()];
        let (cols, rows) = (self.cols as i64, self.rows as i64);
        for y in 0..rows {
            for x in 0..cols {
                if !self.on[(y * cols + x) as usize] {
                    continue;
                }
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (u, v) = (x + dx, y + dy);
                        if dx * dx + dy * dy <= r * r && (0..cols).contains(&u) && (0..rows).contains(&v) {
                            on[(v * cols + u) as usize] = true;
                        }
                    }
                }
            }
        }
        Grid { on, ..*self }
    }

    fn invert(&self) -> Grid {
        Grid {
            on: self.on.iter().map(|v| !v).collect(),
            ..*self
        }
    }
}

/// The points of `mitred` that are more than [`TOL_MM`] out of the area of
/// `lapped` (with its notches filled).
fn outside(lapped: &[Point], mitred: &[Point]) -> Vec<Point> {
    let all = || lapped.iter().chain(mitred);
    let margin = 2.0 * (CLOSE_MM + TOL_MM);
    let x0 = all().map(|p| p.x).fold(f64::MAX, f64::min) - margin;
    let y0 = all().map(|p| p.y).fold(f64::MAX, f64::min) - margin;
    let x1 = all().map(|p| p.x).fold(f64::MIN, f64::max) + margin;
    let y1 = all().map(|p| p.y).fold(f64::MIN, f64::max) + margin;
    let cols = ((x1 - x0) / CELL_MM).ceil() as usize + 1;
    let rows = ((y1 - y0) / CELL_MM).ceil() as usize + 1;
    let cell = |p: &Point| {
        let (x, y) = (((p.x - x0) / CELL_MM).round(), ((p.y - y0) / CELL_MM).round());
        y as usize * cols + x as usize
    };
    let mut grid = Grid {
        cols,
        rows,
        on: vec![false; cols * rows],
    };
    for p in lapped {
        grid.on[cell(p)] = true;
    }
    let close = (CLOSE_MM / CELL_MM).round() as i64;
    // Closing: dilate, then erode (an erosion is a dilation of the outside).
    let area = grid.dilate(1).dilate(close).invert().dilate(close).invert();
    let area = area.dilate((TOL_MM / CELL_MM).round() as i64);
    mitred.iter().filter(|p| !area.on[cell(p)]).copied().collect()
}

fn check(font: &dyn emb_letter::LetterSource, name: &str) {
    let (lapped, mitred) = (design(font, false), design(font, true));
    assert!(
        part_count(&mitred) < part_count(&lapped),
        "{name}: some corners are mitred"
    );
    let out = outside(&thread_points(lapped), &thread_points(mitred));
    assert!(
        out.is_empty(),
        "{name}: {} points out of the letters, for example {:?}",
        out.len(),
        &out[..out.len().min(5)]
    );
}

/// The trims of a design.
fn trims(d: Design) -> usize {
    stitches(&Input::Design(Box::new(d)), None).unwrap().0.trim_count()
}

#[test]
fn mitred_embroidery_font_letters_stay_in_their_footprint() {
    // A font as the converter makes it, with no route: all its corners
    // can join.
    let source = emb_letter::bundled::font("Anton", "Regular").expect("bundled font");
    let chars: Vec<char> = TEXT.chars().collect();
    let f = emb_letter::embfont::convert(&source, "Anton", "", emb_model::LetterStitch::Auto, &chars);
    check(&f, "Anton");
}

#[test]
fn mitred_letters_of_a_routed_font_keep_their_route() {
    // The embroidery fonts of the program are routed (ADR 0116): their
    // letters sew in the order of the font. A corner is mitred where its
    // two strokes sew one after the other (ADR 0121). The stitches stay
    // in the letters, and no letter gets a new trim.
    for name in ["Anton", "Varsity", "Block Sans"] {
        let f = emb_letter::bundled::embroidery(name).expect("bundled embroidery font");
        check(&f, name);
        assert!(trims(design(&f, true)) <= trims(design(&f, false)), "{name}");
        // The letters are still fixed, and a travel run is still a run.
        let d = design(&f, true);
        let ObjectKind::Lettering(l) = &d.objects()[0].kind else {
            unreachable!()
        };
        assert!(l.letters.iter().all(|x| x.fixed || x.parts.is_empty()), "{name}");
        assert!(l.parts().any(|p| matches!(p, ObjectKind::Running { .. })), "{name}");
    }
    // All letters and digits of four fonts: not more trims with the
    // setting on than with it off.
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789";
    for name in ["Block Sans", "Varsity", "Pacifico", "Geo Slab"] {
        let f = emb_letter::bundled::embroidery(name).expect("bundled embroidery font");
        let of = |mitre: bool| {
            let mut d = Design::new("");
            let t = d.add_thread("Navy", Rgb(25, 40, 100));
            let mut l = Lettering::new(
                text,
                emb_letter::LetterSource::font_ref(&f),
                20.0,
                Point::new(0.0, 10.0),
            );
            l.mitre_corners = mitre;
            emb_letter::relayout(&mut l, &f);
            d.add_object(t, ObjectKind::Lettering(Box::new(l)));
            d
        };
        let (lapped, mitred) = (of(false), of(true));
        // Corners join in the fonts with corners; the script (Pacifico) has
        // almost none (one in its "f" before P21 Milestone 14, none since).
        if name == "Pacifico" {
            assert!(part_count(&mitred) <= part_count(&lapped), "{name}");
        } else {
            assert!(part_count(&mitred) < part_count(&lapped), "{name}");
        }
        assert!(trims(mitred) <= trims(lapped), "{name}");
    }
}

#[test]
fn mitred_truetype_letters_stay_in_their_footprint() {
    for style in ["Bold", "Regular"] {
        let f = emb_letter::bundled::font("Lato", style).expect("bundled font");
        check(&f, style);
    }
}

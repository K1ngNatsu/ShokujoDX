//! EPS and PostScript artwork import, also AI files of the old kind
//! (PostScript, not PDF inside) (P18 Milestone 8; ADR 0138).
//!
//! The reader is a PostScript interpreter in pure Rust (`eps/vm.rs`,
//! `eps/ops.rs`, `eps/graphics.rs`), from the public PostScript language
//! reference (third edition) and the EPS file format specification
//! (version 3.0). It runs the program of the file: its procedures and
//! dictionaries, the transform, the graphics state, paths, colors and color
//! spaces. The painted paths become the same objects as the other artwork
//! readers give (fills and lines, or vectors). Text and pictures are not
//! imported (warnings); patterns and gradients are gray; the clip is not
//! used. The interpreter has limits for the steps, the memory and the
//! errors, so a bad file stops with what it drew.
//!
//! AI files of the old kind draw with the operators of the AI script, which
//! a procedure set defines. Such a file does not always contain the set, so
//! the reader runs the script part with its own operators (`eps/ai.rs`).
//! A file runs in the other way too when the first way draws nothing.
//!
//! PostScript units are points (1/72 inch) with y up; the result is in mm
//! with y down and the top-left of the bounding box at (0, 0).

mod ai;
mod filters;
mod graphics;
mod ops;
mod scan;
mod vm;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;

use emb_geom::{Path, Point, Transform, text};
use emb_model::Rgb;

use crate::svg::{FALLBACK, Painted, SvgError, SvgImport, add_painted};
use graphics::{Drawing, El, Paint};
use vm::{Limits, PsError, Vm};

/// mm per PostScript point.
const MM_PER_PT: f64 = 25.4 / 72.0;
/// The start of an EPS file with a binary header (with a preview picture).
const DOS_EPS: [u8; 4] = [0xC5, 0xD0, 0xD3, 0xC6];
/// A letter page, for a PostScript file with no bounding box.
const LETTER: [f64; 4] = [0.0, 0.0, 612.0, 792.0];

/// `true` if `data` is a PostScript or EPS file.
pub fn is_postscript(data: &[u8]) -> bool {
    postscript_part(data).is_some()
}

/// The PostScript part of a file: the part that the binary header of an
/// EPS file gives, or the text from `%!`.
fn postscript_part(data: &[u8]) -> Option<&[u8]> {
    if data.starts_with(&DOS_EPS) {
        let word = |i: usize| -> Option<usize> {
            let b: [u8; 4] = data.get(i..i + 4)?.try_into().ok()?;
            Some(u32::from_le_bytes(b) as usize)
        };
        let (start, len) = (word(4)?, word(8)?);
        return data.get(start..start.checked_add(len)?);
    }
    let head = &data[..data.len().min(1024)];
    let i = head.windows(2).position(|w| w == b"%!")?;
    // Before `%!`: only white space, or the job line of a printer.
    let before = &head[..i];
    (before.iter().all(|&c| scan::is_white(c)) || before.starts_with(b"\x1b%-12345X")).then(|| &data[i..])
}

/// The value of the first comment line `key` that has four numbers.
fn box_comment(ps: &[u8], key: &[u8]) -> Option<[f64; 4]> {
    let mut from = 0;
    while let Some(i) = ps[from..].windows(key.len()).position(|w| w == key) {
        let start = from + i + key.len();
        let end = ps[start..]
            .iter()
            .position(|&c| c == b'\n' || c == b'\r')
            .map_or(ps.len(), |e| start + e);
        let line = String::from_utf8_lossy(&ps[start..end]);
        let v: Vec<f64> = line.split_whitespace().filter_map(|w| w.parse().ok()).collect();
        if v.len() == 4 && v[2] > v[0] && v[3] > v[1] {
            return Some([v[0], v[1], v[2], v[3]]);
        }
        // `(atend)`: the value is in the trailer.
        from = end;
    }
    None
}

fn find(ps: &[u8], what: &[u8]) -> Option<usize> {
    ps.windows(what.len()).position(|w| w == what)
}

/// The number of pages that the comments give.
fn page_count(ps: &[u8]) -> usize {
    let Some(i) = find(ps, b"%%Pages:") else {
        return 1;
    };
    let rest = &ps[i + 8..];
    let end = rest
        .iter()
        .position(|&c| c == b'\n' || c == b'\r')
        .unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end])
        .split_whitespace()
        .next()
        .and_then(|w| w.parse().ok())
        .unwrap_or(1)
}

/// `true` if the file has the comments of an AI script, and not the
/// printing procedure set of newer files (which draws with PostScript).
fn looks_like_ai_script(ps: &[u8]) -> bool {
    (find(ps, b"%AI5_").is_some() || find(ps, b"%AI3_").is_some()) && find(ps, b"AGM_Core").is_none()
}

/// The script part of an AI file: after the setup, to the trailer.
fn ai_script(ps: &[u8]) -> &[u8] {
    let start = find(ps, b"%%EndSetup")
        .or_else(|| find(ps, b"%%EndProlog"))
        .map_or(0, |i| i + 10);
    let rest = &ps[start..];
    let end = [b"%%PageTrailer".as_slice(), b"%%Trailer"]
        .iter()
        .filter_map(|m| find(rest, m))
        .min()
        .unwrap_or(rest.len());
    &rest[..end]
}

/// What one run gave.
struct Run {
    out: Drawing,
    errors: usize,
    first_error: Option<String>,
    stop: Option<PsError>,
}

fn run(ps: &[u8], ai: bool, page: [f64; 4], limits: Limits) -> Run {
    let program = if ai { ai_script(ps) } else { ps };
    let mut vm = Vm::new(program.to_vec(), ai, limits);
    vm.out.page = page;
    let stop = vm.run().err().filter(|e| *e != PsError::Quit);
    Run {
        out: std::mem::take(&mut vm.out),
        errors: vm.errors,
        first_error: vm.first_error.take(),
        stop,
    }
}

fn rgb(p: Paint) -> Rgb {
    match p {
        Paint::Color(c) => c,
        Paint::Pattern => FALLBACK,
    }
}

/// The paths of a painted path, in points.
fn paths(subs: &[graphics::Sub]) -> Vec<Path> {
    let pt = |p: [f64; 2]| Point::new(p[0], p[1]);
    subs.iter()
        .map(|s| {
            let mut path = Path::new();
            for el in &s.els {
                path = match *el {
                    El::Move(p) => path.move_to(pt(p)),
                    El::Line(p) => path.line_to(pt(p)),
                    El::Curve(a, b, c) => path.cubic_to(pt(a), pt(b), pt(c)),
                };
            }
            if s.closed { path.close() } else { path }
        })
        .collect()
}

/// Reads the artwork of an EPS or PostScript file (or an AI file of the
/// old kind). `vectors`: one vector object for each painted path, else
/// fills and running lines (as the SVG import).
pub fn import_eps(data: &[u8], vectors: bool) -> Result<SvgImport, SvgError> {
    import_with(data, vectors, Limits::default())
}

fn import_with(data: &[u8], vectors: bool, limits: Limits) -> Result<SvgImport, SvgError> {
    let Some(ps) = postscript_part(data) else {
        return Err(SvgError::Parse(text::tr("not a PostScript or EPS file").into()));
    };
    let page = box_comment(ps, b"%%HiResBoundingBox:")
        .or_else(|| box_comment(ps, b"%%BoundingBox:"))
        .unwrap_or(LETTER);
    let ai_first = looks_like_ai_script(ps);
    let mut r = run(ps, ai_first, page, limits);
    if r.out.drawn.is_empty() {
        let other = run(ps, !ai_first, page, limits);
        if !other.out.drawn.is_empty() {
            r = other;
        }
    }
    let [x0, _, _, y1] = page;
    let to_mm = Transform::translate(-x0, -y1).then(Transform::scale(MM_PER_PT, -MM_PER_PT));
    let mut out = SvgImport::default();
    for (i, d) in r.out.drawn.iter().enumerate() {
        let painted = Painted {
            subs: paths(&d.subs).iter().map(|p| p.transformed(&to_mm)).collect(),
            fill: d.fill.map(rgb),
            stroke: d.stroke.map(|(p, w)| (rgb(p), (w * MM_PER_PT).max(0.1))),
            name: Some(format!("path {}", i + 1)),
        };
        add_painted(&mut out, painted, vectors);
    }
    let w = &mut out.warnings;
    let pages = page_count(ps);
    if pages > 1 {
        w.push(text::fill("Only the first of {} pages imported.", &[&pages]));
    }
    let o = &r.out;
    if o.text > 0 {
        w.push(text::fill(
            "{} text object(s) not imported. Convert text to outlines first.",
            &[&o.text],
        ));
    }
    if o.images > 0 {
        w.push(text::fill("{} image(s) not imported.", &[&o.images]));
    }
    if o.patterns > 0 {
        w.push(text::fill(
            "{} gradient or pattern paint(s) imported as gray.",
            &[&o.patterns],
        ));
    }
    if o.shadings > 0 {
        w.push(text::fill("{} smooth shading(s) not imported.", &[&o.shadings]));
    }
    if o.hidden_layers > 0 {
        w.push(text::fill("{} hidden layer(s) not imported.", &[&o.hidden_layers]));
    }
    match r.stop {
        Some(PsError::TooManySteps) => w.push(
            text::tr("The PostScript program ran too long; the import has what it drew before it stopped.").into(),
        ),
        Some(PsError::VmError) => w.push(
            text::tr("The PostScript program used too much memory; the import has what it drew before it stopped.")
                .into(),
        ),
        Some(PsError::TooManyShapes) => w.push(text::fill(
            "The PostScript program painted more than {} shapes; the import has the first {}.",
            &[&graphics::MAX_SHAPES, &graphics::MAX_SHAPES],
        )),
        _ => {}
    }
    if r.errors > 0 {
        w.push(text::fill(
            "{} PostScript error(s); the first: {}. Some shapes can be missing.",
            &[&r.errors, &r.first_error.as_deref().unwrap_or("?")],
        ));
    }
    Ok(crate::svg::drop_far(out))
}

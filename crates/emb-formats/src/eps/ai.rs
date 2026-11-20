//! The operators of the AI script (ADR 0138): the drawing part of AI files
//! of the old kind (PostScript, before AI files had a PDF inside) and of
//! the EPS files that such programs saved. The script comes after
//! `%%EndSetup`; its operators are short names that a procedure set
//! defines, and an AI file does not always contain that set. So the
//! reader runs the script with its own operators:
//!
//! - paths: `m`, `l`/`L`, `c`/`C`, `v`/`V` (the first handle is the
//!   current point), `y`/`Y` (the second handle is the end point);
//! - painting: `N` (none), `F` (fill), `S` (stroke), `B` (both); the
//!   lowercase forms close the path first; `H`/`h` as `N`/`n`;
//! - compound paths (`*u` … `*U`: one shape with holes), clip paths (`W`:
//!   the next path is not painted), groups (`u`, `U`, `q`, `Q`);
//! - colors: `g`/`G` (gray), `k`/`K` (CMYK), `x`/`X` (a named CMYK color
//!   with a tint, 0 for the full color), `Xa`/`XA` (RGB), `Xx`/`XX` (a
//!   named color: the last value is 0 for CMYK, 1 for RGB), `p`/`P`
//!   (patterns: gray); `w` (line width);
//! - gradients (`Bb` … `BB`: gray), text objects (`To` … `TO`: counted,
//!   not drawn), pictures (`XI`: counted), layers (`Lb` … `LB`: the first
//!   value of `Lb` is 0 for a hidden layer, which is not imported).
//!
//! Each operator takes all the operands before it; an operator that the
//! reader does not use clears them. The published description of the
//! format (AI file format specification, versions 3 to 7) gives these
//! operators; the reader was tested with scripts that we wrote, not yet
//! with files of testers.

use super::graphics::{self, El, Paint, Sub, cmyk, color_of};
use super::vm::{Obj, OpFn, R, Vm};

pub(super) const AI: &[(&str, OpFn)] = &[
    ("m", m),
    ("l", l),
    ("L", l),
    ("c", c),
    ("C", c),
    ("v", v),
    ("V", v),
    ("y", y),
    ("Y", y),
    ("N", no_paint),
    ("n", close_no_paint),
    ("H", no_paint),
    ("h", close_no_paint),
    ("F", fill),
    ("f", close_fill),
    ("S", stroke),
    ("s", close_stroke),
    ("B", both),
    ("b", close_both),
    ("W", clip),
    ("*u", compound_begin),
    ("*U", compound_end),
    ("u", clear),
    ("U", clear),
    ("q", clear),
    ("Q", clear),
    ("g", fill_gray),
    ("G", stroke_gray),
    ("k", fill_cmyk),
    ("K", stroke_cmyk),
    ("x", fill_custom),
    ("X", stroke_custom),
    ("Xa", fill_rgb),
    ("XA", stroke_rgb),
    ("Xx", fill_named),
    ("XX", stroke_named),
    ("p", fill_pattern),
    ("P", stroke_pattern),
    ("w", width),
    ("Bb", gradient_begin),
    ("BB", gradient_end),
    ("To", text_begin),
    ("TO", text_end),
    ("XI", raster),
    ("Lb", layer_begin),
    ("LB", layer_end),
];

/// The state of the AI script.
#[derive(Debug, Default)]
pub(super) struct AiState {
    compound: Option<Compound>,
    clip_next: bool,
    text_depth: u32,
    /// For each open layer: `true` if it is hidden.
    layers: Vec<bool>,
    gradient: bool,
}

#[derive(Debug, Default)]
struct Compound {
    subs: Vec<Sub>,
    fill: bool,
    stroke: bool,
}

impl AiState {
    /// Paths are not painted: in a text object or in a hidden layer.
    pub(super) fn skips(&self) -> bool {
        self.text_depth > 0 || self.layers.iter().any(|&h| h)
    }
}

fn clear(vm: &mut Vm) -> R {
    vm.ops.clear();
    Ok(())
}

/// The numbers on the stack (all of them, the deepest first).
fn values(vm: &Vm) -> Vec<f64> {
    vm.ops.iter().filter_map(|&o| super::vm::num(o).ok()).collect()
}

/// The last `n` numbers of the stack.
fn last<const N: usize>(vm: &mut Vm) -> R<[f64; N]> {
    let v = vm.nums::<N>()?;
    vm.ops.clear();
    Ok(v)
}

fn point(vm: &Vm, x: f64, y: f64) -> [f64; 2] {
    graphics::apply(&vm.g.ctm, x, y)
}

fn m(vm: &mut Vm) -> R {
    let [x, y] = last::<2>(vm)?;
    let p = point(vm, x, y);
    graphics::move_device(vm, p)
}

fn l(vm: &mut Vm) -> R {
    let [x, y] = last::<2>(vm)?;
    let p = point(vm, x, y);
    graphics::add_device(vm, El::Line(p), p)
}

fn c(vm: &mut Vm) -> R {
    let v = last::<6>(vm)?;
    let (a, b, e) = (point(vm, v[0], v[1]), point(vm, v[2], v[3]), point(vm, v[4], v[5]));
    graphics::add_device(vm, El::Curve(a, b, e), e)
}

fn v(vm: &mut Vm) -> R {
    let v = last::<4>(vm)?;
    let start = vm.g.cur.ok_or(super::vm::PsError::RangeCheck)?;
    let (b, e) = (point(vm, v[0], v[1]), point(vm, v[2], v[3]));
    graphics::add_device(vm, El::Curve(start, b, e), e)
}

fn y(vm: &mut Vm) -> R {
    let v = last::<4>(vm)?;
    let (a, e) = (point(vm, v[0], v[1]), point(vm, v[2], v[3]));
    graphics::add_device(vm, El::Curve(a, e, e), e)
}

/// The paint operators: `close` closes the path first.
fn paint(vm: &mut Vm, close: bool, fill: bool, stroke: bool) -> R {
    vm.ops.clear();
    if close {
        graphics::close_path(vm);
    }
    if std::mem::take(&mut vm.ai.clip_next) {
        graphics::new_path(vm);
        return Ok(());
    }
    if let Some(comp) = vm.ai.compound.as_mut() {
        // A part of a compound path: painted at `*U`.
        comp.subs
            .extend(std::mem::take(&mut vm.g.path).into_iter().filter(|s| s.els.len() > 1));
        comp.fill = fill;
        comp.stroke = stroke;
        graphics::new_path(vm);
        return Ok(());
    }
    paint_now(vm, fill, stroke);
    Ok(())
}

fn paint_now(vm: &mut Vm, fill: bool, stroke: bool) {
    if !(fill || stroke) {
        graphics::new_path(vm);
        return;
    }
    let keep = vm.g.fill;
    if vm.ai.gradient && fill {
        vm.g.fill = Paint::Pattern;
    }
    graphics::paint(vm, fill, stroke);
    vm.g.fill = keep;
}

fn no_paint(vm: &mut Vm) -> R {
    paint(vm, false, false, false)
}

fn close_no_paint(vm: &mut Vm) -> R {
    paint(vm, true, false, false)
}

fn fill(vm: &mut Vm) -> R {
    paint(vm, false, true, false)
}

fn close_fill(vm: &mut Vm) -> R {
    paint(vm, true, true, false)
}

fn stroke(vm: &mut Vm) -> R {
    paint(vm, false, false, true)
}

fn close_stroke(vm: &mut Vm) -> R {
    paint(vm, true, false, true)
}

fn both(vm: &mut Vm) -> R {
    paint(vm, false, true, true)
}

fn close_both(vm: &mut Vm) -> R {
    paint(vm, true, true, true)
}

fn clip(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.clip_next = true;
    Ok(())
}

fn compound_begin(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.compound = Some(Compound::default());
    Ok(())
}

fn compound_end(vm: &mut Vm) -> R {
    vm.ops.clear();
    if let Some(comp) = vm.ai.compound.take() {
        graphics::new_path(vm);
        vm.g.path = comp.subs;
        paint_now(vm, comp.fill, comp.stroke);
    }
    Ok(())
}

fn set(vm: &mut Vm, fill: bool, p: Paint) {
    if fill {
        vm.g.fill = p;
    } else {
        vm.g.stroke = p;
    }
    vm.ops.clear();
}

fn gray(vm: &mut Vm, fill: bool) -> R {
    let [g] = last::<1>(vm)?;
    set(vm, fill, Paint::Color(color_of(graphics::Base::Gray, &[g])));
    Ok(())
}

fn fill_gray(vm: &mut Vm) -> R {
    gray(vm, true)
}

fn stroke_gray(vm: &mut Vm) -> R {
    gray(vm, false)
}

fn four(vm: &mut Vm, fill: bool) -> R {
    let [c, m, y, k] = last::<4>(vm)?;
    set(vm, fill, Paint::Color(cmyk(c, m, y, k)));
    Ok(())
}

fn fill_cmyk(vm: &mut Vm) -> R {
    four(vm, true)
}

fn stroke_cmyk(vm: &mut Vm) -> R {
    four(vm, false)
}

fn rgb_paint(r: f64, g: f64, b: f64) -> Paint {
    Paint::Color(color_of(graphics::Base::Rgb, &[r, g, b]))
}

fn three(vm: &mut Vm, fill: bool) -> R {
    let [r, g, b] = last::<3>(vm)?;
    set(vm, fill, rgb_paint(r, g, b));
    Ok(())
}

fn fill_rgb(vm: &mut Vm) -> R {
    three(vm, true)
}

fn stroke_rgb(vm: &mut Vm) -> R {
    three(vm, false)
}

/// A tint of a color: 0 is the full color, 1 is no ink.
fn tinted(v: f64, tint: f64) -> f64 {
    v * (1.0 - tint.clamp(0.0, 1.0))
}

/// `c m y k (name) tint x`.
fn custom(vm: &mut Vm, fill: bool) -> R {
    let v = values(vm);
    if v.len() < 5 {
        vm.ops.clear();
        return Ok(());
    }
    let n = v.len();
    let tint = v[n - 1];
    let [c, m, y, k] = [v[n - 5], v[n - 4], v[n - 3], v[n - 2]].map(|x| tinted(x, tint));
    set(vm, fill, Paint::Color(cmyk(c, m, y, k)));
    Ok(())
}

fn fill_custom(vm: &mut Vm) -> R {
    custom(vm, true)
}

fn stroke_custom(vm: &mut Vm) -> R {
    custom(vm, false)
}

/// `c m y k (name) tint 0 Xx` or `r g b (name) tint 1 Xx`.
fn named(vm: &mut Vm, fill: bool) -> R {
    let v = values(vm);
    let n = v.len();
    let paint = match v.last().copied() {
        Some(t) if t == 1.0 && n >= 5 => {
            let tint = v[n - 2];
            // A tint of an RGB color goes to white.
            let w = |x: f64| 1.0 - tinted(1.0 - x, tint);
            Some(rgb_paint(w(v[n - 5]), w(v[n - 4]), w(v[n - 3])))
        }
        Some(_) if n >= 6 => {
            let tint = v[n - 2];
            let [c, m, y, k] = [v[n - 6], v[n - 5], v[n - 4], v[n - 3]].map(|x| tinted(x, tint));
            Some(Paint::Color(cmyk(c, m, y, k)))
        }
        _ => None,
    };
    match paint {
        Some(p) => set(vm, fill, p),
        None => vm.ops.clear(),
    }
    Ok(())
}

fn fill_named(vm: &mut Vm) -> R {
    named(vm, true)
}

fn stroke_named(vm: &mut Vm) -> R {
    named(vm, false)
}

fn fill_pattern(vm: &mut Vm) -> R {
    set(vm, true, Paint::Pattern);
    Ok(())
}

fn stroke_pattern(vm: &mut Vm) -> R {
    set(vm, false, Paint::Pattern);
    Ok(())
}

fn width(vm: &mut Vm) -> R {
    let [w] = last::<1>(vm)?;
    vm.g.line_width = w.abs();
    Ok(())
}

fn gradient_begin(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.gradient = true;
    Ok(())
}

fn gradient_end(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.gradient = false;
    Ok(())
}

fn text_begin(vm: &mut Vm) -> R {
    vm.ops.clear();
    if vm.ai.text_depth == 0 {
        vm.out.text += 1;
    }
    vm.ai.text_depth += 1;
    Ok(())
}

fn text_end(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.text_depth = vm.ai.text_depth.saturating_sub(1);
    graphics::new_path(vm);
    Ok(())
}

fn raster(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.out.images += 1;
    Ok(())
}

fn layer_begin(vm: &mut Vm) -> R {
    let hidden = matches!(vm.ops.first(), Some(&Obj::Int(0)) | Some(&Obj::Real(0.0)));
    vm.ops.clear();
    if hidden && !vm.ai.skips() {
        vm.out.hidden_layers += 1;
    }
    vm.ai.layers.push(hidden);
    Ok(())
}

fn layer_end(vm: &mut Vm) -> R {
    vm.ops.clear();
    vm.ai.layers.pop();
    Ok(())
}

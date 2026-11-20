//! The graphics operators of the PostScript reader (ADR 0138): the
//! graphics state, the transform, paths, colors and color spaces, painting,
//! pictures (counted and skipped), forms and fonts (text is counted, not
//! drawn), as the PostScript language reference (chapters 4, 5 and 8)
//! describes them.
//!
//! Paths are kept in the default user space (points, y up): each point
//! goes through the transform when it is added, as in PostScript. A painted
//! path goes to the drawing; `eps.rs` makes objects in mm from it. The clip
//! is not used (as in the PDF import, ADR 0027).

use emb_geom::math;
use emb_model::Rgb;

use super::ops::dict_of;
use super::vm::{Cont, Frame, Obj, OpFn, PsError, Pushed, R, Vm, int, num};

pub(super) const GRAPHICS: &[(&str, OpFn)] = &[
    // The graphics state.
    ("gsave", gsave),
    ("grestore", grestore_op),
    ("grestoreall", grestoreall),
    ("initgraphics", initgraphics),
    ("gstate", gstate),
    ("currentgstate", currentgstate),
    ("setgstate", setgstate),
    ("setlinewidth", setlinewidth),
    ("currentlinewidth", currentlinewidth),
    ("setlinecap", setlinecap),
    ("currentlinecap", currentlinecap),
    ("setlinejoin", setlinejoin),
    ("currentlinejoin", currentlinejoin),
    ("setmiterlimit", setmiterlimit),
    ("currentmiterlimit", currentmiterlimit),
    ("setdash", setdash),
    ("currentdash", currentdash),
    ("setflat", setflat),
    ("currentflat", currentflat),
    ("setstrokeadjust", pop1),
    ("currentstrokeadjust", push_false),
    ("setoverprint", pop1),
    ("currentoverprint", push_false),
    ("setsmoothness", pop1),
    ("currentsmoothness", push_half),
    // Device settings with no effect on the artwork.
    ("setscreen", pop3),
    ("currentscreen", currentscreen),
    ("setcolorscreen", pop12),
    ("currentcolorscreen", currentcolorscreen),
    ("sethalftone", pop1),
    ("currenthalftone", empty_dict),
    ("settransfer", pop1),
    ("currenttransfer", empty_proc),
    ("setcolortransfer", pop4),
    ("currentcolortransfer", four_procs),
    ("setblackgeneration", pop1),
    ("currentblackgeneration", empty_proc),
    ("setundercolorremoval", pop1),
    ("currentundercolorremoval", empty_proc),
    ("setcolorrendering", pop1),
    ("currentcolorrendering", empty_dict),
    ("findcolorrendering", findcolorrendering),
    ("setpagedevice", pop1),
    ("currentpagedevice", currentpagedevice),
    ("nulldevice", nulldevice),
    ("showpage", showpage),
    ("copypage", nothing),
    ("erasepage", nothing),
    // Colors.
    ("setgray", setgray),
    ("currentgray", currentgray),
    ("setrgbcolor", setrgbcolor),
    ("currentrgbcolor", currentrgbcolor),
    ("sethsbcolor", sethsbcolor),
    ("currenthsbcolor", currenthsbcolor),
    ("setcmykcolor", setcmykcolor),
    ("currentcmykcolor", currentcmykcolor),
    ("setcolorspace", setcolorspace),
    ("currentcolorspace", currentcolorspace),
    ("setcolor", setcolor),
    ("currentcolor", currentcolor),
    ("setpattern", setpattern),
    ("makepattern", makepattern),
    // The transform.
    ("matrix", matrix),
    ("initmatrix", initmatrix),
    ("identmatrix", identmatrix),
    ("defaultmatrix", identmatrix),
    ("currentmatrix", currentmatrix),
    ("setmatrix", setmatrix),
    ("translate", translate),
    ("scale", scale),
    ("rotate", rotate),
    ("concat", concat),
    ("concatmatrix", concatmatrix),
    ("transform", transform),
    ("dtransform", dtransform),
    ("itransform", itransform),
    ("idtransform", idtransform),
    ("invertmatrix", invertmatrix),
    // Paths.
    ("newpath", newpath),
    ("currentpoint", currentpoint),
    ("moveto", moveto),
    ("rmoveto", rmoveto),
    ("lineto", lineto),
    ("rlineto", rlineto),
    ("curveto", curveto),
    ("rcurveto", rcurveto),
    ("arc", arc),
    ("arcn", arcn),
    ("arct", arct),
    ("arcto", arcto),
    ("closepath", closepath),
    ("flattenpath", nothing),
    ("reversepath", nothing),
    ("strokepath", strokepath),
    ("clippath", clippath),
    ("pathbbox", pathbbox),
    ("initclip", nothing),
    ("clip", nothing),
    ("eoclip", nothing),
    ("rectclip", rectclip),
    ("setbbox", pop4),
    ("ucache", nothing),
    // Painting.
    ("fill", fill),
    ("eofill", fill),
    ("stroke", stroke),
    ("rectfill", rectfill),
    ("rectstroke", rectstroke),
    ("shfill", shfill),
    ("image", image),
    ("imagemask", imagemask),
    ("colorimage", colorimage),
    ("execform", execform),
    // Fonts and text.
    ("definefont", definefont),
    ("undefinefont", pop1),
    ("findfont", findfont),
    ("scalefont", scalefont),
    ("makefont", makefont),
    ("setfont", setfont),
    ("currentfont", currentfont),
    ("rootfont", currentfont),
    ("selectfont", selectfont),
    ("composefont", composefont),
    ("show", show1),
    ("ashow", show3),
    ("widthshow", show4),
    ("awidthshow", show6),
    ("xshow", show2),
    ("yshow", show2),
    ("xyshow", show2),
    ("glyphshow", show1),
    ("cshow", show2),
    ("kshow", show2),
    ("charpath", show2),
    ("stringwidth", stringwidth),
    ("setcachedevice", pop6),
    ("setcachedevice2", pop10),
    ("setcharwidth", pop2),
];

/// The color spaces that give a color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Base {
    Gray,
    Rgb,
    Cmyk,
}

impl Base {
    fn count(self) -> usize {
        match self {
            Base::Gray => 1,
            Base::Rgb => 3,
            Base::Cmyk => 4,
        }
    }

    fn of_count(n: usize) -> Base {
        match n {
            1 => Base::Gray,
            4 => Base::Cmyk,
            _ => Base::Rgb,
        }
    }
}

/// The current color space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Space {
    Base(Base),
    Pattern,
    Indexed {
        base: Base,
        lookup: Obj,
    },
    /// Separation (one input) and DeviceN: the tint transform gives
    /// `alt` values.
    Tinted {
        inputs: usize,
        alt: Base,
        tint: Obj,
    },
}

/// A paint: a color, or a pattern or a shading (gray in the import).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Paint {
    Color(Rgb),
    Pattern,
}

/// One element of a path, in the default user space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum El {
    Move([f64; 2]),
    Line([f64; 2]),
    Curve([f64; 2], [f64; 2], [f64; 2]),
}

/// A subpath: a move, then lines and curves.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Sub {
    pub els: Vec<El>,
    pub closed: bool,
}

/// The most elements of one path.
const MAX_PATH: usize = 2_000_000;
/// The most paths that a program can paint: a logo has some thousands. More
/// paths stop the run (a program that paints in a loop with no end).
pub(super) const MAX_SHAPES: usize = 100_000;
/// The memory of an element of a path, bytes (for the memory limit).
const EL_BYTES: usize = 56;

#[derive(Debug, Clone)]
pub(super) struct GState {
    pub ctm: [f64; 6],
    pub space: Space,
    pub space_obj: Obj,
    pub comps: Vec<f64>,
    pub fill: Paint,
    pub stroke: Paint,
    pub line_width: f64,
    pub cap: i32,
    pub join: i32,
    pub miter: f64,
    pub flat: f64,
    pub dash: Option<Obj>,
    pub dash_offset: f64,
    pub path: Vec<Sub>,
    /// The number of elements of `path` (for the limit `MAX_PATH`).
    pub path_len: usize,
    pub cur: Option<[f64; 2]>,
    /// After `closepath`: the next line starts a new subpath here.
    pub reopen: Option<[f64; 2]>,
    /// The path is the clip path (`clippath`): it is not painted.
    pub path_is_clip: bool,
    /// After `strokepath`: a fill of the path strokes it with this width.
    pub stroked: Option<f64>,
    pub font: Option<u32>,
    /// After `nulldevice`: nothing is painted.
    pub hidden: bool,
}

impl Default for GState {
    fn default() -> Self {
        let black = Paint::Color(Rgb(0, 0, 0));
        GState {
            ctm: IDENTITY,
            space: Space::Base(Base::Gray),
            space_obj: Obj::Null,
            comps: vec![0.0],
            fill: black,
            stroke: black,
            line_width: 1.0,
            cap: 0,
            join: 0,
            miter: 10.0,
            flat: 1.0,
            dash: None,
            dash_offset: 0.0,
            path: Vec::new(),
            path_len: 0,
            cur: None,
            reopen: None,
            path_is_clip: false,
            stroked: None,
            font: None,
            hidden: false,
        }
    }
}

/// A painted path.
#[derive(Debug, Clone)]
pub(super) struct Drawn {
    pub subs: Vec<Sub>,
    pub fill: Option<Paint>,
    /// The paint and the width in points.
    pub stroke: Option<(Paint, f64)>,
}

/// What the program drew, and the counts of what the import leaves out.
#[derive(Debug, Default)]
pub(super) struct Drawing {
    pub drawn: Vec<Drawn>,
    /// The page (the bounding box): x0, y0, x1, y1 in points.
    pub page: [f64; 4],
    pub text: usize,
    pub images: usize,
    pub patterns: usize,
    pub shadings: usize,
    pub hidden_layers: usize,
}

impl Drawing {
    fn add(&mut self, subs: Vec<Sub>, fill: Option<Paint>, stroke: Option<(Paint, f64)>) {
        // `gsave fill grestore stroke`: one path with a fill and a stroke.
        if fill.is_none()
            && let Some(last) = self.drawn.last_mut()
            && last.fill.is_some()
            && last.stroke.is_none()
            && last.subs == subs
        {
            last.stroke = stroke;
            return;
        }
        self.drawn.push(Drawn { subs, fill, stroke });
    }
}

pub(super) const IDENTITY: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m` first, then `n`.
pub(super) fn mul(m: &[f64; 6], n: &[f64; 6]) -> [f64; 6] {
    let [a, b, c, d, e, f] = *m;
    let [a2, b2, c2, d2, e2, f2] = *n;
    [
        a * a2 + b * c2,
        a * b2 + b * d2,
        c * a2 + d * c2,
        c * b2 + d * d2,
        e * a2 + f * c2 + e2,
        e * b2 + f * d2 + f2,
    ]
}

pub(super) fn apply(m: &[f64; 6], x: f64, y: f64) -> [f64; 2] {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

fn apply_d(m: &[f64; 6], x: f64, y: f64) -> [f64; 2] {
    [m[0] * x + m[2] * y, m[1] * x + m[3] * y]
}

fn invert(m: &[f64; 6]) -> R<[f64; 6]> {
    let [a, b, c, d, e, f] = *m;
    let det = a * d - b * c;
    if det.abs() < 1e-300 {
        return Err(PsError::UndefinedResult);
    }
    Ok([
        d / det,
        -b / det,
        -c / det,
        a / det,
        (c * f - d * e) / det,
        (b * e - a * f) / det,
    ])
}

// ----- Small helpers -----

fn nothing(_: &mut Vm) -> R {
    Ok(())
}

fn popn(vm: &mut Vm, n: usize) -> R {
    vm.need(n)?;
    vm.drop_n(n);
    Ok(())
}

fn pop1(vm: &mut Vm) -> R {
    popn(vm, 1)
}

fn pop2(vm: &mut Vm) -> R {
    popn(vm, 2)
}

fn pop3(vm: &mut Vm) -> R {
    popn(vm, 3)
}

fn pop4(vm: &mut Vm) -> R {
    popn(vm, 4)
}

fn pop6(vm: &mut Vm) -> R {
    popn(vm, 6)
}

fn pop10(vm: &mut Vm) -> R {
    popn(vm, 10)
}

fn pop12(vm: &mut Vm) -> R {
    popn(vm, 12)
}

fn push_false(vm: &mut Vm) -> R {
    vm.push_bool(false)
}

fn push_half(vm: &mut Vm) -> R {
    vm.push(Obj::Real(0.5))
}

fn empty_dict(vm: &mut Vm) -> R {
    let d = vm.new_dict(8)?;
    vm.push(Obj::Dict(d))
}

fn new_proc(vm: &mut Vm) -> R<Obj> {
    Ok(Obj::Arr(vm.new_array(Vec::new())?, true))
}

fn empty_proc(vm: &mut Vm) -> R {
    let p = new_proc(vm)?;
    vm.push(p)
}

fn four_procs(vm: &mut Vm) -> R {
    for _ in 0..4 {
        empty_proc(vm)?;
    }
    Ok(())
}

fn currentscreen(vm: &mut Vm) -> R {
    vm.push(Obj::Int(60))?;
    vm.push(Obj::Int(45))?;
    empty_proc(vm)
}

fn currentcolorscreen(vm: &mut Vm) -> R {
    for _ in 0..4 {
        currentscreen(vm)?;
    }
    Ok(())
}

fn findcolorrendering(vm: &mut Vm) -> R {
    vm.pop()?;
    let n = vm.name_obj("DefaultColorRendering");
    vm.push(n)?;
    vm.push_bool(false)
}

fn currentpagedevice(vm: &mut Vm) -> R {
    let d = vm.new_dict(8)?;
    let [x0, y0, x1, y1] = vm.out.page;
    let size = vm.new_array(vec![Obj::Real(x1 - x0), Obj::Real(y1 - y0)])?;
    let res = vm.new_array(vec![Obj::Int(72), Obj::Int(72)])?;
    for (k, v) in [
        ("PageSize", Obj::Arr(size, false)),
        ("HWResolution", Obj::Arr(res, false)),
        ("Orientation", Obj::Int(0)),
    ] {
        let k = vm.name_obj(k);
        vm.dict_put(d, k, v)?;
    }
    vm.push(Obj::Dict(d))
}

fn nulldevice(vm: &mut Vm) -> R {
    vm.g.hidden = true;
    vm.g.ctm = IDENTITY;
    Ok(())
}

/// `showpage`: the import takes the first page with artwork.
fn showpage(vm: &mut Vm) -> R {
    if vm.out.drawn.is_empty() {
        return Ok(());
    }
    Err(PsError::Quit)
}

// ----- The graphics state -----

/// Pushes a copy of the graphics state. The copy of the path counts for the
/// memory limit.
pub(super) fn gsave_by(vm: &mut Vm, by_save: bool) -> R {
    vm.charge(vm.g.path_len * EL_BYTES)?;
    vm.gstack.push(Pushed {
        g: vm.g.clone(),
        by_save,
    });
    Ok(())
}

fn gsave(vm: &mut Vm) -> R {
    if vm.gstack.len() > 10_000 {
        return Err(PsError::LimitCheck);
    }
    gsave_by(vm, false)
}

/// `grestore`: a state that `save` pushed stays on the stack.
pub(super) fn grestore(vm: &mut Vm) {
    match vm.gstack.last() {
        Some(p) if p.by_save => vm.g = p.g.clone(),
        Some(_) => {
            if let Some(p) = vm.gstack.pop() {
                vm.g = p.g;
            }
        }
        None => {}
    }
}

fn grestore_op(vm: &mut Vm) -> R {
    grestore(vm);
    Ok(())
}

fn grestoreall(vm: &mut Vm) -> R {
    while let Some(p) = vm.gstack.last() {
        if p.by_save {
            vm.g = p.g.clone();
            return Ok(());
        }
        if let Some(p) = vm.gstack.pop() {
            vm.g = p.g;
        }
    }
    Ok(())
}

fn initgraphics(vm: &mut Vm) -> R {
    let font = vm.g.font;
    vm.g = GState {
        font,
        ..GState::default()
    };
    Ok(())
}

fn gstate(vm: &mut Vm) -> R {
    let id = vm.gstates.len() as u32;
    vm.charge(512)?;
    vm.gstates.push(vm.g.clone());
    vm.push(Obj::GState(id))
}

fn currentgstate(vm: &mut Vm) -> R {
    let Obj::GState(id) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    vm.gstates[id as usize] = vm.g.clone();
    Ok(())
}

fn setgstate(vm: &mut Vm) -> R {
    let Obj::GState(id) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    vm.pop()?;
    vm.g = vm.gstates[id as usize].clone();
    Ok(())
}

fn setlinewidth(vm: &mut Vm) -> R {
    let [w] = vm.nums::<1>()?;
    vm.g.line_width = w.abs();
    Ok(())
}

fn currentlinewidth(vm: &mut Vm) -> R {
    let w = vm.g.line_width;
    vm.push_real(w)
}

fn setlinecap(vm: &mut Vm) -> R {
    vm.g.cap = vm.int_at(0)? as i32;
    vm.pop().map(|_| ())
}

fn currentlinecap(vm: &mut Vm) -> R {
    let v = vm.g.cap;
    vm.push(Obj::Int(v))
}

fn setlinejoin(vm: &mut Vm) -> R {
    vm.g.join = vm.int_at(0)? as i32;
    vm.pop().map(|_| ())
}

fn currentlinejoin(vm: &mut Vm) -> R {
    let v = vm.g.join;
    vm.push(Obj::Int(v))
}

fn setmiterlimit(vm: &mut Vm) -> R {
    let [m] = vm.nums::<1>()?;
    vm.g.miter = m;
    Ok(())
}

fn currentmiterlimit(vm: &mut Vm) -> R {
    let v = vm.g.miter;
    vm.push_real(v)
}

fn setdash(vm: &mut Vm) -> R {
    let (a, o) = (vm.top(1)?, vm.num_at(0)?);
    if !matches!(a, Obj::Arr(..)) {
        return Err(PsError::TypeCheck);
    }
    vm.drop_n(2);
    vm.g.dash = Some(a);
    vm.g.dash_offset = o;
    Ok(())
}

fn currentdash(vm: &mut Vm) -> R {
    let a = match vm.g.dash {
        Some(a) => a,
        None => Obj::Arr(vm.new_array(Vec::new())?, false),
    };
    vm.push(a)?;
    let o = vm.g.dash_offset;
    vm.push_real(o)
}

fn setflat(vm: &mut Vm) -> R {
    let [f] = vm.nums::<1>()?;
    vm.g.flat = f;
    Ok(())
}

fn currentflat(vm: &mut Vm) -> R {
    let v = vm.g.flat;
    vm.push_real(v)
}

// ----- Colors -----

fn ch(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The color of values in a base space.
pub(super) fn color_of(base: Base, v: &[f64]) -> Rgb {
    let at = |i: usize| v.get(i).copied().unwrap_or(0.0);
    match base {
        Base::Gray => {
            let g = ch(at(0));
            Rgb(g, g, g)
        }
        Base::Rgb => Rgb(ch(at(0)), ch(at(1)), ch(at(2))),
        Base::Cmyk => cmyk(at(0), at(1), at(2), at(3)),
    }
}

pub(super) fn cmyk(c: f64, m: f64, y: f64, k: f64) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    let f = |v: f64| ch((1.0 - v.clamp(0.0, 1.0)) * (1.0 - k));
    Rgb(f(c), f(m), f(y))
}

fn hsb(h: f64, s: f64, b: f64) -> [f64; 3] {
    let h = (h.clamp(0.0, 1.0) * 6.0) % 6.0;
    let (s, b) = (s.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (b * (1.0 - s), b * (1.0 - s * f), b * (1.0 - s * (1.0 - f)));
    match i as i32 {
        0 => [b, t, p],
        1 => [q, b, p],
        2 => [p, b, t],
        3 => [p, q, b],
        4 => [t, p, b],
        _ => [b, p, q],
    }
}

/// Sets the current color (the fill and the stroke) from base values.
pub(super) fn set_paint_from(vm: &mut Vm, base: Base, v: &[f64]) {
    let c = Paint::Color(color_of(base, v));
    vm.g.fill = c;
    vm.g.stroke = c;
}

fn set_device(vm: &mut Vm, base: Base, v: &[f64]) {
    vm.g.space = Space::Base(base);
    vm.g.space_obj = Obj::Null;
    vm.g.comps = v.to_vec();
    set_paint_from(vm, base, v);
}

fn setgray(vm: &mut Vm) -> R {
    let [g] = vm.nums::<1>()?;
    set_device(vm, Base::Gray, &[g]);
    Ok(())
}

fn setrgbcolor(vm: &mut Vm) -> R {
    let v = vm.nums::<3>()?;
    set_device(vm, Base::Rgb, &v);
    Ok(())
}

fn sethsbcolor(vm: &mut Vm) -> R {
    let [h, s, b] = vm.nums::<3>()?;
    set_device(vm, Base::Rgb, &hsb(h, s, b));
    Ok(())
}

fn setcmykcolor(vm: &mut Vm) -> R {
    let v = vm.nums::<4>()?;
    set_device(vm, Base::Cmyk, &v);
    Ok(())
}

fn current_rgb(vm: &Vm) -> [f64; 3] {
    match vm.g.fill {
        Paint::Color(Rgb(r, g, b)) => [r, g, b].map(|c| f64::from(c) / 255.0),
        Paint::Pattern => [0.5; 3],
    }
}

fn currentgray(vm: &mut Vm) -> R {
    let [r, g, b] = current_rgb(vm);
    vm.push_real(0.3 * r + 0.59 * g + 0.11 * b)
}

fn currentrgbcolor(vm: &mut Vm) -> R {
    for v in current_rgb(vm) {
        vm.push_real(v)?;
    }
    Ok(())
}

fn currenthsbcolor(vm: &mut Vm) -> R {
    let [r, g, b] = current_rgb(vm);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max > 0.0 { d / max } else { 0.0 };
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    for v in [h, s, max] {
        vm.push_real(v)?;
    }
    Ok(())
}

fn currentcmykcolor(vm: &mut Vm) -> R {
    let [r, g, b] = current_rgb(vm);
    let k = 1.0 - r.max(g).max(b);
    let f = |v: f64| if k < 1.0 { (1.0 - v - k) / (1.0 - k) } else { 0.0 };
    for v in [f(r), f(g), f(b), k] {
        vm.push_real(v)?;
    }
    Ok(())
}

/// The base space of a color space operand (for the alternate space of a
/// tint transform and the base of an indexed space).
fn base_of(vm: &mut Vm, o: Obj) -> Base {
    match parse_space(vm, o) {
        Ok(Space::Base(b)) => b,
        _ => Base::Rgb,
    }
}

fn parse_space(vm: &mut Vm, o: Obj) -> R<Space> {
    let (name, params): (Vec<u8>, Vec<Obj>) = match o {
        Obj::Name(n, _) => (vm.name_bytes(n).to_vec(), Vec::new()),
        Obj::Arr(a, _) => {
            let items = vm.arr(a).to_vec();
            match items.first() {
                Some(&Obj::Name(n, _)) => (vm.name_bytes(n).to_vec(), items[1..].to_vec()),
                _ => return Err(PsError::TypeCheck),
            }
        }
        _ => return Err(PsError::TypeCheck),
    };
    Ok(match name.as_slice() {
        b"DeviceGray" | b"CIEBasedA" | b"CalGray" | b"G" => Space::Base(Base::Gray),
        b"DeviceRGB" | b"CIEBasedABC" | b"CIEBasedDEF" | b"CalRGB" | b"Lab" | b"RGB" => Space::Base(Base::Rgb),
        b"DeviceCMYK" | b"CIEBasedDEFG" | b"CMYK" => Space::Base(Base::Cmyk),
        b"Pattern" => Space::Pattern,
        b"ICCBased" => {
            let n = match params.first() {
                Some(&Obj::Dict(d)) => vm.dict_get_name(d, "N").and_then(|v| int(v).ok()).unwrap_or(3),
                _ => 3,
            };
            Space::Base(Base::of_count(n as usize))
        }
        b"Indexed" | b"I" => {
            let base = params.first().map_or(Base::Rgb, |&b| base_of(vm, b));
            Space::Indexed {
                base,
                lookup: params.get(2).copied().unwrap_or(Obj::Null),
            }
        }
        b"Separation" => Space::Tinted {
            inputs: 1,
            alt: params.get(1).map_or(Base::Gray, |&b| base_of(vm, b)),
            tint: params.get(2).copied().unwrap_or(Obj::Null),
        },
        b"DeviceN" => {
            let inputs = match params.first() {
                Some(&Obj::Arr(a, _)) => a.len as usize,
                _ => 1,
            };
            Space::Tinted {
                inputs: inputs.max(1),
                alt: params.get(1).map_or(Base::Gray, |&b| base_of(vm, b)),
                tint: params.get(2).copied().unwrap_or(Obj::Null),
            }
        }
        _ => return Err(PsError::RangeCheck),
    })
}

fn space_count(s: &Space) -> usize {
    match s {
        Space::Base(b) => b.count(),
        Space::Pattern => 0,
        Space::Indexed { .. } => 1,
        Space::Tinted { inputs, .. } => *inputs,
    }
}

/// Sets the color of values `v` in the current space; a tint transform or
/// an indexed procedure runs as PostScript, then `Cont::Tint` sets it.
fn apply_color(vm: &mut Vm, v: Vec<f64>) -> R {
    vm.g.comps = v.clone();
    match vm.g.space {
        Space::Base(b) => {
            set_paint_from(vm, b, &v);
            Ok(())
        }
        Space::Pattern => {
            vm.g.fill = Paint::Pattern;
            vm.g.stroke = Paint::Pattern;
            Ok(())
        }
        Space::Indexed { base, lookup } => {
            let i = v.first().copied().unwrap_or(0.0).max(0.0) as usize;
            let n = base.count();
            match lookup {
                Obj::Str(s, _) => {
                    let bytes = vm.sbytes(s);
                    let vals: Vec<f64> = (0..n)
                        .map(|k| bytes.get(i * n + k).map_or(0.0, |&b| f64::from(b) / 255.0))
                        .collect();
                    set_paint_from(vm, base, &vals);
                    Ok(())
                }
                Obj::Arr(_, true) => {
                    vm.push_int(i as i64)?;
                    vm.push_frame(Frame::Then(Cont::Tint { n, alt: base }))?;
                    vm.execute(lookup)
                }
                _ => {
                    set_paint_from(vm, Base::Gray, &[0.5]);
                    Ok(())
                }
            }
        }
        Space::Tinted { alt, tint, .. } => match tint {
            Obj::Arr(_, true) | Obj::Op(_) | Obj::Name(_, true) => {
                for &x in &v {
                    vm.push_real(x)?;
                }
                vm.push_frame(Frame::Then(Cont::Tint { n: alt.count(), alt }))?;
                vm.execute(tint)
            }
            _ => {
                // A tint with no procedure: more ink is darker.
                let t = v.first().copied().unwrap_or(1.0);
                set_paint_from(vm, Base::Gray, &[1.0 - t]);
                Ok(())
            }
        },
    }
}

fn setcolorspace(vm: &mut Vm) -> R {
    let o = vm.top(0)?;
    let space = parse_space(vm, o)?;
    vm.pop()?;
    vm.g.space = space;
    vm.g.space_obj = o;
    // The first color of each space.
    let v = match space {
        Space::Base(Base::Cmyk) => vec![0.0, 0.0, 0.0, 1.0],
        Space::Base(b) => vec![0.0; b.count()],
        Space::Pattern => Vec::new(),
        Space::Indexed { .. } => vec![0.0],
        Space::Tinted { inputs, .. } => vec![1.0; inputs],
    };
    apply_color(vm, v)
}

fn currentcolorspace(vm: &mut Vm) -> R {
    let o = match vm.g.space_obj {
        Obj::Null => {
            let name = match vm.g.space {
                Space::Base(Base::Gray) => "DeviceGray",
                Space::Base(Base::Cmyk) => "DeviceCMYK",
                _ => "DeviceRGB",
            };
            let n = vm.name_obj(name);
            Obj::Arr(vm.new_array(vec![n])?, false)
        }
        o @ Obj::Name(..) => Obj::Arr(vm.new_array(vec![o])?, false),
        o => o,
    };
    vm.push(o)
}

fn setcolor(vm: &mut Vm) -> R {
    if vm.g.space == Space::Pattern {
        // A pattern dictionary, with the values of an uncolored pattern under it.
        let o = vm.pop()?;
        let _ = o;
        while matches!(vm.top(0), Ok(Obj::Int(_) | Obj::Real(_))) {
            vm.pop()?;
        }
        return apply_color(vm, Vec::new());
    }
    let n = space_count(&vm.g.space);
    let mut v = vec![0.0; n];
    for (k, slot) in v.iter_mut().enumerate() {
        *slot = vm.num_at(n - 1 - k)?;
    }
    vm.drop_n(n);
    apply_color(vm, v)
}

fn currentcolor(vm: &mut Vm) -> R {
    for v in vm.g.comps.clone() {
        vm.push_real(v)?;
    }
    Ok(())
}

fn setpattern(vm: &mut Vm) -> R {
    vm.pop()?;
    while matches!(vm.top(0), Ok(Obj::Int(_) | Obj::Real(_))) {
        vm.pop()?;
    }
    vm.g.space = Space::Pattern;
    apply_color(vm, Vec::new())
}

fn makepattern(vm: &mut Vm) -> R {
    // The pattern dictionary, with the transform: not drawn.
    vm.need(2)?;
    vm.pop()?;
    let d = dict_of(vm.top(0)?)?;
    vm.pop()?;
    let copy = vm.new_dict(16)?;
    let entries = vm.dicts[d as usize].entries.clone();
    for (k, v) in entries {
        vm.dict_put(copy, k, v)?;
    }
    vm.push(Obj::Dict(copy))
}

// ----- The transform -----

fn matrix_of(vm: &Vm, o: Obj) -> R<[f64; 6]> {
    let Obj::Arr(a, _) = o else {
        return Err(PsError::TypeCheck);
    };
    let items = vm.arr(a);
    if items.len() != 6 {
        return Err(PsError::RangeCheck);
    }
    let mut m = [0.0; 6];
    for (slot, &o) in m.iter_mut().zip(items) {
        *slot = num(o)?;
    }
    Ok(m)
}

fn write_matrix(vm: &mut Vm, o: Obj, m: [f64; 6]) -> R {
    let Obj::Arr(a, _) = o else {
        return Err(PsError::TypeCheck);
    };
    if a.len != 6 {
        return Err(PsError::RangeCheck);
    }
    let vals = m.map(Obj::Real);
    vm.arr_mut(a).copy_from_slice(&vals);
    Ok(())
}

fn matrix(vm: &mut Vm) -> R {
    let a = vm.new_array(IDENTITY.map(|v| Obj::Int(v as i32)).to_vec())?;
    vm.push(Obj::Arr(a, false))
}

fn initmatrix(vm: &mut Vm) -> R {
    vm.g.ctm = IDENTITY;
    Ok(())
}

fn identmatrix(vm: &mut Vm) -> R {
    let o = vm.top(0)?;
    write_matrix(vm, o, IDENTITY)
}

fn currentmatrix(vm: &mut Vm) -> R {
    let o = vm.top(0)?;
    let m = vm.g.ctm;
    write_matrix(vm, o, m)
}

fn setmatrix(vm: &mut Vm) -> R {
    let m = matrix_of(vm, vm.top(0)?)?;
    vm.pop()?;
    vm.g.ctm = m;
    Ok(())
}

/// A transform operator: with a matrix operand it fills the matrix, else
/// it changes the current transform.
fn with_matrix(vm: &mut Vm, n: usize, make: fn(&[f64]) -> [f64; 6]) -> R {
    if let Ok(o @ Obj::Arr(..)) = vm.top(0) {
        let mut v = vec![0.0; n];
        for (k, slot) in v.iter_mut().enumerate() {
            *slot = vm.num_at(n - k)?;
        }
        write_matrix(vm, o, make(&v))?;
        vm.drop_n(n + 1);
        return vm.push(o);
    }
    let mut v = vec![0.0; n];
    for (k, slot) in v.iter_mut().enumerate() {
        *slot = vm.num_at(n - 1 - k)?;
    }
    vm.drop_n(n);
    vm.g.ctm = mul(&make(&v), &vm.g.ctm);
    Ok(())
}

fn translate(vm: &mut Vm) -> R {
    with_matrix(vm, 2, |v| [1.0, 0.0, 0.0, 1.0, v[0], v[1]])
}

fn scale(vm: &mut Vm) -> R {
    with_matrix(vm, 2, |v| [v[0], 0.0, 0.0, v[1], 0.0, 0.0])
}

fn rotate(vm: &mut Vm) -> R {
    with_matrix(vm, 1, |v| {
        let (s, c) = math::sin_cos_deg(v[0]);
        [c, s, -s, c, 0.0, 0.0]
    })
}

fn concat(vm: &mut Vm) -> R {
    let m = matrix_of(vm, vm.top(0)?)?;
    vm.pop()?;
    vm.g.ctm = mul(&m, &vm.g.ctm);
    Ok(())
}

fn concatmatrix(vm: &mut Vm) -> R {
    let (a, b, out) = (matrix_of(vm, vm.top(2)?)?, matrix_of(vm, vm.top(1)?)?, vm.top(0)?);
    write_matrix(vm, out, mul(&a, &b))?;
    vm.drop_n(3);
    vm.push(out)
}

/// `transform` and its kin: two numbers and an optional matrix.
fn point_op(vm: &mut Vm, f: fn(&[f64; 6], f64, f64) -> R<[f64; 2]>) -> R {
    let (m, n) = match vm.top(0)? {
        o @ Obj::Arr(..) => (matrix_of(vm, o)?, 1),
        _ => (vm.g.ctm, 0),
    };
    let (x, y) = (vm.num_at(n + 1)?, vm.num_at(n)?);
    let p = f(&m, x, y)?;
    vm.drop_n(n + 2);
    vm.push_real(p[0])?;
    vm.push_real(p[1])
}

fn transform(vm: &mut Vm) -> R {
    point_op(vm, |m, x, y| Ok(apply(m, x, y)))
}

fn dtransform(vm: &mut Vm) -> R {
    point_op(vm, |m, x, y| Ok(apply_d(m, x, y)))
}

fn itransform(vm: &mut Vm) -> R {
    point_op(vm, |m, x, y| Ok(apply(&invert(m)?, x, y)))
}

fn idtransform(vm: &mut Vm) -> R {
    point_op(vm, |m, x, y| Ok(apply_d(&invert(m)?, x, y)))
}

fn invertmatrix(vm: &mut Vm) -> R {
    let (m, out) = (matrix_of(vm, vm.top(1)?)?, vm.top(0)?);
    write_matrix(vm, out, invert(&m)?)?;
    vm.drop_n(2);
    vm.push(out)
}

// ----- Paths -----

/// Ends the current path (with no paint).
pub(super) fn new_path(vm: &mut Vm) {
    vm.g.path.clear();
    vm.g.path_len = 0;
    vm.g.cur = None;
    vm.g.reopen = None;
    vm.g.path_is_clip = false;
    vm.g.stroked = None;
}

fn newpath(vm: &mut Vm) -> R {
    new_path(vm);
    Ok(())
}

fn path_len(vm: &Vm) -> usize {
    vm.g.path_len
}

/// Starts a subpath at device point `p`.
pub(super) fn move_device(vm: &mut Vm, p: [f64; 2]) -> R {
    if path_len(vm) > MAX_PATH {
        return Err(PsError::LimitCheck);
    }
    vm.g.path_is_clip = false;
    match vm.g.path.last_mut() {
        // Two moves: the second one is the start.
        Some(s) if s.els.len() == 1 && !s.closed => s.els[0] = El::Move(p),
        _ => {
            vm.g.path.push(Sub {
                els: vec![El::Move(p)],
                closed: false,
            });
            vm.g.path_len += 1;
        }
    }
    vm.g.cur = Some(p);
    vm.g.reopen = None;
    Ok(())
}

/// Adds a line or a curve that ends at device point `end`.
pub(super) fn add_device(vm: &mut Vm, el: El, end: [f64; 2]) -> R {
    if let Some(start) = vm.g.reopen.take() {
        move_device(vm, start)?;
    }
    if vm.g.cur.is_none() {
        return Err(PsError::RangeCheck);
    }
    if path_len(vm) > MAX_PATH {
        return Err(PsError::LimitCheck);
    }
    match vm.g.path.last_mut() {
        Some(s) => s.els.push(el),
        None => return Err(PsError::RangeCheck),
    }
    vm.g.path_len += 1;
    vm.g.cur = Some(end);
    Ok(())
}

fn user(vm: &Vm, x: f64, y: f64) -> [f64; 2] {
    apply(&vm.g.ctm, x, y)
}

fn moveto(vm: &mut Vm) -> R {
    let [x, y] = vm.nums::<2>()?;
    let p = user(vm, x, y);
    move_device(vm, p)
}

fn rmoveto(vm: &mut Vm) -> R {
    let cur = vm.g.cur.ok_or(PsError::RangeCheck)?;
    let [dx, dy] = vm.nums::<2>()?;
    let d = apply_d(&vm.g.ctm, dx, dy);
    move_device(vm, [cur[0] + d[0], cur[1] + d[1]])
}

fn lineto(vm: &mut Vm) -> R {
    let [x, y] = vm.nums::<2>()?;
    let p = user(vm, x, y);
    add_device(vm, El::Line(p), p)
}

fn rlineto(vm: &mut Vm) -> R {
    let cur = vm.g.cur.ok_or(PsError::RangeCheck)?;
    let [dx, dy] = vm.nums::<2>()?;
    let d = apply_d(&vm.g.ctm, dx, dy);
    let p = [cur[0] + d[0], cur[1] + d[1]];
    add_device(vm, El::Line(p), p)
}

fn curveto(vm: &mut Vm) -> R {
    let v = vm.nums::<6>()?;
    let (a, b, c) = (user(vm, v[0], v[1]), user(vm, v[2], v[3]), user(vm, v[4], v[5]));
    add_device(vm, El::Curve(a, b, c), c)
}

fn rcurveto(vm: &mut Vm) -> R {
    let cur = vm.g.cur.ok_or(PsError::RangeCheck)?;
    let v = vm.nums::<6>()?;
    let rel = |x: f64, y: f64| {
        let d = apply_d(&vm.g.ctm, x, y);
        [cur[0] + d[0], cur[1] + d[1]]
    };
    let (a, b, c) = (rel(v[0], v[1]), rel(v[2], v[3]), rel(v[4], v[5]));
    add_device(vm, El::Curve(a, b, c), c)
}

/// Adds an arc of a circle (user space) as curves of 90° or less: a line
/// to its start if there is a current point.
fn add_arc(vm: &mut Vm, x: f64, y: f64, r: f64, a1: f64, mut a2: f64, clockwise: bool) -> R {
    if clockwise {
        while a2 > a1 {
            a2 -= 360.0;
        }
    } else {
        while a2 < a1 {
            a2 += 360.0;
        }
    }
    let at = |deg: f64| {
        let (s, c) = math::sin_cos_deg(deg);
        [x + r * c, y + r * s]
    };
    let start = at(a1);
    let p = user(vm, start[0], start[1]);
    if vm.g.cur.is_some() || vm.g.reopen.is_some() {
        add_device(vm, El::Line(p), p)?;
    } else {
        move_device(vm, p)?;
    }
    let sweep = a2 - a1;
    let n = (sweep.abs() / 90.0).ceil().clamp(1.0, 16.0) as usize;
    let step = sweep / n as f64;
    // The length of the handles of a curve for an angle.
    let k = 4.0 / 3.0 * math::tan(math::to_radians(step) / 4.0);
    for i in 0..n {
        let (t0, t1) = (a1 + step * i as f64, a1 + step * (i + 1) as f64);
        let (s0, c0) = math::sin_cos_deg(t0);
        let (s1, c1) = math::sin_cos_deg(t1);
        let p0 = [x + r * c0, y + r * s0];
        let p1 = [x + r * c1, y + r * s1];
        let h0 = [p0[0] - k * r * s0, p0[1] + k * r * c0];
        let h1 = [p1[0] + k * r * s1, p1[1] - k * r * c1];
        let (a, b, c) = (user(vm, h0[0], h0[1]), user(vm, h1[0], h1[1]), user(vm, p1[0], p1[1]));
        add_device(vm, El::Curve(a, b, c), c)?;
    }
    Ok(())
}

fn arc(vm: &mut Vm) -> R {
    let [x, y, r, a1, a2] = vm.nums::<5>()?;
    add_arc(vm, x, y, r, a1, a2, false)
}

fn arcn(vm: &mut Vm) -> R {
    let [x, y, r, a1, a2] = vm.nums::<5>()?;
    add_arc(vm, x, y, r, a1, a2, true)
}

/// `arct`: an arc tangent to the line from the current point to (x1, y1)
/// and to the line from (x1, y1) to (x2, y2). Gives the two tangent points.
fn add_arct(vm: &mut Vm, v: [f64; 5]) -> R<[f64; 4]> {
    let [x1, y1, x2, y2, r] = v;
    let cur = vm.g.cur.ok_or(PsError::RangeCheck)?;
    let c = apply(&invert(&vm.g.ctm)?, cur[0], cur[1]);
    let (ux, uy) = (c[0] - x1, c[1] - y1);
    let (wx, wy) = (x2 - x1, y2 - y1);
    let (lu, lw) = (math::hypot(ux, uy), math::hypot(wx, wy));
    if lu == 0.0 || lw == 0.0 || r == 0.0 {
        let p = user(vm, x1, y1);
        add_device(vm, El::Line(p), p)?;
        return Ok([x1, y1, x1, y1]);
    }
    let (ux, uy, wx, wy) = (ux / lu, uy / lu, wx / lw, wy / lw);
    let cos = (ux * wx + uy * wy).clamp(-1.0, 1.0);
    let half = math::atan2((1.0 - cos * cos).max(0.0).sqrt(), cos) / 2.0;
    if half.abs() < 1e-9 {
        let p = user(vm, x1, y1);
        add_device(vm, El::Line(p), p)?;
        return Ok([x1, y1, x1, y1]);
    }
    let d = r / math::tan(half);
    let t1 = [x1 + ux * d, y1 + uy * d];
    let t2 = [x1 + wx * d, y1 + wy * d];
    // The center is on the line that halves the angle.
    let (bx, by) = (ux + wx, uy + wy);
    let lb = math::hypot(bx, by);
    let dist = r / math::sin(half);
    let (cx, cy) = (x1 + bx / lb * dist, y1 + by / lb * dist);
    let a1 = math::to_degrees(math::atan2(t1[1] - cy, t1[0] - cx));
    let a2 = math::to_degrees(math::atan2(t2[1] - cy, t2[0] - cx));
    // The turn from the first line to the second sets the direction.
    let clockwise = ux * wy - uy * wx > 0.0;
    let mut a2 = a2;
    if clockwise {
        while a2 > a1 {
            a2 -= 360.0;
        }
        if a1 - a2 > 180.0 {
            a2 += 360.0;
        }
    } else {
        while a2 < a1 {
            a2 += 360.0;
        }
        if a2 - a1 > 180.0 {
            a2 -= 360.0;
        }
    }
    add_arc(vm, cx, cy, r, a1, a2, clockwise)?;
    Ok([t1[0], t1[1], t2[0], t2[1]])
}

fn arct(vm: &mut Vm) -> R {
    let v = vm.nums::<5>()?;
    add_arct(vm, v).map(|_| ())
}

fn arcto(vm: &mut Vm) -> R {
    let v = vm.nums::<5>()?;
    for t in add_arct(vm, v)? {
        vm.push_real(t)?;
    }
    Ok(())
}

pub(super) fn close_path(vm: &mut Vm) {
    if let Some(s) = vm.g.path.last_mut()
        && !s.closed
        && let Some(El::Move(start)) = s.els.first().copied()
    {
        s.closed = true;
        vm.g.cur = Some(start);
        vm.g.reopen = Some(start);
    }
}

fn closepath(vm: &mut Vm) -> R {
    close_path(vm);
    Ok(())
}

fn strokepath(vm: &mut Vm) -> R {
    let w = stroke_width(vm);
    vm.g.stroked = Some(w);
    Ok(())
}

fn clippath(vm: &mut Vm) -> R {
    let [x0, y0, x1, y1] = vm.out.page;
    new_path(vm);
    move_device(vm, [x0, y0])?;
    for p in [[x1, y0], [x1, y1], [x0, y1]] {
        add_device(vm, El::Line(p), p)?;
    }
    close_path(vm);
    vm.g.path_is_clip = true;
    Ok(())
}

fn pathbbox(vm: &mut Vm) -> R {
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for s in &vm.g.path {
        for el in &s.els {
            let pts: &[[f64; 2]] = match el {
                El::Move(p) | El::Line(p) => std::slice::from_ref(p),
                El::Curve(a, c, d) => &[*a, *c, *d],
            };
            for p in pts {
                b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
            }
        }
    }
    if b[0] > b[2] {
        return Err(PsError::RangeCheck);
    }
    let inv = invert(&vm.g.ctm)?;
    let corners = [[b[0], b[1]], [b[2], b[1]], [b[2], b[3]], [b[0], b[3]]].map(|p| apply(&inv, p[0], p[1]));
    let mut u = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in corners {
        u = [u[0].min(p[0]), u[1].min(p[1]), u[2].max(p[0]), u[3].max(p[1])];
    }
    for v in u {
        vm.push_real(v)?;
    }
    Ok(())
}

fn currentpoint(vm: &mut Vm) -> R {
    let cur = vm.g.cur.ok_or(PsError::RangeCheck)?;
    let p = apply(&invert(&vm.g.ctm)?, cur[0], cur[1]);
    vm.push_real(p[0])?;
    vm.push_real(p[1])
}

/// The numbers of the rectangle operators: four numbers, or an array of
/// groups of four.
fn rects(vm: &mut Vm) -> R<Vec<[f64; 4]>> {
    if let Obj::Arr(a, _) = vm.top(0)? {
        let items = vm.arr(a).to_vec();
        vm.pop()?;
        let mut out = Vec::new();
        for c in items.as_chunks::<4>().0 {
            out.push([num(c[0])?, num(c[1])?, num(c[2])?, num(c[3])?]);
        }
        return Ok(out);
    }
    Ok(vec![vm.nums::<4>()?])
}

fn rect_path(vm: &mut Vm, r: [f64; 4]) -> R {
    let [x, y, w, h] = r;
    let p = user(vm, x, y);
    move_device(vm, p)?;
    for (a, b) in [(x + w, y), (x + w, y + h), (x, y + h)] {
        let q = user(vm, a, b);
        add_device(vm, El::Line(q), q)?;
    }
    close_path(vm);
    Ok(())
}

fn rectclip(vm: &mut Vm) -> R {
    rects(vm)?;
    new_path(vm);
    Ok(())
}

fn rectfill(vm: &mut Vm) -> R {
    let rs = rects(vm)?;
    gsave(vm)?;
    new_path(vm);
    for r in rs {
        rect_path(vm, r)?;
    }
    let painted = paint(vm, true, false);
    grestore(vm);
    painted
}

fn rectstroke(vm: &mut Vm) -> R {
    // A matrix after the rectangle changes only the line width: not used.
    if matches!(vm.top(0)?, Obj::Arr(a, _) if vm.arr(a).len() == 6) && vm.ops.len() >= 5 {
        vm.pop()?;
    }
    let rs = rects(vm)?;
    gsave(vm)?;
    new_path(vm);
    for r in rs {
        rect_path(vm, r)?;
    }
    let painted = paint(vm, false, true);
    grestore(vm);
    painted
}

// ----- Painting -----

fn stroke_width(vm: &Vm) -> f64 {
    let [a, b, c, d, ..] = vm.g.ctm;
    vm.g.line_width * (a * d - b * c).abs().sqrt()
}

/// Paints the current path and ends it. The painted path counts for the
/// memory limit; the path after `MAX_SHAPES` stops the run.
pub(super) fn paint(vm: &mut Vm, fill: bool, stroke: bool) -> R {
    let subs: Vec<Sub> = std::mem::take(&mut vm.g.path)
        .into_iter()
        .filter(|s| s.els.len() > 1)
        .collect();
    let clip = vm.g.path_is_clip;
    let stroked = vm.g.stroked;
    new_path(vm);
    if subs.is_empty() || clip || vm.g.hidden || vm.ai.skips() {
        return Ok(());
    }
    if vm.out.drawn.len() >= MAX_SHAPES {
        return Err(PsError::TooManyShapes);
    }
    vm.charge(subs.iter().map(|s| s.els.len()).sum::<usize>() * EL_BYTES + 64)?;
    let (fill, stroke) = match stroked {
        // The outline of a stroke, filled: the stroke.
        Some(w) if fill => (false, Some(w)),
        _ => (fill, stroke.then(|| stroke_width(vm))),
    };
    let count = |p: Paint, n: &mut usize| {
        if p == Paint::Pattern {
            *n += 1;
        }
    };
    let f = fill.then_some(vm.g.fill);
    let s = stroke.map(|w| (if stroked.is_some() { vm.g.fill } else { vm.g.stroke }, w));
    if let Some(p) = f {
        count(p, &mut vm.out.patterns);
    }
    if let Some((p, _)) = s {
        count(p, &mut vm.out.patterns);
    }
    vm.out.add(subs, f, s);
    Ok(())
}

fn fill(vm: &mut Vm) -> R {
    paint(vm, true, false)
}

fn stroke(vm: &mut Vm) -> R {
    paint(vm, false, true)
}

fn shfill(vm: &mut Vm) -> R {
    vm.pop()?;
    vm.out.shadings += 1;
    Ok(())
}

/// Takes the data of a picture: `bytes` from its sources.
fn take_picture_data(vm: &mut Vm, sources: Vec<Obj>, bytes: i64) -> R {
    vm.out.images += 1;
    match sources.first() {
        Some(Obj::Arr(_, true) | Obj::Name(_, true) | Obj::Op(_)) => vm.push_frame(Frame::Then(Cont::Picture {
            procs: sources,
            next: 0,
            left: bytes,
            first: true,
        })),
        Some(&Obj::File(0, _)) => {
            let n = usize::try_from(bytes).unwrap_or(0);
            super::ops::with_file(vm, 0, |d| ((), n.min(d.len())))
        }
        _ => {
            for s in sources {
                if let Obj::File(f, _) = s
                    && f > 1
                {
                    super::ops::with_file(vm, f, |d| ((), d.len()))?;
                }
            }
            Ok(())
        }
    }
}

/// The size of the data of a picture.
fn data_size(w: i64, h: i64, bits: i64, comps: i64) -> i64 {
    ((w.max(0) * bits.max(1) * comps.max(1) + 7) / 8) * h.max(0)
}

/// A picture with a dictionary (`image` and `imagemask`, LanguageLevel 2).
fn dict_picture(vm: &mut Vm, d: u32, comps: i64) -> R {
    vm.pop()?;
    picture_of_dict(vm, d, comps, true)
}

fn picture_of_dict(vm: &mut Vm, d: u32, comps: i64, follow: bool) -> R {
    let mut get = |k: &str| vm.dict_get_name(d, k);
    let w = get("Width").and_then(|v| int(v).ok()).unwrap_or(0);
    let h = get("Height").and_then(|v| int(v).ok()).unwrap_or(0);
    let bits = get("BitsPerComponent").and_then(|v| int(v).ok()).unwrap_or(1);
    // A masked picture keeps its picture in the data dictionary.
    if follow && let Some(Obj::Dict(data)) = vm.dict_get_name(d, "DataDict") {
        return picture_of_dict(vm, data, comps, false);
    }
    let src = vm.dict_get_name(d, "DataSource").unwrap_or(Obj::Null);
    let multi = vm.dict_get_name(d, "MultipleDataSources") == Some(Obj::Bool(true));
    let sources = match (multi, src) {
        (true, Obj::Arr(a, false)) => vm.arr(a).to_vec(),
        _ => vec![src],
    };
    take_picture_data(vm, sources, data_size(w, h, bits, comps))
}

fn picture_comps(vm: &Vm) -> i64 {
    space_count(&vm.g.space).max(1) as i64
}

fn image(vm: &mut Vm) -> R {
    if let Obj::Dict(d) = vm.top(0)? {
        let comps = picture_comps(vm);
        return dict_picture(vm, d, comps);
    }
    vm.need(5)?;
    let (w, h, bits, src) = (vm.int_at(4)?, vm.int_at(3)?, vm.int_at(2)?, vm.top(0)?);
    vm.drop_n(5);
    take_picture_data(vm, vec![src], data_size(w, h, bits, 1))
}

fn imagemask(vm: &mut Vm) -> R {
    if let Obj::Dict(d) = vm.top(0)? {
        return dict_picture(vm, d, 1);
    }
    vm.need(5)?;
    let (w, h, src) = (vm.int_at(4)?, vm.int_at(3)?, vm.top(0)?);
    vm.drop_n(5);
    take_picture_data(vm, vec![src], data_size(w, h, 1, 1))
}

fn colorimage(vm: &mut Vm) -> R {
    let comps = vm.int_at(0)?;
    let multi = matches!(vm.top(1)?, Obj::Bool(true));
    let n_src = if multi { comps.max(1) as usize } else { 1 };
    vm.need(6 + n_src)?;
    let base = 2 + n_src;
    let (w, h, bits) = (vm.int_at(base + 3)?, vm.int_at(base + 2)?, vm.int_at(base + 1)?);
    let sources: Vec<Obj> = (0..n_src).map(|k| vm.top(2 + n_src - 1 - k)).collect::<R<_>>()?;
    vm.drop_n(base + 4);
    take_picture_data(vm, sources, data_size(w, h, bits, comps))
}

fn execform(vm: &mut Vm) -> R {
    let d = dict_of(vm.top(0)?)?;
    let proc = vm.dict_get_name(d, "PaintProc").ok_or(PsError::UndefinedResult)?;
    let m = match vm.dict_get_name(d, "Matrix") {
        Some(o) => matrix_of(vm, o).unwrap_or(IDENTITY),
        None => IDENTITY,
    };
    gsave(vm)?;
    vm.g.ctm = mul(&m, &vm.g.ctm);
    new_path(vm);
    // The dictionary stays on the stack for the paint procedure.
    vm.push_frame(Frame::Then(Cont::GRestore))?;
    vm.execute(proc)
}

// ----- Fonts and text -----

/// Defines font dictionary `font` with the key `key`.
pub(super) fn define_font(vm: &mut Vm, key: Obj, font: u32) -> R {
    let fid = vm.name_obj("FID");
    vm.dict_put(font, fid, Obj::Font(font))?;
    let fd = vm.fontdir;
    vm.dict_put(fd, key, Obj::Dict(font))
}

fn definefont(vm: &mut Vm) -> R {
    let (key, font) = (vm.top(1)?, dict_of(vm.top(0)?)?);
    vm.drop_n(2);
    define_font(vm, key, font)?;
    vm.push(Obj::Dict(font))
}

/// `findfont`: a font that the file did not define is a plain font (its
/// text is not drawn anyway).
pub(super) fn findfont(vm: &mut Vm) -> R {
    let key = vm.top(0)?;
    let fd = vm.fontdir;
    if let Some(f) = vm.dict_get(fd, key)? {
        vm.pop()?;
        return vm.push(f);
    }
    let d = vm.new_dict(16)?;
    let fm = vm.new_array(vec![
        Obj::Real(0.001),
        Obj::Int(0),
        Obj::Int(0),
        Obj::Real(0.001),
        Obj::Int(0),
        Obj::Int(0),
    ])?;
    let bbox = vm.new_array(vec![Obj::Int(0), Obj::Int(0), Obj::Int(1000), Obj::Int(1000)])?;
    let enc = vm.dict_get_name(vm.systemdict, "StandardEncoding").unwrap_or(Obj::Null);
    for (k, v) in [
        ("FontName", key),
        ("FontType", Obj::Int(1)),
        ("FontMatrix", Obj::Arr(fm, false)),
        ("FontBBox", Obj::Arr(bbox, false)),
        ("Encoding", enc),
    ] {
        let k = vm.name_obj(k);
        vm.dict_put(d, k, v)?;
    }
    define_font(vm, key, d)?;
    vm.pop()?;
    vm.push(Obj::Dict(d))
}

/// A copy of a font with its matrix times `m`.
fn font_with(vm: &mut Vm, font: u32, m: [f64; 6]) -> R<u32> {
    let copy = vm.new_dict(32)?;
    let entries = vm.dicts[font as usize].entries.clone();
    for (k, v) in entries {
        vm.dict_put(copy, k, v)?;
    }
    let old = match vm.dict_get_name(font, "FontMatrix") {
        Some(o) => matrix_of(vm, o).unwrap_or([0.001, 0.0, 0.0, 0.001, 0.0, 0.0]),
        None => [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
    };
    let fm = vm.new_array(mul(&old, &m).map(Obj::Real).to_vec())?;
    let k = vm.name_obj("FontMatrix");
    vm.dict_put(copy, k, Obj::Arr(fm, false))?;
    let fid = vm.name_obj("FID");
    vm.dict_put(copy, fid, Obj::Font(copy))?;
    Ok(copy)
}

fn scalefont(vm: &mut Vm) -> R {
    let (font, s) = (dict_of(vm.top(1)?)?, vm.num_at(0)?);
    let f = font_with(vm, font, [s, 0.0, 0.0, s, 0.0, 0.0])?;
    vm.drop_n(2);
    vm.push(Obj::Dict(f))
}

fn makefont(vm: &mut Vm) -> R {
    let (font, m) = (dict_of(vm.top(1)?)?, matrix_of(vm, vm.top(0)?)?);
    let f = font_with(vm, font, m)?;
    vm.drop_n(2);
    vm.push(Obj::Dict(f))
}

fn setfont(vm: &mut Vm) -> R {
    let f = dict_of(vm.top(0)?)?;
    vm.pop()?;
    vm.g.font = Some(f);
    Ok(())
}

fn currentfont(vm: &mut Vm) -> R {
    match vm.g.font {
        Some(f) => vm.push(Obj::Dict(f)),
        None => {
            let n = vm.name_obj("Courier");
            vm.push(n)?;
            findfont(vm)
        }
    }
}

fn selectfont(vm: &mut Vm) -> R {
    let (key, size) = (vm.top(1)?, vm.top(0)?);
    vm.drop_n(2);
    vm.push(key)?;
    findfont(vm)?;
    vm.push(size)?;
    if matches!(size, Obj::Arr(..)) {
        makefont(vm)?
    } else {
        scalefont(vm)?
    }
    setfont(vm)
}

fn composefont(vm: &mut Vm) -> R {
    vm.need(3)?;
    vm.drop_n(3);
    let d = vm.new_dict(8)?;
    vm.push(Obj::Dict(d))
}

/// A text operator with `n` operands: counted, not drawn.
fn show(vm: &mut Vm, n: usize) -> R {
    vm.need(n)?;
    vm.drop_n(n);
    vm.out.text += 1;
    Ok(())
}

fn show1(vm: &mut Vm) -> R {
    show(vm, 1)
}

fn show2(vm: &mut Vm) -> R {
    show(vm, 2)
}

fn show3(vm: &mut Vm) -> R {
    show(vm, 3)
}

fn show4(vm: &mut Vm) -> R {
    show(vm, 4)
}

fn show6(vm: &mut Vm) -> R {
    show(vm, 6)
}

fn stringwidth(vm: &mut Vm) -> R {
    let Obj::Str(s, _) = vm.top(0)? else {
        return Err(PsError::TypeCheck);
    };
    vm.pop()?;
    // Half an em for each character.
    let em = match vm.g.font {
        Some(f) => match vm.dict_get_name(f, "FontMatrix") {
            Some(o) => matrix_of(vm, o).map_or(0.012, |m| m[0]),
            None => 0.012,
        },
        None => 0.012,
    };
    vm.push_real(f64::from(s.len) * 500.0 * em)?;
    vm.push(Obj::Int(0))
}

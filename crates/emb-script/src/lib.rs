//! Scripts (P13 Milestone 4, ADR 0067): tools that a user writes in Rhai.
//!
//! - [`run`]: runs a script on a copy of a design and returns the changed
//!   design, the selection and the messages, or an error. The caller puts
//!   the design in the document with one edit (one undo step).
//! - [`ScriptInfo::parse`]: the title, description and inputs of a script
//!   from its first `//!` lines.
//! - [`SAMPLES`]: the sample scripts of the program (`data/scripts`).
//!
//! A script sees only the functions of this crate (the list is in
//! `docs/help/scripts.md`): it has no file, network or process functions.
//! [`Limits`] stop a script that runs too long or makes too much.
//!
//! This crate has no UI dependency. The app and `emb-cli` use it.
#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use emb_geom::Transform;
use emb_model::arrange::{OffsetMake, OffsetSpec};
use emb_model::{BorderPlace, Design, MachineProfile, ObjectId, ObjectKind, Rgb, ShapeOp, ShapeOptions, ThreadId};
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope};

/// The file extension of scripts.
pub const FILE_EXTENSION: &str = "rhai";

/// The sample scripts: file name and source.
pub const SAMPLES: &[(&str, &str)] = &[
    (
        "outline-fills.rhai",
        include_str!("../../../data/scripts/outline-fills.rhai"),
    ),
    (
        "satin-density.rhai",
        include_str!("../../../data/scripts/satin-density.rhai"),
    ),
    (
        "name-threads-by-code.rhai",
        include_str!("../../../data/scripts/name-threads-by-code.rhai"),
    ),
    (
        "check-before-output.rhai",
        include_str!("../../../data/scripts/check-before-output.rhai"),
    ),
];

/// The limits of a script.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub time: Duration,
    pub operations: u64,
    /// The most objects of the design after the script.
    pub max_objects: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            time: Duration::from_secs(5),
            operations: 50_000_000,
            max_objects: 20_000,
        }
    }
}

/// The value of an input of a script.
#[derive(Debug, Clone, PartialEq)]
pub enum InputValue {
    Number(f64),
    Bool(bool),
    Text(String),
}

impl InputValue {
    fn to_dynamic(&self) -> Dynamic {
        match self {
            InputValue::Number(v) => Dynamic::from_float(*v),
            InputValue::Bool(v) => Dynamic::from_bool(*v),
            InputValue::Text(v) => Dynamic::from(v.clone()),
        }
    }
}

/// An input of a script: `//! input width: 2.0 "Border width (mm)"`.
#[derive(Debug, Clone, PartialEq)]
pub struct InputSpec {
    pub name: String,
    pub label: String,
    pub default: InputValue,
}

/// The title, description and inputs of a script.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScriptInfo {
    pub title: String,
    pub description: String,
    pub inputs: Vec<InputSpec>,
}

impl ScriptInfo {
    /// Reads the first lines of `src` that start with `//!`: the first is
    /// the title, `input` lines are inputs, the others are the description.
    /// A script with no title gets `fallback`.
    pub fn parse(src: &str, fallback: &str) -> ScriptInfo {
        let mut info = ScriptInfo::default();
        let mut description = Vec::new();
        for line in src.lines().map(str::trim) {
            let Some(text) = line.strip_prefix("//!") else {
                if line.is_empty() {
                    continue;
                }
                break;
            };
            let text = text.trim();
            if let Some(spec) = text.strip_prefix("input ") {
                if let Some(i) = parse_input(spec) {
                    info.inputs.push(i);
                }
            } else if info.title.is_empty() {
                info.title = text.to_string();
            } else {
                description.push(text.to_string());
            }
        }
        if info.title.is_empty() {
            info.title = fallback.to_string();
        }
        info.description = description.join(" ");
        info
    }

    /// The default value of each input.
    pub fn defaults(&self) -> BTreeMap<String, InputValue> {
        self.inputs
            .iter()
            .map(|i| (i.name.clone(), i.default.clone()))
            .collect()
    }
}

/// `name: default "label"`.
fn parse_input(spec: &str) -> Option<InputSpec> {
    let (name, rest) = spec.split_once(':')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let rest = rest.trim();
    // The default: a text in quotes, or a word.
    let (default, rest) = if let Some(r) = rest.strip_prefix('"') {
        let end = r.find('"')?;
        (InputValue::Text(r[..end].to_string()), r[end + 1..].trim())
    } else {
        let (word, r) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let v = match word {
            "true" => InputValue::Bool(true),
            "false" => InputValue::Bool(false),
            w => InputValue::Number(w.parse().ok().filter(|v: &f64| v.is_finite())?),
        };
        (v, r.trim())
    };
    let label = rest.trim_matches('"').trim();
    Some(InputSpec {
        name: name.to_string(),
        label: if label.is_empty() {
            name.to_string()
        } else {
            label.to_string()
        },
        default,
    })
}

/// The result of a script.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub design: Design,
    /// The selection after the script (objects that are in the design).
    pub selection: Vec<ObjectId>,
    pub messages: Vec<String>,
    /// True if the script changed the design.
    pub changed: bool,
}

/// A script that did not run to its end.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptError {
    pub message: String,
    /// The line of the script, if known (1 is the first line).
    pub line: Option<usize>,
    /// The messages before the error.
    pub messages: Vec<String>,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {l}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

/// The state that the functions of a script share.
struct Ctx {
    design: Design,
    selection: Vec<ObjectId>,
    messages: Vec<String>,
    changed: bool,
    machine: Option<MachineProfile>,
}

type Shared = Rc<RefCell<Ctx>>;
type Res<T> = Result<T, Box<EvalAltResult>>;

fn oid(id: i64) -> Res<ObjectId> {
    u64::try_from(id)
        .map(ObjectId)
        .map_err(|_| format!("{id} is not an object").into())
}

fn ids_of(a: &Array) -> Res<Vec<ObjectId>> {
    a.iter()
        .map(|v| {
            v.as_int()
                .map_err(|_| -> Box<EvalAltResult> { "a list of objects has only object numbers".into() })
                .and_then(oid)
        })
        .collect()
}

fn id_array(ids: impl IntoIterator<Item = ObjectId>) -> Array {
    ids.into_iter().map(|id| Dynamic::from_int(id.0 as i64)).collect()
}

fn tid(id: i64) -> Res<ThreadId> {
    u32::try_from(id)
        .map(ThreadId)
        .map_err(|_| format!("{id} is not a thread").into())
}

fn color(hex: &str) -> Res<Rgb> {
    Rgb::from_hex(hex).ok_or_else(|| format!("{hex:?} is not a color like \"#ff0000\"").into())
}

/// The name of an object kind, as in the project file.
pub fn kind_name(k: &ObjectKind) -> &'static str {
    match k {
        ObjectKind::Running { .. } => "running",
        ObjectKind::Satin { .. } => "satin",
        ObjectKind::Fill { .. } => "fill",
        ObjectKind::Stitches { .. } => "stitches",
        ObjectKind::Applique { .. } => "applique",
        ObjectKind::Lettering(_) => "lettering",
        ObjectKind::Vector { .. } => "vector",
        ObjectKind::CrossStitch(_) => "cross_stitch",
        ObjectKind::Bling(_) => "bling",
        ObjectKind::Sketch(_) => "sketch",
    }
}

/// The settings of an object kind as a map (serde names).
fn settings_of(k: &ObjectKind) -> Res<Dynamic> {
    use rhai::serde::to_dynamic;
    match k {
        ObjectKind::Running { params, .. } => to_dynamic(params),
        ObjectKind::Satin { params, .. } => to_dynamic(params),
        ObjectKind::Fill { params, .. } => to_dynamic(params),
        ObjectKind::Applique { params, .. } => to_dynamic(params),
        ObjectKind::Lettering(l) => to_dynamic(l.as_ref()),
        ObjectKind::Vector { style, .. } => to_dynamic(style),
        ObjectKind::CrossStitch(x) => to_dynamic(x.as_ref()),
        ObjectKind::Bling(b) => to_dynamic(b.as_ref()),
        ObjectKind::Sketch(s) => to_dynamic(s.as_ref()),
        ObjectKind::Stitches { .. } => Ok(Dynamic::from_map(Map::new())),
    }
}

/// A whole number where the settings have a decimal number becomes a
/// decimal number, so `p.spacing_mm = 1` works.
fn coerce(new: Dynamic, like: &Dynamic) -> Dynamic {
    if like.is_float() && new.is_int() {
        return Dynamic::from_float(new.as_int().unwrap_or(0) as f64);
    }
    if new.is_map() && like.is_map() {
        let like = like.clone().cast::<Map>();
        let mut m = new.cast::<Map>();
        for (k, v) in m.iter_mut() {
            if let Some(l) = like.get(k) {
                *v = coerce(std::mem::take(v), l);
            }
        }
        return Dynamic::from_map(m);
    }
    if new.is_array() && like.is_array() {
        let like = like.clone().cast::<Array>();
        let a: Array = new
            .cast::<Array>()
            .into_iter()
            .enumerate()
            .map(|(i, v)| match like.get(i).or(like.first()) {
                Some(l) => coerce(v, l),
                None => v,
            })
            .collect();
        return Dynamic::from_array(a);
    }
    new
}

/// Sets the settings of an object kind from a map.
fn set_settings_of(k: &mut ObjectKind, new: Dynamic) -> Res<()> {
    use rhai::serde::from_dynamic;
    let new = coerce(new, &settings_of(k)?);
    let kind = kind_name(k);
    let bad =
        |e: Box<EvalAltResult>| -> Box<EvalAltResult> { format!("these settings do not fit a {kind}: {e}").into() };
    match k {
        ObjectKind::Running { params, .. } => *params = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Satin { params, .. } => *params = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Fill { params, .. } => *params = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Applique { params, .. } => *params = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Lettering(l) => **l = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Vector { style, .. } => *style = from_dynamic(&new).map_err(bad)?,
        ObjectKind::CrossStitch(x) => **x = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Bling(b) => **b = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Sketch(s) => **s = from_dynamic(&new).map_err(bad)?,
        ObjectKind::Stitches { .. } => return Err("stitch objects have no settings".into()),
    }
    Ok(())
}

/// The transform about the center of the objects `ids`.
fn about_center(d: &Design, ids: &[ObjectId], t: Transform) -> Option<Transform> {
    let c = d.bbox_of(ids)?.center();
    Some(
        Transform::translate(-c.x, -c.y)
            .then(t)
            .then(Transform::translate(c.x, c.y)),
    )
}

/// Registers a function twice: for a decimal number and for a whole
/// number as its last value.
macro_rules! with_number {
    ($engine:expr, $name:expr, $ctx:expr, |$c:ident, $a:ident : $at:ty, $v:ident| $body:expr) => {{
        let c1 = $ctx.clone();
        $engine.register_fn($name, move |$a: $at, $v: f64| -> Res<_> {
            let $c = &c1;
            $body
        });
        let c2 = $ctx.clone();
        $engine.register_fn($name, move |$a: $at, $v: i64| -> Res<_> {
            let $c = &c2;
            let $v = $v as f64;
            $body
        });
    }};
}

fn register(engine: &mut Engine, ctx: &Shared) {
    // The design.
    let c = ctx.clone();
    engine.register_fn("objects", move || {
        id_array(c.borrow().design.objects().iter().map(|o| o.id))
    });
    let c = ctx.clone();
    engine.register_fn("selection", move || id_array(c.borrow().selection.clone()));
    let c = ctx.clone();
    engine.register_fn("select", move |a: Array| -> Res<()> {
        let ids = ids_of(&a)?;
        c.borrow_mut().selection = ids;
        Ok(())
    });
    let c = ctx.clone();
    engine.register_fn("design_name", move || c.borrow().design.name.clone());
    let c = ctx.clone();
    engine.register_fn("set_design_name", move |s: &str| {
        let mut x = c.borrow_mut();
        x.design.name = s.to_string();
        x.changed = true;
    });
    let c = ctx.clone();
    engine.register_fn("notes", move || c.borrow().design.notes.clone());
    let c = ctx.clone();
    engine.register_fn("set_notes", move |s: &str| {
        let mut x = c.borrow_mut();
        x.design.notes = s.to_string();
        x.changed = true;
    });

    // One object.
    let object = |c: &Shared, id: i64| -> Res<emb_model::DesignObject> {
        let id = oid(id)?;
        c.borrow()
            .design
            .object(id)
            .cloned()
            .ok_or_else(|| format!("there is no object {}", id.0).into())
    };
    let c = ctx.clone();
    engine.register_fn("kind", move |id: i64| -> Res<String> {
        Ok(kind_name(&object(&c, id)?.kind).to_string())
    });
    let c = ctx.clone();
    engine.register_fn("name", move |id: i64| -> Res<String> {
        let o = object(&c, id)?;
        Ok(o.name.clone().unwrap_or_else(|| o.type_name().to_string()))
    });
    let c = ctx.clone();
    engine.register_fn("set_name", move |id: i64, s: &str| -> Res<()> {
        let id = oid(id)?;
        let mut x = c.borrow_mut();
        let o = x.design.object_mut(id).ok_or("there is no such object")?;
        o.name = (!s.trim().is_empty()).then(|| s.to_string());
        x.changed = true;
        Ok(())
    });
    let c = ctx.clone();
    engine.register_fn("thread", move |id: i64| -> Res<i64> {
        Ok(i64::from(object(&c, id)?.thread.0))
    });
    let c = ctx.clone();
    engine.register_fn("set_thread", move |id: i64, t: i64| -> Res<()> {
        let (id, t) = (oid(id)?, tid(t)?);
        let mut x = c.borrow_mut();
        if x.design.thread(t).is_none() {
            return Err(format!("there is no thread {}", t.0).into());
        }
        x.design.object_mut(id).ok_or("there is no such object")?.thread = t;
        x.changed = true;
        Ok(())
    });
    let c = ctx.clone();
    engine.register_fn("size", move |id: i64| -> Res<Map> {
        let b = object(&c, id)?.control_bbox().ok_or("the object has no size")?;
        let mut m = Map::new();
        m.insert("x".into(), Dynamic::from_float(b.min.x));
        m.insert("y".into(), Dynamic::from_float(b.min.y));
        m.insert("width".into(), Dynamic::from_float(b.width()));
        m.insert("height".into(), Dynamic::from_float(b.height()));
        Ok(m)
    });
    let c = ctx.clone();
    engine.register_fn("settings", move |id: i64| -> Res<Dynamic> {
        settings_of(&object(&c, id)?.kind)
    });
    let c = ctx.clone();
    engine.register_fn("set_settings", move |id: i64, m: Map| -> Res<()> {
        let id = oid(id)?;
        let mut x = c.borrow_mut();
        let o = x.design.object_mut(id).ok_or("there is no such object")?;
        set_settings_of(&mut o.kind, Dynamic::from_map(m))?;
        x.changed = true;
        Ok(())
    });
    let c = ctx.clone();
    engine.register_fn("remove", move |id: i64| -> Res<()> {
        let id = oid(id)?;
        let mut x = c.borrow_mut();
        x.design.remove_object(id).ok_or("there is no such object")?;
        x.selection.retain(|s| *s != id);
        x.changed = true;
        Ok(())
    });
    let c = ctx.clone();
    engine.register_fn("sew_index", move |id: i64| -> Res<i64> {
        let id = oid(id)?;
        c.borrow()
            .design
            .index_of(id)
            .map(|i| i as i64)
            .ok_or_else(|| "there is no such object".into())
    });
    let c = ctx.clone();
    engine.register_fn("move_to", move |id: i64, index: i64| -> Res<()> {
        let id = oid(id)?;
        let mut x = c.borrow_mut();
        let n = x.design.objects().len();
        let at = usize::try_from(index).unwrap_or(0).min(n.saturating_sub(1));
        if !x.design.move_object(id, at) && x.design.object(id).is_none() {
            return Err("there is no such object".into());
        }
        x.changed = true;
        Ok(())
    });

    // Transforms of objects.
    let c = ctx.clone();
    engine.register_fn("move_objects", move |a: Array, dx: Dynamic, dy: Dynamic| -> Res<()> {
        let num = |v: &Dynamic| v.as_float().or_else(|_| v.as_int().map(|i| i as f64));
        let (dx, dy) = (num(&dx).map_err(|_| "a number")?, num(&dy).map_err(|_| "a number")?);
        let ids = ids_of(&a)?;
        let mut x = c.borrow_mut();
        if x.design.transform_objects(&ids, &Transform::translate(dx, dy)) {
            x.changed = true;
        }
        Ok(())
    });
    with_number!(engine, "rotate_objects", ctx, |c, a: Array, deg| {
        let ids = ids_of(&a)?;
        let mut x = c.borrow_mut();
        if let Some(t) = about_center(&x.design, &ids, Transform::rotate_deg(deg))
            && x.design.transform_objects(&ids, &t)
        {
            x.changed = true;
        }
        Ok(())
    });
    with_number!(engine, "scale_objects", ctx, |c, a: Array, f| {
        if !(f.is_finite() && f > 0.0 && f <= 100.0) {
            return Err("the scale must be more than 0 and at most 100".into());
        }
        let ids = ids_of(&a)?;
        let mut x = c.borrow_mut();
        if let Some(t) = about_center(&x.design, &ids, Transform::scale(f, f))
            && x.design.transform_objects(&ids, &t)
        {
            x.changed = true;
        }
        Ok(())
    });

    // Threads.
    let c = ctx.clone();
    engine.register_fn("threads", move || -> Array {
        let x = c.borrow();
        x.design
            .threads
            .iter()
            .map(|t| {
                let mut m = Map::new();
                m.insert("id".into(), Dynamic::from_int(i64::from(t.id.0)));
                m.insert("name".into(), t.name.clone().into());
                m.insert("color".into(), t.color.to_hex().into());
                m.insert("brand".into(), t.brand.clone().unwrap_or_default().into());
                m.insert("code".into(), t.code.clone().unwrap_or_default().into());
                Dynamic::from_map(m)
            })
            .collect()
    });
    let c = ctx.clone();
    engine.register_fn("add_thread", move |name: &str, hex: &str| -> Res<i64> {
        let rgb = color(hex)?;
        let mut x = c.borrow_mut();
        x.changed = true;
        Ok(i64::from(x.design.add_thread(name, rgb).0))
    });
    let look = |c: &Shared, t: i64, f: &dyn Fn(&mut emb_model::ThreadLook)| -> Res<()> {
        let t = tid(t)?;
        let mut x = c.borrow_mut();
        let mut l = emb_model::ThreadLook::of(x.design.thread(t).ok_or("there is no such thread")?);
        f(&mut l);
        let cw = x.design.colorway;
        x.design.set_colorway_look(cw, t, l);
        x.changed = true;
        Ok(())
    };
    let c = ctx.clone();
    engine.register_fn("set_thread_name", move |t: i64, s: &str| -> Res<()> {
        look(&c, t, &|l| l.name = s.to_string())
    });
    let c = ctx.clone();
    engine.register_fn("set_thread_color", move |t: i64, hex: &str| -> Res<()> {
        let rgb = color(hex)?;
        look(&c, t, &|l| l.color = rgb)
    });
    let c = ctx.clone();
    engine.register_fn("set_thread_code", move |t: i64, brand: &str, code: &str| -> Res<()> {
        look(&c, t, &|l| {
            l.brand = Some(brand.to_string()).filter(|s| !s.trim().is_empty());
            l.code = Some(code.to_string()).filter(|s| !s.trim().is_empty());
        })
    });

    // Engine commands.
    let offsets = |c: &Shared, a: &Array, spec: OffsetSpec| -> Res<Array> {
        let ids = ids_of(a)?;
        let mut x = c.borrow_mut();
        let new = x.design.add_offsets(&ids, &spec).map_err(|e| e.to_string())?;
        x.changed = true;
        Ok(id_array(new))
    };
    with_number!(engine, "add_outline", ctx, |c, a: Array, d| {
        if !(d.is_finite() && d.abs() <= 100.0) {
            return Err("the distance must be from -100 to 100 mm".into());
        }
        offsets(
            c,
            &a,
            OffsetSpec {
                distance_mm: d,
                make: OffsetMake::Line,
                ..OffsetSpec::default()
            },
        )
    });
    with_number!(engine, "add_border", ctx, |c, a: Array, w| {
        if !(w.is_finite() && w > 0.0 && w <= 20.0) {
            return Err("the border width must be more than 0 and at most 20 mm".into());
        }
        offsets(
            c,
            &a,
            OffsetSpec {
                distance_mm: 0.0,
                make: OffsetMake::Border {
                    width_mm: w,
                    place: BorderPlace::Outside,
                },
                ..OffsetSpec::default()
            },
        )
    });
    let c = ctx.clone();
    engine.register_fn("shape", move |a: Array, op: &str| -> Res<Array> {
        let key = |s: &str| s.to_lowercase().replace([' ', '-'], "_");
        let op = ShapeOp::ALL
            .into_iter()
            .find(|o| key(o.name()) == key(op))
            .ok_or_else(|| format!("no shaping {op:?}: use weld, intersect, exclude, front_minus_back, back_minus_front, flatten, divide or combine"))?;
        let ids = ids_of(&a)?;
        let mut x = c.borrow_mut();
        let new = x
            .design
            .shape(&ids, op, ShapeOptions::default())
            .map_err(|e| e.to_string())?;
        x.changed = true;
        Ok(id_array(new))
    });

    // The numbers of the stitch plan.
    let c = ctx.clone();
    engine.register_fn("info", move || -> Map {
        let x = c.borrow();
        let opts = x
            .machine
            .as_ref()
            .map(emb_stitch::PlanOptions::for_machine)
            .unwrap_or_default();
        let plan = emb_stitch::build_plan(&x.design, &opts, &mut emb_stitch::StitchCache::default());
        let s = &plan.stitches;
        let mut m = Map::new();
        let int = |v: usize| Dynamic::from_int(v as i64);
        m.insert("stitches".into(), int(s.stitch_count()));
        m.insert("colors".into(), int(s.color_count()));
        m.insert("color_changes".into(), int(s.color_count().saturating_sub(1)));
        m.insert("trims".into(), int(s.trim_count()));
        m.insert("jumps".into(), int(s.jump_count()));
        let (w, h) = s.extents().map_or((0.0, 0.0), |b| (b.width(), b.height()));
        m.insert("width_mm".into(), Dynamic::from_float(w));
        m.insert("height_mm".into(), Dynamic::from_float(h));
        m.insert("warnings".into(), int(plan.warnings.len()));
        m
    });
}

/// The engine with the limits and the functions of a script.
fn engine(ctx: &Shared, limits: &Limits) -> Engine {
    let mut e = Engine::new();
    e.set_max_operations(limits.operations);
    e.set_max_call_levels(64);
    e.set_max_expr_depths(64, 32);
    e.set_max_string_size(1 << 20);
    e.set_max_array_size(100_000);
    e.set_max_map_size(10_000);
    let start = Instant::now();
    let time = limits.time;
    e.on_progress(move |_| (start.elapsed() > time).then(|| Dynamic::from("time")));
    let c = ctx.clone();
    e.on_print(move |s| c.borrow_mut().messages.push(s.to_string()));
    let c = ctx.clone();
    e.on_debug(move |s, _, _| c.borrow_mut().messages.push(s.to_string()));
    register(&mut e, ctx);
    e
}

/// The error of a script, with its line.
fn script_error(e: &EvalAltResult, limits: &Limits, messages: Vec<String>) -> ScriptError {
    let message = match e {
        EvalAltResult::ErrorTerminated(..) => format!(
            "the script ran longer than {} seconds and was stopped",
            limits.time.as_secs()
        ),
        EvalAltResult::ErrorTooManyOperations(..) => "the script did too many operations and was stopped".into(),
        EvalAltResult::ErrorRuntime(v, _) => v.to_string(),
        other => other.to_string(),
    };
    // The message of Rhai ends with the position; the line is given apart.
    let message = match message.rfind(" (line ") {
        Some(i) => message[..i].to_string(),
        None => message,
    };
    ScriptError {
        message,
        line: e.position().line(),
        messages,
    }
}

/// Checks the syntax of a script.
pub fn check(src: &str) -> Result<(), ScriptError> {
    let ctx: Shared = Rc::new(RefCell::new(Ctx {
        design: Design::new(""),
        selection: Vec::new(),
        messages: Vec::new(),
        changed: false,
        machine: None,
    }));
    engine(&ctx, &Limits::default())
        .compile(src)
        .map(|_| ())
        .map_err(|e| ScriptError {
            message: e.0.to_string(),
            line: e.1.line(),
            messages: Vec::new(),
        })
}

/// Runs the script `src` on a copy of `design`, with the selected objects
/// `selection`, the input values `inputs` (the defaults of the script for
/// the ones not given) and the machine of the design for `info()`.
pub fn run(
    src: &str,
    design: &Design,
    selection: &[ObjectId],
    inputs: &BTreeMap<String, InputValue>,
    machine: Option<&MachineProfile>,
    limits: &Limits,
) -> Result<Outcome, ScriptError> {
    let ctx: Shared = Rc::new(RefCell::new(Ctx {
        design: design.clone(),
        selection: selection.to_vec(),
        messages: Vec::new(),
        changed: false,
        machine: machine.cloned(),
    }));
    let eng = engine(&ctx, limits);
    let ast = eng.compile(src).map_err(|e| ScriptError {
        message: e.0.to_string(),
        line: e.1.line(),
        messages: Vec::new(),
    })?;
    let mut input = Map::new();
    for i in ScriptInfo::parse(src, "").inputs {
        input.insert(i.name.as_str().into(), i.default.to_dynamic());
    }
    for (k, v) in inputs {
        input.insert(k.as_str().into(), v.to_dynamic());
    }
    let mut scope = Scope::new();
    scope.push_constant("input", input);
    let result = eng.run_ast_with_scope(&mut scope, &ast);
    drop(eng);
    let ctx = Rc::try_unwrap(ctx)
        .map_err(|_| ScriptError {
            message: "the script state is still in use".into(),
            line: None,
            messages: Vec::new(),
        })?
        .into_inner();
    if let Err(e) = result {
        return Err(script_error(&e, limits, ctx.messages));
    }
    if ctx.design.objects().len() > limits.max_objects {
        return Err(ScriptError {
            message: format!(
                "the design would have {} objects, more than {}",
                ctx.design.objects().len(),
                limits.max_objects
            ),
            line: None,
            messages: ctx.messages,
        });
    }
    let selection = ctx
        .selection
        .into_iter()
        .filter(|id| ctx.design.object(*id).is_some())
        .collect();
    Ok(Outcome {
        design: ctx.design,
        selection,
        messages: ctx.messages,
        changed: ctx.changed,
    })
}

#[cfg(test)]
mod tests;

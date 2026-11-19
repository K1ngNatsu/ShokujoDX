//! The library items as data (the review of P17, lane 2): the motifs, the
//! program-split patterns, the carving stamps, the decorative borders and
//! the clipart items, written as JSON for a contact sheet
//! (`emb-cli content`, `tools/content-sheet.py`). After a change of a
//! content family (ADR 0118), the sheet shows each item.

use emb_geom::{BBox, Path, Point};
use emb_model::family::{Item, Kind, families_of};
use emb_model::{Design, ObjectKind, Rgb, StitchKind};
use serde_json::{Value, json};

fn points(p: &[Point]) -> Value {
    json!(p.iter().map(|q| [q.x, q.y]).collect::<Vec<_>>())
}

fn flat(path: &Path) -> Vec<Value> {
    path.flatten(0.1).into_iter().map(|pl| points(&pl.points)).collect()
}

/// The stitches of the objects of a border round a small box, as lines.
fn border_lines(frame: &emb_model::Frame) -> Vec<Value> {
    let mut d = Design::new("");
    let t = d.add_thread("Border", Rgb(0, 0, 0));
    let b = BBox::from_center_size(Point::ZERO, 34.0, 20.0);
    for kind in emb_model::decor::frame_objects(frame, &b, 2.0) {
        d.add_object(t, kind);
    }
    let plan = crate::plan(&d, &emb_stitch::PlanOptions::default());
    let mut lines: Vec<Vec<Point>> = vec![Vec::new()];
    for r in &plan.stitches.records {
        if r.kind == StitchKind::Stitch {
            lines.last_mut().expect("one line").push(r.pos);
        } else if lines.last().is_some_and(|l| !l.is_empty()) {
            lines.push(Vec::new());
        }
    }
    lines.iter().filter(|l| l.len() >= 2).map(|l| points(l)).collect()
}

/// A clipart item: its fills (with their holes) and its lines, each with
/// the color of its thread.
fn clipart(d: &Design) -> Value {
    let mut objects = Vec::new();
    for o in d.objects() {
        let color = d
            .thread(o.thread)
            .map_or([0, 0, 0], |t| [t.color.0, t.color.1, t.color.2]);
        match &o.kind {
            ObjectKind::Fill { outer, holes, .. } => objects.push(json!({
                "fill": flat(outer),
                "holes": holes.iter().flat_map(flat).collect::<Vec<_>>(),
                "color": color,
            })),
            ObjectKind::Running { path, .. } => objects.push(json!({"run": flat(path), "color": color})),
            _ => {}
        }
    }
    json!({"name": d.name, "keywords": d.notes, "objects": objects})
}

/// All built-in library items: `motifs`, `splits`, `stamps`, `borders` and
/// `clipart` (the clipart items of the families; the eight items that are
/// drawn by hand are in the program crate).
pub fn items() -> Value {
    let motifs: Vec<Value> = emb_model::motif::builtin()
        .iter()
        .map(|m| json!({"name": m.name, "points": points(&m.points)}))
        .collect();
    let splits: Vec<Value> = emb_model::split::split_patterns()
        .iter()
        .map(|p| json!({"name": p.name, "width": p.width_mm, "rows": p.rows}))
        .collect();
    let stamps: Vec<Value> = emb_model::split::stamps()
        .iter()
        .map(|s| json!({"name": s.name, "lines": s.lines.iter().map(|l| points(l)).collect::<Vec<_>>()}))
        .collect();
    let borders: Vec<Value> = emb_model::decor::frames()
        .iter()
        .map(|f| json!({"name": f.name, "shape": f.shape.name(), "lines": border_lines(f)}))
        .collect();
    let art: Vec<Value> = families_of(Kind::Clipart)
        .into_iter()
        .flat_map(|f| f.presets())
        .filter_map(|item| match item {
            Item::Clipart(d) => Some(clipart(&d)),
            _ => None,
        })
        .collect();
    json!({"motifs": motifs, "splits": splits, "stamps": stamps, "borders": borders, "clipart": art})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_items_of_each_kind_are_there() {
        let all = items();
        let n = |kind: &str| all[kind].as_array().unwrap().len();
        assert_eq!(n("motifs"), emb_model::motif::builtin().len());
        assert_eq!(n("splits"), emb_model::split::split_patterns().len());
        assert_eq!(n("stamps"), emb_model::split::stamps().len());
        assert_eq!(n("borders"), emb_model::decor::frames().len());
        assert!(n("clipart") >= 30);
        // Each border has stitches, and each clipart item has objects.
        for b in all["borders"].as_array().unwrap() {
            assert!(!b["lines"].as_array().unwrap().is_empty(), "{}", b["name"]);
        }
        for c in all["clipart"].as_array().unwrap() {
            assert!(!c["objects"].as_array().unwrap().is_empty(), "{}", c["name"]);
        }
    }
}

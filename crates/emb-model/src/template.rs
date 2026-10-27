//! Templates: designs that new designs start from.
//!
//! A template file (`.embtemplate`) has the same contents as a project file
//! (see `project`). It keeps the machine, the hoop, the fabric, the
//! current properties, the styles, the threads and colorways, the guides
//! and the notes. It can also keep objects, for example a name badge with
//! its frame. It does not keep the backdrop picture or the order (P13
//! Milestone 2): each design from a template is a new order.
//!
//! The parameters of the sketches of a template (P14 Milestone 6, ADR 0078)
//! are the questions of the template: a new design from it asks for their
//! values (`parameters`, `Design::set_sketch_parameter`).

use crate::{Design, ModelError, ObjectId, ObjectKind, project};

/// A parameter of a sketch of a template.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateParameter {
    pub sketch: ObjectId,
    /// The name of the sketch object (if it has one).
    pub sketch_name: Option<String>,
    pub name: String,
    pub expr: String,
}

/// The parameters of the sketches of `design`, in sew order.
pub fn parameters(design: &Design) -> Vec<TemplateParameter> {
    design
        .objects()
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Sketch(s) => Some((o, s)),
            _ => None,
        })
        .flat_map(|(o, s)| {
            s.sketch().parameters.iter().map(|p| TemplateParameter {
                sketch: o.id,
                sketch_name: o.name.clone(),
                name: p.name.clone(),
                expr: p.expr.clone(),
            })
        })
        .collect()
}

pub const FILE_EXTENSION: &str = "embtemplate";

/// A template made from a design. Without `objects`, the template has no objects.
pub fn from_design(design: &Design, name: &str, objects: bool) -> Design {
    let mut t = design.clone();
    t.name = name.to_string();
    t.backdrop = None;
    t.order = None;
    if !objects {
        t.objects.clear();
    }
    t
}

/// A new design from a template.
pub fn new_design(template: &Design) -> Design {
    let mut d = template.clone();
    d.name = "Untitled".into();
    d.order = None;
    d
}

pub fn save(template: &Design, path: &std::path::Path) -> Result<(), ModelError> {
    project::save(template, path)
}

pub fn load(path: &std::path::Path) -> Result<Design, ModelError> {
    project::load(path)
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};

    use super::*;
    use crate::{FabricCatalog, ObjectKind, Rgb, RunningParams, Style};

    #[test]
    fn template_round_trip() {
        let mut d = Design::new("Cap logo");
        let t = d.add_thread("navy", Rgb(0, 0, 80));
        d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(10.0, 0.0)),
                params: RunningParams::default(),
            },
        );
        d.machine = Some("topaz20".into());
        d.set_fabric(&FabricCatalog::builtin(), Some("cap"), true);
        d.styles
            .push(Style::from_objects("Outline", d.objects().iter().map(|o| o.as_ref())).unwrap());
        d.notes = "Front panel".into();
        d.order = Some(crate::order::OrderInfo {
            customer: "Riverside FC".into(),
            ..Default::default()
        });

        let empty = from_design(&d, "Caps", false);
        assert!(empty.objects().is_empty());
        assert_eq!(empty.name, "Caps");
        let dir = std::env::temp_dir().join(format!("emb-template-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("Caps.{FILE_EXTENSION}"));
        save(&empty, &path).unwrap();
        let back = load(&path).unwrap();
        std::fs::remove_dir_all(&dir).ok();

        assert!(back.order.is_none(), "a template keeps no order");
        let mut with_order = back.clone();
        with_order.order = d.order.clone();
        assert!(new_design(&with_order).order.is_none());
        let n = new_design(&back);
        assert_eq!(n.name, "Untitled");
        assert_eq!(n.machine.as_deref(), Some("topaz20"));
        assert_eq!(n.fabric.as_deref(), Some("cap"));
        assert_eq!(n.current, d.current);
        assert_eq!(n.styles, d.styles);
        assert_eq!(n.threads, d.threads);
        assert_eq!(n.notes, "Front panel");
        assert_eq!(from_design(&d, "Caps", true).objects().len(), 1);
    }
}

//! Native project file format (`.embproj`). See ADR 0005.
//!
//! The file is pretty-printed JSON:
//!
//! ```json
//! { "format": "emb-project", "version": 1, "design": { ... } }
//! ```
//!
//! The loader rejects a file with a different `format` or a newer `version`.
//! It migrates older versions step by step before it reads the design.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::{Design, ModelError};

pub const FORMAT: &str = "emb-project";
pub const VERSION: u64 = 1;
pub const FILE_EXTENSION: &str = "embproj";

/// The largest distance from the origin that a project file can have (1 km).
/// A larger coordinate comes from a damaged file: the stitches to go there
/// would fill all the memory.
pub const MAX_COORD_MM: f64 = 1_000_000.0;

#[derive(Serialize)]
struct ProjectOut<'a> {
    format: &'static str,
    version: u64,
    design: &'a Design,
}

/// Writes a design as a project file string.
pub fn to_string(design: &Design) -> String {
    let out = ProjectOut {
        format: FORMAT,
        version: VERSION,
        design,
    };
    let mut s = serde_json::to_string_pretty(&out).expect("a design always serializes");
    s.push('\n');
    s
}

/// Reads a design from a project file string.
pub fn from_str(s: &str) -> Result<Design, ModelError> {
    let mut root: Value = serde_json::from_str(s)?;
    let format = root.get("format").and_then(Value::as_str);
    if format != Some(FORMAT) {
        return Err(ModelError::UnsupportedProject(format!(
            "expected format {FORMAT:?}, found {format:?}"
        )));
    }
    let version = root
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| ModelError::UnsupportedProject("version is missing".into()))?;
    if version > VERSION {
        return Err(ModelError::UnsupportedProject(format!(
            "file version {version} is newer than this program (version {VERSION}). Update the program"
        )));
    }
    if version == 0 {
        return Err(ModelError::UnsupportedProject("version 0 is not valid".into()));
    }
    migrate(&mut root, version)?;
    let design_value = root
        .get_mut("design")
        .map(Value::take)
        .ok_or_else(|| ModelError::UnsupportedProject("design is missing".into()))?;
    let design: Design = serde_json::from_value(design_value)?;
    validate(&design)?;
    Ok(design)
}

pub fn save(design: &Design, path: &std::path::Path) -> Result<(), ModelError> {
    std::fs::write(path, to_string(design)).map_err(|source| ModelError::Io {
        path: path.display().to_string(),
        source,
    })
}

pub fn load(path: &std::path::Path) -> Result<Design, ModelError> {
    let s = std::fs::read_to_string(path).map_err(|source| ModelError::Io {
        path: path.display().to_string(),
        source,
    })?;
    from_str(&s)
}

/// Changes the JSON from `version` to the current version, one step at a time.
/// Add one step for each new version.
fn migrate(_root: &mut Value, version: u64) -> Result<(), ModelError> {
    match version {
        VERSION => Ok(()),
        v => Err(ModelError::UnsupportedProject(format!("no migration from version {v}"))),
    }
}

/// Checks references in a design that serde cannot check.
fn validate(design: &Design) -> Result<(), ModelError> {
    let invalid = |message| ModelError::Invalid {
        what: "project file",
        message,
    };
    let mut thread_ids = BTreeSet::new();
    for t in &design.threads {
        if !thread_ids.insert(t.id) {
            return Err(invalid(format!("thread id {} is used more than once", t.id.0)));
        }
    }
    let mut object_ids = BTreeSet::new();
    for o in design.objects() {
        if !object_ids.insert(o.id) {
            return Err(invalid(format!("object id {} is used more than once", o.id.0)));
        }
        if !thread_ids.contains(&o.thread) {
            return Err(invalid(format!(
                "object {} uses thread {}, which is not in the palette",
                o.id.0, o.thread.0
            )));
        }
    }
    let far = |p: emb_geom::Point| {
        !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > MAX_COORD_MM || p.y.abs() > MAX_COORD_MM
    };
    if far(design.hoop.center) {
        return Err(invalid("the hoop center is out of range".into()));
    }
    for o in design.objects() {
        if let Some(b) = o.control_bbox()
            && (far(b.min) || far(b.max))
        {
            return Err(invalid(format!("object {} is out of range", o.id.0)));
        }
    }
    if !design.colorways.is_empty() && design.colorway >= design.colorways.len() {
        return Err(invalid(format!(
            "colorway {} does not exist (the design has {})",
            design.colorway,
            design.colorways.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};

    use super::*;
    use crate::{
        FillParams, HoopSelection, ObjectKind, Rgb, Rotation, SatinGeometry, SatinParams, Underlay, UnderlayLayer,
    };

    fn sample() -> Design {
        let mut d = Design::new("Patch");
        let red = d.add_thread("Red", Rgb(200, 20, 30));
        let white = d.add_thread("White", Rgb(255, 255, 255));
        d.hoop.selection = HoopSelection::Preset {
            id: "topaz20_crown".into(),
        };
        d.machine = Some("husqvarna_topaz20".into());
        d.rotation = Rotation::Deg90;
        d.add_object(
            red,
            ObjectKind::Fill {
                outer: Path::ellipse(Point::ZERO, 25.0, 25.0),
                holes: vec![Path::ellipse(Point::ZERO, 5.0, 5.0)],
                params: FillParams {
                    angle_deg: 30.0,
                    underlay: Underlay::Custom(vec![UnderlayLayer::Tatami {
                        inset_mm: 1.0,
                        spacing_mm: 2.0,
                        angle_deg: 120.0,
                    }]),
                    ..FillParams::default()
                },
            },
        );
        let id = d.add_object(
            white,
            ObjectKind::Satin {
                geometry: SatinGeometry {
                    rail_a: Path::ellipse(Point::ZERO, 27.0, 27.0),
                    rail_b: Path::ellipse(Point::ZERO, 24.0, 24.0),
                    rungs: vec![(Point::new(27.0, 0.0), Point::new(24.0, 0.0))],
                },
                params: SatinParams::default(),
            },
        );
        d.object_mut(id).unwrap().start = Some(Point::new(27.0, 0.0));
        d
    }

    /// The P7 settings: hoop angle and placement, auto start and end,
    /// hoopings, product (with a picture) and repeats.
    #[test]
    fn round_trip_keeps_the_hooping_product_and_repeats() {
        let mut d = sample();
        d.hoop.angle_deg = 15.0;
        d.hoop.auto_center = true;
        d.start_end = crate::StartEnd {
            start: Some(crate::Anchor::DesignBox { x: 0.0, y: 0.5 }),
            end: Some(crate::Anchor::Point {
                point: Point::new(3.0, 4.0),
            }),
        };
        let crown = crate::HoopCatalog::builtin().get("topaz20_crown").unwrap().clone();
        d.hooping = crate::HoopingLayout::auto(&d, &crown, 20.0, 5.0);
        d.hooping
            .as_mut()
            .unwrap()
            .splits
            .push(vec![Point::new(0.0, -10.0), Point::new(1.0, 10.0)]);
        let mut v = crate::ProductView::new(crate::ProductKind::Polo, Rgb(10, 20, 30));
        v.picture = Some(crate::ProductPicture {
            name: "shirt.png".into(),
            data: std::sync::Arc::new(vec![1, 2, 3, 250]),
            pixels: (2, 1),
            width_mm: 500.0,
        });
        d.product = Some(v);
        d.repeat = Some(crate::RepeatSpec::side_by_side(&d, 2.0));
        let back = from_str(&to_string(&d)).unwrap();
        assert!(back.same_content(&d));
        assert_eq!(
            back.product.as_ref().unwrap().picture.as_ref().unwrap().data.as_slice(),
            &[1, 2, 3, 250]
        );
        // A file of P6 (no P7 fields) reads with the defaults.
        let old = from_str(&to_string(&sample())).unwrap();
        assert!(old.hooping.is_none() && old.product.is_none() && old.repeat.is_none());
        assert_eq!(old.hoop.angle_deg, 0.0);
    }

    #[test]
    fn round_trip_keeps_content() {
        let d = sample();
        let s = to_string(&d);
        let back = from_str(&s).unwrap();
        assert!(back.same_content(&d));
        // Revisions are fresh after a load.
        assert_ne!(back.objects()[0].revision, d.objects()[0].revision);
        // Writing again gives the same text.
        assert_eq!(to_string(&back), s);
    }

    #[test]
    fn colorways_and_background_round_trip() {
        let mut d = sample();
        d.background = Some(Rgb(250, 240, 220));
        d.add_colorway("Night");
        d.threads[0].color = Rgb(1, 2, 3);
        d.background = Some(Rgb(0, 0, 0));
        let back = from_str(&to_string(&d)).unwrap();
        assert!(back.same_content(&d));
        let mut back = back;
        back.switch_colorway(0);
        assert_eq!(back.background, Some(Rgb(250, 240, 220)));

        d.colorway = 7;
        assert!(from_str(&to_string(&d)).is_err());
    }

    #[test]
    fn machine_functions_round_trip() {
        let mut d = sample();
        let id = d.objects()[0].id;
        let f = &mut d.object_mut(id).unwrap().functions;
        f.stop = true;
        f.tie_in = false;
        f.trim = crate::TrimMode::Never;
        let s = to_string(&d);
        assert!(s.contains("\"trim\": \"never\""), "{s}");
        let back = from_str(&s).unwrap();
        assert!(back.same_content(&d));
        // Default functions are not written.
        assert_eq!(s.matches("\"functions\"").count(), 1);
    }

    #[test]
    fn monogram_team_name_and_blend_records_round_trip() {
        let mut d = sample();
        d.monograms.push(crate::MonogramRecord {
            group: 1,
            initials: "ABC".into(),
            spec: crate::MonogramSpec::new(crate::FontRef::default()),
            center: Point::new(1.0, 2.0),
        });
        let mut names = crate::TeamNames::default();
        names.import_text("Ann\t7\nBea\t9", false);
        d.team_names = Some(names);
        d.blends.push(crate::BlendRecord { group: 2 });
        let back = from_str(&to_string(&d)).unwrap();
        assert!(back.same_content(&d));
        // The comparison sees a change in each record.
        let mut other = back.clone();
        other.monograms[0].initials = "ABD".into();
        assert!(!other.same_content(&d));
        let mut other = back.clone();
        other.team_names = None;
        assert!(!other.same_content(&d));
        let mut other = back;
        other.blends[0].group = 3;
        assert!(!other.same_content(&d));
    }

    #[test]
    fn stitch_objects_round_trip() {
        let mut d = Design::new("s");
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        d.add_object(
            t,
            ObjectKind::Stitches {
                path: Path::new()
                    .move_to(Point::new(0.0, 0.0))
                    .line_to(Point::new(1.0, 0.0))
                    .move_to(Point::new(5.0, 0.0))
                    .line_to(Point::new(5.0, 0.0)),
                trims: vec![1],
            },
        );
        let s = to_string(&d);
        assert!(s.contains("\"type\": \"stitches\""), "{s}");
        assert!(from_str(&s).unwrap().same_content(&d));
    }

    #[test]
    fn floats_round_trip_exactly() {
        let mut d = Design::new("f");
        let t = d.add_thread("t", Rgb(0, 0, 0));
        let p = Point::new(0.1 + 0.2, 1.0 / 3.0);
        d.add_object(
            t,
            ObjectKind::Running {
                path: Path::new().move_to(p).line_to(Point::new(-1e-9, 123456.789)),
                params: Default::default(),
            },
        );
        let back = from_str(&to_string(&d)).unwrap();
        assert!(back.same_content(&d));
    }

    #[test]
    fn rejects_wrong_format_and_newer_version() {
        let s = to_string(&sample());
        assert!(matches!(
            from_str(&s.replace("emb-project", "other")),
            Err(ModelError::UnsupportedProject(_))
        ));
        assert!(matches!(
            from_str(&s.replace("\"version\": 1", "\"version\": 99")),
            Err(ModelError::UnsupportedProject(_))
        ));
    }

    #[test]
    fn rejects_missing_thread() {
        let mut d = sample();
        d.threads.pop();
        assert!(matches!(from_str(&to_string(&d)), Err(ModelError::Invalid { .. })));
    }

    #[test]
    fn rejects_coordinates_out_of_range() {
        let mut d = sample();
        d.hoop.center = Point::new(1e9, 0.0);
        assert!(
            matches!(from_str(&to_string(&d)), Err(ModelError::Invalid { .. })),
            "the hoop"
        );
        let mut d = sample();
        let id = d.objects()[0].id;
        d.transform_objects(&[id], &emb_geom::Transform::translate(0.0, 2.0 * MAX_COORD_MM));
        assert!(
            matches!(from_str(&to_string(&d)), Err(ModelError::Invalid { .. })),
            "an object"
        );
        assert!(from_str(&to_string(&sample())).is_ok());
    }
}

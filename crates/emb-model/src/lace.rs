//! Free-standing lace templates (P11 Milestone 5, ADR 0048): a lace
//! ornament and a lace edge, our own designs.
//!
//! Each design is on water-soluble stabilizer (the fabric preset
//! `water_soluble`): a lace mesh under it, satin columns with lace underlay
//! on the mesh, and a border that touches the mesh all round, so each part
//! holds to the others when the stabilizer washes away.

use emb_geom::math::sin_cos_deg;
use emb_geom::{Path, Point};

use crate::{Design, FillParams, FillPattern, ObjectKind, Rgb, SatinGeometry, SatinParams, ThreadId, Underlay};

/// The fabric of the templates.
pub const FABRIC: &str = "water_soluble";

/// A function that makes a template design.
pub type MakeDesign = fn() -> Design;

/// The built-in lace templates: name and maker.
pub const TEMPLATES: [(&str, MakeDesign); 2] = [("Lace ornament", ornament), ("Lace edge", edge)];

/// Satin columns of lace: a little denser than normal, with lace underlay.
fn lace_satin() -> SatinParams {
    SatinParams {
        spacing_mm: 0.35,
        pull_compensation_mm: 0.2,
        underlay: Underlay::Lace,
        ..SatinParams::default()
    }
}

fn mesh(angle_deg: f64) -> FillParams {
    FillParams {
        angle_deg,
        ..FillParams::default().with_pattern(FillPattern::Mesh { angle_deg: 90.0 })
    }
}

fn satin(d: &mut Design, t: ThreadId, center: Path, width: f64) {
    d.add_object(
        t,
        ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(&center, width),
            params: lace_satin(),
        },
    );
}

fn segment(a: Point, b: Point) -> Path {
    Path::new().move_to(a).line_to(b)
}

fn lace_design(name: &str) -> (Design, ThreadId) {
    let mut d = Design::new(name);
    d.fabric = Some(FABRIC.into());
    let t = d.add_thread("White", Rgb(248, 248, 244));
    (d, t)
}

/// A snowflake ornament 80 mm across with a loop to hang it: a round mesh,
/// six spokes with branches, a ring in the middle, a border ring and the
/// loop at the top.
pub fn ornament() -> Design {
    let (mut d, t) = lace_design("Lace ornament");
    let r = 40.0;
    d.add_object(
        t,
        ObjectKind::Fill {
            outer: Path::ellipse(Point::ZERO, r - 2.0, r - 2.0),
            holes: Vec::new(),
            params: mesh(45.0),
        },
    );
    let at = |radius: f64, deg: f64| {
        let (s, c) = sin_cos_deg(deg);
        Point::new(radius * c, radius * s)
    };
    for k in 0..6 {
        let a = f64::from(k) * 60.0 - 90.0;
        satin(&mut d, t, segment(at(5.0, a), at(r - 4.0, a)), 2.4);
        for side in [-1.0, 1.0] {
            let from = at(20.0, a);
            let (s, c) = sin_cos_deg(a + side * 40.0);
            satin(&mut d, t, segment(from, from + Point::new(c, s) * 9.0), 1.8);
        }
    }
    satin(&mut d, t, Path::ellipse(Point::ZERO, 6.0, 6.0), 3.0);
    satin(&mut d, t, Path::ellipse(Point::ZERO, r - 1.5, r - 1.5), 4.0);
    // The loop overlaps the border ring at the top.
    satin(&mut d, t, Path::ellipse(Point::new(0.0, -(r + 3.5)), 5.0, 5.0), 2.5);
    d
}

/// A lace edge 120 mm long and 24 mm high: a band of mesh with a straight
/// satin top, eight scallops along the bottom and satin ends.
pub fn edge() -> Design {
    let (mut d, t) = lace_design("Lace edge");
    let (half, top, bottom) = (60.0, -8.0, 8.0);
    let n = 8;
    let w = 2.0 * half / f64::from(n);
    // The scallops: half circles below the band, right to left.
    let mut outline = vec![Point::new(-half, top), Point::new(half, top), Point::new(half, bottom)];
    let mut scallops = Vec::new();
    for k in (0..n).rev() {
        let cx = -half + w * (f64::from(k) + 0.5);
        let arc: Vec<Point> = (0..=16)
            .map(|i| {
                let (s, c) = sin_cos_deg(f64::from(i) * 180.0 / 16.0);
                Point::new(cx + c * w / 2.0, bottom + s * w / 2.0)
            })
            .collect();
        outline.extend(arc.iter().skip(1).copied());
        scallops.push(arc);
    }
    d.add_object(
        t,
        ObjectKind::Fill {
            outer: Path::polygon(&outline),
            holes: Vec::new(),
            params: mesh(45.0),
        },
    );
    satin(&mut d, t, segment(Point::new(-half, top), Point::new(half, top)), 3.0);
    for arc in scallops.iter().rev() {
        let path = arc[1..].iter().fold(Path::new().move_to(arc[0]), |p, q| p.line_to(*q));
        satin(&mut d, t, path, 3.0);
    }
    for x in [-half, half] {
        satin(&mut d, t, segment(Point::new(x, top), Point::new(x, bottom)), 3.0);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_templates_are_lace_on_water_soluble_stabilizer() {
        let fabrics = crate::FabricCatalog::builtin();
        for (name, make) in TEMPLATES {
            let d = make();
            assert_eq!(d.name, name);
            assert!(fabrics.of(&d).is_lace(), "{name}");
            let meshes = d
                .objects()
                .iter()
                .filter(|o| matches!(&o.kind, ObjectKind::Fill { params, .. } if matches!(params.pattern, FillPattern::Mesh { .. })))
                .count();
            assert_eq!(meshes, 1, "{name}");
            assert!(
                d.objects()
                    .iter()
                    .all(|o| !matches!(&o.kind, ObjectKind::Satin { params, .. } if params.underlay != Underlay::Lace)),
                "{name}"
            );
        }
        let e = d_size(&edge());
        assert!((e.0 - 120.0).abs() < 4.0 && (e.1 - 24.0).abs() < 4.0, "{e:?}");
    }

    fn d_size(d: &Design) -> (f64, f64) {
        let b = d
            .objects()
            .iter()
            .filter_map(|o| o.control_bbox())
            .reduce(emb_geom::BBox::union)
            .unwrap();
        (b.width(), b.height())
    }
}

//! Appliqué: a piece of fabric sewn on the design. See
//! `docs/design/p3-plan.md` (Milestone 7).
//!
//! An appliqué object sews in this order, for all its pieces together:
//!
//! 1. The placement line on the outline. The machine stops; the operator
//!    puts the fabric on.
//! 2. The tack-down stitches just inside the outline. They hold the fabric.
//!    If `trim_stop` is set, the machine stops; the operator cuts away the
//!    fabric outside the tack-down line.
//! 3. The cover: a satin border over the cut edge.
//!
//! Partial appliqué: edges in `open_edges` get no cover (for example where
//! another object covers that edge). Combined appliqué: several pieces in
//! one object sew all placement lines, then all tack-downs, then all covers,
//! so there are fewer stops.
//!
//! Layers (P10 Milestone 6): a piece can be on a layer of fabric over the
//! first one (up to [`MAX_LAYERS`]). Each layer in turn sews its placement
//! lines and tack-downs (the machine stops to cut the fabric of the layer
//! before it, and to put on its fabric); the covers of all layers come
//! last, the bottom layer first. Each piece can have its own fabric color.

use emb_geom::{Path, Point};
use serde::{Deserialize, Serialize};

use crate::{Rgb, SatinParams, Underlay};

/// The display color of appliqué fabric with no color set: a light gray.
pub const DEFAULT_FABRIC_COLOR: Rgb = Rgb(200, 200, 205);

/// The most fabric layers of one appliqué object.
pub const MAX_LAYERS: u8 = 4;

fn is_zero(v: &u8) -> bool {
    *v == 0
}

/// One piece of appliqué fabric: its outline, holes in it, and the edges of
/// the outline that get no cover.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppliquePiece {
    pub outer: Path,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Path>,
    /// Edges of `outer` (see [`outline_edges`]) with no cover stitches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub open_edges: Vec<usize>,
    /// The layer of fabric (0: the first, up to `MAX_LAYERS - 1`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub layer: u8,
    /// The color of the fabric of this piece, for the display (else the
    /// color of the object).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabric: Option<Rgb>,
}

impl AppliquePiece {
    pub fn new(outer: Path, holes: Vec<Path>) -> AppliquePiece {
        AppliquePiece {
            outer,
            holes,
            open_edges: Vec::new(),
            layer: 0,
            fabric: None,
        }
    }

    /// The fabric color of the piece for the display.
    pub fn fabric_color(&self, params: &AppliqueParams) -> Rgb {
        self.fabric.or(params.fabric).unwrap_or(DEFAULT_FABRIC_COLOR)
    }
}

/// How the tack-down holds the fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TackDown {
    /// A running line.
    #[default]
    Run,
    /// A narrow zigzag. It holds fabric that frays better.
    Zigzag,
    /// No tack-down (for example for felt with a wide cover).
    None,
}

/// Parameters of an appliqué object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppliqueParams {
    /// Sew a placement line and stop, so that the operator knows where to
    /// put the fabric. Off: the fabric is put on before the object starts.
    pub placement: bool,
    pub placement_stitch_mm: f64,
    pub tack_down: TackDown,
    /// Distance of the tack-down line inside the outline.
    pub tack_inset_mm: f64,
    /// Width and stitch spacing of a zigzag tack-down.
    pub tack_width_mm: f64,
    pub tack_spacing_mm: f64,
    /// Stop after the tack-down, so that the operator can cut the fabric.
    pub trim_stop: bool,
    /// Sew the satin cover.
    pub cover: bool,
    pub cover_width_mm: f64,
    /// How far the cover goes outside the outline.
    pub cover_outside_mm: f64,
    /// Density, pull compensation and underlay of the cover satin.
    pub cover_satin: SatinParams,
    /// The color of the appliqué fabric, for the display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fabric: Option<Rgb>,
}

impl Default for AppliqueParams {
    fn default() -> Self {
        AppliqueParams {
            placement: true,
            placement_stitch_mm: 2.5,
            tack_down: TackDown::Run,
            tack_inset_mm: 0.5,
            tack_width_mm: 1.2,
            tack_spacing_mm: 1.5,
            trim_stop: true,
            cover: true,
            cover_width_mm: 3.0,
            cover_outside_mm: 1.0,
            cover_satin: SatinParams {
                // The tack-down is the underlay of the cover.
                underlay: Underlay::None,
                ..SatinParams::default()
            },
            fabric: None,
        }
    }
}

/// The edges of the first subpath of `path`, each flattened to points: one
/// edge for each line or curve, and one for the closing line if it has a
/// length. The last point of an edge is the first point of the next one.
pub fn outline_edges(path: &Path, tolerance: f64) -> Vec<Vec<Point>> {
    use emb_geom::path::PathCmd;
    let mut out: Vec<Vec<Point>> = Vec::new();
    let (mut start, mut pen) = (None, Point::ZERO);
    for cmd in &path.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => {
                if start.is_some() {
                    break;
                }
                start = Some(p);
                pen = p;
            }
            PathCmd::LineTo(p) => {
                if p != pen {
                    out.push(vec![pen, p]);
                }
                pen = p;
            }
            PathCmd::CubicTo(c1, c2, p) => {
                let pts = Path::new()
                    .move_to(pen)
                    .cubic_to(c1, c2, p)
                    .flatten(tolerance)
                    .into_iter()
                    .next()
                    .map_or_else(|| vec![pen, p], |l| l.points);
                out.push(pts);
                pen = p;
            }
            PathCmd::Close => {
                if let Some(s) = start
                    && s != pen
                {
                    out.push(vec![pen, s]);
                }
                break;
            }
        }
    }
    // An outline that ends at its start without a Close has no closing edge.
    if let (Some(s), Some(last)) = (start, out.last())
        && last.last() != Some(&s)
        && !path.cmds.iter().any(|c| matches!(c, PathCmd::Close))
    {
        out.push(vec![pen, s]);
    }
    out
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;

    use super::*;

    #[test]
    fn edges_of_a_rectangle_and_a_curve() {
        let r = Path::rect(BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 5.0)));
        let e = outline_edges(&r, 0.05);
        assert_eq!(e.len(), 4);
        assert_eq!(e[0][0], Point::new(0.0, 0.0));
        assert_eq!(e[3].last(), Some(&Point::new(0.0, 0.0)));
        let c = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .cubic_to(Point::new(0.0, 10.0), Point::new(10.0, 10.0), Point::new(10.0, 0.0))
            .close();
        let e = outline_edges(&c, 0.05);
        assert_eq!(e.len(), 2, "the curve and the closing line");
        assert!(e[0].len() > 4);
        // A polygon from points without Close still closes.
        let open = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(5.0, 0.0))
            .line_to(Point::new(5.0, 5.0));
        assert_eq!(outline_edges(&open, 0.05).len(), 3);
    }

    #[test]
    fn params_serialize_short_and_default() {
        let p: AppliqueParams = serde_json::from_str(r#"{"cover_width_mm":4.0}"#).unwrap();
        assert_eq!(p.cover_width_mm, 4.0);
        assert!(p.placement && p.trim_stop);
        assert_eq!(p.cover_satin.underlay, Underlay::None);
        let s = serde_json::to_string(&TackDown::Zigzag).unwrap();
        assert_eq!(s, "\"zigzag\"");
    }
}

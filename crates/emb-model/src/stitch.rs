//! Flat stitch lists. Machine formats read and write [`StitchDesign`];
//! the renderer draws it. These types are here, not in `emb-stitch`, so that
//! `emb-formats` and `emb-render` do not depend on the stitch engine.

use emb_geom::{BBox, Point};
use serde::{Deserialize, Serialize};

use crate::Thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StitchKind {
    /// Needle goes down at the position.
    Stitch,
    /// Frame moves to the position with no needle penetration.
    Jump,
    /// Thread is cut. The position is where the cut happens.
    Trim,
    /// Machine changes to the next thread.
    ColorChange,
    /// Machine stops (for example for an applique step).
    Stop,
    /// End of the design.
    End,
    /// The machine sews slowly from here (a speed function of industrial
    /// formats). The position is the needle position; there is no move.
    Slow,
    /// The machine sews at its normal speed again.
    Fast,
}

impl StitchKind {
    /// `true` for a speed change (`Slow`, `Fast`): no move, no needle.
    pub fn is_speed(self) -> bool {
        matches!(self, StitchKind::Slow | StitchKind::Fast)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StitchRecord {
    pub pos: Point,
    pub kind: StitchKind,
}

impl StitchRecord {
    pub fn new(pos: Point, kind: StitchKind) -> Self {
        Self { pos, kind }
    }

    pub fn stitch(x: f64, y: f64) -> Self {
        Self::new(Point::new(x, y), StitchKind::Stitch)
    }
}

/// A complete stitch plan in mm, ready for export.
///
/// `threads[0]` is the first color. Each `ColorChange` record moves to the next thread.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StitchDesign {
    pub threads: Vec<Thread>,
    pub records: Vec<StitchRecord>,
}

impl StitchDesign {
    fn count(&self, kind: StitchKind) -> usize {
        self.records.iter().filter(|r| r.kind == kind).count()
    }

    pub fn stitch_count(&self) -> usize {
        self.count(StitchKind::Stitch)
    }

    pub fn trim_count(&self) -> usize {
        self.count(StitchKind::Trim)
    }

    pub fn jump_count(&self) -> usize {
        self.count(StitchKind::Jump)
    }

    /// Number of color blocks: color changes plus one, or zero for an empty design.
    pub fn color_count(&self) -> usize {
        if self.records.is_empty() {
            0
        } else {
            self.count(StitchKind::ColorChange) + 1
        }
    }

    /// Bounding box of all stitch and jump positions.
    pub fn extents(&self) -> Option<BBox> {
        BBox::from_points(
            self.records
                .iter()
                .filter(|r| matches!(r.kind, StitchKind::Stitch | StitchKind::Jump))
                .map(|r| r.pos),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![
                StitchRecord::stitch(0.0, 0.0),
                StitchRecord::stitch(1.0, 0.0),
                StitchRecord::new(Point::new(1.0, 0.0), StitchKind::Trim),
                StitchRecord::new(Point::new(5.0, 5.0), StitchKind::Jump),
                StitchRecord::new(Point::new(5.0, 5.0), StitchKind::ColorChange),
                StitchRecord::stitch(5.0, 6.0),
                StitchRecord::new(Point::new(5.0, 6.0), StitchKind::End),
            ],
        };
        assert_eq!(d.stitch_count(), 3);
        assert_eq!(d.trim_count(), 1);
        assert_eq!(d.color_count(), 2);
        let e = d.extents().unwrap();
        assert_eq!((e.width(), e.height()), (5.0, 6.0));
        assert_eq!(StitchDesign::default().color_count(), 0);
    }
}

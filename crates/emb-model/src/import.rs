//! A design from a machine file: one stitch object per color block.
//!
//! - A color change starts a new object with the next thread.
//! - A stop ends the object; the object gets the "stop after" function, and
//!   the next object has the same thread. The object trims before the stop
//!   only if the file has a trim there (TBF keeps both; appliqué trims
//!   before each stop).
//! - A jump starts a new part (subpath) of the object; a trim before it is
//!   kept in `trims`.
//! - The objects have no tie-in and tie-off of their own: the lock stitches
//!   are in the file.
//! - A file with fewer threads than color blocks (DST has none) gets
//!   default colors.
//!
//! [`records_of_stitch_objects`] goes the other way (P22 Milestone 5,
//! ADR 0172): the records of stitch objects as a machine file has them,
//! for the check and the fix of a machine file.

use emb_geom::{Path, Point};

use crate::{
    Design, Functions, ObjectId, ObjectKind, Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId, TrimMode,
};

/// Colors for files without thread colors.
const DEFAULT_COLORS: [(u8, u8, u8); 8] = [
    (30, 30, 30),
    (200, 30, 45),
    (30, 90, 190),
    (40, 150, 80),
    (230, 170, 30),
    (140, 60, 170),
    (240, 120, 30),
    (120, 120, 120),
];

/// One object while it is read.
#[derive(Default)]
struct Block {
    subs: Vec<Vec<Point>>,
    trims: Vec<usize>,
    /// The next stitch starts a new part.
    new_part: bool,
    trim_pending: bool,
}

impl Block {
    fn stitch(&mut self, p: Point) {
        if self.new_part || self.subs.is_empty() {
            if self.trim_pending && !self.subs.is_empty() {
                self.trims.push(self.subs.len());
            }
            self.subs.push(Vec::new());
            self.new_part = false;
            self.trim_pending = false;
        }
        if let Some(s) = self.subs.last_mut() {
            s.push(p);
        }
    }

    fn kind(self) -> Option<ObjectKind> {
        if self.subs.iter().map(Vec::len).sum::<usize>() == 0 {
            return None;
        }
        let mut path = Path::new();
        for s in &self.subs {
            if let Some((&first, rest)) = s.split_first() {
                path = path.move_to(first);
                for &p in rest {
                    path = path.line_to(p);
                }
            }
        }
        Some(ObjectKind::Stitches {
            path,
            trims: self.trims,
        })
    }
}

/// Makes a design with one stitch object per color block (see the module notes).
pub fn design_from_stitches(sd: &StitchDesign, name: &str) -> Design {
    let mut design = Design::new(name);
    let objects = objects_from_stitches(sd);
    let mut thread_ids = Vec::new();
    let blocks = objects.iter().map(|o| o.0 + 1).max().unwrap_or(0).max(sd.threads.len());
    for i in 0..blocks {
        let id = match sd.threads.get(i) {
            Some(t) => {
                let id = design.add_thread(t.name.clone(), t.color);
                if let Some(x) = design.threads.iter_mut().find(|x| x.id == id) {
                    x.brand.clone_from(&t.brand);
                    x.code.clone_from(&t.code);
                }
                id
            }
            None => {
                let (r, g, b) = DEFAULT_COLORS[i % DEFAULT_COLORS.len()];
                design.add_thread(format!("Color {}", i + 1), Rgb(r, g, b))
            }
        };
        thread_ids.push(id);
    }
    for (t, kind, functions) in objects {
        let id = design.add_object(thread_ids[t], kind);
        if let Some(o) = design.object_mut(id) {
            o.functions = functions;
        }
    }
    design
}

/// The stitch objects of the records of `sd` (see the module notes): for
/// each, the number of its color block (its thread in `sd.threads`), its
/// kind and its functions.
pub fn objects_from_stitches(sd: &StitchDesign) -> Vec<(usize, ObjectKind, Functions)> {
    let mut thread = 0usize;
    let mut block = Block::default();
    // Objects: (thread index, kind, stop after, trim after, slow).
    let mut objects: Vec<(usize, ObjectKind, bool, TrimMode, bool)> = Vec::new();
    // A speed change ends an object: the objects after `Slow` sew slowly.
    let mut slow = false;
    let finish = |block: &mut Block,
                  objects: &mut Vec<(usize, ObjectKind, bool, TrimMode, bool)>,
                  t: usize,
                  stop: bool,
                  slow: bool| {
        // At a stop, a trim that is not followed by a stitch is the trim before the stop.
        let trim = match (stop, block.trim_pending) {
            (false, _) => TrimMode::Auto,
            (true, true) => TrimMode::Always,
            (true, false) => TrimMode::Never,
        };
        if let Some(k) = std::mem::take(block).kind() {
            objects.push((t, k, stop, trim, slow));
        }
    };
    for r in &sd.records {
        match r.kind {
            StitchKind::Stitch => block.stitch(r.pos),
            StitchKind::Jump => block.new_part = true,
            StitchKind::Trim => {
                block.new_part = true;
                block.trim_pending = true;
            }
            StitchKind::ColorChange => {
                finish(&mut block, &mut objects, thread, false, slow);
                thread += 1;
            }
            StitchKind::Stop => finish(&mut block, &mut objects, thread, true, slow),
            StitchKind::Slow | StitchKind::Fast => {
                let now = r.kind == StitchKind::Slow;
                if now != slow {
                    finish(&mut block, &mut objects, thread, false, slow);
                    slow = now;
                }
            }
            StitchKind::End => break,
        }
    }
    finish(&mut block, &mut objects, thread, false, slow);
    objects
        .into_iter()
        .map(|(t, kind, stop, trim, slow)| {
            (
                t,
                kind,
                Functions {
                    tie_in: false,
                    tie_off: false,
                    trim,
                    stop,
                    slow,
                },
            )
        })
        .collect()
}

/// The records of the stitch objects `ids` of `d`, in this order, as a
/// machine file has them: the reverse of [`objects_from_stitches`]. Where
/// the thread changes, a trim and a color change; before each part of an
/// object, a jump (and a trim if the part is in `trims`); the trim before
/// a stop, the stops and the speed changes; an end. The threads are the
/// threads of the color blocks (with their ids in `d`). Also, for each
/// record, the number of its object in `ids`. The objects that are not
/// stitch objects are left out.
pub fn records_of_stitch_objects(d: &Design, ids: &[ObjectId]) -> (StitchDesign, Vec<usize>) {
    let mut sd = StitchDesign::default();
    let mut owner = Vec::new();
    let mut push = |sd: &mut StitchDesign, k: usize, kind: StitchKind, pos: Point| {
        sd.records.push(StitchRecord::new(pos, kind));
        owner.push(k);
    };
    let thread_of = |id: ThreadId| {
        d.thread(id).cloned().unwrap_or_else(|| Thread {
            id,
            ..Default::default()
        })
    };
    let mut needle = Point::ZERO;
    let mut thread: Option<ThreadId> = None;
    let mut slow = false;
    let mut last = 0;
    for (k, id) in ids.iter().enumerate() {
        let Some(o) = d.object(*id) else { continue };
        let ObjectKind::Stitches { path, trims } = &o.kind else {
            continue;
        };
        let parts = stitch_subpaths(path);
        if parts.iter().all(Vec::is_empty) {
            continue;
        }
        match thread {
            None => sd.threads.push(thread_of(o.thread)),
            Some(t) if t != o.thread => {
                if sd.records.last().is_none_or(|r| r.kind != StitchKind::Trim) {
                    push(&mut sd, k, StitchKind::Trim, needle);
                }
                push(&mut sd, k, StitchKind::ColorChange, needle);
                sd.threads.push(thread_of(o.thread));
            }
            Some(_) => {}
        }
        thread = Some(o.thread);
        if o.functions.slow != slow {
            slow = o.functions.slow;
            let kind = if slow { StitchKind::Slow } else { StitchKind::Fast };
            push(&mut sd, k, kind, needle);
        }
        for (i, part) in parts.iter().enumerate() {
            let Some((&first, rest)) = part.split_first() else {
                continue;
            };
            if i > 0 && trims.contains(&i) {
                push(&mut sd, k, StitchKind::Trim, needle);
            }
            push(&mut sd, k, StitchKind::Jump, first);
            push(&mut sd, k, StitchKind::Stitch, first);
            for &p in rest {
                push(&mut sd, k, StitchKind::Stitch, p);
            }
            needle = *part.last().unwrap_or(&first);
        }
        if o.functions.stop {
            if o.functions.trim == TrimMode::Always {
                push(&mut sd, k, StitchKind::Trim, needle);
            }
            push(&mut sd, k, StitchKind::Stop, needle);
        }
        last = k;
    }
    push(&mut sd, last, StitchKind::End, needle);
    (sd, owner)
}

/// The subpaths of a stitch object as needle points. All points stay, also
/// repeated ones; curves count as their end points (the stitch engine
/// sews them so: `emb_stitch::stitch_subpaths` is this function).
pub fn stitch_subpaths(path: &Path) -> Vec<Vec<Point>> {
    use emb_geom::path::PathCmd;
    let mut out: Vec<Vec<Point>> = Vec::new();
    for cmd in &path.cmds {
        match *cmd {
            PathCmd::MoveTo(p) => out.push(vec![p]),
            PathCmd::LineTo(p) | PathCmd::CubicTo(_, _, p) => match out.last_mut() {
                Some(s) => s.push(p),
                None => out.push(vec![p]),
            },
            PathCmd::Close => {
                if let Some(s) = out.last_mut()
                    && let Some(&first) = s.first()
                {
                    s.push(first);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StitchRecord, Thread, ThreadId};

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    #[test]
    fn blocks_parts_trims_and_stops() {
        let sd = StitchDesign {
            threads: vec![Thread {
                id: ThreadId(0),
                name: "Red".into(),
                color: Rgb(200, 0, 0),
                brand: Some("Test".into()),
                code: Some("7".into()),
                ..Default::default()
            }],
            records: vec![
                rec(0.0, 0.0, StitchKind::Jump),
                rec(0.0, 0.0, StitchKind::Stitch),
                rec(1.0, 0.0, StitchKind::Stitch),
                rec(1.0, 0.0, StitchKind::Trim),
                rec(9.0, 0.0, StitchKind::Jump),
                rec(9.0, 0.0, StitchKind::Stitch),
                rec(9.0, 1.0, StitchKind::Stitch),
                rec(15.0, 1.0, StitchKind::Jump),
                rec(15.0, 1.0, StitchKind::Stitch),
                rec(15.0, 1.0, StitchKind::Stop),
                rec(16.0, 1.0, StitchKind::Stitch),
                rec(16.0, 1.0, StitchKind::ColorChange),
                rec(20.0, 0.0, StitchKind::Jump),
                rec(20.0, 0.0, StitchKind::Stitch),
                rec(21.0, 0.0, StitchKind::Stitch),
                rec(21.0, 0.0, StitchKind::End),
            ],
        };
        let d = design_from_stitches(&sd, "x");
        assert_eq!(d.objects().len(), 3);
        assert_eq!(d.threads.len(), 2);
        assert_eq!(d.threads[0].code.as_deref(), Some("7"));
        assert_eq!(d.threads[1].name, "Color 2");
        let o = &d.objects()[0];
        let ObjectKind::Stitches { path, trims } = &o.kind else {
            panic!()
        };
        // Three parts: (0,0)-(1,0); after a trim (9,0)-(9,1); after a jump (15,1).
        assert_eq!(path.cmds.len(), 5);
        assert_eq!(trims, &vec![1]);
        assert!(o.functions.stop && o.functions.trim == TrimMode::Never && !o.functions.tie_in);
        assert_eq!(d.objects()[1].thread, d.objects()[0].thread);
        assert_eq!(d.objects()[2].thread, d.threads[1].id);
    }

    #[test]
    fn a_trim_before_a_stop_is_kept() {
        let sd = StitchDesign {
            threads: vec![],
            records: vec![
                rec(0.0, 0.0, StitchKind::Stitch),
                rec(1.0, 0.0, StitchKind::Stitch),
                rec(1.0, 0.0, StitchKind::Trim),
                rec(1.0, 0.0, StitchKind::Stop),
                rec(2.0, 0.0, StitchKind::Stitch),
                rec(3.0, 0.0, StitchKind::Stitch),
                rec(3.0, 0.0, StitchKind::End),
            ],
        };
        let d = design_from_stitches(&sd, "x");
        let f = d.objects()[0].functions;
        assert!(f.stop && f.trim == TrimMode::Always);
        assert_eq!(d.objects()[1].functions.trim, TrimMode::Auto);
    }

    /// The records of the stitch objects of a file make the same objects
    /// again (ADR 0172).
    #[test]
    fn the_records_of_stitch_objects_make_the_same_objects() {
        let sd = StitchDesign {
            threads: Vec::new(),
            records: vec![
                rec(0.0, 0.0, StitchKind::Jump),
                rec(0.0, 0.0, StitchKind::Stitch),
                rec(2.0, 0.0, StitchKind::Stitch),
                rec(2.0, 0.0, StitchKind::Trim),
                rec(9.0, 0.0, StitchKind::Jump),
                rec(9.0, 0.0, StitchKind::Stitch),
                rec(9.0, 2.0, StitchKind::Stitch),
                rec(12.0, 2.0, StitchKind::Jump),
                rec(12.0, 2.0, StitchKind::Stitch),
                rec(13.0, 2.0, StitchKind::Stitch),
                rec(13.0, 2.0, StitchKind::Trim),
                rec(13.0, 2.0, StitchKind::Stop),
                rec(14.0, 2.0, StitchKind::Stitch),
                rec(15.0, 2.0, StitchKind::Stitch),
                rec(15.0, 2.0, StitchKind::Trim),
                rec(15.0, 2.0, StitchKind::ColorChange),
                rec(15.0, 2.0, StitchKind::Slow),
                rec(20.0, 0.0, StitchKind::Jump),
                rec(20.0, 0.0, StitchKind::Stitch),
                rec(21.0, 0.0, StitchKind::Stitch),
                rec(21.0, 0.0, StitchKind::End),
            ],
        };
        let d = design_from_stitches(&sd, "x");
        let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
        let (again, owner) = records_of_stitch_objects(&d, &ids);
        assert_eq!(owner.len(), again.records.len());
        assert_eq!(again.threads.len(), 2);
        assert_eq!(again.threads[1].id, d.objects()[2].thread);
        let back = objects_from_stitches(&again);
        let before = objects_from_stitches(&sd);
        assert_eq!(back, before);
        // The needle points are the same, in the same order.
        let points = |s: &StitchDesign| -> Vec<Point> {
            s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect()
        };
        assert_eq!(points(&again), points(&sd));
    }

    /// A slow part of a machine file (U01) is an object with Sew slowly.
    #[test]
    fn speed_changes_make_slow_objects() {
        let sd = StitchDesign {
            threads: Vec::new(),
            records: vec![
                rec(0.0, 0.0, StitchKind::Stitch),
                rec(2.0, 0.0, StitchKind::Stitch),
                rec(2.0, 0.0, StitchKind::Slow),
                rec(4.0, 0.0, StitchKind::Stitch),
                rec(6.0, 0.0, StitchKind::Stitch),
                rec(6.0, 0.0, StitchKind::Fast),
                rec(8.0, 0.0, StitchKind::Stitch),
                rec(9.0, 0.0, StitchKind::Stitch),
                rec(9.0, 0.0, StitchKind::End),
            ],
        };
        let d = design_from_stitches(&sd, "speed");
        let slow: Vec<bool> = d.objects().iter().map(|o| o.functions.slow).collect();
        assert_eq!(slow, [false, true, false]);
    }
}

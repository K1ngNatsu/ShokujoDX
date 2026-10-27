//! The needles of a multi-needle machine (ADR 0063): the threads that are
//! on the machine, the needles that the user gives to threads of a design,
//! and the needle of each color block.
//!
//! [`assign`] gives each color block a needle, in sew order. A thread that
//! is on a needle stays there; a new thread goes on a free needle; when the
//! design has more threads than the machine has needles, the operator
//! changes the thread on a needle (a thread change, and the machine stops).

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use crate::{Design, MachineProfile, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId, ThreadLook};

/// The most needles that a machine can have. U01 files have codes for
/// needles 1 to 15 only.
pub const MAX_NEEDLES: u8 = 15;
/// The numbers of needles that a machine can have.
pub const NEEDLES_RANGE: RangeInclusive<u8> = 1..=MAX_NEEDLES;

/// A thread of a design that the user gave a needle (1 is the first needle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedlePin {
    pub thread: ThreadId,
    pub needle: u8,
}

/// The needles of one machine and the threads that are on them now. The
/// program settings keep one for each machine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NeedleSetup {
    /// The number of needles. `None`: the number of the machine profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    /// The thread on each needle (index 0 is needle 1). `None`: no thread,
    /// or not known. The list can be shorter than the number of needles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<Option<ThreadLook>>,
}

impl NeedleSetup {
    /// The thread on `needle` (1 is the first needle).
    pub fn thread(&self, needle: u8) -> Option<&ThreadLook> {
        self.threads.get(usize::from(needle).checked_sub(1)?)?.as_ref()
    }

    /// The number of needles of `machine` with this setup: the user's
    /// number, else the number of the profile. `None` for a machine with
    /// one needle.
    pub fn count_for(setup: Option<&NeedleSetup>, machine: &MachineProfile) -> Option<u8> {
        setup
            .and_then(|s| s.count)
            .or(machine.needles)
            .map(|n| n.min(MAX_NEEDLES))
            .filter(|n| *n > 1)
    }

    /// True if the setup has nothing that the profile does not give.
    pub fn is_empty(&self) -> bool {
        self.count.is_none() && self.threads.iter().all(Option::is_none)
    }
}

/// True if `a` and `b` are the same thread: the same brand and code, or
/// (if one has no code) the same color.
pub fn same_thread(a: &ThreadLook, b: &ThreadLook) -> bool {
    match (&a.code, &b.code) {
        (Some(x), Some(y)) => x.trim().eq_ignore_ascii_case(y.trim()) && a.brand == b.brand,
        _ => a.color == b.color,
    }
}

/// The needle of each color block of a design.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NeedlePlan {
    /// The number of needles of the machine.
    pub count: u8,
    /// The needle of each color block, in sew order (1 is the first needle).
    pub blocks: Vec<u8>,
    /// The color blocks before which the operator changes the thread on
    /// the needle of the block (the machine stops), in sew order.
    pub changes: Vec<usize>,
    /// For each needle (index 0 is needle 1): the first color block that
    /// the needle sews. Its thread must be on the needle at the start.
    pub start: Vec<Option<usize>>,
    /// The needles whose thread at the start is not the thread that is on
    /// the machine now: the operator puts the thread on before the start.
    pub loads: Vec<u8>,
}

/// What is on a needle while the blocks get their needles.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Slot {
    Empty,
    /// A thread that the design does not sew.
    Foreign,
    /// The thread of a group of blocks.
    Holds(usize),
}

/// Gives each color block a needle (ADR 0063). `blocks` has the thread of
/// each color block in sew order (`None`: a block with no known thread).
/// `pins` are the needles that the user gave threads; a pin of a needle
/// that the machine does not have is not used.
pub fn assign(blocks: &[Option<&Thread>], count: u8, setup: &NeedleSetup, pins: &[NeedlePin]) -> NeedlePlan {
    let n = usize::from(count.clamp(1, MAX_NEEDLES));
    let looks: Vec<Option<ThreadLook>> = blocks.iter().map(|t| t.map(ThreadLook::of)).collect();
    // The group of each block: the first block with the same thread.
    let mut group = Vec::with_capacity(blocks.len());
    for i in 0..blocks.len() {
        let same = |j: usize| match (blocks[i], blocks[j], &looks[i], &looks[j]) {
            (Some(a), Some(b), Some(la), Some(lb)) => a.id == b.id || same_thread(la, lb),
            _ => false,
        };
        group.push((0..i).find(|&j| same(j)).map_or(i, |j| group[j]));
    }
    // The blocks of each group, in sew order.
    let mut uses: Vec<Vec<usize>> = vec![Vec::new(); blocks.len()];
    for (i, &g) in group.iter().enumerate() {
        uses[g].push(i);
    }
    let next_use = |g: usize, after: usize| uses[g].iter().copied().find(|&j| j > after);
    let pin = |i: usize| {
        let t = blocks[i]?;
        pins.iter()
            .find(|p| p.thread == t.id)
            .map(|p| usize::from(p.needle))
            .filter(|p| (1..=n).contains(p))
            .map(|p| p - 1)
    };
    // The groups that are pinned to each needle.
    let mut reserved: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, &g) in group.iter().enumerate() {
        if let Some(k) = pin(i)
            && !reserved[k].contains(&g)
        {
            reserved[k].push(g);
        }
    }
    let mut slot: Vec<Slot> = (0..n)
        .map(|k| match setup.thread(k as u8 + 1) {
            None => Slot::Empty,
            Some(l) => (0..blocks.len())
                .find(|&i| looks[i].as_ref().is_some_and(|li| same_thread(l, li)))
                .map_or(Slot::Foreign, |i| Slot::Holds(group[i])),
        })
        .collect();
    let mut used = vec![false; n];
    let mut plan = NeedlePlan {
        count: n as u8,
        start: vec![None; n],
        ..NeedlePlan::default()
    };
    for i in 0..blocks.len() {
        let g = group[i];
        let holds = |slot: &[Slot], k: usize| slot[k] == Slot::Holds(g);
        let free = |k: usize, pins_count: bool| !pins_count || reserved[k].iter().all(|&h| h == g);
        let needed_later = |s: Slot| match s {
            Slot::Holds(h) => next_use(h, i).is_some(),
            _ => false,
        };
        let choose = |slot: &[Slot], used: &[bool]| {
            for pins_count in [true, false] {
                let ok = |k: &usize| free(*k, pins_count);
                if let Some(k) = (0..n).filter(ok).find(|&k| !used[k] && slot[k] == Slot::Empty) {
                    return k;
                }
                if let Some(k) = (0..n).filter(ok).find(|&k| !used[k] && !needed_later(slot[k])) {
                    return k;
                }
                if let Some(k) = (0..n).filter(ok).find(|&k| used[k] && !needed_later(slot[k])) {
                    return k;
                }
                // The needle whose thread is needed again latest.
                let latest = (0..n).filter(ok).max_by_key(|&k| {
                    let at = match slot[k] {
                        Slot::Holds(h) => next_use(h, i).unwrap_or(usize::MAX),
                        _ => usize::MAX,
                    };
                    (at, std::cmp::Reverse(k))
                });
                if let Some(k) = latest {
                    return k;
                }
            }
            0
        };
        let k = pin(i)
            .or_else(|| (0..n).find(|&k| holds(&slot, k)))
            .unwrap_or_else(|| choose(&slot, &used));
        if !holds(&slot, k) && used[k] {
            plan.changes.push(i);
        }
        if !used[k] {
            plan.start[k] = Some(i);
            let on_machine = setup.thread(k as u8 + 1);
            let same = match (on_machine, &looks[i]) {
                (Some(a), Some(b)) => same_thread(a, b),
                _ => false,
            };
            if !same {
                plan.loads.push(k as u8 + 1);
            }
        }
        slot[k] = Slot::Holds(g);
        used[k] = true;
        plan.blocks.push(k as u8 + 1);
    }
    plan
}

impl NeedlePlan {
    /// The stitches with a stop after the color change of each thread
    /// change, so the machine is on the needle when it stops.
    pub fn with_stops(&self, s: &StitchDesign) -> StitchDesign {
        if self.changes.is_empty() {
            return s.clone();
        }
        let mut out = StitchDesign {
            threads: s.threads.clone(),
            records: Vec::with_capacity(s.records.len() + self.changes.len()),
        };
        let mut block = 0;
        for r in &s.records {
            out.records.push(*r);
            if r.kind == StitchKind::ColorChange {
                block += 1;
                if self.changes.contains(&block) {
                    out.records.push(StitchRecord::new(r.pos, StitchKind::Stop));
                }
            }
        }
        out
    }

    /// The needles in sew order, as text: "1, 2, 3, 1".
    pub fn sequence_text(&self) -> String {
        self.blocks.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")
    }

    /// The threads on the machine after the design: the thread of the last
    /// block of each needle that the design uses, and the other needles as
    /// they are in `setup`.
    pub fn end_setup(&self, blocks: &[Option<&Thread>], setup: &NeedleSetup) -> NeedleSetup {
        let mut threads: Vec<Option<ThreadLook>> = (1..=self.count).map(|k| setup.thread(k).cloned()).collect();
        for (i, &k) in self.blocks.iter().enumerate() {
            if let (Some(slot), Some(t)) = (threads.get_mut(usize::from(k) - 1), blocks.get(i).copied().flatten()) {
                *slot = Some(ThreadLook::of(t));
            }
        }
        NeedleSetup {
            count: setup.count,
            threads,
        }
    }
}

/// The thread of each color block of `s`. A block with no thread in the
/// list is `None`.
pub fn block_threads(s: &StitchDesign) -> Vec<Option<&Thread>> {
    (0..s.color_count()).map(|i| s.threads.get(i)).collect()
}

/// The needle plan of `design` with the stitches `s` for `machine`, or
/// `None` if the machine has one needle or nothing sews.
pub fn plan_for(
    design: &Design,
    s: &StitchDesign,
    machine: &MachineProfile,
    setup: Option<&NeedleSetup>,
) -> Option<NeedlePlan> {
    let count = NeedleSetup::count_for(setup, machine)?;
    if s.stitch_count() == 0 {
        return None;
    }
    let empty = NeedleSetup::default();
    Some(assign(
        &block_threads(s),
        count,
        setup.unwrap_or(&empty),
        &design.needles,
    ))
}

impl Design {
    /// The needle that the user gave `thread`, if any.
    pub fn needle_pin(&self, thread: ThreadId) -> Option<u8> {
        self.needles.iter().find(|p| p.thread == thread).map(|p| p.needle)
    }

    /// Pins the needles of a machine file (TBF or U01) to the threads of a
    /// design that [`crate::design_from_stitches`] made from it: thread `i`
    /// is color block `i`. A needle out of range is not pinned.
    pub fn pin_block_needles(&mut self, needles: &[u8]) {
        let pins: Vec<(ThreadId, u8)> = self.threads.iter().zip(needles).map(|(t, &n)| (t.id, n)).collect();
        for (t, n) in pins {
            self.set_needle_pin(t, Some(n));
        }
    }

    /// Gives `thread` the needle `needle`, or takes its pin away (`None`).
    pub fn set_needle_pin(&mut self, thread: ThreadId, needle: Option<u8>) {
        self.needles.retain(|p| p.thread != thread);
        if let Some(needle) = needle.filter(|n| NEEDLES_RANGE.contains(n)) {
            self.needles.push(NeedlePin { thread, needle });
            self.needles.sort_by_key(|p| p.thread);
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;

    use super::*;
    use crate::Rgb;

    fn thread(id: u32, color: Rgb) -> Thread {
        Thread {
            id: ThreadId(id),
            name: format!("T{id}"),
            color,
            brand: None,
            code: None,
        }
    }

    const RED: Rgb = Rgb(200, 0, 0);
    const BLUE: Rgb = Rgb(0, 0, 200);
    const GREEN: Rgb = Rgb(0, 150, 0);
    const GOLD: Rgb = Rgb(220, 180, 0);

    fn plan(ts: &[Thread], order: &[usize], count: u8, setup: &NeedleSetup, pins: &[NeedlePin]) -> NeedlePlan {
        let blocks: Vec<Option<&Thread>> = order.iter().map(|&i| Some(&ts[i])).collect();
        assign(&blocks, count, setup, pins)
    }

    #[test]
    fn a_thread_used_two_times_keeps_its_needle() {
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, GREEN)];
        let p = plan(&ts, &[0, 1, 0, 2], 15, &NeedleSetup::default(), &[]);
        assert_eq!(p.blocks, [1, 2, 1, 3]);
        assert!(p.changes.is_empty());
        assert_eq!(p.loads, [1, 2, 3], "an empty machine gets all three threads");
        assert_eq!(p.start[..3], [Some(0), Some(1), Some(3)]);
        assert_eq!(p.sequence_text(), "1, 2, 1, 3");
    }

    #[test]
    fn the_same_color_with_another_id_is_the_same_thread() {
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, RED)];
        assert_eq!(plan(&ts, &[0, 1, 2], 6, &NeedleSetup::default(), &[]).blocks, [1, 2, 1]);
        // Codes win over colors: two reds with different codes are two threads.
        let mut a = thread(1, RED);
        a.code = Some("1147".into());
        let mut b = thread(2, RED);
        b.code = Some("1037".into());
        assert_eq!(plan(&[a, b], &[0, 1], 6, &NeedleSetup::default(), &[]).blocks, [1, 2]);
    }

    #[test]
    fn threads_on_the_machine_stay_where_they_are() {
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, GREEN)];
        let setup = NeedleSetup {
            count: None,
            threads: vec![
                Some(ThreadLook::of(&thread(9, GOLD))),
                None,
                Some(ThreadLook::of(&thread(8, BLUE))),
            ],
        };
        let p = plan(&ts, &[0, 1, 2], 4, &setup, &[]);
        // Blue is on needle 3; red and green go on the empty needles 2 and 4;
        // the gold thread on needle 1 is not needed but stays.
        assert_eq!(p.blocks, [2, 3, 4]);
        assert_eq!(p.loads, [2, 4]);
        assert!(p.changes.is_empty());
        // With only three needles, needle 1 (gold, not needed) takes green.
        let p = plan(&ts, &[0, 1, 2], 3, &setup, &[]);
        assert_eq!(p.blocks, [2, 3, 1]);
        assert_eq!(p.loads, [2, 1]);
    }

    #[test]
    fn more_threads_than_needles_change_the_thread_needed_latest() {
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, GREEN), thread(4, GOLD)];
        // Two needles; red is needed again, blue is not.
        let p = plan(&ts, &[0, 1, 2, 0, 3], 2, &NeedleSetup::default(), &[]);
        assert_eq!(p.blocks, [1, 2, 2, 1, 1]);
        assert_eq!(p.changes, [2, 4], "green replaces blue, then gold replaces red");
        // Red, blue, green, blue, gold, red: at green both threads are needed
        // again, and red is needed latest, so green goes on needle 1.
        let p = plan(&ts, &[0, 1, 2, 1, 3, 0], 2, &NeedleSetup::default(), &[]);
        assert_eq!(p.blocks, [1, 2, 1, 2, 1, 1]);
        assert_eq!(p.changes, [2, 4, 5]);
    }

    #[test]
    fn pins_win_and_other_threads_keep_off_pinned_needles() {
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, GREEN)];
        let pins = [NeedlePin {
            thread: ThreadId(3),
            needle: 1,
        }];
        let p = plan(&ts, &[0, 1, 2], 6, &NeedleSetup::default(), &pins);
        assert_eq!(p.blocks, [2, 3, 1]);
        // A pin of a needle that the machine does not have is not used.
        let pins = [NeedlePin {
            thread: ThreadId(1),
            needle: 9,
        }];
        assert_eq!(
            plan(&ts, &[0, 1, 2], 6, &NeedleSetup::default(), &pins).blocks,
            [1, 2, 3]
        );
        // Two threads pinned to one needle: the second is a thread change.
        let pins = [
            NeedlePin {
                thread: ThreadId(1),
                needle: 2,
            },
            NeedlePin {
                thread: ThreadId(2),
                needle: 2,
            },
        ];
        let p = plan(&ts, &[0, 1, 2], 6, &NeedleSetup::default(), &pins);
        assert_eq!(p.blocks, [2, 2, 1]);
        assert_eq!(p.changes, [1]);
    }

    #[test]
    fn a_thread_on_the_machine_that_is_needed_later_is_not_taken() {
        // Needle 1 has green, which block 2 needs; red and blue go elsewhere.
        let ts = [thread(1, RED), thread(2, BLUE), thread(3, GREEN)];
        let setup = NeedleSetup {
            count: None,
            threads: vec![Some(ThreadLook::of(&thread(7, GREEN)))],
        };
        let p = plan(&ts, &[0, 1, 2], 3, &setup, &[]);
        assert_eq!(p.blocks, [2, 3, 1]);
        assert_eq!(p.loads, [2, 3]);
    }

    #[test]
    fn stops_come_after_the_color_change_of_a_thread_change() {
        let rec = |x: f64, k| StitchRecord::new(Point::new(x, 0.0), k);
        let s = StitchDesign {
            threads: vec![thread(1, RED), thread(2, BLUE), thread(3, GREEN)],
            records: vec![
                rec(0.0, StitchKind::Stitch),
                rec(1.0, StitchKind::ColorChange),
                rec(1.0, StitchKind::Stitch),
                rec(2.0, StitchKind::ColorChange),
                rec(2.0, StitchKind::Stitch),
                rec(2.0, StitchKind::End),
            ],
        };
        let blocks = block_threads(&s);
        let p = assign(&blocks, 2, &NeedleSetup::default(), &[]);
        assert_eq!(p.changes, [2]);
        let out = p.with_stops(&s);
        let kinds: Vec<_> = out.records.iter().map(|r| r.kind).collect();
        assert_eq!(kinds[3..5], [StitchKind::ColorChange, StitchKind::Stop]);
        assert_eq!(out.records.len(), s.records.len() + 1);
        // After the design, needle 1 has green and needle 2 blue.
        let end = p.end_setup(&blocks, &NeedleSetup::default());
        assert_eq!(end.thread(1).map(|t| t.color), Some(GREEN));
        assert_eq!(end.thread(2).map(|t| t.color), Some(BLUE));
    }

    #[test]
    fn single_needle_machines_have_no_plan() {
        let cat = crate::MachineCatalog::builtin();
        let topaz = cat.get("husqvarna_topaz20").unwrap();
        assert_eq!(NeedleSetup::count_for(None, topaz), None);
        let tajima = cat.get("tajima").unwrap();
        assert_eq!(NeedleSetup::count_for(None, tajima), Some(15));
        let six = NeedleSetup {
            count: Some(6),
            threads: vec![],
        };
        assert_eq!(NeedleSetup::count_for(Some(&six), tajima), Some(6));
        // A user can give a home machine profile more needles.
        assert_eq!(NeedleSetup::count_for(Some(&six), topaz), Some(6));
    }

    #[test]
    fn pins_in_the_design() {
        let mut d = Design::new("d");
        let t = d.add_thread("Red", RED);
        d.set_needle_pin(t, Some(4));
        assert_eq!(d.needle_pin(t), Some(4));
        d.set_needle_pin(t, Some(99));
        assert_eq!(d.needle_pin(t), None, "not a needle number");
        d.set_needle_pin(t, Some(2));
        d.set_needle_pin(t, None);
        assert!(d.needles.is_empty());
    }
}

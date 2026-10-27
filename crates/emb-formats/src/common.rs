//! Parts that the formats of Milestone 3 (EXP, JEF, XXX, SEW, HUS, SHV) share.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/EmbEncoder.py`: `interpolate_gap_stitches`, `jump_to`,
//!   `needle_to` (the long-move split, as in `dst` and `pes`)
//! - `pyembroidery/EmbPattern.py`: `add_stitch_relative`, `interpolate_trims`
//! - `pyembroidery/ReadHelper.py`: `signed8`, `signed16`
//!
//! The older modules (`dst`, `pes`, `vp3`) keep their own copies of the
//! other parts, because their byte checks against pyembroidery are done;
//! `dst` and `pes` use `gap_points`.

use emb_geom::Point;
use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

/// Formats in this crate use 0.1 mm.
pub(crate) const UNITS_PER_MM: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Low {
    Stitch,
    Jump,
    Trim,
    ColorChange,
    Stop,
    End,
    /// Speed changes (U01 and DAT write and read them; the other writers
    /// leave them out).
    Slow,
    Fast,
}

/// A record at an absolute position in file units (0.1 mm, Y down).
pub(crate) type Rec = (Low, i32, i32);

pub(crate) fn to_units(p: Point) -> (i32, i32) {
    ((p.x * UNITS_PER_MM).round() as i32, (p.y * UNITS_PER_MM).round() as i32)
}

pub(crate) fn signed8(b: u8) -> i32 {
    i32::from(b as i8)
}

pub(crate) fn signed16(v: u16) -> i32 {
    i32::from(v as i16)
}

/// Python `round` for a float: half to even.
fn round_half_even(x: f64) -> i32 {
    let r = x.round();
    if (r - x).abs() == 0.5 {
        (2.0 * (x / 2.0).round()) as i32
    } else {
        r as i32
    }
}

/// Adds jumps from `from` toward `to` so that the last move fits in one
/// record of at most `max` units per axis. As `interpolate_gap_stitches`:
/// the steps add up as floats, and the writer rounds each move half to even
/// (`int(round(x - xx))`), so the jump positions are the same as there.
fn gap_jumps(out: &mut Vec<Rec>, from: (i32, i32), to: (i32, i32), max: i32) {
    out.extend(gap_points(from, to, max).into_iter().map(|(x, y)| (Low::Jump, x, y)));
}

/// The positions of the jumps that `gap_jumps` adds. The `dst` and `pes`
/// writers use it too.
pub(crate) fn gap_points(from: (i32, i32), to: (i32, i32), max: i32) -> Vec<(i32, i32)> {
    let (dx, dy) = (f64::from(to.0 - from.0), f64::from(to.1 - from.1));
    let max = f64::from(max);
    if dx.abs() <= max && dy.abs() <= max {
        return Vec::new();
    }
    let steps = (dx / max).abs().ceil().max((dy / max).abs().ceil());
    let (sx, sy) = (dx / steps, dy / steps);
    let (mut qx, mut qy) = (f64::from(from.0), f64::from(from.1));
    let (mut wx, mut wy) = from;
    (1..steps as i32)
        .map(|_| {
            qx += sx;
            qy += sy;
            wx += round_half_even(qx - f64::from(wx));
            wy += round_half_even(qy - f64::from(wy));
            (wx, wy)
        })
        .collect()
}

/// The stitch list as records in file units. Moves longer than `max` units
/// get jumps before them. The list ends with one `End`.
///
/// As the encoder of pyembroidery: after a trim, a stop or a color change
/// (and at the start), the thread is "trimmed", and a format with
/// `full_jump` gets a jump to the next stitch before it.
pub(crate) fn encode(design: &StitchDesign, max: i32, full_jump: bool) -> Vec<Rec> {
    encode_with(design, max, full_jump, false)
}

/// As [`encode`]. With `start_change`, a `ColorChange` also comes before the
/// first stitch: the formats that set a needle for each color (TBF) set the
/// first needle there (`declare_not_trimmed` in `EmbEncoder.py`).
pub(crate) fn encode_with(design: &StitchDesign, max: i32, full_jump: bool, start_change: bool) -> Vec<Rec> {
    encode_all(design, max, full_jump, start_change, false)
}

/// As [`encode_with`]; with `speeds` the speed changes (`Slow`, `Fast`)
/// stay in the records (U01), else they are left out.
pub(crate) fn encode_all(
    design: &StitchDesign,
    max: i32,
    full_jump: bool,
    start_change: bool,
    speeds: bool,
) -> Vec<Rec> {
    let mut out: Vec<Rec> = Vec::with_capacity(design.records.len() + 16);
    let mut cur = (0, 0);
    let mut trimmed = true;
    let mut started = !start_change;
    for r in &design.records {
        let p = to_units(r.pos);
        match r.kind {
            StitchKind::Stitch => {
                if !started {
                    out.push((Low::ColorChange, cur.0, cur.1));
                    started = true;
                }
                let before = out.len();
                gap_jumps(&mut out, cur, p, max);
                // The needle is at the last gap jump, or still where it was.
                let needle = if out.len() > before {
                    out.last().map_or(cur, |l| (l.1, l.2))
                } else {
                    cur
                };
                if trimmed && full_jump && needle != p {
                    out.push((Low::Jump, p.0, p.1));
                }
                trimmed = false;
                out.push((Low::Stitch, p.0, p.1));
                cur = p;
            }
            StitchKind::Jump => {
                if p != cur {
                    gap_jumps(&mut out, cur, p, max);
                    out.push((Low::Jump, p.0, p.1));
                    cur = p;
                }
            }
            StitchKind::Trim => {
                trimmed = true;
                out.push((Low::Trim, cur.0, cur.1));
            }
            StitchKind::ColorChange => {
                trimmed = true;
                started = true;
                out.push((Low::ColorChange, cur.0, cur.1));
            }
            StitchKind::Stop => {
                trimmed = true;
                out.push((Low::Stop, cur.0, cur.1));
            }
            StitchKind::Slow if speeds => out.push((Low::Slow, cur.0, cur.1)),
            StitchKind::Fast if speeds => out.push((Low::Fast, cur.0, cur.1)),
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::End => break,
        }
    }
    out.push((Low::End, cur.0, cur.1));
    out
}

/// Builds records from relative moves, as `EmbPattern.add_stitch_relative`.
#[derive(Debug, Default)]
pub(crate) struct Pattern {
    pub recs: Vec<Rec>,
    x: i32,
    y: i32,
}

impl Pattern {
    pub fn add(&mut self, kind: Low, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
        self.recs.push((kind, self.x, self.y));
    }

    pub fn stitch(&mut self, dx: i32, dy: i32) {
        self.add(Low::Stitch, dx, dy);
    }

    pub fn jump(&mut self, dx: i32, dy: i32) {
        self.add(Low::Jump, dx, dy);
    }

    pub fn here(&mut self, kind: Low) {
        self.add(kind, 0, 0);
    }
}

/// Port of `EmbPattern.interpolate_trims(jumps, distance, clipping=True)`:
/// untrimmed jumps become a trim when there are `jumps` of them, or when
/// together they move more than `distance` units on one axis. Jumps that
/// move zero in total are removed.
pub(crate) fn interpolate_trims(mut s: Vec<Rec>, jumps: Option<usize>, distance: Option<i32>) -> Vec<Rec> {
    let (mut x, mut y) = (0, 0);
    let (mut jump_count, mut jump_start, mut jump_dx, mut jump_dy) = (0usize, 0usize, 0, 0);
    let mut jumping = false;
    let mut trimmed = true;
    let mut i = 0usize;
    while i < s.len() {
        let (cmd, sx, sy) = s[i];
        let (dx, dy) = (sx - x, sy - y);
        (x, y) = (sx, sy);
        match cmd {
            Low::Stitch => {
                trimmed = false;
                jumping = false;
            }
            Low::ColorChange | Low::Trim => {
                trimmed = true;
                jumping = false;
            }
            _ => {}
        }
        if cmd == Low::Jump {
            if !jumping {
                (jump_dx, jump_dy, jump_count, jump_start) = (0, 0, 0, i);
                jumping = true;
            }
            jump_count += 1;
            jump_dx += dx;
            jump_dy += dy;
            let far = distance.is_some_and(|d| jump_dx.abs() > d || jump_dy.abs() > d);
            if !trimmed && (Some(jump_count) == jumps || far) {
                // The trim goes where the jumps start (the needle position before them).
                let at = if jump_start > 0 {
                    (s[jump_start - 1].1, s[jump_start - 1].2)
                } else {
                    (0, 0)
                };
                s.insert(jump_start, (Low::Trim, at.0, at.1));
                jump_start += 1;
                i += 1;
                trimmed = true;
            }
            if jump_dx == 0 && jump_dy == 0 {
                // No net movement: remove the jumps (a trim before them stays).
                s.drain(jump_start..=i);
                i = jump_start;
                continue;
            }
        }
        i += 1;
    }
    s
}

/// The records as a stitch design in mm. The list ends with one `End`.
pub(crate) fn to_design(recs: Vec<Rec>, threads: Vec<Thread>) -> StitchDesign {
    let mm = |v: i32| f64::from(v) / UNITS_PER_MM;
    let mut records: Vec<StitchRecord> = recs
        .into_iter()
        .take_while(|r| r.0 != Low::End)
        .map(|(k, x, y)| {
            let kind = match k {
                Low::Stitch => StitchKind::Stitch,
                Low::Jump => StitchKind::Jump,
                Low::Trim => StitchKind::Trim,
                Low::ColorChange => StitchKind::ColorChange,
                Low::Stop => StitchKind::Stop,
                Low::End => StitchKind::End,
                Low::Slow => StitchKind::Slow,
                Low::Fast => StitchKind::Fast,
            };
            StitchRecord::new(Point::new(mm(x), mm(y)), kind)
        })
        .collect();
    let end = records.last().map(|r| r.pos).unwrap_or_default();
    records.push(StitchRecord::new(end, StitchKind::End));
    StitchDesign { threads, records }
}

/// A thread table entry: color, name and catalog code. `None` is an unused slot.
pub(crate) type TableThread = Option<(u8, u8, u8, &'static str, &'static str)>;

/// The thread of a table entry, with the brand name of the table.
pub(crate) fn table_thread(id: usize, t: TableThread, brand: &str) -> Thread {
    let (r, g, b, name, code) = t.unwrap_or((0, 0, 0, "Unknown", ""));
    Thread {
        id: ThreadId(id as u32),
        name: name.to_string(),
        color: Rgb(r, g, b),
        brand: Some(brand.to_string()),
        code: (!code.is_empty()).then(|| code.to_string()),
    }
}

/// A thread with only a color.
pub(crate) fn color_thread(id: usize, c: Rgb) -> Thread {
    Thread {
        id: ThreadId(id as u32),
        name: c.to_hex(),
        color: c,
        brand: None,
        code: None,
    }
}

/// Reads little- and big-endian integers. Reads past the end give `None`.
pub(crate) struct Bytes<'a> {
    pub b: &'a [u8],
    pub at: usize,
}

impl<'a> Bytes<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Bytes { b, at: 0 }
    }

    pub fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.at)?;
        self.at += 1;
        Some(v)
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let s = self.b.get(self.at..self.at + N)?;
        self.at += N;
        s.try_into().ok()
    }

    pub fn u16le(&mut self) -> Option<u16> {
        self.take::<2>().map(u16::from_le_bytes)
    }

    pub fn u16be(&mut self) -> Option<u16> {
        self.take::<2>().map(u16::from_be_bytes)
    }

    pub fn i32le(&mut self) -> Option<i32> {
        self.take::<4>().map(i32::from_le_bytes)
    }

    pub fn u32be(&mut self) -> Option<u32> {
        self.take::<4>().map(u32::from_be_bytes)
    }

    pub fn skip(&mut self, n: usize) {
        self.at += n;
    }

    pub fn seek(&mut self, at: usize) {
        self.at = at;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    /// The design that the stitch plan makes: jump, stitches, trim, a long
    /// jump, color change, end. Two threads (red, blue).
    pub fn sample() -> StitchDesign {
        StitchDesign {
            threads: vec![color_thread(0, Rgb(200, 30, 40)), color_thread(1, Rgb(10, 20, 250))],
            records: vec![
                rec(-5.0, -5.0, StitchKind::Jump),
                rec(-5.0, -5.0, StitchKind::Stitch),
                rec(-2.5, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Trim),
                rec(40.0, 30.0, StitchKind::Jump),
                rec(40.0, 30.0, StitchKind::Stitch),
                rec(42.0, 30.0, StitchKind::Stitch),
                rec(42.0, 30.0, StitchKind::Trim),
                rec(42.0, 30.0, StitchKind::ColorChange),
                rec(10.0, 10.0, StitchKind::Jump),
                rec(10.0, 10.0, StitchKind::Stitch),
                rec(10.0, 12.5, StitchKind::Stitch),
                rec(10.0, 12.5, StitchKind::Trim),
                rec(10.0, 12.5, StitchKind::End),
            ],
        }
    }

    #[test]
    fn long_moves_get_jumps() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![
                StitchRecord::new(Point::new(0.0, 0.0), StitchKind::Stitch),
                StitchRecord::new(Point::new(30.0, 0.0), StitchKind::Stitch),
                StitchRecord::new(Point::new(30.0, 0.0), StitchKind::End),
            ],
        };
        let recs = encode(&d, 127, false);
        let kinds: Vec<Low> = recs.iter().map(|r| r.0).collect();
        assert_eq!(
            kinds,
            [Low::Stitch, Low::Jump, Low::Jump, Low::Stitch, Low::End],
            "300 units in three moves of 100"
        );
    }

    #[test]
    fn trims_from_jumps_by_count_and_distance() {
        let recs = vec![
            (Low::Stitch, 0, 0),
            (Low::Stitch, 10, 0),
            (Low::Jump, 30, 0),
            (Low::Jump, 60, 0),
            (Low::Stitch, 60, 0),
        ];
        let by_distance = interpolate_trims(recs.clone(), None, Some(30));
        assert_eq!(by_distance[2], (Low::Trim, 10, 0), "50 units > 30");
        let none = interpolate_trims(recs.clone(), None, Some(60));
        assert!(none.iter().all(|r| r.0 != Low::Trim));
        let by_count = interpolate_trims(recs, Some(2), None);
        assert_eq!(by_count[2].0, Low::Trim);
        // Jumps that go nowhere are removed.
        let zero = interpolate_trims(
            vec![
                (Low::Stitch, 5, 5),
                (Low::Jump, 9, 5),
                (Low::Jump, 5, 5),
                (Low::Stitch, 6, 5),
            ],
            None,
            None,
        );
        assert_eq!(zero, [(Low::Stitch, 5, 5), (Low::Stitch, 6, 5)]);
    }

    #[test]
    fn python_rounding() {
        assert_eq!(
            [2.5, 3.5, -2.5, 2.4999, -0.5, 0.5].map(round_half_even),
            [2, 4, -2, 2, 0, 0]
        );
    }

    #[test]
    fn bytes_reader() {
        let mut b = Bytes::new(&[1, 2, 3, 4, 5]);
        assert_eq!(b.u16le(), Some(0x0201));
        assert_eq!(b.u16be(), Some(0x0304));
        assert_eq!(b.u16le(), None);
        assert_eq!(signed8(0xff), -1);
        assert_eq!(signed16(0x8000), -32768);
    }
}

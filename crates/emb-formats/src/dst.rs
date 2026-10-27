//! Tajima DST read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/DstWriter.py`: `encode_record` (lines 18-104), `write` (107-185)
//! - `pyembroidery/DstReader.py`: `decode_dx`, `decode_dy` (5-32), `dst_read_stitches` (67-100)
//! - `pyembroidery/EmbEncoder.py`: `interpolate_gap_stitches` (613-645), the long-move split
//! - `pyembroidery/EmbPattern.py`: `bounds` (325-342), `interpolate_trims` (741-)
//!
//! Format summary: a 512-byte text header, then 3-byte records. Units are
//! 0.1 mm, the Y axis points up in the file, and one record moves at most
//! ±121 units per axis. A trim is three jump records that net zero
//! movement. Color change and end records have no movement. DST has no
//! thread colors.
//!
//! DST has no trim code. A run of three or more jump records after stitching
//! reads back as a trim, so a long jump while sewing (more than 24.2 mm) also
//! reads back with a trim. The stitch plan always trims before a long jump.
//!
//! Deviations from pyembroidery, on purpose:
//! - The label (`LA`) is cut to 16 ASCII characters, so the fixed header
//!   layout holds for long or non-ASCII names.
//! - Coordinates are rounded half away from zero (Rust `f64::round`);
//!   Python `round` rounds half to even. This changes a value only when a
//!   coordinate is exactly on 0.05 mm.

use emb_geom::Point;
use emb_model::{StitchDesign, StitchKind, StitchRecord};

pub const UNITS_PER_MM: f64 = 10.0;
/// Largest move of one record, per axis, in 0.1 mm (`MAX_STITCH_DISTANCE`, `MAX_JUMP_DISTANCE`).
pub const MAX_DELTA: i32 = 121;
const HEADER_SIZE: usize = 512;
/// Jumps that make a trim (`trim_at` default).
const TRIM_JUMPS: usize = 3;

/// How DST and TAP files write a trim (P10 Milestone 6): the number of jump
/// records and the move of each jump in file units (0.1 mm, even). The jumps
/// go out and back, so the needle is at the same place after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimCode {
    pub jumps: usize,
    pub step: i32,
}

impl Default for TrimCode {
    fn default() -> Self {
        TrimCode {
            jumps: TRIM_JUMPS,
            step: 4,
        }
    }
}

impl TrimCode {
    /// The trim code of a machine profile (its values, else the default).
    pub fn for_machine(m: &emb_model::MachineProfile) -> TrimCode {
        let d = TrimCode::default();
        TrimCode {
            jumps: m.trim_jumps.map_or(d.jumps, |n| n.clamp(1, 9) as usize),
            step: m
                .trim_jump_mm
                .map_or(d.step, |v| (((v * UNITS_PER_MM) / 2.0).round() as i32).clamp(1, 60) * 2),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DstError {
    #[error("file is shorter than the 512-byte DST header")]
    TooShort,
}

/// A DST file that was read.
#[derive(Debug, Clone, PartialEq)]
pub struct DstFile {
    pub label: String,
    /// In mm, Y down. DST has no thread colors, so `threads` is empty.
    pub design: StitchDesign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Low {
    Stitch,
    Jump,
    Trim,
    ColorChange,
    Stop,
    End,
}

fn bit(b: u32) -> u8 {
    1 << b
}

/// One 3-byte record. `x`, `y` are the move in file units with Y down
/// (the function flips Y). Port of `encode_record`.
fn encode_record(x: i32, y: i32, kind: Low) -> [u8; 3] {
    let (mut x, mut y) = (x, -y);
    let (mut b0, mut b1, mut b2) = (0u8, 0u8, 0u8);
    match kind {
        Low::Stitch | Low::Jump => {
            if kind == Low::Jump {
                b2 |= bit(7);
            }
            b2 |= bit(0) | bit(1);
            // (limit, value, byte, bit for +, bit for -) in pyembroidery order.
            let steps_x: [(i32, i32, usize, u32, u32); 5] = [
                (40, 81, 2, 2, 3),
                (13, 27, 1, 2, 3),
                (4, 9, 0, 2, 3),
                (1, 3, 1, 0, 1),
                (0, 1, 0, 0, 1),
            ];
            let steps_y: [(i32, i32, usize, u32, u32); 5] = [
                (40, 81, 2, 5, 4),
                (13, 27, 1, 5, 4),
                (4, 9, 0, 5, 4),
                (1, 3, 1, 7, 6),
                (0, 1, 0, 7, 6),
            ];
            let mut bytes = [b0, b1, b2];
            for (v, steps) in [(&mut x, steps_x), (&mut y, steps_y)] {
                for (limit, value, byte, plus, minus) in steps {
                    if *v > limit {
                        bytes[byte] |= bit(plus);
                        *v -= value;
                    }
                    if *v < -limit {
                        bytes[byte] |= bit(minus);
                        *v += value;
                    }
                }
            }
            debug_assert!(x == 0 && y == 0, "move exceeds ±{MAX_DELTA} units");
            [b0, b1, b2] = bytes;
        }
        // Color change and stop have the same record in DST.
        Low::ColorChange | Low::Stop => b2 = 0b1100_0011,
        Low::End => b2 = 0b1111_0011,
        Low::Trim => unreachable!("a trim is written as jumps"),
    }
    [b0, b1, b2]
}

fn getbit(b: u8, pos: u32) -> i32 {
    i32::from((b >> pos) & 1)
}

/// Port of `decode_dx`.
fn decode_dx(b0: u8, b1: u8, b2: u8) -> i32 {
    getbit(b2, 2) * 81 - getbit(b2, 3) * 81 + getbit(b1, 2) * 27 - getbit(b1, 3) * 27 + getbit(b0, 2) * 9
        - getbit(b0, 3) * 9
        + getbit(b1, 0) * 3
        - getbit(b1, 1) * 3
        + getbit(b0, 0)
        - getbit(b0, 1)
}

/// Port of `decode_dy`. Returns Y down.
fn decode_dy(b0: u8, b1: u8, b2: u8) -> i32 {
    let y = getbit(b2, 5) * 81 - getbit(b2, 4) * 81 + getbit(b1, 5) * 27 - getbit(b1, 4) * 27 + getbit(b0, 5) * 9
        - getbit(b0, 4) * 9
        + getbit(b1, 7) * 3
        - getbit(b1, 6) * 3
        + getbit(b0, 7)
        - getbit(b0, 6);
    -y
}

fn to_units(p: Point) -> (i32, i32) {
    ((p.x * UNITS_PER_MM).round() as i32, (p.y * UNITS_PER_MM).round() as i32)
}

/// Adds jumps from `from` toward `to` so that the last step to `to` fits in
/// one record (`interpolate_gap_stitches`; see `common::gap_points`).
fn gap_jumps(out: &mut Vec<(Low, i32, i32)>, from: (i32, i32), to: (i32, i32)) {
    out.extend(
        crate::common::gap_points(from, to, MAX_DELTA)
            .into_iter()
            .map(|(x, y)| (Low::Jump, x, y)),
    );
}

/// Converts the stitch list to low-level records at absolute positions in file units.
fn encode(design: &StitchDesign) -> Vec<(Low, i32, i32)> {
    let mut out: Vec<(Low, i32, i32)> = Vec::with_capacity(design.records.len() + 16);
    let mut cur = (0, 0);
    for r in &design.records {
        let p = to_units(r.pos);
        match r.kind {
            StitchKind::Stitch => {
                gap_jumps(&mut out, cur, p);
                out.push((Low::Stitch, p.0, p.1));
                cur = p;
            }
            StitchKind::Jump => {
                if p != cur {
                    gap_jumps(&mut out, cur, p);
                    out.push((Low::Jump, p.0, p.1));
                    cur = p;
                }
            }
            StitchKind::Trim => out.push((Low::Trim, cur.0, cur.1)),
            StitchKind::ColorChange => out.push((Low::ColorChange, cur.0, cur.1)),
            StitchKind::Stop => out.push((Low::Stop, cur.0, cur.1)),
            // The format has no speed function.
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::End => break,
        }
    }
    out.push((Low::End, cur.0, cur.1));
    out
}

/// Header label: up to 16 ASCII characters.
pub(crate) fn label_text(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '_' })
        .take(16)
        .collect()
}

/// Writes a DST file. Port of `DstWriter.write` with default settings.
pub fn write(design: &StitchDesign, label: &str) -> Vec<u8> {
    write_with(design, label, TrimCode::default())
}

/// As [`write()`], with the trim code of the machine.
pub fn write_with(design: &StitchDesign, label: &str, trim: TrimCode) -> Vec<u8> {
    let low = encode(design);
    let colors = low.iter().filter(|(k, _, _)| *k == Low::ColorChange).count();
    // Bounds of all records, as `EmbPattern.bounds` (Y down, as pyembroidery writes it).
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (0, 0, 0, 0);
    if let Some(&(_, x, y)) = low.first() {
        (min_x, min_y, max_x, max_y) = (x, y, x, y);
    }
    for &(_, x, y) in &low {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let (ax, ay) = low.last().map(|&(_, x, y)| (x, -y)).unwrap_or((0, 0));
    let signed = |v: i32| {
        if v >= 0 {
            format!("+{:5}", v)
        } else {
            format!("-{:5}", v.abs())
        }
    };

    let mut header = String::new();
    header.push_str(&format!("LA:{:<16}\r", label_text(label)));
    header.push_str(&format!("ST:{:7}\r", low.len()));
    header.push_str(&format!("CO:{:3}\r", colors));
    header.push_str(&format!("+X:{:5}\r", max_x.abs()));
    header.push_str(&format!("-X:{:5}\r", min_x.abs()));
    header.push_str(&format!("+Y:{:5}\r", max_y.abs()));
    header.push_str(&format!("-Y:{:5}\r", min_y.abs()));
    header.push_str(&format!("AX:{}\r", signed(ax)));
    header.push_str(&format!("AY:{}\r", signed(ay)));
    header.push_str(&format!("MX:+{:5}\r", 0));
    header.push_str(&format!("MY:+{:5}\r", 0));
    header.push_str(&format!("PD:{:>6}\r", "******"));

    let mut bytes = header.into_bytes();
    bytes.push(0x1a);
    bytes.resize(HEADER_SIZE.max(bytes.len()), b' ');
    write_records(&low, &mut bytes, trim);
    bytes
}

/// The records of a DST file with no header. Happy TAP files are this.
pub(crate) fn body(design: &StitchDesign, trim: TrimCode) -> Vec<u8> {
    let mut bytes = Vec::new();
    write_records(&encode(design), &mut bytes, trim);
    bytes
}

fn write_records(low: &[(Low, i32, i32)], bytes: &mut Vec<u8>, trim: TrimCode) {
    let (mut xx, mut yy) = (0, 0);
    for &(kind, x, y) in low {
        let (dx, dy) = (x - xx, y - yy);
        xx += dx;
        yy += dy;
        if kind == Low::Trim {
            // Jumps out and back (by default three: half out, full back,
            // half out again). Net movement zero.
            if trim.jumps <= 1 {
                bytes.extend(encode_record(0, 0, Low::Jump));
                continue;
            }
            let mut delta = -trim.step;
            bytes.extend(encode_record(-delta / 2, -delta / 2, Low::Jump));
            for _ in 1..trim.jumps - 1 {
                bytes.extend(encode_record(delta, delta, Low::Jump));
                delta = -delta;
            }
            bytes.extend(encode_record(delta / 2, delta / 2, Low::Jump));
        } else {
            bytes.extend(encode_record(dx, dy, kind));
        }
    }
}

/// Reads a DST file. Port of `DstReader.read` with default settings: a run
/// of three jumps after stitching is a trim, and jump runs with no net
/// movement are removed.
pub fn read(bytes: &[u8]) -> Result<DstFile, DstError> {
    if bytes.len() < HEADER_SIZE {
        return Err(DstError::TooShort);
    }
    let mut label = String::new();
    for line in bytes[..HEADER_SIZE].split(|b| *b == b'\r' || *b == b'\n') {
        if let Ok(s) = std::str::from_utf8(line)
            && let Some(rest) = s.trim().strip_prefix("LA:")
        {
            label = rest.trim().to_string();
        }
    }

    Ok(DstFile {
        label,
        design: read_body(&bytes[HEADER_SIZE..]),
    })
}

/// Reads an EXY file (Eltac): the DST records after a 0x100-byte header
/// (`pyembroidery/ExyReader.py`).
pub fn read_exy(bytes: &[u8]) -> Result<StitchDesign, DstError> {
    let body = bytes.get(0x100..).ok_or(DstError::TooShort)?;
    Ok(read_body(body))
}

/// Reads DST records with no header (a Happy TAP file). Port of
/// `dst_read_stitches` with default settings.
pub(crate) fn read_body(bytes: &[u8]) -> StitchDesign {
    let mut recs: Vec<(Low, i32, i32)> = Vec::new();
    let (mut x, mut y) = (0, 0);
    for c in bytes.as_chunks::<3>().0 {
        let (b0, b1, b2) = (c[0], c[1], c[2]);
        x += decode_dx(b0, b1, b2);
        y += decode_dy(b0, b1, b2);
        if b2 & 0b1111_0011 == 0b1111_0011 {
            break;
        } else if b2 & 0b1100_0011 == 0b1100_0011 {
            recs.push((Low::ColorChange, x, y));
        } else if b2 & 0b0100_0011 == 0b0100_0011 {
            // Sequin mode: not used for embroidery here. The move still counts.
            continue;
        } else if b2 & 0b1000_0011 == 0b1000_0011 {
            recs.push((Low::Jump, x, y));
        } else {
            recs.push((Low::Stitch, x, y));
        }
    }
    let recs = interpolate_trims(recs);

    let mm = |v: i32| f64::from(v) / UNITS_PER_MM;
    let mut records: Vec<StitchRecord> = recs
        .into_iter()
        .map(|(k, x, y)| {
            let kind = match k {
                Low::Stitch => StitchKind::Stitch,
                Low::Jump => StitchKind::Jump,
                Low::Trim => StitchKind::Trim,
                Low::ColorChange => StitchKind::ColorChange,
                Low::Stop => StitchKind::Stop,
                Low::End => StitchKind::End,
            };
            StitchRecord::new(Point::new(mm(x), mm(y)), kind)
        })
        .collect();
    let end = records.last().map(|r| r.pos).unwrap_or_default();
    records.push(StitchRecord::new(end, StitchKind::End));
    StitchDesign {
        threads: Vec::new(),
        records,
    }
}

/// Port of `EmbPattern.interpolate_trims(3, None, clipping=True)`.
fn interpolate_trims(mut s: Vec<(Low, i32, i32)>) -> Vec<(Low, i32, i32)> {
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
            if !trimmed && jump_count == TRIM_JUMPS {
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
                // No net movement: remove the jumps (the trim, if any, stays).
                s.drain(jump_start..=i);
                i = jump_start;
                jumping = false;
                continue;
            }
        }
        i += 1;
    }
    s
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    #[test]
    fn record_encoding_matches_pyembroidery_examples() {
        // +0.1 mm in X: bit 0 of byte 0.
        assert_eq!(encode_record(1, 0, Low::Stitch), [0x01, 0x00, 0x03]);
        // 0.1 mm down (Y down) is -1 in the file: bit 6 of byte 0.
        assert_eq!(encode_record(0, 1, Low::Stitch), [0x40, 0x00, 0x03]);
        // +12.1 mm jump: 81 + 27 + 9 + 3 + 1.
        assert_eq!(encode_record(121, 0, Low::Jump), [0x05, 0x05, 0x87]);
        assert_eq!(encode_record(0, 0, Low::ColorChange), [0, 0, 0xc3]);
        assert_eq!(encode_record(0, 0, Low::End), [0, 0, 0xf3]);
    }

    #[test]
    fn every_move_decodes_back() {
        for dx in -MAX_DELTA..=MAX_DELTA {
            for dy in -MAX_DELTA..=MAX_DELTA {
                let [b0, b1, b2] = encode_record(dx, dy, Low::Stitch);
                assert_eq!((decode_dx(b0, b1, b2), decode_dy(b0, b1, b2)), (dx, dy));
                assert_eq!(b2 & 0b1000_0011, 0b0000_0011, "a stitch is not a jump");
            }
        }
    }

    #[test]
    fn the_trim_code_of_the_machine() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![
                rec(0.0, 0.0, StitchKind::Stitch),
                rec(5.0, 0.0, StitchKind::Stitch),
                rec(5.0, 0.0, StitchKind::Trim),
                rec(20.0, 0.0, StitchKind::Jump),
                rec(20.0, 0.0, StitchKind::Stitch),
                rec(25.0, 0.0, StitchKind::Stitch),
                rec(25.0, 0.0, StitchKind::End),
            ],
        };
        // The default is the trim of pyembroidery (three jumps of 0.4 mm).
        assert_eq!(write_with(&d, "t", TrimCode::default()), write(&d, "t"));
        let five = TrimCode { jumps: 5, step: 10 };
        let b = write_with(&d, "t", five);
        assert_eq!(b.len(), write(&d, "t").len() + 2 * 3, "two more jumps");
        // It reads back with the same stitches and a trim.
        let back = read(&b).unwrap().design;
        let stitches = |s: &StitchDesign| {
            s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect::<Vec<_>>()
        };
        assert_eq!(stitches(&back), stitches(&d));
        assert_eq!(back.records.iter().filter(|r| r.kind == StitchKind::Trim).count(), 1);
        // From a machine profile.
        let mut m = emb_model::MachineCatalog::builtin().get("generic").unwrap().clone();
        assert_eq!(TrimCode::for_machine(&m), TrimCode::default());
        m.trim_jumps = Some(4);
        m.trim_jump_mm = Some(1.0);
        assert_eq!(TrimCode::for_machine(&m), TrimCode { jumps: 4, step: 10 });
    }

    #[test]
    fn header_layout() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![
                rec(1.0, 0.0, StitchKind::Jump),
                rec(1.0, 0.0, StitchKind::Stitch),
                rec(2.0, -1.0, StitchKind::Stitch),
                rec(2.0, -1.0, StitchKind::ColorChange),
                rec(3.0, 2.0, StitchKind::Stitch),
                rec(3.0, 2.0, StitchKind::End),
            ],
        };
        let b = write(&d, "A very long design name, with é");
        assert_eq!(b.len(), 512 + 3 * 6);
        let h = std::str::from_utf8(&b[..512]).unwrap();
        assert!(h.starts_with("LA:A very long desi\r"), "{h}");
        assert!(h.contains("ST:      6\r"));
        assert!(h.contains("CO:  1\r"));
        // Bounds of the records (not of the origin): x 1..3 mm, y -1..2 mm.
        assert!(h.contains("+X:   30\r-X:   10\r+Y:   20\r-Y:   10\r"));
        assert!(h.contains("AX:+   30\rAY:-   20\r"));
        assert!(h.contains("PD:******\r\x1a"));
        assert_eq!(read(&b).unwrap().label, "A very long desi");
    }

    #[test]
    fn round_trip_with_long_jump_trim_and_color_change() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![
                rec(-5.0, -5.0, StitchKind::Jump),
                rec(-5.0, -5.0, StitchKind::Stitch),
                rec(-2.5, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Trim),
                rec(40.0, 30.0, StitchKind::Jump), // 50 mm: needs several records
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
        };
        let back = read(&write(&d, "t")).unwrap().design;
        let stitches = |d: &StitchDesign| -> Vec<Point> {
            d.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect()
        };
        let (a, b) = (stitches(&d), stitches(&back));
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(&b) {
            assert!(p.distance(*q) < 0.051, "{p:?} vs {q:?}");
        }
        assert_eq!(back.trim_count(), d.trim_count());
        assert_eq!(back.color_count(), d.color_count());
        assert_eq!(back.records.last().map(|r| r.kind), Some(StitchKind::End));
    }

    #[test]
    fn short_file_is_an_error() {
        assert_eq!(read(&[0u8; 100]), Err(DstError::TooShort));
    }

    proptest! {
        #[test]
        fn random_walk_round_trip(steps in prop::collection::vec((-300i32..300, -300i32..300, 0u8..10), 1..60)) {
            // Positions on the 0.1 mm grid, so the round trip is exact.
            let mut records = Vec::new();
            let (mut x, mut y) = (0i32, 0i32);
            records.push(rec(0.0, 0.0, StitchKind::Stitch));
            for (dx, dy, k) in steps {
                x += dx;
                y += dy;
                let p = (f64::from(x) / 10.0, f64::from(y) / 10.0);
                match k {
                    // A jump while sewing must fit in two records: three or more
                    // jump records read back as a trim (see the module comment).
                    0 if dx.abs() <= MAX_DELTA && dy.abs() <= MAX_DELTA => {
                        records.push(rec(p.0, p.1, StitchKind::Jump))
                    }
                    1 => {
                        let last = records.last().unwrap().pos;
                        records.push(StitchRecord::new(last, StitchKind::Trim));
                        records.push(rec(p.0, p.1, StitchKind::Jump));
                    }
                    _ => {}
                }
                records.push(rec(p.0, p.1, StitchKind::Stitch));
            }
            let d = StitchDesign { threads: vec![], records };
            let back = read(&write(&d, "p")).unwrap().design;
            let s = |d: &StitchDesign| -> Vec<(i64, i64)> {
                d.records.iter().filter(|r| r.kind == StitchKind::Stitch)
                    .map(|r| ((r.pos.x * 10.0).round() as i64, (r.pos.y * 10.0).round() as i64)).collect()
            };
            prop_assert_eq!(s(&d), s(&back));
            prop_assert_eq!(back.trim_count(), d.trim_count());
        }
    }
}

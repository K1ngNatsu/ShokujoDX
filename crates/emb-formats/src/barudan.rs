//! Barudan U01 read and write, and DAT read (Barudan or Sunstar). U01 and
//! the Barudan DAT keep the speed functions (`Slow`, `Fast`).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/U01Writer.py`: `write` (lines 12-89; `MAX_JUMP_DISTANCE`
//!   and `MAX_STITCH_DISTANCE` 127, `FULL_JUMP = False`,
//!   `THREAD_CHANGE_COMMAND = NEEDLE_SET`, `WRITES_SPEEDS`)
//! - `pyembroidery/U01Reader.py`: `read_u01_stitches` (4-83), `read` (86-89)
//! - `pyembroidery/DatReader.py`: `read_barudan_dat` (4-87),
//!   `read_sunstar_dat_stitches` (90-125), `read` (135-138)
//! - `pyembroidery/EmbEncoder.py`: the needle of each color, as in `tbf`
//!
//! Format summary (U01): a 0x100-byte header (0x80 bytes of "0", then the
//! extents, the record count and the last position), then 3-byte records
//! `command, dy, dx` in 0.1 mm. Bit 7 of the command is always on; bit 6
//! is Y down, bit 5 is X left. The low 5 bits: 0 stitch, 1 jump, 2 and 3
//! fast (stitch, jump), 4 and 5 slow (stitch, jump), 6 and 7 trim, 8 stop,
//! 9 to 0x17 needle 1 to 15, 0x18 end. A needle set comes before the first
//! stitch and at each color change. The needles come from the needle plan
//! of the machine (ADR 0063); with no plan, color 1 is needle 1, to needle
//! 5, then needle 1 again (pyembroidery). The file ends with `F8 00 00`.
//!
//! DAT: Barudan DAT files have the U01 records from the start of the file;
//! Sunstar DAT files have other 3-byte records from 0x100.
//!
//! Deviations from pyembroidery, on purpose:
//! - A DAT file that is not a Barudan file is read again from the start as
//!   a Sunstar file. pyembroidery keeps the records that it read before.

use emb_model::StitchDesign;

use crate::common::{Low, Pattern, encode_all, to_design};

/// The largest move of one record, in 0.1 mm.
pub const MAX_DELTA: i32 = 127;
const HEADER_SIZE: usize = 0x100;
/// pyembroidery's default `needle_count`.
const NEEDLES: usize = 5;

fn needle(i: usize) -> u8 {
    (i % NEEDLES + 1) as u8
}

/// Writes a U01 file with pyembroidery's needles (1 to 5, then 1 again).
pub fn write(design: &StitchDesign) -> Vec<u8> {
    write_with(design, &[])
}

/// Writes a U01 file. `needles` has the needle of each color block (1 to
/// 15, ADR 0063); a block with no needle in the list gets pyembroidery's
/// needle.
pub fn write_with(design: &StitchDesign, needles: &[u8]) -> Vec<u8> {
    let low = encode_all(design, MAX_DELTA, false, true, true);
    let mut b = vec![b'0'; 0x80];
    if low.is_empty() {
        return b;
    }
    // Bounds of all records, as `EmbPattern.bounds`.
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (low[0].1, low[0].2, low[0].1, low[0].2);
    for &(_, x, y) in &low {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let i16le = |b: &mut Vec<u8>, v: i32| b.extend((v as i16).to_le_bytes());
    i16le(&mut b, min_x);
    i16le(&mut b, -max_y);
    i16le(&mut b, max_x);
    i16le(&mut b, -min_y);
    b.extend(0u32.to_le_bytes());
    b.extend(((low.len() + 1) as u32).to_le_bytes());
    let (lx, ly) = low.last().map_or((0, 0), |r| (r.1, r.2));
    i16le(&mut b, lx);
    i16le(&mut b, -ly);
    b.resize(HEADER_SIZE, 0);

    let (mut xx, mut yy) = (0, 0);
    let (mut fast, mut slow) = (false, false);
    let mut sets = 0usize;
    for (kind, x, y) in low {
        let (dx, dy) = (x - xx, y - yy);
        (xx, yy) = (x, y);
        let mut cmd: u8 = 0x80;
        if dy >= 0 {
            cmd |= 0x40;
        }
        if dx <= 0 {
            cmd |= 0x20;
        }
        let speed = |cmd: &mut u8, fast: &mut bool, slow: &mut bool| {
            if std::mem::take(fast) {
                *cmd |= 0x02;
            }
            if std::mem::take(slow) {
                *cmd |= 0x04;
            }
        };
        match kind {
            Low::Slow => {
                slow = true;
                continue;
            }
            Low::Fast => {
                fast = true;
                continue;
            }
            Low::Stitch => speed(&mut cmd, &mut fast, &mut slow),
            // With fast and slow both on, a jump is a trim (as pyembroidery).
            Low::Jump => {
                speed(&mut cmd, &mut fast, &mut slow);
                cmd |= 0x01;
            }
            Low::Stop => cmd |= 0x08,
            Low::Trim => cmd |= 0x07,
            Low::ColorChange => {
                cmd |= 0x08;
                cmd += needles.get(sets).copied().unwrap_or_else(|| needle(sets)).clamp(1, 15);
                sets += 1;
            }
            Low::End => break,
        }
        b.extend([cmd, dy.unsigned_abs() as u8, dx.unsigned_abs() as u8]);
    }
    b.extend([0xf8, 0x00, 0x00]);
    b
}

/// The U01 (and Barudan DAT) records from `bytes`, and the needle of each
/// color block. `None` if a record has no bit 7 (`strict`: then it is not
/// a Barudan file).
fn read_records(bytes: &[u8], strict: bool) -> Option<(StitchDesign, Vec<u8>)> {
    let mut p = Pattern::default();
    let mut stitched = false;
    let mut needles = Vec::new();
    for c in bytes.as_chunks::<3>().0 {
        let ctrl = c[0];
        if strict && ctrl & 0x80 == 0 {
            return None;
        }
        let (mut dy, mut dx) = (-i32::from(c[1]), i32::from(c[2]));
        if ctrl & 0x20 != 0 {
            dx = -dx;
        }
        if ctrl & 0x40 != 0 {
            dy = -dy;
        }
        let moved = dx != 0 || dy != 0;
        let then_move = |p: &mut Pattern| {
            if moved {
                p.jump(dx, dy);
            }
        };
        match ctrl & 0x1f {
            0x00 => {
                p.stitch(dx, dy);
                stitched = true;
            }
            0x01 => p.jump(dx, dy),
            c @ (0x02 | 0x04) => {
                p.here(if c == 0x02 { Low::Fast } else { Low::Slow });
                if moved {
                    p.stitch(dx, dy);
                    stitched = true;
                }
            }
            c @ (0x03 | 0x05) => {
                p.here(if c == 0x03 { Low::Fast } else { Low::Slow });
                then_move(&mut p);
            }
            0x06 | 0x07 => {
                p.here(Low::Trim);
                then_move(&mut p);
            }
            0x08 => {
                p.here(Low::Stop);
                then_move(&mut p);
            }
            c @ 0x09..=0x17 => {
                if stitched {
                    p.here(Low::ColorChange);
                    needles.push(c - 8);
                } else {
                    // A needle set before the first stitch: the needle of block 1.
                    needles.clear();
                    needles.push(c - 8);
                }
                then_move(&mut p);
            }
            _ => break,
        }
    }
    Some((to_design(p.recs, Vec::new()), needles))
}

/// Reads a U01 file. It has no colors, so `threads` is empty; a needle
/// change after the first stitch is a color change.
pub fn read(bytes: &[u8]) -> StitchDesign {
    read_records(bytes.get(HEADER_SIZE..).unwrap_or(&[]), false)
        .unwrap_or_default()
        .0
}

/// The needle of each color block of a U01 file (ADR 0063).
pub fn read_needles(bytes: &[u8]) -> Vec<u8> {
    read_records(bytes.get(HEADER_SIZE..).unwrap_or(&[]), false)
        .unwrap_or_default()
        .1
}

/// Reads a DAT file: Barudan (U01 records from the start), else Sunstar.
pub fn read_dat(bytes: &[u8]) -> StitchDesign {
    if let Some((d, _)) = read_records(bytes, true) {
        return d;
    }
    let mut p = Pattern::default();
    for c in bytes.get(0x100..).unwrap_or(&[]).as_chunks::<3>().0 {
        let (mut x, mut y) = (i32::from(c[0] & 0x7f), i32::from(c[1] & 0x7f));
        if c[0] & 0x80 != 0 {
            x = -x;
        }
        if c[1] & 0x80 != 0 {
            y = -y;
        }
        y = -y;
        let moved = x != 0 || y != 0;
        match c[2] {
            0x07 | 0x84 => p.stitch(x, y),
            0x04 => p.jump(x, y),
            k @ (0x80 | 0x87) => {
                p.here(if k == 0x80 { Low::Trim } else { Low::ColorChange });
                if moved {
                    p.jump(x, y);
                }
            }
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::{StitchKind, StitchRecord};

    use super::*;
    use crate::common::tests::sample;

    fn kinds(d: &StitchDesign) -> Vec<StitchKind> {
        d.records.iter().map(|r| r.kind).collect()
    }

    #[test]
    fn header_and_records() {
        let b = write(&sample());
        assert!(b[..0x80].iter().all(|&c| c == b'0'));
        assert_eq!((b.len() - HEADER_SIZE) % 3, 0, "3-byte records after the header");
        assert_eq!(&b[b.len() - 3..], &[0xf8, 0, 0]);
        // The needle set of color 1 (C01 = 0x09) comes before the first stitch.
        let cmds: Vec<u8> = b[HEADER_SIZE..].chunks(3).map(|c| c[0] & 0x1f).collect();
        let first_stitch = cmds.iter().position(|&c| c == 0).unwrap();
        assert!(cmds[..first_stitch].contains(&0x09), "{cmds:?}");
    }

    #[test]
    fn round_trip_keeps_stitches_colors_and_speeds() {
        let mut d = sample();
        // Slow before the second stitch, fast before the fourth.
        let i = d.records.iter().position(|r| r.kind == StitchKind::Stitch).unwrap() + 1;
        let at = d.records[i].pos;
        d.records.insert(i, StitchRecord::new(at, StitchKind::Slow));
        d.records
            .insert(i + 3, StitchRecord::new(d.records[i + 2].pos, StitchKind::Fast));
        let back = read(&write(&d));
        assert_eq!(back.stitch_count(), d.stitch_count());
        assert_eq!(back.color_count(), d.color_count());
        assert_eq!(back.records.iter().filter(|r| r.kind == StitchKind::Slow).count(), 1);
        assert_eq!(back.records.iter().filter(|r| r.kind == StitchKind::Fast).count(), 1);
        let s = |d: &StitchDesign| -> Vec<Point> {
            d.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect()
        };
        assert_eq!(s(&back), s(&d));
        // A Barudan DAT file is the same records with no header.
        let dat = &write(&d)[HEADER_SIZE..];
        assert_eq!(s(&read_dat(dat)), s(&d));
        assert_eq!(kinds(&read(&write(&d))), kinds(&back));
    }

    #[test]
    fn needles_of_the_plan_are_written_and_read() {
        let d = sample();
        assert_eq!(read_needles(&write(&d)), [1, 2], "pyembroidery's needles");
        let b = write_with(&d, &[7, 15]);
        assert_eq!(read_needles(&b), [7, 15]);
        assert_eq!(read(&b).stitch_count(), d.stitch_count());
    }

    #[test]
    fn sunstar_dat() {
        let mut b = vec![0x00; 0x100];
        b.extend([
            10, 0, 0x07, 0x85, 0x0a, 0x04, 0, 0, 0x80, 0, 0, 0x87, 5, 5, 0x84, 0, 0, 0,
        ]);
        let d = read_dat(&b);
        assert_eq!(
            kinds(&d),
            [
                StitchKind::Stitch,
                StitchKind::Jump,
                StitchKind::Trim,
                StitchKind::ColorChange,
                StitchKind::Stitch,
                StitchKind::End
            ]
        );
        assert_eq!(d.records[1].pos, Point::new(0.5, -1.0));
    }
}

//! Read-only formats of other machines and programs (P8 Milestone 6):
//! BRO, EMD, INB, JPX, KSM, MAX, MIT, NEW, PCD, PCQ, PCS, PCM, SPX, STC,
//! ZHS and ZXY. Each one is a small stitch block after a header.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/BroReader.py` (Bits & Volts), `EmdReader.py` (Elna),
//!   `InbReader.py` (Inbro), `JpxReader.py` (Janome), `KsmReader.py`
//!   (Pfaff), `MaxReader.py` (Pfaff), `MitReader.py` (Mitsubishi),
//!   `NewReader.py` (Ameco), `PcdReader.py`, `PcqReader.py`,
//!   `PcsReader.py`, `PcmReader.py` (Pfaff), `SpxReader.py`, `StcReader.py`
//!   (Gunold), `ZhsReader.py` (Zeng Hsing), `ZxyReader.py`: the `read`
//!   function and the stitch loop of each file
//! - `pyembroidery/EmbPattern.py`: `stitch`, `move`, `stitch_abs`,
//!   `move_abs`, `color_change` (relative and absolute positions)
//!
//! Units are 0.1 mm, Y down, as in the other formats, except where the
//! reader says. A needle change before the first stitch is not a color
//! change (as in `dsz`); after it, it is one.
//!
//! None of these formats has a public description of its own; the readers
//! do what pyembroidery does. pyembroidery has no writer for them.

use emb_geom::Point;
use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

use crate::common::{Bytes, Low, Pattern, UNITS_PER_MM, color_thread, signed8, signed16, to_design};

/// Builds records at positions in file units that need not be whole
/// numbers (MAX, PCx, MIT).
#[derive(Default)]
struct FPattern {
    recs: Vec<StitchRecord>,
    x: f64,
    y: f64,
}

impl FPattern {
    fn abs(&mut self, kind: StitchKind, x: f64, y: f64) {
        (self.x, self.y) = (x, y);
        self.recs
            .push(StitchRecord::new(Point::new(x / UNITS_PER_MM, y / UNITS_PER_MM), kind));
    }

    fn rel(&mut self, kind: StitchKind, dx: f64, dy: f64) {
        self.abs(kind, self.x + dx, self.y + dy);
    }

    fn design(mut self, threads: Vec<Thread>) -> StitchDesign {
        let end = self.recs.last().map(|r| r.pos).unwrap_or_default();
        self.recs.push(StitchRecord::new(end, StitchKind::End));
        StitchDesign {
            threads,
            records: self.recs,
        }
    }
}

/// A needle change: a color change after the first stitch.
fn needle(p: &mut Pattern) {
    if p.recs.iter().any(|r| r.0 == Low::Stitch) {
        p.here(Low::ColorChange);
    }
}

fn from(b: &[u8], at: usize) -> &[u8] {
    b.get(at..).unwrap_or(&[])
}

fn signed24(v: u32) -> i32 {
    let v = (v & 0xff_ffff) as i32;
    if v > 0x7f_ffff { v - 0x100_0000 } else { v }
}

fn u24le(r: &mut Bytes) -> Option<u32> {
    Some(u32::from(r.u8()?) | u32::from(r.u8()?) << 8 | u32::from(r.u8()?) << 16)
}

fn u24be(r: &mut Bytes) -> Option<u32> {
    Some(u32::from(r.u8()?) << 16 | u32::from(r.u8()?) << 8 | u32::from(r.u8()?))
}

/// Bits & Volts BRO: 2-byte stitches, `80 nn` controls, from 0x100.
pub fn read_bro(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let mut r = Bytes::new(from(bytes, 0x100));
    while let (Some(b0), Some(b1)) = (r.u8(), r.u8()) {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        // The control is the next byte; the byte after 0x80 is not used.
        let Some(control) = r.u8() else { break };
        let long_move = |p: &mut Pattern, r: &mut Bytes| -> bool {
            match (r.u16le(), r.u16le()) {
                (Some(x), Some(y)) => {
                    p.jump(signed16(x), -signed16(y));
                    true
                }
                _ => false,
            }
        };
        match control {
            0x00 => {}
            0x02 | 0xe0 => break,
            0x7e | 0x03 => {
                if !long_move(&mut p, &mut r) {
                    break;
                }
            }
            0xe1..=0xef => {
                needle(&mut p);
                if !long_move(&mut p, &mut r) {
                    break;
                }
            }
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

/// Elna EMD: 2-byte stitches, `80 nn` controls, from 0x30.
pub fn read_emd(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let mut it = from(bytes, 0x30).as_chunks::<2>().0.iter();
    while let Some(&[b0, b1]) = it.next() {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        match b1 {
            0x80 => {
                let Some(&[x, y]) = it.next() else { break };
                p.jump(signed8(x), -signed8(y));
            }
            0x2a => p.here(Low::ColorChange),
            0x7d => {}
            0xad | 0x90 => p.here(Low::Trim),
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

/// Inbro INB: 3-byte records `x, y, control`, from 0x2000.
pub fn read_inb(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    for c in from(bytes, 0x2000).as_chunks::<3>().0 {
        let (mut x, mut y, ctrl) = (i32::from(c[0]), -i32::from(c[1]), c[2]);
        if ctrl & 0x20 != 0 {
            y = -y;
        }
        if ctrl & 0x40 != 0 {
            x = -x;
        }
        match ctrl & 0x0f {
            0x00 => p.stitch(x, y),
            0x01 => p.add(Low::ColorChange, x, y),
            0x02 => p.jump(x, y),
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

/// Janome JPX: a header with the stitch start and the color indices, then
/// EXP-like 2-byte records.
pub fn read_jpx(bytes: &[u8]) -> StitchDesign {
    let mut r = Bytes::new(bytes);
    let start = r.i32le().unwrap_or(0).max(0) as usize;
    r.skip(0x1c);
    let colors = r.i32le().unwrap_or(0).max(0);
    r.skip(0x18);
    let mut threads = Vec::new();
    for i in 0..colors {
        let Some(index) = r.i32le() else { break };
        // pyembroidery gives a random color; a gray of the index is the same each time.
        let g = (40 + (index.rem_euclid(8) * 25)) as u8;
        threads.push(Thread {
            id: ThreadId(i as u32),
            name: format!("JPX index {index}"),
            color: Rgb(g, g, g),
            brand: None,
            code: None,
        });
    }
    let mut p = Pattern::default();
    let mut it = from(bytes, start).as_chunks::<2>().0.iter();
    while let Some(&[b0, b1]) = it.next() {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        let Some(&[c0, c1]) = it.next() else { break };
        let (x, y) = (signed8(c0), -signed8(c1));
        match b1 {
            0x02 => p.jump(x, y),
            0x01 => {
                p.here(Low::ColorChange);
                // As pyembroidery: only a move on both axes.
                if x != 0 && y != 0 {
                    p.jump(x, y);
                }
            }
            _ => break,
        }
    }
    to_design(p.recs, threads)
}

/// Pfaff KSM: 3-byte records `y, x, control`, from 0x200.
pub fn read_ksm(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let (mut trimmed, mut stitched) = (false, false);
    for c in from(bytes, 0x200).as_chunks::<3>().0 {
        let (mut y, mut x, mut ctrl) = (-i32::from(c[0]), i32::from(c[1]), c[2]);
        if ctrl & 0x40 != 0 {
            x = -x;
        }
        if ctrl & 0x20 != 0 {
            y = -y;
        }
        ctrl &= 0x1f;
        if x != 0 || y != 0 {
            if trimmed {
                p.jump(x, y);
            } else {
                p.stitch(x, y);
                stitched = true;
            }
        }
        match ctrl {
            0x00 => {}
            0x07 | 0x13 | 0x1d => {
                if stitched {
                    p.here(Low::Trim);
                }
                trimmed = true;
            }
            0x17..=0x19 | 0x1b => trimmed = false,
            0x0b..=0x12 => {
                needle(&mut p);
                trimmed = true;
            }
            0x05 => p.here(Low::Stop),
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

/// Pfaff MAX: absolute 24-bit positions in units of 0.1235 mm, from 0xD5.
pub fn read_max(bytes: &[u8]) -> StitchDesign {
    const RATIO: f64 = 1.235;
    let mut r = Bytes::new(bytes);
    r.seek(0xd5);
    let count = r.i32le().unwrap_or(0).max(0);
    let mut p = FPattern::default();
    for _ in 0..count {
        let (Some(x), Some(_), Some(y), Some(_)) = (u24le(&mut r), r.u8(), u24le(&mut r), r.u8()) else {
            break;
        };
        p.abs(
            StitchKind::Stitch,
            f64::from(signed24(x)) * RATIO,
            f64::from(signed24(y)) * RATIO,
        );
    }
    p.design(Vec::new())
}

/// Mitsubishi MIT: 2-byte records with 5-bit moves of 0.2 mm.
pub fn read_mit(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let mut previous = -1i32;
    for c in bytes.as_chunks::<2>().0 {
        let (mut x, mut y) = (i32::from(c[0] & 0x1f) * 2, -(i32::from(c[1] & 0x1f) * 2));
        if c[0] & 0x80 != 0 {
            x = -x;
        }
        if c[1] & 0x80 != 0 {
            y = -y;
        }
        let ctrl = i32::from((c[0] & 0x60) >> 3 | (c[1] & 0x60) >> 5);
        match ctrl {
            0b1100 => p.jump(x, y),
            0b1000 => {
                if previous == 0b0111 {
                    p.here(Low::ColorChange);
                }
            }
            // 0 comes at the end; pyembroidery reads on after it.
            0b0000 => {}
            _ => p.stitch(x, y),
        }
        previous = ctrl;
    }
    to_design(p.recs, Vec::new())
}

/// Ameco NEW: a stitch count, then 3-byte records `x, y, control`.
pub fn read_new(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    for c in from(bytes, 2).as_chunks::<3>().0 {
        let (mut x, mut y, mut ctrl) = (i32::from(c[0]), -i32::from(c[1]), c[2]);
        if ctrl & 0x40 != 0 {
            x = -x;
        }
        if ctrl & 0x20 != 0 {
            y = -y;
        }
        ctrl &= !0xe0;
        if ctrl == 0 {
            p.stitch(x, y);
        } else if ctrl == 0x11 {
            break;
        } else if ctrl & 0x02 != 0 {
            p.here(Low::ColorChange);
        } else if ctrl & 0x01 != 0 {
            p.jump(x, y);
        }
    }
    to_design(p.recs, Vec::new())
}

/// Pfaff PCD, PCQ and PCS: the colors (24-bit, big-endian), then absolute
/// 24-bit positions in units of 1/6 mm.
pub fn read_pcs(bytes: &[u8]) -> StitchDesign {
    let mut r = Bytes::new(bytes);
    r.skip(2); // version, hoop size
    let count = r.u16le().unwrap_or(0);
    let mut threads = Vec::new();
    for i in 0..count {
        let Some(c) = u24be(&mut r) else { break };
        threads.push(color_thread(i as usize, rgb(c)));
        r.skip(1);
    }
    r.skip(2); // stitch count
    pc_stitches(&mut r, threads, |r| {
        let (_, x, _, y) = (r.u8()?, u24le(r)?, r.u8()?, u24le(r)?);
        Some((x, y))
    })
}

/// Pfaff PCM: the colors as indices of a table of 16, then absolute
/// 24-bit positions (big-endian) in units of 1/6 mm.
pub fn read_pcm(bytes: &[u8]) -> StitchDesign {
    const TABLE: [u32; 16] = [
        0x000000, 0x000080, 0x0000ff, 0x008080, 0x00ffff, 0x800080, 0xff00ff, 0x800000, 0xff0000, 0x008000, 0x00ff00,
        0x808000, 0xffff00, 0x808080, 0xc0c0c0, 0xffffff,
    ];
    let mut r = Bytes::new(bytes);
    r.seek(2);
    let Some(count) = r.u16be() else {
        return StitchDesign::default();
    };
    let mut threads = Vec::new();
    for i in 0..count {
        let Some(k) = r.u16be() else { break };
        let c = TABLE[usize::from(k) % TABLE.len()];
        let mut t = color_thread(i as usize, rgb(c));
        t.name = format!("PCM Color {}", usize::from(k) % TABLE.len() + 1);
        threads.push(t);
    }
    r.skip(2); // stitch count
    pc_stitches(&mut r, threads, |r| {
        let (x, _, y, _) = (u24be(r)?, r.u8()?, u24be(r)?, r.u8()?);
        Some((x, y))
    })
}

fn rgb(c: u32) -> Rgb {
    Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// The stitch loop of the PCx formats: a position, then a control byte.
fn pc_stitches(
    r: &mut Bytes,
    threads: Vec<Thread>,
    mut pos: impl FnMut(&mut Bytes) -> Option<(u32, u32)>,
) -> StitchDesign {
    const RATIO: f64 = 5.0 / 3.0;
    let mut p = FPattern::default();
    while let Some((x, y)) = pos(r) {
        let Some(ctrl) = r.u8() else { break };
        let (x, y) = (f64::from(signed24(x)) * RATIO, -f64::from(signed24(y)) * RATIO);
        if ctrl == 0 {
            p.abs(StitchKind::Stitch, x, y);
        } else if ctrl & 0x01 != 0 {
            p.rel(StitchKind::ColorChange, 0.0, 0.0);
        } else if ctrl & 0x04 != 0 {
            p.abs(StitchKind::Jump, x, y);
        } else {
            break;
        }
    }
    p.design(threads)
}

/// SPX: 16-bit moves (big-endian), from 0x11E.
pub fn read_spx(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let mut r = Bytes::new(from(bytes, 0x11e));
    loop {
        // Two bytes that are not used before and after each stitch.
        if r.u16le().is_none() {
            break;
        }
        let (Some(y), Some(x), Some(c)) = (r.u16be(), r.u16be(), r.u8()) else {
            break;
        };
        let dy = -signed16(y) - signed8(c);
        let dx = signed16(x);
        if r.u16le().is_none() {
            break;
        }
        p.stitch(dx, dy);
    }
    to_design(p.recs, Vec::new())
}

/// Gunold STC: 3-byte records `x, y, control`, from 0x28.
pub fn read_stc(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    for c in from(bytes, 0x28).as_chunks::<3>().0 {
        let (x, y, ctrl) = (signed8(c[0]), -signed8(c[1]), c[2]);
        match ctrl {
            0x01 => p.stitch(x, y),
            0x00 => p.jump(x, y),
            25 => break,
            _ => needle(&mut p),
        }
    }
    to_design(p.recs, Vec::new())
}

/// Zeng Hsing ZHS: a header with the colors and their names, then 3-byte
/// records with the bits of X and Y mixed.
pub fn read_zhs(bytes: &[u8]) -> StitchDesign {
    let mut r = Bytes::new(bytes);
    r.seek(0x0f);
    let (Some(start), Some(header)) = (r.i32le(), r.i32le()) else {
        return StitchDesign::default();
    };
    r.seek(header.max(0) as usize);
    let count = r.u8().unwrap_or(0);
    let mut threads = Vec::new();
    for i in 0..count {
        let Some(c) = u24be(&mut r) else { break };
        threads.push(color_thread(usize::from(i), rgb(c)));
    }
    if let Some(len) = r.u16le() {
        let at = r.at;
        let text = bytes.get(at..at + usize::from(len)).unwrap_or(&[]);
        // "&$chart&#description&#code.." for each thread.
        let text = String::from_utf8_lossy(text);
        for (t, data) in threads.iter_mut().zip(text.split("&$").skip(1)) {
            let parts: Vec<&str> = data.split("&#").collect();
            if let Some(c) = parts.first().filter(|c| !c.is_empty()) {
                t.brand = Some((*c).to_string());
            }
            if let Some(d) = parts.get(1).filter(|d| !d.is_empty()) {
                t.name = (*d).to_string();
            }
            if let Some(code) = parts.get(2).filter(|c| c.len() > 3) {
                t.code = Some(code[..code.len() - 2].to_string());
            }
        }
    }
    let mut p = Pattern::default();
    let (mut xx, mut yy) = (0, 0);
    let bit = |v: u8, m: u8| i32::from(v & m);
    for c in from(bytes, start.max(0) as usize).as_chunks::<3>().0 {
        let ctrl = c[0];
        if ctrl == 0x10 {
            continue; // a check sum
        }
        let fix = |v: i32| {
            let v = signed8(v as u8);
            if v >= 63 {
                v + 1
            } else if v <= -63 {
                v - 1
            } else {
                v
            }
        };
        let x = fix(bit(c[1], 0x01)
            + bit(c[2], 0x02)
            + bit(c[1], 0x04)
            + bit(c[2], 0x08)
            + bit(c[1], 0x10)
            + bit(c[2], 0x20)
            + bit(c[1], 0x40)
            + bit(c[2], 0x80));
        let y = fix(bit(c[2], 0x01)
            + bit(c[1], 0x02)
            + bit(c[2], 0x04)
            + bit(c[1], 0x08)
            + bit(c[2], 0x10)
            + bit(c[1], 0x20)
            + bit(c[2], 0x40)
            + bit(c[1], 0x80));
        xx += x;
        yy += y;
        match ctrl {
            0x02 => {
                p.stitch(xx, -yy);
                (xx, yy) = (0, 0);
            }
            0x01 => {
                p.jump(xx, -yy);
                (xx, yy) = (0, 0);
            }
            0x04 => {
                (xx, yy) = (0, 0);
                p.here(Low::ColorChange);
            }
            0x80 => break,
            _ => {}
        }
    }
    to_design(p.recs, threads)
}

/// ZXY: a header length, then 3-byte records `control, x, y`.
pub fn read_zxy(bytes: &[u8]) -> StitchDesign {
    let mut r = Bytes::new(bytes);
    r.seek(1);
    let skip = usize::from(r.u16be().unwrap_or(0));
    let mut p = Pattern::default();
    for c in from(bytes, 3 + skip).as_chunks::<3>().0 {
        let (mut ctrl, mut x, mut y) = (c[0], i32::from(c[1]), -i32::from(c[2]));
        if ctrl & 0x08 != 0 {
            x = -x;
        }
        if ctrl & 0x04 != 0 {
            y = -y;
        }
        ctrl &= !0x0c;
        if ctrl == 0 {
            p.stitch(x, y);
        } else if ctrl & 0x02 != 0 {
            p.jump(x, y);
        } else if ctrl & 0x20 != 0 {
            if c[1] == 0xff {
                break;
            }
            needle(&mut p);
        }
    }
    to_design(p.recs, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(d: &StitchDesign) -> Vec<StitchKind> {
        d.records.iter().map(|r| r.kind).collect()
    }

    fn at(d: &StitchDesign, i: usize) -> (f64, f64) {
        let p = d.records[i].pos;
        ((p.x * 1000.0).round() / 1000.0, (p.y * 1000.0).round() / 1000.0)
    }

    use StitchKind::{ColorChange, End, Jump, Stitch, Stop, Trim};

    #[test]
    fn bro_emd_and_inb() {
        let mut b = vec![0; 0x100];
        b.extend([
            10, 5, 0x80, 0, 0x03, 0x2c, 0x01, 0x0a, 0x00, 0x80, 0, 0xe2, 0, 0, 0, 0, 5, 5, 0x80, 0, 0x02,
        ]);
        let d = read_bro(&b);
        assert_eq!(kinds(&d), [Stitch, Jump, ColorChange, Jump, Stitch, End]);
        assert_eq!(at(&d, 0), (1.0, -0.5));
        assert_eq!(at(&d, 1), (31.0, -1.5), "0x12c = 300 units");

        let mut b = vec![0; 0x30];
        b.extend([10, 0, 0x80, 0x80, 20, 20, 0x80, 0x2a, 1, 1, 0x80, 0xad, 0x80, 0xfd]);
        let d = read_emd(&b);
        assert_eq!(kinds(&d), [Stitch, Jump, ColorChange, Stitch, Trim, End]);

        let mut b = vec![0; 0x2000];
        b.extend([5, 5, 0x00, 5, 5, 0x60, 0, 0, 0x01, 7, 0, 0x02, 0, 0, 0x04]);
        let d = read_inb(&b);
        assert_eq!(kinds(&d), [Stitch, Stitch, ColorChange, Jump, End]);
        assert_eq!(at(&d, 1), (0.0, 0.0), "both signs set: back to the start");
    }

    #[test]
    fn jpx_ksm_and_stc() {
        // Stitch start at 0x44, 2 colors at 0x20, their indices from 0x3c.
        let mut b = vec![0u8; 0x44];
        b[..4].copy_from_slice(&0x44i32.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&2i32.to_le_bytes());
        b[0x3c..0x40].copy_from_slice(&3i32.to_le_bytes());
        b.extend([10, 10, 0x80, 0x02, 5, 5, 0x80, 0x01, 1, 1, 20, 0, 0x80, 0x10, 0, 0]);
        let d = read_jpx(&b);
        assert_eq!(d.threads.len(), 2);
        assert_eq!(kinds(&d), [Stitch, Jump, ColorChange, Jump, Stitch, End]);

        let mut b = vec![0; 0x200];
        b.extend([
            0, 10, 0x00, 0, 0, 0x07, 5, 5, 0x00, 0, 0, 0x17, 0, 3, 0x40, 0, 0, 0x0b, 0, 0, 0x05, 0, 0, 0x08,
        ]);
        let d = read_ksm(&b);
        assert_eq!(kinds(&d), [Stitch, Trim, Jump, Stitch, ColorChange, Stop, End]);
        assert_eq!(at(&d, 3), (1.2, -0.5));

        let mut b = vec![0; 0x28];
        b.extend([0, 0, 3, 10, 0, 0x01, 0, 0, 4, 0xf6, 0xf6, 0x00, 0, 0, 25]);
        let d = read_stc(&b);
        assert_eq!(
            kinds(&d),
            [Stitch, ColorChange, Jump, End],
            "a needle before the first stitch is not a change"
        );
    }

    #[test]
    fn max_mit_new_and_spx() {
        let mut b = vec![0u8; 0xd5];
        b.extend(2i32.to_le_bytes());
        b.extend([100, 0, 0, 0, 0xce, 0xff, 0xff, 0]);
        b.extend([0, 0, 0, 0, 0, 0, 0, 0]);
        let d = read_max(&b);
        assert_eq!(kinds(&d), [Stitch, Stitch, End]);
        assert_eq!(at(&d, 0), (12.35, -6.175));

        // Control 0b0111 (a stitch): bits 0x20 of the first byte, 0x60 of
        // the second; then 0b1000 (a color change after it), then 0 (end).
        let b = [0x25, 0x63, 0x40, 0x00, 0x00, 0x00];
        let d = read_mit(&b);
        assert_eq!(kinds(&d), [Stitch, ColorChange, End]);
        assert_eq!(at(&d, 0), (1.0, -0.6));

        let b = [0, 9, 10, 0, 0x00, 0, 10, 0x20, 0, 0, 0x02, 5, 5, 0x01, 0, 0, 0x11];
        let d = read_new(&b);
        assert_eq!(kinds(&d), [Stitch, Stitch, ColorChange, Jump, End]);
        assert_eq!(at(&d, 1), (1.0, 1.0));

        let mut b = vec![0; 0x11e];
        b.extend([0, 0, 0, 20, 0, 10, 0, 0, 0]);
        let d = read_spx(&b);
        assert_eq!(kinds(&d), [Stitch, End]);
        assert_eq!(at(&d, 0), (1.0, -2.0));
    }

    #[test]
    fn pcs_and_pcm() {
        let mut b = vec![0x32, 0x03, 2, 0];
        b.extend([0xff, 0x00, 0x00, 0, 0x00, 0x00, 0xff, 0]);
        b.extend([3, 0]);
        b.extend([0, 60, 0, 0, 0, 30, 0, 0, 0x00]); // x 60, y 30 (units of 1/6 mm)
        b.extend([0, 0, 0, 0, 0, 0, 0, 0, 0x01]);
        b.extend([0, 0, 0, 0, 0, 0, 0, 0, 0x04]);
        let d = read_pcs(&b);
        assert_eq!(d.threads.len(), 2);
        assert_eq!(d.threads[1].color, Rgb(0, 0, 255));
        assert_eq!(kinds(&d), [Stitch, ColorChange, Jump, End]);
        assert_eq!(at(&d, 0), (10.0, -5.0));

        let mut b = vec![0, 0, 0, 1, 0, 8, 0, 1];
        b.extend([0, 0, 60, 0, 0, 0, 30, 0, 0x00]);
        let d = read_pcm(&b);
        assert_eq!(d.threads[0].color, Rgb(255, 0, 0));
        assert_eq!(at(&d, 0), (10.0, -5.0));
    }

    #[test]
    fn zhs_and_zxy() {
        let mut b = vec![0u8; 0x17];
        b[0x0f..0x13].copy_from_slice(&0x40i32.to_le_bytes());
        b[0x13..0x17].copy_from_slice(&0x17i32.to_le_bytes());
        b.extend([1, 0xff, 0x00, 0x00]);
        let names = "&$Madeira&#Red&#1147xx";
        b.extend((names.len() as u16).to_le_bytes());
        b.extend(names.bytes());
        b.resize(0x40, 0);
        // x = 10: bits 0x02 and 0x08 are in the third byte.
        b.extend([0x02, 0x00, 0x0a, 0x04, 0, 0, 0x80, 0, 0]);
        let d = read_zhs(&b);
        assert_eq!(d.threads[0].name, "Red");
        assert_eq!(d.threads[0].code.as_deref(), Some("1147"));
        assert_eq!(kinds(&d), [Stitch, ColorChange, End]);
        assert_eq!(at(&d, 0), (1.0, 0.0));

        let b = [
            0, 0, 2, 9, 9, 0x00, 10, 5, 0x02, 3, 3, 0x20, 0, 4, 0x00, 7, 7, 0x20, 0xff, 0,
        ];
        let d = read_zxy(&b);
        assert_eq!(kinds(&d), [Stitch, Jump, ColorChange, Stitch, End]);
    }
}

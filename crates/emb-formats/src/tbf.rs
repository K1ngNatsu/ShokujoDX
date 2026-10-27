//! Tajima TBF read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/TbfWriter.py`: `write` (lines 13-122; `MAX_JUMP_DISTANCE`
//!   127, `EXPLICIT_TRIM`, `THREAD_CHANGE_COMMAND = NEEDLE_SET`)
//! - `pyembroidery/TbfReader.py`: `read` (5-58)
//! - `pyembroidery/EmbEncoder.py`: `build_thread_change_sequence` (123-),
//!   `declare_not_trimmed` (357-), `next_change_sequence` (585-): the needle
//!   of each color, with the default `needle_count` of 5
//!
//! Format summary: a 0x600-byte header, then 3-byte records. The header has
//! "3.00", the label at 0x83, the counts and extents as in DST, the needle
//! of each needle set (`DO`, one byte each, from 0x10A) and the thread
//! colors (`DA`, from 0x20E: `E r g b` and a space for each thread). A
//! record is `dx, -dy, command` in 0.1 mm with Y up: `80` stitch, `90` jump,
//! `86` trim, `81` needle set, `40` stop, `8F` end. A needle set comes
//! before the first stitch and at each color change.
//!
//! Needles: the needle plan of the machine gives the needle of each color
//! block (ADR 0063). With no plan, color 1 is needle 1, color 2 is needle 2,
//! and so on to needle 5, then needle 1 again (pyembroidery's default).
//!
//! Deviations from pyembroidery, on purpose:
//! - The label is cut to 16 ASCII characters (as in `dst`).
//! - The header holds at most 72 threads and 256 needle sets. pyembroidery
//!   writes past the header fields (or stops with an error) when there are
//!   more.

use emb_model::{Rgb, StitchDesign};

use crate::common::{Bytes, Low, Pattern, color_thread, encode_with, signed8, to_design};

/// Largest move of one record, per axis, in 0.1 mm.
pub const MAX_DELTA: i32 = 127;
const HEADER_SIZE: usize = 0x600;
/// pyembroidery's default `needle_count`.
const NEEDLES: usize = 5;
/// Thread entries that fit between 0x20E and 0x376.
const MAX_THREADS: usize = (0x376 - 0x20E) / 5;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TbfError {
    #[error("file is shorter than the 1536-byte TBF header")]
    TooShort,
}

/// The needle of needle set `i` (from 0).
fn needle(i: usize) -> u8 {
    (i % NEEDLES + 1) as u8
}

/// Writes a TBF file with pyembroidery's needles. `label` goes in the
/// header (`LA`).
pub fn write(design: &StitchDesign, label: &str) -> Vec<u8> {
    write_with(design, label, &[])
}

/// Writes a TBF file. `needles` has the needle of each color block (ADR
/// 0063); a block with no needle in the list gets pyembroidery's needle.
pub fn write_with(design: &StitchDesign, label: &str, needles: &[u8]) -> Vec<u8> {
    let low = encode_with(design, MAX_DELTA, false, true);
    let sets = low.iter().filter(|r| r.0 == Low::ColorChange).count();
    // Bounds of all records, as `EmbPattern.bounds`.
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
            format!("+{v:5}")
        } else {
            format!("-{:5}", v.abs())
        }
    };

    let mut b = b"3.00".to_vec();
    b.resize(0x80, b' ');
    let mut text = String::new();
    text.push_str(&format!("LA:{:<16}\r", crate::dst::label_text(label)));
    text.push_str(&format!("ST:{:7}\r", low.len()));
    text.push_str(&format!("CO:{sets:3}\r"));
    text.push_str(&format!("+X:{:5}\r", max_x.abs()));
    text.push_str(&format!("-X:{:5}\r", min_x.abs()));
    text.push_str(&format!("+Y:{:5}\r", max_y.abs()));
    text.push_str(&format!("-Y:{:5}\r", min_y.abs()));
    text.push_str(&format!("AX:{}\r", signed(ax)));
    text.push_str(&format!("AY:{}\r", signed(ay)));
    // TP and JC: pyembroidery writes these values; their use is not known.
    text.push_str(&format!("TP:{:<32}\r", "EG/"));
    text.push_str("JC:3\r");
    text.push_str("DO:");
    b.extend(text.bytes());
    let mut order = [0u8; 0x100];
    for (i, n) in order.iter_mut().enumerate().take(sets) {
        *n = needles.get(i).copied().filter(|&k| k > 0).unwrap_or_else(|| needle(i));
    }
    b.extend(order);
    b.extend(b"\rDA:");
    for t in design.threads.iter().take(MAX_THREADS) {
        let Rgb(r, g, bl) = t.color;
        b.extend([0x45, r, g, bl, 0x20]);
    }
    b.resize(0x376, b' ');
    b.extend([0x0d, 0x1a]);
    b.resize(HEADER_SIZE, b' ');

    let (mut xx, mut yy) = (0, 0);
    for (kind, x, y) in low {
        let (dx, dy) = (x - xx, y - yy);
        (xx, yy) = (x, y);
        let cmd = match kind {
            Low::Stitch => 0x80,
            Low::Jump => 0x90,
            Low::Stop => 0x40,
            Low::Trim => 0x86,
            Low::ColorChange => 0x81,
            Low::End => 0x8f,
            Low::Slow | Low::Fast => continue,
        };
        b.extend([dx as u8, (-dy) as u8, cmd]);
    }
    b.push(0x1a);
    b
}

/// Reads a TBF file. The first needle set (before the first stitch) is not
/// a color change; each later one is.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, TbfError> {
    read_with_needles(bytes).map(|(d, _)| d)
}

/// The needle of each color block of a TBF file (ADR 0063).
pub fn read_needles(bytes: &[u8]) -> Vec<u8> {
    read_with_needles(bytes).map(|(_, n)| n).unwrap_or_default()
}

fn read_with_needles(bytes: &[u8]) -> Result<(StitchDesign, Vec<u8>), TbfError> {
    if bytes.len() < HEADER_SIZE {
        return Err(TbfError::TooShort);
    }
    let order = &bytes[0x10A..0x20A];
    let mut threads = Vec::new();
    let mut r = Bytes::new(bytes);
    r.seek(0x20E);
    while r.u8() == Some(0x45) {
        let (Some(red), Some(g), Some(bl)) = (r.u8(), r.u8(), r.u8()) else {
            break;
        };
        r.skip(1);
        threads.push(color_thread(threads.len(), Rgb(red, g, bl)));
    }

    let mut p = Pattern::default();
    let mut sets = 0usize;
    let mut stitched = false;
    let mut needles = Vec::new();
    for c in bytes[HEADER_SIZE..].as_chunks::<3>().0 {
        let (x, y) = (signed8(c[0]), -signed8(c[1]));
        match c[2] {
            0x80 => {
                p.stitch(x, y);
                stitched = true;
            }
            0x81 => {
                let n = order.get(sets).copied().unwrap_or(0);
                sets += 1;
                if n == 0 {
                    // Needle 0 should not occur; pyembroidery reads it as a stop.
                    p.here(Low::Stop);
                } else if stitched {
                    p.here(Low::ColorChange);
                    needles.push(n);
                } else {
                    // A needle set before the first stitch: the needle of block 1.
                    needles.clear();
                    needles.push(n);
                }
            }
            0x90 if x == 0 && y == 0 => p.here(Low::Trim),
            0x90 => p.jump(x, y),
            0x40 => p.here(Low::Stop),
            0x86 => p.here(Low::Trim),
            _ => break,
        }
    }
    Ok((to_design(p.recs, threads), needles))
}

#[cfg(test)]
mod tests {
    use emb_model::StitchKind;

    use super::*;
    use crate::common::tests::sample;

    #[test]
    fn header_layout() {
        let b = write(&sample(), "A long design name with é");
        assert_eq!(&b[..4], b"3.00");
        assert_eq!(&b[0x80..0x94], b"LA:A long design na\r");
        let text = std::str::from_utf8(&b[0x94..0x10A]).unwrap();
        // Two needle sets: one at the start, one at the color change.
        assert!(text.starts_with("ST:"), "{text}");
        assert!(text.contains("CO:  2\r"), "{text}");
        assert!(text.contains("TP:EG/"), "{text}");
        assert!(text.ends_with("JC:3\rDO:"), "{text}");
        assert_eq!(&b[0x10A..0x10D], &[1, 2, 0]);
        assert_eq!(&b[0x20A..0x20E], b"\rDA:");
        assert_eq!(&b[0x20E..0x218], &[0x45, 200, 30, 40, 0x20, 0x45, 10, 20, 250, 0x20]);
        assert_eq!(&b[0x376..0x378], &[0x0d, 0x1a]);
        assert_eq!(b.len() % 3, HEADER_SIZE % 3 + 1, "records and one 0x1A");
        assert_eq!(*b.last().unwrap(), 0x1a);
        assert_eq!(&b[b.len() - 4..b.len() - 1], &[0, 0, 0x8f]);
    }

    #[test]
    fn needles_repeat_after_five() {
        assert_eq!((0..7).map(needle).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 1, 2]);
    }

    #[test]
    fn needles_of_the_plan_are_written_and_read() {
        let d = sample();
        assert_eq!(read_needles(&write(&d, "t")), [1, 2]);
        let b = write_with(&d, "t", &[12, 3]);
        assert_eq!(&b[0x10A..0x10D], &[12, 3, 0]);
        assert_eq!(read_needles(&b), [12, 3]);
    }

    #[test]
    fn round_trip() {
        let d = sample();
        let back = read(&write(&d, "t")).unwrap();
        let kinds = |s: &StitchDesign| s.records.iter().map(|r| r.kind).collect::<Vec<_>>();
        let stitches = |s: &StitchDesign| {
            s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect::<Vec<_>>()
        };
        assert_eq!(stitches(&back), stitches(&d));
        let count = |s: &StitchDesign, k| kinds(s).iter().filter(|x| **x == k).count();
        // Trims are explicit, so the long jump does not add one.
        assert_eq!(count(&back, StitchKind::Trim), count(&d, StitchKind::Trim));
        assert_eq!(count(&back, StitchKind::ColorChange), 1);
        assert_eq!(back.threads.len(), 2);
        assert_eq!(back.threads[1].color, Rgb(10, 20, 250));
        assert_eq!(back.records.last().unwrap().kind, StitchKind::End);
    }

    #[test]
    fn short_file_is_an_error() {
        assert_eq!(read(&[0u8; 100]), Err(TbfError::TooShort));
    }
}

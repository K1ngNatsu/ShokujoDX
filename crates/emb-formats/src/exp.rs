//! Melco / Bernina EXP read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/ExpWriter.py`: `write` (lines 11-42; `MAX_JUMP_DISTANCE` 127)
//! - `pyembroidery/ExpReader.py`: `read_exp_stitches` (4-37)
//!
//! Format summary: no header, no colors. Each record is two signed bytes
//! (dx, -dy) in 0.1 mm, Y up in the file. `80 xx` starts a command with two
//! more bytes: `80 04 dx dy` jump, `80 80 07 00` trim, `80 01 00 00` color
//! change. A stop is written as a color change.

use emb_model::StitchDesign;

use crate::common::{Low, Pattern, encode, signed8, to_design};

/// Largest move of one record, per axis, in 0.1 mm.
pub const MAX_DELTA: i32 = 127;

pub fn write(design: &StitchDesign) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut xx, mut yy) = (0, 0);
    for (kind, x, y) in encode(design, MAX_DELTA, true) {
        let (dx, dy) = (x - xx, y - yy);
        (xx, yy) = (x, y);
        match kind {
            Low::Stitch => out.extend([dx as u8, (-dy) as u8]),
            Low::Jump => out.extend([0x80, 0x04, dx as u8, (-dy) as u8]),
            Low::Trim => out.extend([0x80, 0x80, 0x07, 0x00]),
            Low::ColorChange | Low::Stop => out.extend([0x80, 0x01, 0x00, 0x00]),
            Low::Slow | Low::Fast => {}
            Low::End => {}
        }
    }
    out
}

/// Reads a STX file (Data Stitch): the EXP records from the offset at
/// 0x14 of its header (`pyembroidery/StxReader.py`).
pub fn read_stx(bytes: &[u8]) -> StitchDesign {
    let at = bytes
        .get(0x14..0x18)
        .map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]).max(0) as usize);
    read(bytes.get(at..).unwrap_or(&[]))
}

/// Reads an EXP file. It has no colors, so `threads` is empty.
pub fn read(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    let mut it = bytes.as_chunks::<2>().0.iter();
    while let Some(&[b0, b1]) = it.next() {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        let Some(&[c0, c1]) = it.next() else { break };
        let (x, y) = (signed8(c0), -signed8(c1));
        match b1 {
            0x80 => p.here(Low::Trim),
            0x02 => p.stitch(x, y),
            0x04 => p.jump(x, y),
            0x01 => {
                p.here(Low::ColorChange);
                if x != 0 || y != 0 {
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
    use emb_model::StitchKind;

    use super::*;
    use crate::common::tests::sample;

    #[test]
    fn bytes_of_simple_records() {
        let b = write(&sample());
        // Jump -50, -50 (Y up in the file: +50), then a stitch of 0.
        assert_eq!(&b[..6], &[0x80, 0x04, 0xce, 0x32, 0x00, 0x00]);
        assert!(b.windows(4).any(|w| w == [0x80, 0x80, 0x07, 0x00]));
        assert!(b.windows(4).any(|w| w == [0x80, 0x01, 0x00, 0x00]));
    }

    #[test]
    fn round_trip() {
        let d = sample();
        let back = read(&write(&d));
        // The long jump (45 mm, 35 mm) is three jump records in the file.
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
        assert_eq!(count(&back, StitchKind::Trim), 3);
        assert_eq!(count(&back, StitchKind::ColorChange), 1);
        assert_eq!(back.records.last().unwrap().kind, StitchKind::End);
    }
}

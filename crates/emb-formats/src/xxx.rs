//! Singer XXX read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/XxxWriter.py`: `write_xxx_header_b` (10-40),
//!   `write_xxx_stitches` (64-106), `write_xxx_colors` (109-123), `write`
//!   (126-139); `MAX_JUMP_DISTANCE` 124
//! - `pyembroidery/XxxReader.py`: `read` (9-51)
//!
//! Format summary: a 256-byte little-endian header (record count, color
//! count, size, end position), then the stitches from 0x100. A record is two
//! signed bytes (dx, -dy) in 0.1 mm; `7D x16 y16` is a long move; `7F 01`
//! jump, `7F 03` trim, `7F 08` color change, each with dx and dy; `7F 7F 02 14`
//! is the end. Then the colors: `00 r g b` for each thread.
//!
//! The writer makes a long stitch a `7D` record, and the reader makes a `7D`
//! record a jump, as in pyembroidery. The stitch plan has no stitch longer
//! than 12.4 mm, so this does not occur in practice.

use emb_model::{Rgb, StitchDesign};

use crate::common::{Bytes, Low, Pattern, color_thread, encode, signed8, signed16, to_design};

/// Largest move of one record, per axis, in 0.1 mm.
pub const MAX_DELTA: i32 = 124;
const STITCH_START: usize = 0x100;

pub fn write(design: &StitchDesign) -> Vec<u8> {
    let recs = encode(design, MAX_DELTA, false);
    let u16le = |out: &mut Vec<u8>, v: i32| out.extend([v as u8, (v >> 8) as u8]);
    let u32le = |out: &mut Vec<u8>, v: i32| out.extend(v.to_le_bytes());
    let zeros = |out: &mut Vec<u8>, n: usize| out.extend(std::iter::repeat_n(0u8, n));
    let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for &(_, x, y) in &recs {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    let last = recs.last().map_or((0, 0), |r| (r.1, r.2));

    let mut out = Vec::new();
    zeros(&mut out, 0x17);
    u32le(&mut out, recs.len() as i32 - 1);
    zeros(&mut out, 0x0c);
    u32le(&mut out, design.threads.len() as i32);
    u16le(&mut out, 0);
    u16le(&mut out, right - left);
    u16le(&mut out, bottom - top);
    u16le(&mut out, last.0);
    u16le(&mut out, -last.1);
    u16le(&mut out, -left);
    u16le(&mut out, bottom);
    zeros(&mut out, 0x42);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    zeros(&mut out, 0x73);
    u16le(&mut out, 0x20);
    zeros(&mut out, 0x08);
    // The end of the stitches, written below.
    let end_at = out.len();
    u32le(&mut out, 0);
    debug_assert_eq!(out.len(), STITCH_START);

    let (mut xx, mut yy) = (0, 0);
    for (kind, x, y) in recs {
        let (dx, dy) = (x - xx, y - yy);
        (xx, yy) = (x, y);
        match kind {
            Low::ColorChange | Low::Stop => out.extend([0x7f, 0x08, dx as u8, (-dy) as u8]),
            Low::End => break,
            Low::Stitch if -124 < dx && dx < 124 && -124 < dy && dy < 124 => out.extend([dx as u8, (-dy) as u8]),
            Low::Stitch => {
                out.push(0x7d);
                u16le(&mut out, dx);
                u16le(&mut out, -dy);
            }
            Low::Trim => out.extend([0x7f, 0x03, dx as u8, (-dy) as u8]),
            Low::Jump => out.extend([0x7f, 0x01, dx as u8, (-dy) as u8]),
            Low::Slow | Low::Fast => {}
        }
    }
    let end = out.len() as i32;
    out[end_at..end_at + 4].copy_from_slice(&end.to_le_bytes());
    out.extend([0x7f, 0x7f, 0x02, 0x14]);

    out.extend([0, 0]);
    for t in &design.threads {
        out.extend([0, t.color.0, t.color.1, t.color.2]);
    }
    for _ in design.threads.len()..21 {
        u32le(&mut out, 0);
    }
    u32le(&mut out, 0xffff_ff00_u32 as i32);
    out.extend([0x00, 0x01]);
    out
}

pub fn read(bytes: &[u8]) -> StitchDesign {
    let mut b = Bytes::new(bytes);
    b.seek(0x27);
    let colors = b.u16le().unwrap_or(0) as usize;
    b.seek(STITCH_START);
    let mut p = Pattern::default();
    while let Some(b1) = b.u8() {
        if b1 == 0x7d || b1 == 0x7e {
            let (Some(x), Some(y)) = (b.u16le(), b.u16le()) else {
                break;
            };
            p.jump(signed16(x), -signed16(y));
            continue;
        }
        let Some(b2) = b.u8() else { break };
        if b1 != 0x7f {
            p.stitch(signed8(b1), -signed8(b2));
            continue;
        }
        let (Some(b3), Some(b4)) = (b.u8(), b.u8()) else { break };
        let (x, y) = (signed8(b3), -signed8(b4));
        match b2 {
            0x01 => p.jump(x, y),
            0x03 => {
                p.here(Low::Trim);
                if x != 0 || y != 0 {
                    p.jump(x, y);
                }
            }
            0x08 | 0x0a..=0x17 => p.here(Low::ColorChange),
            0x7f | 0x18 => break,
            _ => {}
        }
    }
    b.skip(2);
    let mut threads = Vec::new();
    for i in 0..colors {
        let Some(c) = b.u32be() else { break };
        threads.push(color_thread(i, Rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)));
    }
    to_design(p.recs, threads)
}

#[cfg(test)]
mod tests {
    use emb_model::StitchKind;

    use super::*;
    use crate::common::tests::sample;

    #[test]
    fn header_and_colors() {
        let d = sample();
        let b = write(&d);
        let u32at = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        assert_eq!(u32at(0x27), 2, "colors");
        let end = u32at(0xfc) as usize;
        assert_eq!(&b[end..end + 4], &[0x7f, 0x7f, 0x02, 0x14]);
        assert_eq!(&b[end + 6..end + 10], &[0, 200, 30, 40]);
        assert_eq!(b.len(), end + 4 + 2 + 21 * 4 + 4 + 2);
    }

    #[test]
    fn round_trip() {
        let d = sample();
        let back = read(&write(&d));
        let pick = |s: &StitchDesign, k: StitchKind| {
            s.records
                .iter()
                .filter(|r| r.kind == k)
                .map(|r| r.pos)
                .collect::<Vec<_>>()
        };
        assert_eq!(pick(&back, StitchKind::Stitch), pick(&d, StitchKind::Stitch));
        assert_eq!(pick(&back, StitchKind::Trim), pick(&d, StitchKind::Trim));
        assert_eq!(
            back.threads.iter().map(|t| t.color).collect::<Vec<_>>(),
            [Rgb(200, 30, 40), Rgb(10, 20, 250)]
        );
    }
}

//! Barudan DSB read (read only). BAI and Ricoma machines also read DSB.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/DsbReader.py`: `b_stitch_encoding_read` (lines 4-40),
//!   `read` (43-45), which reads the 512-byte DST header first
//!
//! Format summary: the 512-byte DST header, then 3-byte records
//! `command, dy, dx`. The movement is 0 to 255 units of 0.1 mm per axis;
//! bit 6 of the command makes Y go up, bit 5 makes X go left. Command low
//! bits 0 is a stitch and 1 a jump; `E7` trim, `E8` stop, `E9`-`F7` needle
//! 1-15, `F8` end.
//!
//! pyembroidery has no DSB writer, so DSB is read only here.

use emb_model::StitchDesign;

use crate::common::{Low, Pattern, to_design};
use crate::dst::DstError;

const HEADER_SIZE: usize = 512;

/// Reads a DSB file. It has no colors, so `threads` is empty. A needle
/// change after the first stitch is a color change.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, DstError> {
    if bytes.len() < HEADER_SIZE {
        return Err(DstError::TooShort);
    }
    let mut p = Pattern::default();
    let mut stitched = false;
    for c in bytes[HEADER_SIZE..].as_chunks::<3>().0 {
        let ctrl = c[0];
        let (mut x, mut y) = (i32::from(c[2]), -i32::from(c[1]));
        if ctrl & 0x40 != 0 {
            y = -y;
        }
        if ctrl & 0x20 != 0 {
            x = -x;
        }
        match ctrl {
            _ if ctrl & 0x1f == 0 => {
                p.stitch(x, y);
                stitched = true;
            }
            _ if ctrl & 0x1f == 1 => p.jump(x, y),
            0xf8 => break,
            0xe7 => p.here(Low::Trim),
            0xe8 => p.here(Low::Stop),
            0xe9..=0xf7 => {
                if stitched {
                    p.here(Low::ColorChange);
                }
            }
            _ => break,
        }
    }
    Ok(to_design(p.recs, Vec::new()))
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::StitchKind;

    use super::*;

    #[test]
    fn reads_records() {
        let mut b = vec![b' '; HEADER_SIZE];
        b.extend([
            0xe9, 0, 0, // needle 1 before the first stitch: not a color change
            0x01, 20, 10, // jump +1.0 mm X, -2.0 mm Y (Y down)
            0x00, 0, 5, // stitch +0.5 mm X
            0x60, 5, 5, // stitch -0.5 mm X, +0.5 mm Y
            0xe7, 0, 0, // trim
            0xea, 0, 0, // needle 2
            0x00, 0, 1, // stitch
            0xf8, 0, 0, // end
            0x00, 0, 9, // after the end: not read
        ]);
        let d = read(&b).unwrap();
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        use StitchKind::*;
        assert_eq!(kinds, [Jump, Stitch, Stitch, Trim, ColorChange, Stitch, End]);
        assert_eq!(d.records[0].pos, Point::new(1.0, -2.0));
        assert_eq!(d.records[2].pos, Point::new(1.0, -1.5));
        assert_eq!(d.records[5].pos, Point::new(1.1, -1.5));
        assert_eq!(read(&[0; 10]), Err(DstError::TooShort));
    }
}

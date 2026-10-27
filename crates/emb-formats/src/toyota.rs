//! Toyota 10O and 100 read (read only).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/A10oReader.py`: `read_10o_sttiches` (lines 1-37)
//! - `pyembroidery/A100Reader.py`: `read_100_stitches` (1-26)
//!
//! Format summary: no header and no colors. Units are 0.1 mm.
//! - 10O: 3-byte records `command, dy, dx` with 0 to 255 units per axis;
//!   bit 6 of the command makes Y go up, bit 5 makes X go left. Command low
//!   bits 0 is a stitch and `10` a jump; `8A` start, `85` color change,
//!   `82` stop, `81` trim, `87` end.
//! - 100: 4-byte records `command, ?, dx, dy`; a value above 0x80 is
//!   negative (sign and size, not two's complement), Y up. Command `61` is
//!   a stitch, an odd command a jump, any other a color change.
//!
//! pyembroidery has no writer for these formats, so they are read only here.

use emb_model::StitchDesign;

use crate::common::{Low, Pattern, to_design};

/// Reads a 10O file. It has no colors, so `threads` is empty.
pub fn read_10o(bytes: &[u8]) -> StitchDesign {
    let mut p = Pattern::default();
    for c in bytes.as_chunks::<3>().0 {
        let ctrl = c[0];
        let (mut x, mut y) = (i32::from(c[2]), -i32::from(c[1]));
        if ctrl & 0x20 != 0 {
            x = -x;
        }
        if ctrl & 0x40 != 0 {
            y = -y;
        }
        match ctrl {
            _ if ctrl & 0x1f == 0 => p.stitch(x, y),
            _ if ctrl & 0x1f == 0x10 => p.jump(x, y),
            0x8a => {}
            0x85 => p.here(Low::ColorChange),
            0x82 => p.here(Low::Stop),
            0x81 => p.here(Low::Trim),
            _ => break,
        }
    }
    to_design(p.recs, Vec::new())
}

/// Reads a 100 file. It has no colors, so `threads` is empty.
pub fn read_100(bytes: &[u8]) -> StitchDesign {
    // Sign and size: a byte above 0x80 is negative.
    let value = |b: u8| {
        let v = i32::from(b);
        if v > 0x80 { -(v - 0x80) } else { v }
    };
    let mut p = Pattern::default();
    for c in bytes.as_chunks::<4>().0 {
        let (x, y) = (value(c[2]), -value(c[3]));
        if c[0] == 0x61 {
            p.stitch(x, y);
        } else if c[0] & 0x01 != 0 {
            p.jump(x, y);
        } else {
            p.here(Low::ColorChange);
        }
    }
    to_design(p.recs, Vec::new())
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::StitchKind;

    use super::*;
    use StitchKind::*;

    fn kinds(d: &StitchDesign) -> Vec<StitchKind> {
        d.records.iter().map(|r| r.kind).collect()
    }

    #[test]
    fn reads_10o_records() {
        let d = read_10o(&[
            0x8a, 0, 0, // start
            0x10, 20, 10, // jump +1.0 mm X, -2.0 mm Y (Y down)
            0x00, 0, 5, // stitch +0.5 mm X
            0x60, 5, 5, // stitch -0.5 mm X, +0.5 mm Y
            0x81, 0, 0, // trim
            0x85, 0, 0, // color change
            0x00, 0, 1, // stitch
            0x87, 0, 0, // end
            0x00, 0, 9, // after the end: not read
        ]);
        assert_eq!(kinds(&d), [Jump, Stitch, Stitch, Trim, ColorChange, Stitch, End]);
        assert_eq!(d.records[0].pos, Point::new(1.0, -2.0));
        assert_eq!(d.records[2].pos, Point::new(1.0, -1.5));
        assert_eq!(d.records[5].pos, Point::new(1.1, -1.5));
    }

    #[test]
    fn reads_100_records() {
        let d = read_100(&[
            0x01, 0, 10, 20, // jump +1.0 mm X, -2.0 mm Y (Y down)
            0x61, 0, 5, 0, // stitch +0.5 mm X
            0x61, 0, 0x85, 0x85, // stitch -0.5 mm X, +0.5 mm Y
            0x00, 0, 0, 0, // color change
            0x61, 0, 1, 0, // stitch
        ]);
        assert_eq!(kinds(&d), [Jump, Stitch, Stitch, ColorChange, Stitch, End]);
        assert_eq!(d.records[2].pos, Point::new(1.0, -1.5));
        assert_eq!(d.records[4].pos, Point::new(1.1, -1.5));
    }
}

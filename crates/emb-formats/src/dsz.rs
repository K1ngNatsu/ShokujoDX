//! ZSK DSZ read (read only).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/DszReader.py`: `z_stitch_encoding_read` (lines 4-38),
//!   `read` (41-43), which reads the 512-byte DST header first
//!
//! Format summary: the 512-byte DST header, then 3-byte records
//! `dy, dx, command`. The movement is 0 to 255 units of 0.1 mm per axis;
//! bit 5 of the command makes Y go up, bit 6 makes X go left. Command low
//! bits 0 is a stitch and 1 a jump; `82` stop, `9B` trim, `83`-`9A` needle
//! changes. The file has no end record.
//!
//! pyembroidery has no DSZ writer, so DSZ is read only here.
//!
//! FXY (Fortron) and GT (Gold Thread) files have the same records after a
//! header of 0x100 and 0x200 bytes (`pyembroidery/FxyReader.py`,
//! `pyembroidery/GtReader.py`).

use emb_model::StitchDesign;

use crate::common::{Low, Pattern, to_design};
use crate::dst::DstError;

const HEADER_SIZE: usize = 512;

/// Reads a DSZ file. It has no colors, so `threads` is empty. A needle
/// change after the first stitch is a color change.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, DstError> {
    read_from(bytes, HEADER_SIZE)
}

/// Reads a FXY file (the DSZ records after a 0x100-byte header).
pub fn read_fxy(bytes: &[u8]) -> Result<StitchDesign, DstError> {
    read_from(bytes, 0x100)
}

/// Reads a GT file (the DSZ records after a 0x200-byte header).
pub fn read_gt(bytes: &[u8]) -> Result<StitchDesign, DstError> {
    read_from(bytes, 0x200)
}

fn read_from(bytes: &[u8], header: usize) -> Result<StitchDesign, DstError> {
    if bytes.len() < header {
        return Err(DstError::TooShort);
    }
    let mut p = Pattern::default();
    let mut stitched = false;
    for c in bytes[header..].as_chunks::<3>().0 {
        let ctrl = c[2];
        let (mut x, mut y) = (i32::from(c[1]), -i32::from(c[0]));
        if ctrl & 0x40 != 0 {
            x = -x;
        }
        if ctrl & 0x20 != 0 {
            y = -y;
        }
        match ctrl {
            _ if ctrl & 0x1f == 0 => {
                p.stitch(x, y);
                stitched = true;
            }
            _ if ctrl & 0x1f == 1 => p.jump(x, y),
            0x82 => p.here(Low::Stop),
            0x9b => p.here(Low::Trim),
            0x83..=0x9a => {
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
            0, 0, 0x83, // needle before the first stitch: not a color change
            20, 10, 0x01, // jump +1.0 mm X, -2.0 mm Y (Y down)
            0, 5, 0x00, // stitch +0.5 mm X
            5, 5, 0x60, // stitch -0.5 mm X, +0.5 mm Y
            0, 0, 0x9b, // trim
            0, 0, 0x85, // needle change
            0, 1, 0x80, // stitch (low bits 0)
            0, 0, 0x82, // stop
        ]);
        let d = read(&b).unwrap();
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        use StitchKind::*;
        assert_eq!(kinds, [Jump, Stitch, Stitch, Trim, ColorChange, Stitch, Stop, End]);
        assert_eq!(d.records[0].pos, Point::new(1.0, -2.0));
        assert_eq!(d.records[2].pos, Point::new(1.0, -1.5));
        assert_eq!(d.records[5].pos, Point::new(1.1, -1.5));
    }
}

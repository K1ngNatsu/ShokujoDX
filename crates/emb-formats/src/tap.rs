//! Happy TAP read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/TapReader.py`: `read` (line 4), which reads the file with
//!   `DstReader.dst_read_stitches` (67-100)
//!
//! Format summary: the 3-byte records of DST with no header. pyembroidery
//! has no TAP writer; the writer here is the DST writer (`dst`) without the
//! 512-byte header, so the records are the same as in a DST file of the
//! design. As in DST, a trim is three jumps, and the file has no colors.

use emb_model::StitchDesign;

pub fn write(design: &StitchDesign) -> Vec<u8> {
    write_with(design, crate::dst::TrimCode::default())
}

/// As [`write()`], with the trim code of the machine.
pub fn write_with(design: &StitchDesign, trim: crate::dst::TrimCode) -> Vec<u8> {
    crate::dst::body(design, trim)
}

/// Reads a TAP file. It has no colors, so `threads` is empty.
pub fn read(bytes: &[u8]) -> StitchDesign {
    crate::dst::read_body(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::tests::sample;

    #[test]
    fn records_are_dst_without_the_header() {
        let d = sample();
        let tap = write(&d);
        let dst = crate::dst::write(&d, "t");
        assert_eq!(tap, dst[512..]);
        assert_eq!(read(&tap), crate::dst::read(&dst).unwrap().design);
    }
}

//! Janome SEW read (read only).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/SewReader.py`: `read_sew_stitches` (5-27), `read` (30-39)
//! - `pyembroidery/EmbThreadSew.py`: the thread table (`tables::SEW_THREADS`)
//!
//! Format summary: a little-endian color count and one table index per
//! color; the stitches start at 0x1D78. A record is two signed bytes
//! (dx, -dy) in 0.1 mm; `80 xx` starts a command with two more bytes: odd
//! `xx` is a color change, `02` or `04` a jump, `10` a stitch.

use emb_model::StitchDesign;

use crate::common::{Bytes, Low, Pattern, signed8, table_thread, to_design};
use crate::tables::SEW_THREADS;

const STITCH_START: usize = 0x1d78;

pub fn read(bytes: &[u8]) -> StitchDesign {
    let mut b = Bytes::new(bytes);
    let colors = b.u16le().unwrap_or(0);
    let mut threads = Vec::new();
    for i in 0..usize::from(colors) {
        let Some(index) = b.u16le() else { break };
        let index = usize::from(index) % SEW_THREADS.len();
        threads.push(table_thread(i, SEW_THREADS[index], "Janome"));
    }
    b.seek(STITCH_START);
    let mut p = Pattern::default();
    while let (Some(b0), Some(b1)) = (b.u8(), b.u8()) {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        let (Some(c0), Some(c1)) = (b.u8(), b.u8()) else { break };
        match b1 {
            c if c & 1 == 1 => p.here(Low::ColorChange),
            0x04 | 0x02 => p.jump(signed8(c0), -signed8(c1)),
            0x10 => p.stitch(signed8(c0), -signed8(c1)),
            _ => break,
        }
    }
    to_design(p.recs, threads)
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::StitchKind;

    use super::*;

    /// A file made from the reader's rules (no SEW sample file is public).
    #[test]
    fn reads_a_built_file() {
        let mut f = vec![0u8; STITCH_START];
        f[0..2].copy_from_slice(&2u16.to_le_bytes());
        f[2..4].copy_from_slice(&10u16.to_le_bytes()); // Red
        f[4..6].copy_from_slice(&2u16.to_le_bytes()); // White
        f.extend([10, 0xfb]); // stitch +1.0 mm, +0.5 mm down (Y up in the file)
        f.extend([0x80, 0x02, 50, 0]); // jump +5 mm
        f.extend([0x80, 0x01, 0, 0]); // color change
        f.extend([0x80, 0x10, 0xf6, 0]); // stitch -1 mm
        f.extend([0x80, 0x98, 0, 0]); // unknown (even): the end
        let d = read(&f);
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                StitchKind::Stitch,
                StitchKind::Jump,
                StitchKind::ColorChange,
                StitchKind::Stitch,
                StitchKind::End
            ]
        );
        assert_eq!(d.records[0].pos, Point::new(1.0, 0.5));
        assert_eq!(d.records[3].pos, Point::new(5.0, 0.5));
        assert_eq!(d.threads[0].name, "Red");
        assert_eq!(d.threads[1].name, "White");
    }
}

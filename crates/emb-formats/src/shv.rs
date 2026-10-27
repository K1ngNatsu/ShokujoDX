//! Husqvarna Viking SHV read (read only).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/ShvReader.py`: `read` (15-79)
//! - `pyembroidery/EmbThreadShv.py`: the thread table (`tables::SHV_THREADS`)
//!
//! Format summary: a text header, the design name, a small preview picture,
//! then for each color the big-endian stitch count and a table index. The
//! stitches follow: two signed bytes (dx, dy) in 0.1 mm, Y down. `80 01`
//! with two big-endian 16-bit moves starts a jump; the moves after it are
//! jumps until `80 02`. A color ends after its stitch count.

use emb_model::StitchDesign;

use crate::common::{Bytes, Low, Pattern, signed8, signed16, table_thread, to_design};
use crate::tables::SHV_THREADS;

pub fn read(bytes: &[u8]) -> StitchDesign {
    let mut b = Bytes::new(bytes);
    b.seek(0x56);
    let name_len = usize::from(b.u8().unwrap_or(0));
    b.skip(name_len);
    let (w, h) = (usize::from(b.u8().unwrap_or(0)), usize::from(b.u8().unwrap_or(0)));
    // The preview picture: two pixels per byte.
    b.skip(4 + h.div_ceil(2) * w);
    let colors = usize::from(b.u8().unwrap_or(0));
    b.skip(18);
    let mut threads = Vec::new();
    let mut per_color = Vec::new();
    for i in 0..colors {
        let (Some(count), Some(code)) = (b.u32be(), b.u8()) else {
            break;
        };
        threads.push(table_thread(
            i,
            SHV_THREADS[usize::from(code) % SHV_THREADS.len()],
            "Husqvarna Viking",
        ));
        per_color.push(u64::from(count));
        b.skip(9);
    }
    b.at = b.at.saturating_sub(2);

    let mut p = Pattern::default();
    let mut in_jump = false;
    let mut since_stop: u64 = 0;
    let mut color = 0;
    let mut max = per_color.first().copied().unwrap_or(0);
    while let (Some(b0), Some(b1)) = (b.u8(), b.u8()) {
        let kind = if in_jump { Low::Jump } else { Low::Stitch };
        if since_stop >= max {
            p.here(Low::ColorChange);
            since_stop = 0;
            color += 1;
            max = per_color.get(color).copied().unwrap_or(u64::from(u32::MAX));
        }
        if b0 == 0x80 {
            since_stop += 1;
            match b1 {
                3 => continue,
                2 => {
                    in_jump = false;
                    continue;
                }
                1 => {
                    since_stop += 2;
                    let (Some(x), Some(y)) = (b.u16be(), b.u16be()) else {
                        break;
                    };
                    in_jump = true;
                    p.jump(signed16(x), signed16(y));
                    continue;
                }
                _ => {}
            }
        }
        since_stop += 1;
        p.add(kind, signed8(b0), signed8(b1));
    }
    to_design(p.recs, threads)
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::StitchKind;

    use super::*;

    /// A file made from the reader's rules (no SHV sample file is public).
    #[test]
    fn reads_a_built_file() {
        let mut f = vec![0u8; 0x56];
        f.push(4);
        f.extend(b"test");
        f.extend([2, 3]); // a 2 x 3 preview: 2 * 2 bytes
        f.extend([0; 4 + 4]);
        f.push(2); // colors
        f.extend([0; 18]);
        // Color 1: 3 records, code 3 (Red). Color 2: many, code 1 (Blue).
        f.extend(3u32.to_be_bytes());
        f.push(3);
        f.extend([0; 9]);
        f.extend(100u32.to_be_bytes());
        f.push(1);
        f.extend([0; 9]);
        f.truncate(f.len() - 2);
        f.extend([10, 5]); // stitch +1.0, +0.5 (Y down)
        f.extend([10, 0]);
        f.extend([0x80, 0x03]); // ignored, but counts
        f.extend([0x80, 0x01, 0, 50, 0, 0]); // jump +5 mm (second color)
        f.extend([1, 0]); // a jump, because a jump is open
        f.extend([0x80, 0x02]); // the jump ends
        f.extend([0xf6, 0]); // stitch -1 mm
        let d = read(&f);
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                StitchKind::Stitch,
                StitchKind::Stitch,
                StitchKind::ColorChange,
                StitchKind::Jump,
                StitchKind::Jump,
                StitchKind::Stitch,
                StitchKind::End
            ]
        );
        assert_eq!(d.records[0].pos, Point::new(1.0, 0.5));
        assert_eq!(d.records[4].pos, Point::new(7.1, 0.5));
        assert_eq!(d.records[5].pos, Point::new(6.1, 0.5));
        assert_eq!(d.threads[0].name, "Red");
        assert_eq!(d.threads[1].name, "Blue");
    }
}

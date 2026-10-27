//! Janome JEF read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/JefWriter.py`: `write` (lines 21-169), `get_jef_hoop_size`
//!   (172-181), `write_hoop_edge_distance` (184-194); default settings
//!   (`trims` off, `MAX_JUMP_DISTANCE` 127)
//! - `pyembroidery/JefReader.py`: `read_jef_stitches` (5-52), `read` (55-71)
//! - `pyembroidery/EmbPattern.py`: `bounds`, `fix_color_count`
//! - `pyembroidery/EmbThreadJef.py`: the thread table (`tables::JEF_THREADS`)
//!
//! Format summary: a 116-byte little-endian header (stitch offset, date,
//! color count, point count, hoop code, extents and hoop edge distances),
//! then one palette index and one `0x0D` per color, then the stitches.
//! Records are two signed bytes (dx, -dy) in 0.1 mm; `80 02 dx dy` is a
//! jump, `80 01 dx dy` a color change, `80 10` the end. Palette index 0
//! marks a stop. JEF has no trim: the machine cuts the thread at a long
//! jump, and the reader makes a jump longer than 3 mm into a trim.
//!
//! Deviations from pyembroidery, on purpose:
//! - A missing thread (more color blocks than threads) is black, not a
//!   random color, so the output is the same each time.
//! - The date comes from the caller, so tests can fix it.

use emb_model::{Rgb, StitchDesign, Thread};

use crate::common::{Bytes, Low, Pattern, encode, interpolate_trims, signed8, table_thread, to_design};
use crate::pes::{half_round_even, nearest};
use crate::tables::JEF_THREADS;

/// Largest move of one record, per axis, in 0.1 mm.
pub const MAX_DELTA: i32 = 127;
const HEADER_SIZE: usize = 0x74;
const BRAND: &str = "Janome";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JefError {
    #[error("file is shorter than the JEF header")]
    TooShort,
}

fn table_colors() -> Vec<Option<Rgb>> {
    JEF_THREADS
        .iter()
        .map(|t| t.map(|(r, g, b, _, _)| Rgb(r, g, b)))
        .collect()
}

/// The same thread in the sense of `EmbThread.__eq__`.
fn same(a: &Thread, b: &Thread) -> bool {
    a.color == b.color && a.name == b.name && a.code == b.code && a.brand == b.brand
}

/// The hoop code for the design size in 0.1 mm (`get_jef_hoop_size`).
fn hoop_code(width: i32, height: i32) -> i32 {
    match (width, height) {
        (w, h) if w < 500 && h < 500 => 1,
        (w, h) if w < 1260 && h < 1100 => 3,
        (w, h) if w < 1400 && h < 2000 => 2,
        (w, h) if w < 2000 && h < 2000 => 4,
        _ => 0,
    }
}

fn hoop_edge(out: &mut Vec<u8>, x: i32, y: i32) {
    let v = if x.min(y) >= 0 { [x, y, x, y] } else { [-1; 4] };
    for i in v {
        out.extend(i.to_le_bytes());
    }
}

/// Writes a JEF file. `date` is `YYYYMMDDhhmmss` (14 digits).
pub fn write(design: &StitchDesign, date: &str) -> Vec<u8> {
    let recs = encode(design, MAX_DELTA, true);

    // `fix_color_count`: one thread for each color block that has stitches.
    let mut blocks = 0;
    let mut init = true;
    for r in &recs {
        match r.0 {
            Low::Stitch if init => {
                blocks += 1;
                init = false;
            }
            Low::ColorChange => init = true,
            _ => {}
        }
    }
    let mut threads = design.threads.clone();
    while threads.len() < blocks {
        threads.push(crate::common::color_thread(threads.len(), Rgb(0, 0, 0)));
    }

    // The palette: the nearest table color for each color block, never the
    // same index twice in a row for different threads; 0 marks a stop.
    let mut table = table_colors();
    let mut palette: Vec<i32> = Vec::new();
    let (mut last_index, mut last_thread): (Option<usize>, Option<&Thread>) = (None, None);
    let mut toggled = false;
    let mut next = 0;
    for r in &recs {
        if r.0 == Low::ColorChange || next == 0 {
            let Some(thread) = threads.get(next) else { break };
            next += 1;
            let mut index = nearest(thread.color, &table);
            if index.is_some() && index == last_index && last_thread.is_some_and(|t| !same(t, thread)) {
                let i = index.unwrap_or(0);
                let saved = table[i].take();
                index = nearest(thread.color, &table);
                table[i] = saved;
            }
            let index = index.unwrap_or(0);
            palette.push(index as i32);
            last_index = Some(index);
            last_thread = Some(thread);
            toggled = false;
        }
        if r.0 == Low::Stop {
            toggled = !toggled;
            palette.push(if toggled { 0 } else { last_index.unwrap_or(0) as i32 });
        }
    }
    let color_count = palette.len() as i32;

    let mut points = 1;
    for r in &recs {
        points += match r.0 {
            Low::Stitch => 1,
            Low::Jump | Low::ColorChange | Low::Stop => 2,
            Low::Trim | Low::Slow | Low::Fast => 0,
            Low::End => break,
        };
    }
    let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for &(_, x, y) in &recs {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    let (width, height) = (right - left, bottom - top);
    let (half_w, half_h) = (half_round_even(width), half_round_even(height));

    let mut out = Vec::new();
    let int = |out: &mut Vec<u8>, v: i32| out.extend(v.to_le_bytes());
    int(&mut out, HEADER_SIZE as i32 + color_count * 8);
    int(&mut out, 0x14);
    let date: String = date.chars().chain(std::iter::repeat('0')).take(14).collect();
    out.extend(date.as_bytes().iter().take(14));
    out.extend([0, 0]);
    int(&mut out, color_count);
    int(&mut out, points);
    int(&mut out, hoop_code(width, height));
    for v in [half_w, half_h, half_w, half_h] {
        int(&mut out, v);
    }
    hoop_edge(&mut out, 550 - half_w, 550 - half_h);
    hoop_edge(&mut out, 250 - half_w, 250 - half_h);
    hoop_edge(&mut out, 700 - half_w, 1000 - half_h);
    hoop_edge(&mut out, 700 - half_w, 1000 - half_h);
    debug_assert_eq!(out.len(), HEADER_SIZE);
    for p in &palette {
        int(&mut out, *p);
    }
    for _ in 0..color_count {
        int(&mut out, 0x0D);
    }

    let (mut xx, mut yy) = (0, 0);
    for (kind, x, y) in recs {
        let (dx, dy) = (x - xx, y - yy);
        (xx, yy) = (x, y);
        match kind {
            Low::Stitch => out.extend([dx as u8, (-dy) as u8]),
            Low::ColorChange | Low::Stop => out.extend([0x80, 0x01, dx as u8, (-dy) as u8]),
            Low::Trim | Low::Slow | Low::Fast => {}
            Low::Jump => out.extend([0x80, 0x02, dx as u8, (-dy) as u8]),
            Low::End => break,
        }
    }
    out.extend([0x80, 0x10]);
    out
}

/// Reads a JEF file.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, JefError> {
    if bytes.len() < HEADER_SIZE {
        return Err(JefError::TooShort);
    }
    let mut b = Bytes::new(bytes);
    let stitch_offset = b.i32le().unwrap_or(0).max(0) as usize;
    b.skip(20);
    let count = b.i32le().unwrap_or(0).max(0) as usize;
    b.skip(88);
    // `None` marks a stop (palette index 0).
    let mut list: Vec<Option<Thread>> = Vec::new();
    for _ in 0..count {
        let Some(v) = b.i32le() else { break };
        let index = v.unsigned_abs() as usize;
        list.push((index != 0).then(|| table_thread(0, JEF_THREADS[index % JEF_THREADS.len()], BRAND)));
    }

    let mut p = Pattern::default();
    b.seek(stitch_offset);
    let mut color_index = 1;
    while let (Some(b0), Some(b1)) = (b.u8(), b.u8()) {
        if b0 != 0x80 {
            p.stitch(signed8(b0), -signed8(b1));
            continue;
        }
        let (Some(c0), Some(c1)) = (b.u8(), b.u8()) else { break };
        match b1 {
            0x02 => p.jump(signed8(c0), -signed8(c1)),
            0x01 => {
                if matches!(list.get(color_index), Some(None)) {
                    p.here(Low::Stop);
                    list.remove(color_index);
                } else {
                    p.here(Low::ColorChange);
                    color_index += 1;
                }
            }
            _ => break,
        }
    }
    // A jump longer than 3 mm (30 units) is a trim.
    let recs = interpolate_trims(p.recs, None, Some(30));
    let threads = list
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(i, mut t)| {
            t.id = emb_model::ThreadId(i as u32);
            t
        })
        .collect();
    Ok(to_design(recs, threads))
}

#[cfg(test)]
mod tests {
    use emb_model::StitchKind;

    use super::*;
    use crate::common::tests::sample;

    #[test]
    fn header_layout() {
        let b = write(&sample(), "20261021120000");
        let int = |at: usize| i32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        assert_eq!(int(0), 0x74 + 2 * 8, "two colors");
        assert_eq!(int(4), 0x14);
        assert_eq!(&b[8..22], b"20261021120000");
        assert_eq!(int(24), 2);
        // Extents 47 mm x 35 mm (from -5,-5 to 42,30): the 50 x 50 hoop.
        assert_eq!(int(32), 1);
        assert_eq!(hoop_code(1200, 1000), 3);
        assert_eq!(hoop_code(1300, 1900), 2);
        assert_eq!(hoop_code(1900, 1900), 4);
        assert_eq!(hoop_code(2600, 2000), 0);
        assert_eq!([int(36), int(40)], [235, 175]);
        // Palette: the nearest table color of each thread, then 0x0D per color.
        let table = table_colors();
        assert_eq!(int(0x74) as usize, nearest(Rgb(200, 30, 40), &table).unwrap());
        assert_eq!(int(0x78) as usize, nearest(Rgb(10, 20, 250), &table).unwrap());
        assert_eq!(int(0x7c), 0x0d);
        assert_eq!(&b[b.len() - 2..], &[0x80, 0x10]);
    }

    #[test]
    fn round_trip_with_trims_from_long_jumps() {
        let d = sample();
        let back = read(&write(&d, "20000101000000")).unwrap();
        let stitches = |s: &StitchDesign| {
            s.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect::<Vec<_>>()
        };
        assert_eq!(stitches(&back), stitches(&d));
        let count = |k| back.records.iter().filter(|r| r.kind == k).count();
        // The trim before the 45 mm jump comes back from the long jump; the
        // one before the color change and the one at the end do not (JEF
        // has no trims).
        assert_eq!(count(StitchKind::Trim), 1, "{:?}", back.records);
        assert_eq!(count(StitchKind::ColorChange), 1);
        assert_eq!(back.threads.len(), 2);
        assert_eq!(back.threads[0].brand.as_deref(), Some("Janome"));
    }

    #[test]
    fn a_stop_is_palette_index_zero() {
        let mut d = sample();
        d.records.insert(
            4,
            emb_model::StitchRecord::new(emb_geom::Point::new(0.0, -5.0), StitchKind::Stop),
        );
        let b = write(&d, "20000101000000");
        let int = |at: usize| i32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        assert_eq!(int(24), 3);
        assert_eq!(int(0x74 + 4), 0, "the stop");
        let back = read(&b).unwrap();
        assert_eq!(back.records.iter().filter(|r| r.kind == StitchKind::Stop).count(), 1);
        assert_eq!(back.threads.len(), 2);
    }

    #[test]
    fn short_file() {
        assert_eq!(read(&[0; 10]), Err(JefError::TooShort));
    }
}

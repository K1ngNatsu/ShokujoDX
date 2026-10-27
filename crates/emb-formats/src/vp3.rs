//! Husqvarna Viking / Pfaff VP3 read and write.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/Vp3Writer.py`: `write`, `write_file`, `write_design_block`,
//!   `write_vp3_colorblock`, `vp3_write_thread`, `write_stitches_block`
//! - `pyembroidery/Vp3Reader.py`: `read`, `vp3_read_colorblock`, `vp3_read_thread`
//! - `pyembroidery/EmbPattern.py`: `bounds`, `get_as_colorblocks`
//!
//! Format summary: big-endian binary blocks. Each block starts with a
//! 3-byte tag and a 32-bit length of the rest of the block. A file block
//! holds one design block, which holds one color block per thread. Header
//! positions are in 0.001 mm with Y up. Stitches are moves in 0.1 mm with
//! Y down: two signed bytes, or `80 01`, two signed 16-bit values, `80 02`
//! for a long move. `80 03` is a trim. VP3 has no jump record: a jump
//! becomes part of the move of the next stitch. A color block starts with
//! the thread (color, catalog number, description, brand).
//!
//! The reader makes the first stitch of each color block, and the first
//! stitch after a trim, into a jump plus a stitch, because the thread is
//! not connected there. So a design from the stitch plan reads back with
//! the same records.
//!
//! Deviations from pyembroidery, on purpose:
//! - The design center is an integer in 0.1 mm, and the color block start
//!   positions use the same integer center. pyembroidery cuts the center to
//!   an integer for the header only, so a reader can be 0.1 mm off when the
//!   design width or height is odd.
//! - The reader removes a trim that comes directly after a trim (the
//!   writer adds `80 03` for the end, after the last trim, as pyembroidery does).
//! - VP3 has no stop record. A machine that reads VP3 stops at each color
//!   block, so a stop becomes a new color block with the same thread. A
//!   reader sees it as a color change.
//! - Coordinates are rounded half away from zero, as in `dst`.

use emb_geom::Point;
use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

pub const UNITS_PER_MM: f64 = 10.0;
/// Header positions are in 0.001 mm: file units times this.
const HEADER_SCALE: i32 = 100;
/// Largest move of a short stitch record, per axis, in 0.1 mm.
const SHORT_MAX: i32 = 127;
const MAGIC: &[u8] = b"%vsm%\0";
/// The producer text that pyembroidery writes.
const PRODUCER: &str = "Produced by     Software Ltd";
/// Thread color for a color block with no thread in the palette.
const FILLER: Rgb = Rgb(0, 0, 0);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Vp3Error {
    #[error("not a VP3 file (the file does not start with \"%vsm%\")")]
    BadMagic,
    #[error("the VP3 file ends before the data is complete")]
    Truncated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Low {
    Stitch,
    Jump,
    Trim,
    ColorChange,
    End,
}

fn to_units(p: Point) -> (i32, i32) {
    ((p.x * UNITS_PER_MM).round() as i32, (p.y * UNITS_PER_MM).round() as i32)
}

/// Converts the stitch list to records at absolute positions in file units, Y down.
fn encode(design: &StitchDesign) -> Vec<(Low, i32, i32)> {
    let mut out = Vec::with_capacity(design.records.len() + 1);
    let mut cur = (0, 0);
    for r in &design.records {
        let p = to_units(r.pos);
        match r.kind {
            StitchKind::Stitch => {
                out.push((Low::Stitch, p.0, p.1));
                cur = p;
            }
            StitchKind::Jump => {
                if p != cur {
                    out.push((Low::Jump, p.0, p.1));
                    cur = p;
                }
            }
            StitchKind::Trim => out.push((Low::Trim, cur.0, cur.1)),
            StitchKind::ColorChange => out.push((Low::ColorChange, cur.0, cur.1)),
            // A new color block with the same thread (see the module notes).
            StitchKind::Stop => out.push((Low::ColorChange, cur.0, cur.1)),
            // The format has no speed function.
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::End => break,
        }
    }
    out.push((Low::End, cur.0, cur.1));
    out
}

/// Big-endian writer with placeholders for block lengths.
struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i16(&mut self, v: i32) {
        self.0.extend((v as i16).to_be_bytes());
    }
    fn u16(&mut self, v: usize) {
        self.0.extend((v as u16).to_be_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_be_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    /// Length in bytes, then UTF-8 text (`vp3_write_string_8`).
    fn string_8(&mut self, s: &str) {
        self.u16(s.len());
        self.bytes(s.as_bytes());
    }
    /// Length in bytes, then UTF-16BE text (`vp3_write_string_16`).
    fn string_16(&mut self, s: &str) {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.u16(units.len() * 2);
        for u in units {
            self.0.extend(u.to_be_bytes());
        }
    }
    /// Writes a block tag and a length placeholder. Returns the placeholder position.
    fn begin_block(&mut self, tag: &[u8; 3]) -> usize {
        self.bytes(tag);
        let at = self.0.len();
        self.i32(0);
        at
    }
    /// Sets the placeholder to the number of bytes after it (`vp3_patch_byte_offset`).
    fn end_block(&mut self, at: usize) {
        let len = (self.0.len() - at - 4) as i32;
        self.0[at..at + 4].copy_from_slice(&len.to_be_bytes());
    }
}

fn write_thread(o: &mut Out, thread: Option<&Thread>) {
    // One color, no transition.
    o.bytes(&[0x01, 0x00]);
    let color = thread.map_or(FILLER, |t| t.color);
    o.bytes(&[color.0, color.1, color.2]);
    // No parts, no length, type 5, weight 40 (rayon 40).
    o.bytes(&[0x00, 0x00, 0x00, 0x05, 0x28]);
    let catalog = thread.and_then(|t| t.code.as_deref()).unwrap_or("");
    o.string_8(catalog);
    let description = match thread {
        Some(t) if !t.name.is_empty() => t.name.clone(),
        _ => color.to_hex(),
    };
    o.string_8(&description);
    o.string_8(thread.and_then(|t| t.brand.as_deref()).unwrap_or(""));
}

/// Writes the stitches of one color block, starting from `first`.
fn write_stitches(o: &mut Out, block: &[(Low, i32, i32)], first: (i32, i32)) {
    let at = o.begin_block(b"\x00\x01\x00");
    o.bytes(b"\x0a\xf6\x00");
    let (mut lx, mut ly) = first;
    for &(kind, x, y) in block {
        match kind {
            // The machine does not trim at the end by itself, so the end is a trim.
            Low::End => {
                o.bytes(b"\x80\x03");
                break;
            }
            Low::Trim => o.bytes(b"\x80\x03"),
            // VP3 has no record for these. A jump becomes part of the next stitch move.
            Low::ColorChange | Low::Jump => {}
            Low::Stitch => {
                let (dx, dy) = (x - lx, y - ly);
                (lx, ly) = (x, y);
                if dx.abs() <= SHORT_MAX && dy.abs() <= SHORT_MAX {
                    o.u8(dx as i8 as u8);
                    o.u8(dy as i8 as u8);
                } else {
                    debug_assert!(dx.abs() <= i32::from(i16::MAX) && dy.abs() <= i32::from(i16::MAX));
                    o.bytes(b"\x80\x01");
                    o.i16(dx);
                    o.i16(dy);
                    o.bytes(b"\x80\x02");
                }
            }
        }
    }
    o.end_block(at);
}

/// Writes a VP3 file. Port of `Vp3Writer.write` with default settings.
pub fn write(design: &StitchDesign) -> Vec<u8> {
    let low = encode(design);

    // Bounds of all records, as `EmbPattern.bounds`.
    let (mut left, mut top, mut right, mut bottom) = (0, 0, 0, 0);
    if let Some(&(_, x, y)) = low.first() {
        (left, top, right, bottom) = (x, y, x, y);
    }
    for &(_, x, y) in &low {
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    let (width, height) = (right - left, bottom - top);
    let (half_w, half_h) = (width / 2, height / 2);
    // Integer division cuts toward zero, as Python `int()` does in pyembroidery.
    let (cx, cy) = ((left + right) / 2, (top + bottom) / 2);

    // Color blocks: each color change starts a new block (`get_as_colorblocks`).
    let mut blocks: Vec<&[(Low, i32, i32)]> = Vec::new();
    let mut start = 0;
    for (i, &(kind, _, _)) in low.iter().enumerate() {
        if kind == Low::ColorChange {
            blocks.push(&low[start..i]);
            start = i;
        }
    }
    blocks.push(&low[start..]);
    // The thread of each block: a color change takes the next thread, a stop
    // keeps the thread.
    let mut block_threads = vec![0];
    for r in &design.records {
        match r.kind {
            StitchKind::ColorChange => block_threads.push(block_threads.last().map_or(0, |t| t + 1)),
            StitchKind::Stop => block_threads.push(*block_threads.last().unwrap_or(&0)),
            _ => {}
        }
    }
    let ends = low.iter().filter(|r| r.0 == Low::End).count();

    let mut o = Out(Vec::new());
    o.bytes(MAGIC);
    o.string_16(PRODUCER);

    let file_block = o.begin_block(b"\x00\x02\x00");
    o.string_16(""); // Notes and settings.
    o.i32(right * HEADER_SCALE);
    o.i32(-top * HEADER_SCALE);
    o.i32(left * HEADER_SCALE);
    o.i32(-bottom * HEADER_SCALE);
    o.i32((low.len() - ends) as i32);
    o.u8(0);
    o.u8(blocks.len() as u8);
    o.u8(12);
    o.u8(0);
    o.u8(1); // Number of designs.

    let design_block = o.begin_block(b"\x00\x03\x00");
    o.i32(cx * HEADER_SCALE);
    o.i32(-cy * HEADER_SCALE);
    o.bytes(&[0, 0, 0]);
    o.i32(-half_w * HEADER_SCALE);
    o.i32(half_w * HEADER_SCALE);
    o.i32(-half_h * HEADER_SCALE);
    o.i32(half_h * HEADER_SCALE);
    o.i32(width * HEADER_SCALE);
    o.i32(height * HEADER_SCALE);
    o.string_16(""); // Notes and settings.
    o.bytes(b"\x64\x64");
    o.i32(4096);
    o.i32(0);
    o.i32(0);
    o.i32(4096);
    o.bytes(b"xxPP\x01\x00");
    o.string_16(PRODUCER);
    o.u16(blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        let at = o.begin_block(b"\x00\x05\x00");
        // The first block starts at the origin; the others where the color change is.
        let first = match block.first() {
            Some(&(_, x, y)) if i > 0 => (x, y),
            _ => (0, 0),
        };
        let last = block.last().map_or((0, 0), |&(_, x, y)| (x, y));
        o.i32((first.0 - cx) * HEADER_SCALE);
        o.i32(-(first.1 - cy) * HEADER_SCALE);
        write_thread(&mut o, block_threads.get(i).and_then(|&t| design.threads.get(t)));
        o.i32((last.0 - first.0) * HEADER_SCALE);
        o.i32(-(last.1 - first.1) * HEADER_SCALE);
        write_stitches(&mut o, block, first);
        o.u8(0);
        o.end_block(at);
    }
    o.end_block(design_block);
    o.end_block(file_block);
    o.0
}

/// Big-endian reader that returns `Truncated` at the end of the data.
struct In<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Vp3Error> {
        let end = self.pos.checked_add(n).ok_or(Vp3Error::Truncated)?;
        let s = self.b.get(self.pos..end).ok_or(Vp3Error::Truncated)?;
        self.pos = end;
        Ok(s)
    }
    fn skip(&mut self, n: usize) -> Result<(), Vp3Error> {
        self.take(n).map(|_| ())
    }
    fn u8(&mut self) -> Result<u8, Vp3Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<usize, Vp3Error> {
        let s = self.take(2)?;
        Ok(usize::from(u16::from_be_bytes([s[0], s[1]])))
    }
    fn i32(&mut self) -> Result<i32, Vp3Error> {
        let s = self.take(4)?;
        Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn skip_string(&mut self) -> Result<(), Vp3Error> {
        let n = self.u16()?;
        self.skip(n)
    }
    fn string_8(&mut self) -> Result<String, Vp3Error> {
        let n = self.u16()?;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
}

fn read_thread(r: &mut In, index: usize) -> Result<Thread, Vp3Error> {
    let colors = r.u8()?;
    r.skip(1)?; // Transition.
    let mut color = FILLER;
    for _ in 0..colors {
        let c = r.take(3)?;
        color = Rgb(c[0], c[1], c[2]);
        r.skip(3)?; // Parts, length.
    }
    r.skip(2)?; // Thread type, weight.
    let code = r.string_8()?;
    let description = r.string_8()?;
    let brand = r.string_8()?;
    let some = |s: String| (!s.is_empty()).then_some(s);
    Ok(Thread {
        id: ThreadId(index as u32),
        name: if description.is_empty() {
            color.to_hex()
        } else {
            description
        },
        color,
        brand: some(brand),
        code: some(code),
    })
}

/// Records of one design being read, in file units, Y down.
struct Reader {
    out: Vec<(StitchKind, i32, i32)>,
    cur: (i32, i32),
    /// The thread is not connected: the next stitch starts with a jump.
    loose: bool,
}

impl Reader {
    fn push(&mut self, kind: StitchKind) {
        self.out.push((kind, self.cur.0, self.cur.1));
    }

    fn stitch(&mut self, dx: i32, dy: i32) {
        self.cur = (self.cur.0 + dx, self.cur.1 + dy);
        if self.loose && (dx, dy) != (0, 0) {
            self.push(StitchKind::Jump);
        }
        self.loose = false;
        self.push(StitchKind::Stitch);
    }

    fn trim(&mut self) {
        // Two trims with nothing between them are one trim.
        if self.out.last().map(|r| r.0) != Some(StitchKind::Trim) {
            self.push(StitchKind::Trim);
        }
        self.loose = true;
    }
}

/// Reads one color block (`vp3_read_colorblock`).
fn read_block(r: &mut In, rd: &mut Reader, center: (i32, i32), index: usize) -> Result<Thread, Vp3Error> {
    r.skip(3)?; // 00 05 00
    let len = r.i32()?;
    let block_end = r
        .pos
        .checked_add(usize::try_from(len).map_err(|_| Vp3Error::Truncated)?);
    let block_end = block_end.filter(|&e| e <= r.b.len()).ok_or(Vp3Error::Truncated)?;
    let sx = r.i32()? / HEADER_SCALE;
    let sy = -(r.i32()? / HEADER_SCALE);
    let start = (sx + center.0, sy + center.1);
    if start != rd.cur {
        rd.cur = start;
        rd.push(StitchKind::Jump);
    }
    let thread = read_thread(r, index)?;
    r.skip(15)?; // Block shift (8), 00 01 00, stitch block length (4).
    r.skip(3)?; // 0A F6 00
    let data = r.b.get(r.pos..block_end).ok_or(Vp3Error::Truncated)?;
    r.pos = block_end;

    let s = |i: usize| i32::from(data[i] as i8);
    let s16 = |i: usize| i32::from(i16::from_be_bytes([data[i], data[i + 1]]));
    let mut i = 0;
    while i + 1 < data.len() {
        let (x, y) = (data[i], data[i + 1]);
        i += 2;
        if x != 0x80 {
            rd.stitch(s(i - 2), s(i - 1));
            continue;
        }
        match y {
            0x01 => {
                if i + 4 > data.len() {
                    break;
                }
                rd.stitch(s16(i), s16(i + 2));
                // The `80 02` that follows has no effect.
                i += 6;
            }
            0x03 => rd.trim(),
            _ => {}
        }
    }
    Ok(thread)
}

/// Reads a VP3 file. Port of `Vp3Reader.read`.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, Vp3Error> {
    if !bytes.starts_with(&MAGIC[..5]) {
        return Err(Vp3Error::BadMagic);
    }
    let mut r = In { b: bytes, pos: 6 };
    r.skip_string()?; // Producer.
    r.skip(7)?; // 00 02 00, file block length.
    r.skip_string()?; // Notes and settings.
    r.skip(32)?; // Extents, stitch count, color count, number of designs, 00 03 00, length.
    let cx = r.i32()? / HEADER_SCALE;
    let cy = -(r.i32()? / HEADER_SCALE);
    r.skip(27)?;
    r.skip_string()?;
    r.skip(24)?;
    r.skip_string()?; // Producer.
    let count = r.u16()?;

    let mut rd = Reader {
        out: Vec::new(),
        cur: (0, 0),
        loose: true,
    };
    let mut threads = Vec::with_capacity(count);
    for i in 0..count {
        if i > 0 {
            rd.push(StitchKind::ColorChange);
            rd.loose = true;
        }
        threads.push(read_block(&mut r, &mut rd, (cx, cy), i)?);
    }
    rd.push(StitchKind::End);

    let mm = |v: i32| f64::from(v) / UNITS_PER_MM;
    let records = rd
        .out
        .into_iter()
        .map(|(kind, x, y)| StitchRecord::new(Point::new(mm(x), mm(y)), kind))
        .collect();
    Ok(StitchDesign { threads, records })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    fn thread(id: u32, color: Rgb, name: &str) -> Thread {
        Thread {
            id: ThreadId(id),
            name: name.into(),
            color,
            brand: Some("Robison-Anton".into()),
            code: Some(format!("{}", 2000 + id)),
        }
    }

    /// The design that the stitch plan makes: jump, stitches, trim, color change, end.
    fn two_colors() -> StitchDesign {
        StitchDesign {
            threads: vec![thread(0, Rgb(200, 30, 40), "Red"), thread(1, Rgb(10, 20, 250), "Blue")],
            records: vec![
                rec(-5.0, -5.0, StitchKind::Jump),
                rec(-5.0, -5.0, StitchKind::Stitch),
                rec(-2.5, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Stitch),
                rec(0.0, -5.0, StitchKind::Trim),
                rec(40.0, 30.0, StitchKind::Jump),
                rec(40.0, 30.0, StitchKind::Stitch),
                rec(42.0, 30.0, StitchKind::Stitch),
                rec(42.0, 30.0, StitchKind::Trim),
                rec(42.0, 30.0, StitchKind::ColorChange),
                rec(10.0, 10.0, StitchKind::Jump),
                rec(10.0, 10.0, StitchKind::Stitch),
                rec(10.0, 12.5, StitchKind::Stitch),
                rec(10.0, 12.5, StitchKind::Trim),
                rec(10.0, 12.5, StitchKind::End),
            ],
        }
    }

    #[test]
    fn round_trip_keeps_records_and_threads() {
        let d = two_colors();
        let back = read(&write(&d)).unwrap();
        assert_eq!(back.records, d.records);
        assert_eq!(back.threads, d.threads);
    }

    #[test]
    fn a_stop_is_a_color_block_with_the_same_thread() {
        let mut d = two_colors();
        // A stop in the first color, after the first trim.
        d.records.insert(5, rec(0.0, -5.0, StitchKind::Stop));
        let back = read(&write(&d)).unwrap();
        let names: Vec<&str> = back.threads.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Red", "Red", "Blue"]);
        let changes = back
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::ColorChange)
            .count();
        assert_eq!(changes, 2);
        let stitches = |s: &StitchDesign| s.records.iter().filter(|r| r.kind == StitchKind::Stitch).count();
        assert_eq!(stitches(&back), stitches(&d));
    }

    #[test]
    fn layout_matches_pyembroidery() {
        let b = write(&two_colors());
        assert!(b.starts_with(b"%vsm%\0\x00\x38\x00P\x00r\x00o"));
        // File block length: everything after its length field.
        let at = 6 + 2 + 56 + 3;
        assert_eq!(&b[at - 3..at], b"\x00\x02\x00");
        let len = i32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize;
        assert_eq!(at + 4 + len, b.len());
        // Extents: right 420, -top 50, left -50, -bottom -300 (0.1 mm), times 100.
        let ints: Vec<i32> = (0..4)
            .map(|k| i32::from_be_bytes(b[at + 6 + 4 * k..at + 10 + 4 * k].try_into().unwrap()))
            .collect();
        assert_eq!(ints, [42_000, 5_000, -5_000, -30_000]);
        // Long moves use 80 01 .. 80 02; the end adds a trim after the last trim.
        let has = |pat: &[u8]| b.windows(pat.len()).any(|w| w == pat);
        assert!(has(b"\x80\x01\x01\x90\x01\x5e\x80\x02"), "40 mm, 35 mm move");
        assert!(has(b"\x80\x03\x80\x03\x00"), "trim, end, block end");
        assert!(has(b"\x00\x0d\x52\x6f\x62\x69\x73\x6f\x6e\x2d\x41\x6e\x74\x6f\x6e"));
    }

    /// `tests/corpus/machine/line-art-sampler.pyembroidery.vp3` is the DST
    /// next to it, converted by pyembroidery (commit c16d1b4) with three thread colors.
    #[test]
    fn reads_a_file_from_pyembroidery() {
        let vp3 = read(include_bytes!(
            "../../../tests/corpus/machine/line-art-sampler.pyembroidery.vp3"
        ))
        .unwrap();
        let dst = crate::dst::read(include_bytes!("../../../tests/corpus/machine/line-art-sampler.dst")).unwrap();
        let stitches = |d: &StitchDesign| -> Vec<Point> {
            d.records
                .iter()
                .filter(|r| r.kind == StitchKind::Stitch)
                .map(|r| r.pos)
                .collect()
        };
        assert_eq!(stitches(&vp3).len(), 268);
        assert_eq!(stitches(&vp3), stitches(&dst.design));
        let colors: Vec<String> = vp3.threads.iter().map(|t| t.color.to_hex()).collect();
        assert_eq!(colors, ["#1e3c8c", "#c81e2d", "#28965a"]);
        assert_eq!(vp3.trim_count(), dst.design.trim_count());
    }

    #[test]
    fn missing_threads_get_a_filler() {
        let mut d = two_colors();
        d.threads.truncate(1);
        let back = read(&write(&d)).unwrap();
        assert_eq!(back.threads.len(), 2);
        assert_eq!(back.threads[1].color, FILLER);
        assert_eq!(back.threads[1].name, "#000000");
    }

    #[test]
    fn errors() {
        assert_eq!(read(b"#PES0001"), Err(Vp3Error::BadMagic));
        let b = write(&two_colors());
        for n in [6, 40, 120, b.len() / 2, b.len() - 3] {
            assert_eq!(read(&b[..n]), Err(Vp3Error::Truncated), "cut at {n}");
        }
    }

    #[test]
    fn empty_design() {
        let back = read(&write(&StitchDesign::default())).unwrap();
        assert_eq!(back.stitch_count(), 0);
        assert_eq!(back.records.last().map(|r| r.kind), Some(StitchKind::End));
    }

    proptest! {
        #[test]
        fn random_walk_round_trip(steps in prop::collection::vec((-400i32..400, -400i32..400, 0u8..12), 1..80)) {
            // Positions on the 0.1 mm grid, so the round trip is exact.
            let mut records = Vec::new();
            let (mut x, mut y) = (0i32, 0i32);
            let mut colors = 1;
            for (dx, dy, k) in steps {
                // A jump with no move is not written, so every step moves.
                let dx = if (dx, dy) == (0, 0) { 1 } else { dx };
                x += dx;
                y += dy;
                let p = (f64::from(x) / 10.0, f64::from(y) / 10.0);
                let last = records.last().map_or(Point::default(), |r: &StitchRecord| r.pos);
                match k {
                    0 => {
                        records.push(StitchRecord::new(last, StitchKind::Trim));
                        records.push(rec(p.0, p.1, StitchKind::Jump));
                    }
                    1 if !records.is_empty() => {
                        records.push(StitchRecord::new(last, StitchKind::Trim));
                        records.push(StitchRecord::new(last, StitchKind::ColorChange));
                        records.push(rec(p.0, p.1, StitchKind::Jump));
                        colors += 1;
                    }
                    _ if records.is_empty() => records.push(rec(p.0, p.1, StitchKind::Jump)),
                    _ => {}
                }
                records.push(rec(p.0, p.1, StitchKind::Stitch));
            }
            let end = records.last().unwrap().pos;
            records.push(StitchRecord::new(end, StitchKind::Trim));
            records.push(StitchRecord::new(end, StitchKind::End));
            let threads = (0..colors).map(|i| thread(i, Rgb(i as u8, 0, 0), "t")).collect();
            let d = StitchDesign { threads, records };
            let back = read(&write(&d)).unwrap();
            prop_assert_eq!(back.records, d.records);
            prop_assert_eq!(back.threads.len(), colors as usize);
        }
    }
}

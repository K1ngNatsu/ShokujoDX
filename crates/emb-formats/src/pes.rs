//! Brother PES (version 1) and PEC write; PES and PEC read.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/PesWriter.py`: `write_version_1`, `write_pes_header_v1`,
//!   `write_pes_blocks`, `write_pes_sewsegheader`, `get_as_segments_blocks`,
//!   `write_pes_embsewseg_segments`
//! - `pyembroidery/PecWriter.py`: `write_pec`, `write_pec_header`,
//!   `write_pec_block`, `write_pec_graphics`, `write_value`, `pec_encode`
//! - `pyembroidery/PecGraphics.py`: `blank`, `draw_scaled`, `graphic_mark_bit`
//! - `pyembroidery/EmbThreadPec.py`: `get_thread_set`
//! - `pyembroidery/EmbThread.py`: `build_unique_palette`,
//!   `find_nearest_color_index`, `color_distance_red_mean`
//! - `pyembroidery/PesReader.py`, `pyembroidery/PecReader.py`: `read`, `read_pec`,
//!   `read_pec_stitches`, `process_pec_colors`
//! - `pyembroidery/PecWriter.py`: `write` (lines 28-32), the PEC file
//! - `pyembroidery/PhbReader.py`, `pyembroidery/PhcReader.py`: `read`, the
//!   PEC stitches inside the Brother PHB and PHC files
//! - `pyembroidery/EmbPattern.py`: `interpolate_duplicate_color_as_stop`
//!
//! Format summary: a PES file has a PES part (the design as segments, for
//! Brother software) and a PEC part (what the machine sews). Units are
//! 0.1 mm, Y down. The PEC part has a 512-byte header with the colors as
//! indices into the 64-color Brother palette, the stitch block, and 48 × 38
//! pixel icons. A stitch move per axis is one byte (-63..62) or two bytes
//! (12-bit value with flags). A jump after the first one has the trim flag;
//! a trim with no jump after it is not written.
//!
//! Deviations from pyembroidery, on purpose:
//! - A color block with no thread gets black, not a random color.
//! - The palette is matched in the order of first use of each color.
//!   pyembroidery uses a Python set, so its order can change between runs.
//! - The reader puts a trim before a color change, as the stitch plan
//!   does (the PEC data has the trim flag on the jump after the color change).
//! - Coordinates are rounded half away from zero, as in `dst`. With that
//!   rounding, the corpus designs give the same bytes as pyembroidery.

use emb_geom::Point;
use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

pub const UNITS_PER_MM: f64 = 10.0;
/// Largest move of one record, per axis, in 0.1 mm (`MAX_JUMP_DISTANCE`, `MAX_STITCH_DISTANCE`).
pub const MAX_DELTA: i32 = 2047;
const JUMP_CODE: u8 = 0x10;
const TRIM_CODE: u8 = 0x20;
const FLAG_LONG: u8 = 0x80;
const ICON_STRIDE: usize = 6;
const ICON_HEIGHT: usize = 38;

/// The Brother thread palette that PEC colors index (`get_thread_set`). Index 0 is unused.
pub const PEC_THREADS: [(u8, u8, u8, &str); 64] = [
    (14, 31, 124, "Prussian Blue"),
    (10, 85, 163, "Blue"),
    (0, 135, 119, "Teal Green"),
    (75, 107, 175, "Cornflower Blue"),
    (237, 23, 31, "Red"),
    (209, 92, 0, "Reddish Brown"),
    (145, 54, 151, "Magenta"),
    (228, 154, 203, "Light Lilac"),
    (145, 95, 172, "Lilac"),
    (158, 214, 125, "Mint Green"),
    (232, 169, 0, "Deep Gold"),
    (254, 186, 53, "Orange"),
    (255, 255, 0, "Yellow"),
    (112, 188, 31, "Lime Green"),
    (186, 152, 0, "Brass"),
    (168, 168, 168, "Silver"),
    (125, 111, 0, "Russet Brown"),
    (255, 255, 179, "Cream Brown"),
    (79, 85, 86, "Pewter"),
    (0, 0, 0, "Black"),
    (11, 61, 145, "Ultramarine"),
    (119, 1, 118, "Royal Purple"),
    (41, 49, 51, "Dark Gray"),
    (42, 19, 1, "Dark Brown"),
    (246, 74, 138, "Deep Rose"),
    (178, 118, 36, "Light Brown"),
    (252, 187, 197, "Salmon Pink"),
    (254, 55, 15, "Vermilion"),
    (240, 240, 240, "White"),
    (106, 28, 138, "Violet"),
    (168, 221, 196, "Seacrest"),
    (37, 132, 187, "Sky Blue"),
    (254, 179, 67, "Pumpkin"),
    (255, 243, 107, "Cream Yellow"),
    (208, 166, 96, "Khaki"),
    (209, 84, 0, "Clay Brown"),
    (102, 186, 73, "Leaf Green"),
    (19, 74, 70, "Peacock Blue"),
    (135, 135, 135, "Gray"),
    (216, 204, 198, "Warm Gray"),
    (67, 86, 7, "Dark Olive"),
    (253, 217, 222, "Flesh Pink"),
    (249, 147, 188, "Pink"),
    (0, 56, 34, "Deep Green"),
    (178, 175, 212, "Lavender"),
    (104, 106, 176, "Wisteria Violet"),
    (239, 227, 185, "Beige"),
    (247, 56, 102, "Carmine"),
    (181, 75, 100, "Amber Red"),
    (19, 43, 26, "Olive Green"),
    (199, 1, 86, "Dark Fuchsia"),
    (254, 158, 50, "Tangerine"),
    (168, 222, 235, "Light Blue"),
    (0, 103, 62, "Emerald Green"),
    (78, 41, 144, "Purple"),
    (47, 126, 32, "Moss Green"),
    (255, 204, 204, "Flesh Pink"),
    (255, 217, 17, "Harvest Gold"),
    (9, 91, 166, "Electric Blue"),
    (240, 249, 112, "Lemon Yellow"),
    (227, 243, 91, "Fresh Green"),
    (255, 153, 0, "Orange"),
    (255, 240, 141, "Cream Yellow"),
    (255, 200, 200, "Applique"),
];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PesError {
    #[error("not a PES or PEC file")]
    BadMagic,
    #[error("the PES file ends before the data is complete")]
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

/// Python `round` of `x / 2` for an integer `x`: half to even.
pub(crate) fn half_round_even(x: i32) -> i32 {
    let lo = x.div_euclid(2);
    if x.rem_euclid(2) == 0 || lo % 2 == 0 {
        lo
    } else {
        lo + 1
    }
}

/// `color_distance_red_mean`.
fn color_distance(a: Rgb, b: Rgb) -> i64 {
    let (r1, g1, b1) = (i32::from(a.0), i32::from(a.1), i32::from(a.2));
    let (r2, g2, b2) = (i32::from(b.0), i32::from(b.1), i32::from(b.2));
    let red_mean = half_round_even(r1 + r2);
    let (r, g, b) = (i64::from(r1 - r2), i64::from(g1 - g2), i64::from(b1 - b2));
    let rm = i64::from(red_mean);
    (((512 + rm) * r * r) >> 8) + 4 * g * g + (((767 - rm) * b * b) >> 8)
}

/// `find_nearest_color_index`: the last of equal distances wins. `None` entries are skipped.
pub(crate) fn nearest(color: Rgb, values: &[Option<Rgb>]) -> Option<usize> {
    let mut best = None;
    let mut best_d = i64::MAX;
    for (i, v) in values.iter().enumerate() {
        if let Some(c) = v {
            let d = color_distance(color, *c);
            if d <= best_d {
                best_d = d;
                best = Some(i);
            }
        }
    }
    best
}

/// The PEC palette with index 0 unused, as `get_thread_set`.
fn pec_palette() -> Vec<Option<Rgb>> {
    std::iter::once(None)
        .chain(PEC_THREADS.iter().map(|&(r, g, b, _)| Some(Rgb(r, g, b))))
        .collect()
}

/// `build_unique_palette`: each different color takes the nearest palette
/// entry that is still free. Returns the palette index for each thread.
fn unique_palette(colors: &[Rgb]) -> Vec<u8> {
    let mut palette = pec_palette();
    let mut chart: Vec<Option<Rgb>> = vec![None; palette.len()];
    let mut seen: Vec<Rgb> = Vec::new();
    for &c in colors {
        if seen.contains(&c) {
            continue;
        }
        seen.push(c);
        let Some(i) = nearest(c, &palette) else { break };
        palette[i] = None;
        chart[i] = Some(c);
    }
    colors.iter().map(|&c| nearest(c, &chart).unwrap_or(0) as u8).collect()
}

fn to_units(p: Point) -> (i32, i32) {
    ((p.x * UNITS_PER_MM).round() as i32, (p.y * UNITS_PER_MM).round() as i32)
}

/// Adds jumps toward `to` so that the last move fits in one record (`interpolate_gap_stitches`).
fn gap_jumps(out: &mut Vec<(Low, i32, i32)>, from: (i32, i32), to: (i32, i32)) {
    out.extend(
        crate::common::gap_points(from, to, MAX_DELTA)
            .into_iter()
            .map(|(x, y)| (Low::Jump, x, y)),
    );
}

/// Records at absolute positions in file units, and one thread per color
/// block. A stop becomes a color change to the same thread (`interpolate_stop_as_duplicate_color`).
fn encode(design: &StitchDesign) -> (Vec<(Low, i32, i32)>, Vec<Rgb>) {
    let thread = |i: usize| design.threads.get(i).map_or(Rgb(0, 0, 0), |t| t.color);
    let mut colors = vec![thread(0)];
    let mut next_thread = 1;
    let mut out = Vec::with_capacity(design.records.len() + 1);
    let mut cur = (0, 0);
    // After a trim, stop or color change the encoder jumps to the next
    // stitch (`FULL_JUMP`, see `common::encode`).
    let mut trimmed = true;
    for r in &design.records {
        let p = to_units(r.pos);
        match r.kind {
            StitchKind::Stitch => {
                let before = out.len();
                gap_jumps(&mut out, cur, p);
                let needle = if out.len() > before {
                    out.last().map_or(cur, |l| (l.1, l.2))
                } else {
                    cur
                };
                if trimmed && needle != p {
                    out.push((Low::Jump, p.0, p.1));
                }
                trimmed = false;
                out.push((Low::Stitch, p.0, p.1));
                cur = p;
            }
            StitchKind::Jump => {
                if p != cur {
                    gap_jumps(&mut out, cur, p);
                    out.push((Low::Jump, p.0, p.1));
                    cur = p;
                }
            }
            StitchKind::Trim => {
                trimmed = true;
                out.push((Low::Trim, cur.0, cur.1));
            }
            StitchKind::ColorChange => {
                trimmed = true;
                out.push((Low::ColorChange, cur.0, cur.1));
                colors.push(thread(next_thread));
                next_thread += 1;
            }
            StitchKind::Stop => {
                trimmed = true;
                out.push((Low::ColorChange, cur.0, cur.1));
                colors.push(*colors.last().expect("one color at least"));
            }
            // The format has no speed function.
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::End => break,
        }
    }
    out.push((Low::End, cur.0, cur.1));
    (out, colors)
}

/// Bounds of all records (`EmbPattern.bounds`): left, top, right, bottom.
fn bounds(low: &[(Low, i32, i32)]) -> (i32, i32, i32, i32) {
    let Some(&(_, x0, y0)) = low.first() else {
        return (0, 0, 0, 0);
    };
    low.iter().fold((x0, y0, x0, y0), |(l, t, r, b), &(_, x, y)| {
        (l.min(x), t.min(y), r.max(x), b.max(y))
    })
}

fn u16le(out: &mut Vec<u8>, v: i32) {
    out.extend((v as u16).to_le_bytes());
}

fn f32le(out: &mut Vec<u8>, v: f64) {
    out.extend((v as f32).to_le_bytes());
}

/// Length as a 16-bit value, then the text (`write_pes_string_16`).
fn string_16(out: &mut Vec<u8>, s: &str) {
    u16le(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

/// `write_value`: one byte for -63..62, else two bytes with the long flag and `flag`.
fn write_value(out: &mut Vec<u8>, value: i32, long: bool, flag: u8) {
    if !long && -64 < value && value < 63 {
        out.push((value & 0x7f) as u8);
    } else {
        let v = (value & 0x0fff) | 0x8000 | (i32::from(flag) << 8);
        out.push(((v >> 8) & 0xff) as u8);
        out.push((v & 0xff) as u8);
    }
}

/// `pec_encode`.
fn pec_encode(out: &mut Vec<u8>, low: &[(Low, i32, i32)]) {
    let mut color_two = true;
    let mut jumping = true;
    let mut init = true;
    let (mut xx, mut yy) = (0, 0);
    let stitch = |out: &mut Vec<u8>, dx: i32, dy: i32| {
        write_value(out, dx, false, 0);
        write_value(out, dy, false, 0);
    };
    for &(kind, x, y) in low {
        let (dx, dy) = (x - xx, y - yy);
        xx += dx;
        yy += dy;
        match kind {
            Low::Stitch => {
                if jumping {
                    if dx != 0 && dy != 0 {
                        stitch(out, 0, 0);
                    }
                    jumping = false;
                }
                stitch(out, dx, dy);
            }
            Low::Jump => {
                jumping = true;
                let flag = if init { JUMP_CODE } else { TRIM_CODE };
                write_value(out, dx, true, flag);
                write_value(out, dy, true, flag);
            }
            Low::ColorChange => {
                if jumping {
                    stitch(out, 0, 0);
                    jumping = false;
                }
                out.extend_from_slice(&[0xfe, 0xb0, if color_two { 2 } else { 1 }]);
                color_two = !color_two;
            }
            Low::Trim => {}
            Low::End => {
                out.push(0xff);
                break;
            }
        }
        init = false;
    }
}

/// The empty icon: a frame with round corners (`PecGraphics.blank`).
fn blank_icon() -> Vec<u8> {
    let mut g = Vec::with_capacity(ICON_STRIDE * ICON_HEIGHT);
    for row in 0..ICON_HEIGHT {
        let r: [u8; ICON_STRIDE] = match row {
            0 | 37 => [0; 6],
            1 | 36 => [0xf0, 0xff, 0xff, 0xff, 0xff, 0x0f],
            2 | 35 => [0x08, 0, 0, 0, 0, 0x10],
            3 | 34 => [0x04, 0, 0, 0, 0, 0x20],
            _ => [0x02, 0, 0, 0, 0, 0x40],
        };
        g.extend_from_slice(&r);
    }
    g
}

/// `draw_scaled`: marks the points in the icon, scaled to fit with a margin of `buffer` pixels.
fn draw_scaled(ext: (i32, i32, i32, i32), points: &[(i32, i32)], icon: &mut [u8], buffer: f64) {
    let (left, top, right, bottom) = (f64::from(ext.0), f64::from(ext.1), f64::from(ext.2), f64::from(ext.3));
    let mut dw = right - left;
    let mut dh = bottom - top;
    if dw == 0.0 {
        dw = 1.0;
    }
    if dh == 0.0 {
        dh = 1.0;
    }
    let gw = (ICON_STRIDE * 8) as f64;
    let gh = (icon.len() / ICON_STRIDE) as f64;
    let scale = ((gw - buffer) / dw).min((gh - buffer) / dh);
    let (cx, cy) = ((right + left) / 2.0, (bottom + top) / 2.0);
    let tx = -cx * scale + gw / 2.0;
    let ty = -cy * scale + gh / 2.0;
    for &(x, y) in points {
        let px = (f64::from(x) * scale + tx).floor() as i64;
        let py = (f64::from(y) * scale + ty).floor() as i64;
        if px < 0 || py < 0 {
            continue;
        }
        let i = py as usize * ICON_STRIDE + px as usize / 8;
        if let Some(b) = icon.get_mut(i) {
            *b |= 1 << (px % 8);
        }
    }
}

/// The PEC part (`write_pec`): header, stitch block, icons.
fn write_pec(out: &mut Vec<u8>, low: &[(Low, i32, i32)], colors: &[Rgb], label: &str) {
    let ext = bounds(low);
    // Header (`write_pec_header`).
    let name: String = label
        .chars()
        .map(|c| if c.is_ascii() && !c.is_ascii_control() { c } else { '_' })
        .take(8)
        .collect();
    out.extend_from_slice(format!("LA:{name:<16}\r").as_bytes());
    out.extend_from_slice(&[0x20; 12]);
    out.extend_from_slice(&[0xff, 0x00, ICON_STRIDE as u8, ICON_HEIGHT as u8]);
    let indices = unique_palette(colors);
    if indices.is_empty() {
        out.extend_from_slice(b"\x20\x20\x20\x20\x64\x20\x00\x20\x00\x20\x20\x20\xff");
    } else {
        out.extend_from_slice(&[0x20; 12]);
        out.push((indices.len() - 1) as u8);
        out.extend_from_slice(&indices);
    }
    out.extend(std::iter::repeat_n(0x20, 463usize.saturating_sub(indices.len())));

    // Stitch block (`write_pec_block`).
    let start = out.len();
    out.extend_from_slice(&[0, 0, 0, 0, 0, 0x31, 0xff, 0xf0]);
    u16le(out, ext.2 - ext.0);
    u16le(out, ext.3 - ext.1);
    u16le(out, 0x1e0);
    u16le(out, 0x1b0);
    pec_encode(out, low);
    let len = (out.len() - start) as u32;
    out[start + 2..start + 5].copy_from_slice(&len.to_le_bytes()[..3]);

    // Icons (`write_pec_graphics`): all stitches, then one per color block.
    let stitches = |rs: &[(Low, i32, i32)]| -> Vec<(i32, i32)> {
        rs.iter().filter(|r| r.0 == Low::Stitch).map(|r| (r.1, r.2)).collect()
    };
    let mut icon = blank_icon();
    draw_scaled(ext, &stitches(low), &mut icon, 4.0);
    out.extend_from_slice(&icon);
    // A color block ends with its color change (`get_as_colorblocks`).
    let mut block_start = 0;
    for (i, r) in low.iter().enumerate() {
        if r.0 == Low::ColorChange {
            let mut icon = blank_icon();
            draw_scaled(ext, &stitches(&low[block_start..=i]), &mut icon, 5.0);
            out.extend_from_slice(&icon);
            block_start = i + 1;
        }
    }
    let mut icon = blank_icon();
    draw_scaled(ext, &stitches(&low[block_start..]), &mut icon, 5.0);
    out.extend_from_slice(&icon);
}

/// One PES segment: points, color (palette index), and flag (1 for a jump, 0 for stitches).
type Segment = (Vec<(i32, i32)>, i32, i32);

/// `get_as_segments_blocks`: runs of the same command. A jump run is a
/// segment from the last stitch to the jump end; color changes set the color.
fn segments(low: &[(Low, i32, i32)], colors: &[Rgb], adjust: (i32, i32)) -> Vec<Segment> {
    let palette = pec_palette();
    let code = |i: usize| nearest(colors.get(i).copied().unwrap_or(Rgb(0, 0, 0)), &palette).unwrap_or(0) as i32;
    let mut color_index = 0;
    let mut color = code(color_index);
    let mut stitched = (0, 0);
    let mut out = Vec::new();
    let mut i = 0;
    while i < low.len() {
        let kind = low[i].0;
        let mut j = i;
        while j < low.len() && low[j].0 == kind {
            j += 1;
        }
        let run = &low[i..j];
        match kind {
            Low::Jump => {
                let last = run[run.len() - 1];
                out.push((
                    vec![
                        (stitched.0 - adjust.0, stitched.1 - adjust.1),
                        (last.1 - adjust.0, last.2 - adjust.1),
                    ],
                    color,
                    1,
                ));
            }
            Low::ColorChange => {
                // Each color change in the run moves to the next thread.
                color_index += run.len();
                color = code(color_index);
            }
            Low::Stitch => {
                let pts = run.iter().map(|r| (r.1 - adjust.0, r.2 - adjust.1)).collect();
                stitched = (run[run.len() - 1].1, run[run.len() - 1].2);
                out.push((pts, color, 0));
            }
            Low::Trim | Low::End => {}
        }
        i = j;
    }
    out
}

/// Writes a PES version 1 file. Port of `PesWriter.write` with default settings.
pub fn write(design: &StitchDesign, label: &str) -> Vec<u8> {
    let (low, colors) = encode(design);
    let (left, top, right, bottom) = bounds(&low);
    let (width, height) = (f64::from(right - left), f64::from(bottom - top));

    let mut out = b"#PES0001".to_vec();
    let pec_at = out.len();
    out.extend_from_slice(&[0; 4]);
    // Header: scale to fit, 130 × 180 hoop, one block object.
    for v in [1, 1, 1] {
        u16le(&mut out, v);
    }
    u16le(&mut out, 0xffff);
    u16le(&mut out, 0);

    // CEmbOne: the sew segment header (`write_pes_sewsegheader`).
    string_16(&mut out, "CEmbOne");
    out.extend_from_slice(&[0; 16]);
    let (hoop_w, hoop_h) = (1300.0, 1800.0);
    let trans_x = 350.0 + hoop_w / 2.0 - width / 2.0;
    let trans_y = 100.0 + height + hoop_h / 2.0 - height / 2.0;
    for v in [1.0, 0.0, 0.0, 1.0, trans_x, trans_y] {
        f32le(&mut out, v);
    }
    for v in [1, 0, 0, right - left, bottom - top] {
        u16le(&mut out, v);
    }
    out.extend_from_slice(&[0; 8]);
    let sections_at = out.len();
    u16le(&mut out, 0);
    u16le(&mut out, 0xffff);
    u16le(&mut out, 0);

    // CSewSeg: the segments (`write_pes_embsewseg_segments`).
    string_16(&mut out, "CSewSeg");
    let segs = segments(&low, &colors, (left, bottom));
    let mut color_log: Vec<(usize, i32)> = Vec::new();
    for (n, (pts, color, flag)) in segs.iter().enumerate() {
        if n > 0 {
            u16le(&mut out, 0x8003);
        }
        if color_log.last().map(|c| c.1) != Some(*color) {
            color_log.push((n, *color));
        }
        u16le(&mut out, *flag);
        u16le(&mut out, *color);
        u16le(&mut out, pts.len() as i32);
        for &(x, y) in pts {
            u16le(&mut out, x);
            u16le(&mut out, y);
        }
    }
    u16le(&mut out, color_log.len() as i32);
    for &(section, color) in &color_log {
        u16le(&mut out, section as i32);
        u16le(&mut out, color);
    }
    let n = segs.len() as u16;
    out[sections_at..sections_at + 2].copy_from_slice(&n.to_le_bytes());
    u16le(&mut out, 0);
    u16le(&mut out, 0);

    let pec = out.len() as u32;
    out[pec_at..pec_at + 4].copy_from_slice(&pec.to_le_bytes());
    write_pec(&mut out, &low, &colors, label);
    out
}

fn get(b: &[u8], i: usize) -> Result<u8, PesError> {
    b.get(i).copied().ok_or(PesError::Truncated)
}

fn signed12(v: u16) -> i32 {
    let v = i32::from(v & 0x0fff);
    if v > 0x7ff { v - 0x1000 } else { v }
}

fn signed7(v: u8) -> i32 {
    let v = i32::from(v);
    if v > 63 { v - 128 } else { v }
}

/// The threads of PEC palette indices (`get_thread_set`; index 0 and
/// indices past the palette are "Unknown").
fn pec_threads(indices: &[u8]) -> Vec<Thread> {
    indices
        .iter()
        .enumerate()
        .map(|(i, &k)| {
            let idx = usize::from(k) % 65;
            let (r, g, bl, name) = if idx == 0 {
                (0, 0, 0, "Unknown")
            } else {
                PEC_THREADS[idx - 1]
            };
            Thread {
                id: ThreadId(i as u32),
                name: name.to_string(),
                color: Rgb(r, g, bl),
                brand: Some("Brother".into()),
                code: Some(idx.to_string()),
            }
        })
        .collect()
}

/// The PEC stitch records from `start` (`read_pec_stitches`), with an `End`.
fn pec_stitches(b: &[u8], start: usize) -> Result<Vec<StitchRecord>, PesError> {
    let mut i = start;
    let mut recs: Vec<StitchRecord> = Vec::new();
    let (mut x, mut y) = (0, 0);
    let mm = |v: i32| f64::from(v) / UNITS_PER_MM;
    loop {
        let v1 = get(b, i)?;
        let Ok(v2) = get(b, i + 1) else { break };
        i += 2;
        if v1 == 0xff && v2 == 0x00 {
            break;
        }
        if v1 == 0xfe && v2 == 0xb0 {
            i += 1;
            let p = Point::new(mm(x), mm(y));
            recs.push(StitchRecord::new(p, StitchKind::ColorChange));
            continue;
        }
        let (mut jump, mut trim) = (false, false);
        let (dx, v2) = if v1 & FLAG_LONG != 0 {
            jump |= v1 & JUMP_CODE != 0;
            trim |= v1 & TRIM_CODE != 0;
            let dx = signed12(u16::from(v1) << 8 | u16::from(v2));
            let v2 = get(b, i)?;
            i += 1;
            (dx, v2)
        } else {
            (signed7(v1), v2)
        };
        let dy = if v2 & FLAG_LONG != 0 {
            jump |= v2 & JUMP_CODE != 0;
            trim |= v2 & TRIM_CODE != 0;
            let v3 = get(b, i)?;
            i += 1;
            signed12(u16::from(v2) << 8 | u16::from(v3))
        } else {
            signed7(v2)
        };
        if trim && !jump {
            let here = Point::new(mm(x), mm(y));
            // The stitch plan trims before a color change.
            let n = recs.len();
            if n > 0 && recs[n - 1].kind == StitchKind::ColorChange && recs[n - 1].pos == here {
                recs.insert(n - 1, StitchRecord::new(here, StitchKind::Trim));
            } else {
                recs.push(StitchRecord::new(here, StitchKind::Trim));
            }
        }
        x += dx;
        y += dy;
        let kind = if jump || trim {
            StitchKind::Jump
        } else {
            StitchKind::Stitch
        };
        recs.push(StitchRecord::new(Point::new(mm(x), mm(y)), kind));
    }
    recs.push(StitchRecord::new(Point::new(mm(x), mm(y)), StitchKind::End));
    Ok(recs)
}

/// Reads the PEC part at `at` (`read_pec`). Thread colors come from the Brother palette.
fn read_pec(b: &[u8], at: usize) -> Result<StitchDesign, PesError> {
    // "LA:", label (16), then 15 bytes, stride, height, 12 bytes, color changes.
    let changes = usize::from(get(b, at + 48)?);
    let count = changes + 1;
    let indices: Vec<u8> = (0..count).map(|i| get(b, at + 49 + i)).collect::<Result<_, _>>()?;
    let mut threads = pec_threads(&indices);
    // The stitch block starts at 512; the stitches 16 bytes after it.
    let recs = pec_stitches(b, at + 512 + 16)?;
    // A stitch with no move right after a jump is the needle going down at the jump end.
    threads.truncate(recs.iter().filter(|r| r.kind == StitchKind::ColorChange).count() + 1);
    Ok(StitchDesign { threads, records: recs })
}

/// Reads a PES file (any version) or a PEC file. Port of `PesReader.read`;
/// thread colors come from the PEC part.
pub fn read(bytes: &[u8]) -> Result<StitchDesign, PesError> {
    if bytes.starts_with(b"#PEC0001") {
        return read_pec(bytes, 8);
    }
    if !bytes.starts_with(b"#PES") {
        return Err(PesError::BadMagic);
    }
    let at = u32::from_le_bytes([get(bytes, 8)?, get(bytes, 9)?, get(bytes, 10)?, get(bytes, 11)?]) as usize;
    read_pec(bytes, at)
}

/// Writes a PEC file: `#PEC0001` and the PEC part of a PES file.
pub fn write_pec_file(design: &StitchDesign, label: &str) -> Vec<u8> {
    let (low, colors) = encode(design);
    let mut out = b"#PEC0001".to_vec();
    write_pec(&mut out, &low, &colors, label);
    out
}

fn u16_at(b: &[u8], i: usize) -> Result<usize, PesError> {
    Ok(usize::from(u16::from_le_bytes([get(b, i)?, get(b, i + 1)?])))
}

fn u32_at(b: &[u8], i: usize) -> Result<usize, PesError> {
    Ok(u32::from_le_bytes([get(b, i)?, get(b, i + 1)?, get(b, i + 2)?, get(b, i + 3)?]) as usize)
}

/// `interpolate_duplicate_color_as_stop`: a color change to the same color
/// as the block before is a stop, and the thread goes out of the list.
fn duplicate_color_as_stop(d: &mut StitchDesign) {
    let mut k = 0usize;
    let mut init = true;
    let mut last_change: Option<usize> = None;
    for i in 0..d.records.len() {
        match d.records[i].kind {
            StitchKind::Stitch if init => {
                match last_change {
                    Some(c) if k != 0 && k < d.threads.len() && d.threads[k - 1].color == d.threads[k].color => {
                        d.threads.remove(k);
                        d.records[c].kind = StitchKind::Stop;
                    }
                    _ => k += 1,
                }
                init = false;
            }
            StitchKind::ColorChange => {
                init = true;
                last_change = Some(i);
            }
            _ => {}
        }
    }
    for (i, t) in d.threads.iter_mut().enumerate() {
        t.id = ThreadId(i as u32);
    }
}

/// Reads a Brother PHB file: the colors in its header, the PEC stitches
/// after its blocks.
pub fn read_phb(b: &[u8]) -> Result<StitchDesign, PesError> {
    let count = u16_at(b, 0x71)?;
    let indices: Vec<u8> = (0..count).map(|i| get(b, 0x73 + i)).collect::<Result<_, _>>()?;
    let mut at = 0x52 + u32_at(b, 0x54)?;
    at += u32_at(b, at)? + 2;
    at += u32_at(b, at)?;
    let count2 = usize::from(get(b, at + 14)?);
    let start = at + 15 + count2 + 0x15;
    let mut d = StitchDesign {
        threads: pec_threads(&indices),
        records: pec_stitches(b, start)?,
    };
    duplicate_color_as_stop(&mut d);
    Ok(d)
}

/// Reads a Brother PHC file: the colors and icons in its header, the PEC
/// stitches after them.
pub fn read_phc(b: &[u8]) -> Result<StitchDesign, PesError> {
    let count = u16_at(b, 0x4d)?;
    let indices: Vec<u8> = (0..count).map(|i| get(b, 0x4f + i)).collect::<Result<_, _>>()?;
    let pec_add = usize::from(get(b, 0x2b)?);
    let pec_offset = u16_at(b, 0x30)?;
    let mut at = pec_offset + pec_add;
    at += 2 + u16_at(b, at)?;
    at += 4 + u32_at(b, at)? + 10;
    let count2 = usize::from(get(b, at)?);
    let start = at + 1 + count2 + 0x1d;
    let mut d = StitchDesign {
        threads: pec_threads(&indices),
        records: pec_stitches(b, start)?,
    };
    duplicate_color_as_stop(&mut d);
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(x: f64, y: f64, kind: StitchKind) -> StitchRecord {
        StitchRecord::new(Point::new(x, y), kind)
    }

    fn thread(color: Rgb) -> Thread {
        Thread {
            id: ThreadId(0),
            name: "t".into(),
            color,
            brand: None,
            code: None,
        }
    }

    fn two_colors() -> StitchDesign {
        StitchDesign {
            threads: vec![thread(Rgb(237, 23, 31)), thread(Rgb(10, 85, 163))],
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
    fn round_trip_keeps_records_except_the_last_trim() {
        let d = two_colors();
        let back = read(&write(&d, "test")).unwrap();
        // PEC has no trim without a jump after it, so the last trim is not in the file.
        let mut want = d.records.clone();
        want.remove(want.len() - 2);
        assert_eq!(back.records, want);
        // Exact palette colors stay the same.
        let colors: Vec<Rgb> = back.threads.iter().map(|t| t.color).collect();
        assert_eq!(colors, [Rgb(237, 23, 31), Rgb(10, 85, 163)]);
    }

    /// `tests/corpus/machine/line-art-sampler.pyembroidery.pes` is the DST
    /// next to it, converted by pyembroidery (commit c16d1b4) with three thread colors.
    #[test]
    fn reads_a_file_from_pyembroidery() {
        let pes = read(include_bytes!(
            "../../../tests/corpus/machine/line-art-sampler.pyembroidery.pes"
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
        // pyembroidery adds a stitch with no move at the end of each jump.
        let mut a = stitches(&pes);
        a.dedup();
        let mut b = stitches(&dst.design);
        b.dedup();
        assert_eq!(a, b);
        // The colors are the nearest Brother palette colors.
        let names: Vec<&str> = pes.threads.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert_eq!(pes.color_count(), 3);
    }

    #[test]
    fn layout() {
        let b = write(&two_colors(), "A long design name");
        let pec = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        assert!(b[pec..].starts_with(b"LA:A long d        \r"));
        // Two colors: count - 1, then palette indices (Red is 5, Blue is 2).
        assert_eq!(&b[pec + 48..pec + 51], &[1, 5, 2]);
        // Stitch block length, then icons: one for all stitches and one per color.
        let len = u32::from_le_bytes([b[pec + 514], b[pec + 515], b[pec + 516], 0]) as usize;
        assert_eq!(b.len(), pec + 512 + len + 3 * ICON_STRIDE * ICON_HEIGHT);
        assert_eq!(read(b"#PEC0001").err(), Some(PesError::Truncated));
        assert_eq!(read(b"%vsm%").err(), Some(PesError::BadMagic));
    }

    #[test]
    fn value_encoding() {
        let mut v = Vec::new();
        write_value(&mut v, 62, false, 0);
        write_value(&mut v, 63, false, 0);
        write_value(&mut v, -63, false, 0);
        write_value(&mut v, -64, false, 0);
        write_value(&mut v, -1, true, TRIM_CODE);
        assert_eq!(v, [62, 0x80, 63, 0x41, 0x8f, 0xc0, 0xaf, 0xff]);
    }

    #[test]
    fn palette_matching() {
        // Exact colors map to their entries; two colors near one entry get different entries.
        assert_eq!(unique_palette(&[Rgb(0, 0, 0), Rgb(240, 240, 240)]), [20, 29]);
        let near_red = unique_palette(&[Rgb(237, 23, 31), Rgb(236, 22, 30)]);
        assert_ne!(near_red[0], near_red[1]);
        assert_eq!(half_round_even(5), 2);
        assert_eq!(half_round_even(7), 4);
        assert_eq!(half_round_even(6), 3);
    }

    #[test]
    fn blank_icon_matches_pyembroidery() {
        let hex: String = blank_icon().iter().map(|b| format!("{b:02x}")).collect();
        assert!(hex.starts_with("000000000000f0ffffffff0f080000000010040000000020020000000040"));
        assert!(hex.ends_with("040000000020080000000010f0ffffffff0f000000000000"));
        assert_eq!(hex.len(), 2 * 228);
    }

    #[test]
    fn pec_file_reads_back() {
        let d = two_colors();
        let b = write_pec_file(&d, "pec");
        assert!(b.starts_with(b"#PEC0001LA:pec"));
        let back = read(&b).unwrap();
        assert_eq!(back.stitch_count(), d.stitch_count());
        assert_eq!(back.color_count(), d.color_count());
    }

    /// A PHB file made by hand: the header with two colors (the same red
    /// twice), the three block offsets, then a PEC stitch block.
    #[test]
    fn phb_reads_pec_stitches_and_a_duplicate_color_is_a_stop() {
        let mut b = vec![0u8; 0x200];
        b[..8].copy_from_slice(b"#PHB0003");
        b[0x71..0x73].copy_from_slice(&2u16.to_le_bytes());
        b[0x73] = 5;
        b[0x74] = 5;
        // at = 0x52 + 0x2e = 0x80; at += 0x10 + 2 = 0x92; at += 0x20 = 0xb2.
        b[0x54..0x58].copy_from_slice(&0x2eu32.to_le_bytes());
        b[0x80..0x84].copy_from_slice(&0x10u32.to_le_bytes());
        b[0x92..0x96].copy_from_slice(&0x20u32.to_le_bytes());
        b[0xb2 + 14] = 1; // color count 2 - 1
        let start = 0xb2 + 15 + 1 + 0x15;
        let stitches = [10, 0, 10, 0, 0xfe, 0xb0, 0x01, 0, 10, 0xff, 0x00];
        b[start..start + stitches.len()].copy_from_slice(&stitches);
        let d = read_phb(&b).unwrap();
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                StitchKind::Stitch,
                StitchKind::Stitch,
                StitchKind::Stop,
                StitchKind::Stitch,
                StitchKind::End
            ]
        );
        assert_eq!(d.threads.len(), 1, "the second red is the same thread");
    }
}

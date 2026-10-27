//! Synthetic stitch data to test how fast the canvas draws (test only; see
//! `canvas::tests::mesh_build_timing`).
//!
//! This is not real stitch generation. It makes tatami-like rows inside
//! circles, with several colors, so the canvas has a realistic load.

use emb_geom::Point;
use emb_model::{Rgb, StitchDesign, StitchKind, StitchRecord, Thread, ThreadId};

const COLORS: [(&str, Rgb); 4] = [
    ("Red", Rgb(200, 30, 45)),
    ("Gold", Rgb(230, 170, 30)),
    ("Blue", Rgb(30, 90, 190)),
    ("Green", Rgb(40, 150, 80)),
];

/// Makes a design with about `count` stitches in four circles.
pub fn make(count: usize) -> StitchDesign {
    let threads: Vec<Thread> = COLORS
        .iter()
        .enumerate()
        .map(|(i, (name, color))| Thread {
            id: ThreadId(i as u32),
            name: (*name).into(),
            color: *color,
            brand: None,
            code: None,
        })
        .collect();

    let per_block = count / COLORS.len();
    let centers = [(-55.0, -45.0), (55.0, -45.0), (-55.0, 45.0), (55.0, 45.0)];
    let radius = 42.0;
    let stitch_len = 3.0;

    let mut records = Vec::with_capacity(count + 16);
    for (block, &(cx, cy)) in centers.iter().enumerate() {
        if block > 0 {
            let last = records.last().map(|r: &StitchRecord| r.pos).unwrap_or_default();
            records.push(StitchRecord::new(last, StitchKind::Trim));
            records.push(StitchRecord::new(last, StitchKind::ColorChange));
        }
        // Choose the row spacing so that the block has about `per_block` stitches.
        // Area / (spacing * stitch_len) = stitches.
        let area = std::f64::consts::PI * radius * radius;
        let spacing = (area / (per_block.max(1) as f64 * stitch_len)).max(0.01);
        let mut y = -radius + spacing / 2.0;
        let mut row = 0usize;
        while y < radius {
            let half = (radius * radius - y * y).sqrt();
            let n = ((2.0 * half) / stitch_len).ceil().max(1.0) as usize;
            // Offset each row a little, like a tatami pattern.
            let shift = (row % 3) as f64 * stitch_len / 3.0;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                let mut x = -half + t * 2.0 * half;
                if i > 0 && i < n {
                    x = (x + shift).min(half);
                }
                let x = if row.is_multiple_of(2) { x } else { -x };
                records.push(StitchRecord::new(Point::new(cx + x, cy + y), StitchKind::Stitch));
            }
            y += spacing;
            row += 1;
        }
    }
    if let Some(last) = records.last().map(|r| r.pos) {
        records.push(StitchRecord::new(last, StitchKind::End));
    }
    StitchDesign { threads, records }
}

#[cfg(test)]
mod tests {
    #[test]
    fn count_is_close_to_request() {
        for n in [10_000, 100_000] {
            let d = super::make(n);
            let got = d.stitch_count() as f64;
            assert!((got / n as f64 - 1.0).abs() < 0.1, "{n} -> {got}");
            assert_eq!(d.color_count(), 4);
        }
    }
}

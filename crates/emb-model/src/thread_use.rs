//! Thread use and cost (P11 Milestone 1): the length of the top thread of
//! each color and of the bobbin thread that a design uses.
//!
//! The estimate comes from the stitches:
//!
//! - `L`: the length of the stitches on the fabric (from one needle point to
//!   the next; jumps and trims are not thread).
//! - The top thread lies on the front along `L`, and the tension pulls it to
//!   the back on each side of the bobbin thread: on the back the bobbin
//!   thread shows in the middle [`BOBBIN_SHARE`] of each stitch, the top
//!   thread in the rest. So the top thread is `L × (2 − BOBBIN_SHARE)` and
//!   the bobbin thread `L × BOBBIN_SHARE`.
//! - At each needle point both threads go into the fabric to the middle and
//!   out again: the fabric thickness for each thread.
//! - At the start of each color and after each trim the machine leaves a
//!   tail of top thread ([`TAIL_MM`]).
//!
//! The numbers are estimates: the tension, the thread and the fabric change
//! them. The exit test of the milestone weighs the spools of two sew-outs.

use serde::{Deserialize, Serialize};

use crate::{StitchDesign, StitchKind};

/// The part of each stitch where the bobbin thread shows on the back.
pub const BOBBIN_SHARE: f64 = 0.4;
/// The tail of top thread at a start or a trim, in mm.
pub const TAIL_MM: f64 = 15.0;
/// The fabric thickness when the design has no fabric, in mm.
pub const DEFAULT_THICKNESS_MM: f64 = 0.6;

/// The thread of one color block, in sew order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColorUse {
    pub top_mm: f64,
    pub bobbin_mm: f64,
}

/// The thread of a design.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThreadUse {
    /// One for each color block, in sew order.
    pub colors: Vec<ColorUse>,
    pub top_mm: f64,
    pub bobbin_mm: f64,
}

/// The prices that the user sets for the cost of the thread. A price of 0
/// gives no cost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThreadPrices {
    /// The length of the thread on one spool of top thread, in m.
    pub spool_m: f64,
    pub spool_price: f64,
    /// The length of the thread on one bobbin, in m.
    pub bobbin_m: f64,
    pub bobbin_price: f64,
    /// The currency, as text before the number (for example "$" or "€").
    pub currency: String,
}

impl Default for ThreadPrices {
    fn default() -> Self {
        ThreadPrices {
            spool_m: 1000.0,
            spool_price: 0.0,
            bobbin_m: 100.0,
            bobbin_price: 0.0,
            currency: String::new(),
        }
    }
}

impl ThreadPrices {
    /// True if a price is set, so there is a cost to show.
    pub fn is_set(&self) -> bool {
        self.spool_price > 0.0 || self.bobbin_price > 0.0
    }

    /// The cost of the thread of `u`.
    pub fn cost(&self, u: &ThreadUse) -> f64 {
        let part = |mm: f64, len_m: f64, price: f64| {
            if len_m > 0.0 { mm / 1000.0 / len_m * price } else { 0.0 }
        };
        part(u.top_mm, self.spool_m, self.spool_price) + part(u.bobbin_mm, self.bobbin_m, self.bobbin_price)
    }

    /// The cost as text, with the currency.
    pub fn cost_text(&self, u: &ThreadUse) -> String {
        format!("{}{:.2}", self.currency, self.cost(u))
    }
}

/// The thread that `s` uses, on a fabric `thickness_mm` thick.
pub fn thread_use(s: &StitchDesign, thickness_mm: f64) -> ThreadUse {
    let t = thickness_mm.max(0.0);
    // Length on the fabric, needle points and tails of each block.
    let mut blocks: Vec<(f64, usize, usize)> = vec![(0.0, 0, 0)];
    let mut prev: Option<(StitchKind, emb_geom::Point)> = None;
    let mut tail_next = true;
    for r in &s.records {
        let b = blocks.last_mut().expect("one block at least");
        match r.kind {
            StitchKind::Stitch => {
                b.1 += 1;
                if let Some((StitchKind::Stitch, p)) = prev {
                    b.0 += p.distance(r.pos);
                }
                if tail_next {
                    b.2 += 1;
                    tail_next = false;
                }
            }
            StitchKind::Trim => tail_next = true,
            StitchKind::ColorChange => {
                blocks.push((0.0, 0, 0));
                tail_next = true;
            }
            _ => {}
        }
        prev = Some((r.kind, r.pos));
    }
    let colors: Vec<ColorUse> = blocks
        .iter()
        .map(|&(len, n, tails)| ColorUse {
            top_mm: len * (2.0 - BOBBIN_SHARE) + n as f64 * t + tails as f64 * TAIL_MM,
            bobbin_mm: len * BOBBIN_SHARE + n as f64 * t,
        })
        .collect();
    ThreadUse {
        top_mm: colors.iter().map(|c| c.top_mm).sum(),
        bobbin_mm: colors.iter().map(|c| c.bobbin_mm).sum(),
        colors,
    }
}

/// A length of thread as text: mm under 1 m, else m.
pub fn length_text(mm: f64) -> String {
    if mm >= 1000.0 {
        format!("{:.1} m", mm / 1000.0)
    } else {
        format!("{mm:.0} mm")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emb_geom::Point;

    use crate::StitchRecord;

    fn design() -> StitchDesign {
        // Color 1: 10 stitches of 2 mm in a line, a trim, 2 stitches 3 mm apart.
        // Color 2: 3 stitches 1 mm apart.
        let mut r: Vec<StitchRecord> = (0..=10)
            .map(|i| StitchRecord::stitch(f64::from(i) * 2.0, 0.0))
            .collect();
        r.push(StitchRecord::new(Point::new(20.0, 0.0), StitchKind::Trim));
        r.push(StitchRecord::new(Point::new(30.0, 0.0), StitchKind::Jump));
        r.push(StitchRecord::stitch(30.0, 0.0));
        r.push(StitchRecord::stitch(33.0, 0.0));
        r.push(StitchRecord::new(Point::new(33.0, 0.0), StitchKind::ColorChange));
        for i in 0..3 {
            r.push(StitchRecord::stitch(f64::from(i), 5.0));
        }
        StitchDesign {
            threads: Vec::new(),
            records: r,
        }
    }

    #[test]
    fn top_and_bobbin_from_length_needle_points_and_tails() {
        let u = thread_use(&design(), 1.0);
        assert_eq!(u.colors.len(), 2);
        // Color 1: L = 20 + 3, 13 needle points, 2 tails (start, after the trim).
        let c = u.colors[0];
        assert!((c.top_mm - (23.0 * 1.6 + 13.0 + 30.0)).abs() < 1e-9, "{c:?}");
        assert!((c.bobbin_mm - (23.0 * 0.4 + 13.0)).abs() < 1e-9, "{c:?}");
        // Color 2: L = 2, 3 needle points, 1 tail.
        let c = u.colors[1];
        assert!((c.top_mm - (2.0 * 1.6 + 3.0 + 15.0)).abs() < 1e-9, "{c:?}");
        assert!((u.top_mm - u.colors.iter().map(|c| c.top_mm).sum::<f64>()).abs() < 1e-9);
        // A thicker fabric takes more thread.
        assert!(thread_use(&design(), 3.0).top_mm > u.top_mm);
    }

    #[test]
    fn cost_from_the_prices_of_a_spool_and_a_bobbin() {
        let u = ThreadUse {
            colors: Vec::new(),
            top_mm: 50_000.0,
            bobbin_mm: 10_000.0,
        };
        let p = ThreadPrices {
            spool_m: 1000.0,
            spool_price: 4.0,
            bobbin_m: 100.0,
            bobbin_price: 0.5,
            currency: "$".into(),
        };
        // 50 m of 1000 m at 4, and 10 m of 100 m at 0.5.
        assert!((p.cost(&u) - (0.2 + 0.05)).abs() < 1e-9);
        assert_eq!(p.cost_text(&u), "$0.25");
        assert!(!ThreadPrices::default().is_set());
        assert_eq!(length_text(999.0), "999 mm");
        assert_eq!(length_text(12_340.0), "12.3 m");
    }
}

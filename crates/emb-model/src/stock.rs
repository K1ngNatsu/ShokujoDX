//! Thread stock (P13 Milestone 3, ADR 0066): the threads that a shop has,
//! the color matching with only them, and the threads to buy for an
//! order.
//!
//! - [`ThreadStock`]: a list of threads (brand, code, name, color) with the
//!   number of spools, the length of a spool and the lowest number of
//!   spools to keep. CSV import and export.
//! - [`ThreadStock::as_chart`]: the threads in stock as a thread chart, so
//!   all color matching of the program can use them.
//! - [`threads_to_buy`]: for an order, the top thread of each thread of
//!   the design against the stock.

use serde::{Deserialize, Serialize};

use crate::chart::{ChartThread, Lab, ThreadChart};
use crate::{Rgb, StitchDesign, ThreadLook};

/// The name of the chart of the threads in stock.
pub const STOCK_CHART: &str = "Threads in stock";

/// One thread of the stock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StockThread {
    pub brand: String,
    pub code: String,
    pub name: String,
    pub color: Rgb,
    /// The spools that the shop has (a part of a spool counts as one).
    pub spools: u32,
    /// The length of the thread on one spool, in m. `None`: the length of
    /// the thread prices (Options).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spool_m: Option<f64>,
    /// The lowest number of spools to keep. Under it, the thread is on the
    /// list of threads to buy.
    pub min_spools: u32,
}

impl Default for StockThread {
    fn default() -> Self {
        StockThread {
            brand: String::new(),
            code: String::new(),
            name: String::new(),
            color: Rgb(128, 128, 128),
            spools: 1,
            spool_m: None,
            min_spools: 0,
        }
    }
}

/// The most spools of one thread.
pub const MAX_SPOOLS: u32 = 100_000;
/// The longest spool, in m.
pub const MAX_SPOOL_M: f64 = 100_000.0;

impl StockThread {
    /// The stock thread of a chart thread.
    pub fn of_chart(chart: &ThreadChart, k: usize) -> StockThread {
        let t = &chart.threads[k];
        StockThread {
            brand: t.brand.clone().unwrap_or_else(|| chart.name.clone()),
            code: t.code.clone().unwrap_or_default(),
            name: t.name.clone(),
            color: t.color,
            ..StockThread::default()
        }
    }

    /// The look of the thread, for a design thread.
    pub fn look(&self) -> ThreadLook {
        ThreadLook {
            name: self.name.clone(),
            color: self.color,
            brand: Some(self.brand.clone()).filter(|b| !b.trim().is_empty()),
            code: Some(self.code.clone()).filter(|c| !c.trim().is_empty()),
        }
    }

    /// True if `look` is this thread: the same brand and code, or (with no
    /// code) the same color and name.
    pub fn is(&self, look: &ThreadLook) -> bool {
        let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
        let brand = look.brand.as_deref().unwrap_or("");
        match look.code.as_deref().filter(|c| !c.trim().is_empty()) {
            Some(code) if !self.code.trim().is_empty() => same(code, &self.code) && same(brand, &self.brand),
            _ => self.code.trim().is_empty() && look.color == self.color && same(&look.name, &self.name),
        }
    }

    /// The thread on hand, in m.
    pub fn length_m(&self, default_spool_m: f64) -> f64 {
        f64::from(self.spools) * self.spool_length(default_spool_m)
    }

    /// The length of one spool, in m.
    pub fn spool_length(&self, default_spool_m: f64) -> f64 {
        self.spool_m
            .filter(|m| m.is_finite() && *m > 0.0)
            .unwrap_or(default_spool_m)
            .clamp(1.0, MAX_SPOOL_M)
    }
}

/// The threads that a shop has.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ThreadStock {
    pub threads: Vec<StockThread>,
}

impl ThreadStock {
    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// The index of the stock thread that `look` is.
    pub fn find(&self, look: &ThreadLook) -> Option<usize> {
        self.threads.iter().position(|t| t.is(look))
    }

    /// True if `look` is in stock (at least one spool).
    pub fn has(&self, look: &ThreadLook) -> bool {
        self.find(look).is_some_and(|i| self.threads[i].spools > 0)
    }

    /// Adds a thread, or adds its spools to the same thread. Returns its index.
    pub fn add(&mut self, t: StockThread) -> usize {
        match self.find(&t.look()) {
            Some(i) => {
                let s = &mut self.threads[i];
                s.spools = s.spools.saturating_add(t.spools).min(MAX_SPOOLS);
                i
            }
            None => {
                self.threads.push(t);
                self.threads.len() - 1
            }
        }
    }

    /// The threads with at least one spool as a chart ([`STOCK_CHART`]).
    /// Each chart thread keeps its brand. `None` if no thread is in stock.
    pub fn as_chart(&self) -> Option<ThreadChart> {
        let threads: Vec<ChartThread> = self
            .threads
            .iter()
            .filter(|t| t.spools > 0)
            .map(|t| ChartThread {
                code: Some(t.code.clone()).filter(|c| !c.trim().is_empty()),
                name: t.name.clone(),
                color: t.color,
                brand: Some(t.brand.clone()).filter(|b| !b.trim().is_empty()),
            })
            .collect();
        (!threads.is_empty()).then(|| ThreadChart {
            name: STOCK_CHART.into(),
            threads,
        })
    }

    /// The nearest thread in stock to `color`, and the color difference
    /// (CIE76).
    pub fn nearest(&self, color: Rgb) -> Option<(usize, f64)> {
        let lab = Lab::of(color);
        self.threads
            .iter()
            .enumerate()
            .filter(|(_, t)| t.spools > 0)
            .map(|(i, t)| (i, lab.distance(Lab::of(t.color))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The stock with each value in its range.
    pub fn cleaned(&self) -> ThreadStock {
        let threads = self
            .threads
            .iter()
            .map(|t| StockThread {
                spools: t.spools.min(MAX_SPOOLS),
                min_spools: t.min_spools.min(MAX_SPOOLS),
                spool_m: t
                    .spool_m
                    .filter(|m| m.is_finite() && *m > 0.0)
                    .map(|m| m.clamp(1.0, MAX_SPOOL_M)),
                ..t.clone()
            })
            .collect();
        ThreadStock { threads }
    }

    /// The header of the CSV file.
    pub const CSV_HEADER: &'static str = "brand,code,name,color,spools,spool_m,min_spools";

    /// The stock as CSV, with a header line.
    pub fn to_csv(&self) -> String {
        let mut out = format!("{}\n", Self::CSV_HEADER);
        for t in &self.threads {
            out.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                csv_field(&t.brand),
                csv_field(&t.code),
                csv_field(&t.name),
                t.color.to_hex(),
                t.spools,
                t.spool_m.map(|m| format!("{m}")).unwrap_or_default(),
                t.min_spools
            ));
        }
        out
    }

    /// Reads a CSV file of [`ThreadStock::to_csv`]. A first line with no
    /// color is a header. The columns after the color can be left out
    /// (one spool, the default length, no lowest number).
    pub fn from_csv(text: &str) -> Result<ThreadStock, String> {
        let mut stock = ThreadStock::default();
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() {
                continue;
            }
            let cells = csv_cells(line);
            let cell = |k: usize| cells.get(k).map(|s| s.trim()).unwrap_or("");
            let Some(color) = Rgb::from_hex(cell(3)) else {
                if i == 0 {
                    continue;
                }
                return Err(format!("line {}: expected brand,code,name,#rrggbb", i + 1));
            };
            let number = |k: usize, default: u32| -> Result<u32, String> {
                match cell(k) {
                    "" => Ok(default),
                    v => v
                        .parse::<u32>()
                        .map(|n| n.min(MAX_SPOOLS))
                        .map_err(|_| format!("line {}: {v:?} is not a whole number", i + 1)),
                }
            };
            let spool_m = match cell(5) {
                "" => None,
                v => Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|m| m.is_finite() && *m > 0.0)
                        .ok_or_else(|| format!("line {}: {v:?} is not a length in m", i + 1))?
                        .min(MAX_SPOOL_M),
                ),
            };
            stock.add(StockThread {
                brand: cell(0).to_string(),
                code: cell(1).to_string(),
                name: cell(2).to_string(),
                color,
                spools: number(4, 1)?,
                spool_m,
                min_spools: number(6, 0)?,
            });
        }
        Ok(stock)
    }
}

/// One change of Use Threads in Stock.
#[derive(Debug, Clone, PartialEq)]
pub struct StockChange {
    pub thread: crate::ThreadId,
    pub from: ThreadLook,
    pub to: ThreadLook,
    /// The color difference (CIE76; about 2.3 is the smallest that a
    /// person can see).
    pub delta_e: f64,
}

impl crate::Design {
    /// Gives each used thread of the current colorway that is not in stock
    /// the look of the nearest thread in stock. Returns the changes.
    pub fn use_stock_threads(&mut self, stock: &ThreadStock) -> Vec<StockChange> {
        let cw = self.colorway;
        let used = self.used_threads();
        let mut changes = Vec::new();
        for (id, look) in self.colorway_looks(cw) {
            if !used.contains(&id) || stock.has(&look) {
                continue;
            }
            let Some((k, delta_e)) = stock.nearest(look.color) else {
                continue;
            };
            let to = stock.threads[k].look();
            self.set_colorway_look(cw, id, to.clone());
            changes.push(StockChange {
                thread: id,
                from: look,
                to,
                delta_e,
            });
        }
        changes
    }
}

/// A CSV field, quoted if it has a comma, a quote or a line break.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// The cells of a CSV line (quoted cells can have commas and `""`).
fn csv_cells(line: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                chars.next();
                cells.last_mut().expect("one cell").push('"');
            }
            ('"', _) => quoted = !quoted,
            (',', false) => cells.push(String::new()),
            _ => cells.last_mut().expect("one cell").push(c),
        }
    }
    cells
}

/// Why a thread is on the list of threads to buy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyReason {
    /// The stock does not have the thread.
    NotInStock,
    /// The order needs more thread than the stock has.
    TooLittle,
    /// After the order, the stock has fewer spools than the lowest number.
    UnderTheLowest,
}

impl BuyReason {
    pub fn label(self) -> &'static str {
        match self {
            BuyReason::NotInStock => "not in stock",
            BuyReason::TooLittle => "not enough for the order",
            BuyReason::UnderTheLowest => "under the lowest number after the order",
        }
    }

    pub const ALL: [BuyReason; 3] = [BuyReason::NotInStock, BuyReason::TooLittle, BuyReason::UnderTheLowest];
}

/// One thread to buy.
#[derive(Debug, Clone, PartialEq)]
pub struct BuyLine {
    pub look: ThreadLook,
    /// The top thread that the order needs, in m.
    pub need_m: f64,
    /// The thread on hand, in m.
    pub have_m: f64,
    pub spools: u32,
    pub reason: BuyReason,
}

impl BuyLine {
    /// The thread as text: name, brand and code.
    pub fn thread_text(&self) -> String {
        match (&self.look.brand, &self.look.code) {
            (Some(b), Some(c)) => format!("{} ({b} {c})", self.look.name),
            (None, Some(c)) => format!("{} ({c})", self.look.name),
            _ => self.look.name.clone(),
        }
    }
}

/// The top thread of each different thread of `s`, in mm (a thread that
/// sews two times is one thread, `needles::same_thread`), in the order of
/// the first block of each thread.
pub fn thread_lengths(s: &StitchDesign, thickness_mm: f64) -> Vec<(ThreadLook, f64)> {
    let u = crate::thread_use::thread_use(s, thickness_mm);
    let mut out: Vec<(ThreadLook, f64)> = Vec::new();
    for (i, c) in u.colors.iter().enumerate() {
        let Some(t) = s.threads.get(i) else { continue };
        if c.top_mm <= 0.0 {
            continue;
        }
        let look = ThreadLook::of(t);
        match out.iter_mut().find(|(l, _)| crate::needles::same_thread(l, &look)) {
            Some((_, mm)) => *mm += c.top_mm,
            None => out.push((look, c.top_mm)),
        }
    }
    out
}

/// The threads to buy to sew `quantity` pieces of `s` from `stock`. A
/// spool has `default_spool_m` m if the stock gives no length. The order
/// needs the top thread of the design (`thread_use`) times the quantity;
/// the bobbin thread is not in the list.
pub fn threads_to_buy(
    stock: &ThreadStock,
    s: &StitchDesign,
    thickness_mm: f64,
    quantity: u32,
    default_spool_m: f64,
) -> Vec<BuyLine> {
    let default_spool_m = if default_spool_m.is_finite() && default_spool_m > 0.0 {
        default_spool_m.clamp(1.0, MAX_SPOOL_M)
    } else {
        1000.0
    };
    let q = f64::from(quantity.max(1));
    let mut out = Vec::new();
    for (look, mm) in thread_lengths(s, thickness_mm) {
        let need_m = mm / 1000.0 * q;
        let (have_m, spool_m, min, reason) = match stock.find(&look).map(|i| &stock.threads[i]) {
            Some(t) if t.spools > 0 => {
                let have = t.length_m(default_spool_m);
                let spool = t.spool_length(default_spool_m);
                let reason = if need_m > have {
                    BuyReason::TooLittle
                } else {
                    BuyReason::UnderTheLowest
                };
                (have, spool, t.min_spools, reason)
            }
            Some(t) => (
                0.0,
                t.spool_length(default_spool_m),
                t.min_spools,
                BuyReason::NotInStock,
            ),
            None => (0.0, default_spool_m, 0, BuyReason::NotInStock),
        };
        // Spools after the order, as whole spools that are left (a spool
        // that is begun counts as one).
        let left_m = have_m - need_m;
        let left_spools = if left_m > 0.0 { (left_m / spool_m).ceil() } else { 0.0 };
        let buy = if reason == BuyReason::NotInStock {
            ((need_m / spool_m).ceil()).max(1.0) + f64::from(min)
        } else if left_m < 0.0 {
            (-left_m / spool_m).ceil() + f64::from(min)
        } else if left_spools < f64::from(min) {
            f64::from(min) - left_spools
        } else {
            0.0
        };
        if buy > 0.0 {
            out.push(BuyLine {
                look,
                need_m,
                have_m,
                spools: buy.min(f64::from(MAX_SPOOLS)) as u32,
                reason,
            });
        }
    }
    out
}

/// The threads to buy as CSV, with a header line.
pub fn buy_csv(lines: &[BuyLine]) -> String {
    let mut out = String::from("brand,code,name,color,spools_to_buy,need_m,have_m,reason\n");
    for l in lines {
        out.push_str(&format!(
            "{},{},{},{},{},{:.0},{:.0},{}\n",
            csv_field(l.look.brand.as_deref().unwrap_or("")),
            csv_field(l.look.code.as_deref().unwrap_or("")),
            csv_field(&l.look.name),
            l.look.color.to_hex(),
            l.spools,
            l.need_m,
            l.have_m,
            l.reason.label()
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StitchKind, StitchRecord, Thread, ThreadId};
    use emb_geom::Point;

    fn red() -> StockThread {
        StockThread {
            brand: "Madeira Polyneon".into(),
            code: "1747".into(),
            name: "Red".into(),
            color: Rgb(200, 20, 30),
            spools: 2,
            spool_m: Some(1000.0),
            min_spools: 1,
        }
    }

    fn look(brand: &str, code: &str, color: Rgb) -> ThreadLook {
        ThreadLook {
            name: "x".into(),
            color,
            brand: Some(brand.into()),
            code: Some(code.into()),
        }
    }

    #[test]
    fn a_thread_is_found_by_brand_and_code() {
        let mut s = ThreadStock::default();
        s.add(red());
        assert!(s.has(&look("madeira polyneon", " 1747", Rgb(0, 0, 0))));
        assert!(!s.has(&look("Isacord", "1747", Rgb(200, 20, 30))));
        // With no code: the same color and name.
        let mut plain = red();
        plain.code.clear();
        plain.name = "Blush".into();
        s.add(plain);
        let l = ThreadLook {
            name: "Blush".into(),
            color: Rgb(200, 20, 30),
            brand: None,
            code: None,
        };
        assert_eq!(s.find(&l), Some(1));
        // The same thread again adds its spools.
        assert_eq!(s.add(red()), 0);
        assert_eq!(s.threads[0].spools, 4);
        // No spools: not in stock, and not in the chart.
        s.threads[1].spools = 0;
        assert!(!s.has(&l));
        assert_eq!(s.as_chart().unwrap().threads.len(), 1);
    }

    #[test]
    fn the_chart_keeps_the_brand() {
        let mut s = ThreadStock::default();
        s.add(red());
        let chart = s.as_chart().unwrap();
        assert_eq!(chart.name, STOCK_CHART);
        let l = crate::colorway::chart_look(&chart, 0);
        assert_eq!(l.brand.as_deref(), Some("Madeira Polyneon"));
        assert_eq!(l.code.as_deref(), Some("1747"));
        assert!(ThreadStock::default().as_chart().is_none());
        let (i, d) = s.nearest(Rgb(190, 30, 30)).unwrap();
        assert_eq!(i, 0);
        assert!(d > 0.0 && d < 10.0);
    }

    #[test]
    fn csv_round_trip() {
        let mut s = ThreadStock::default();
        s.add(red());
        s.add(StockThread {
            brand: "Isacord".into(),
            code: "0015".into(),
            name: "White, bright".into(),
            color: Rgb(255, 255, 255),
            spools: 5,
            spool_m: None,
            min_spools: 2,
        });
        let csv = s.to_csv();
        assert!(csv.contains("\"White, bright\""));
        assert_eq!(ThreadStock::from_csv(&csv).unwrap(), s);
        // Short lines: one spool, no length, no lowest number.
        let short = ThreadStock::from_csv("Sulky,1001,White,#ffffff\n").unwrap();
        assert_eq!(short.threads[0].spools, 1);
        assert!(ThreadStock::from_csv("brand,code\nSulky,1001,White,notacolor\n").is_err());
        assert!(ThreadStock::from_csv("Sulky,1001,White,#ffffff,many\n").is_err());
    }

    /// A design of 100 mm of red, 50 mm of blue and 100 mm of red again.
    fn design() -> StitchDesign {
        let t = |id, name: &str, color, code: &str| Thread {
            id: ThreadId(id),
            name: name.into(),
            color,
            brand: Some("Madeira Polyneon".into()),
            code: Some(code.into()),
        };
        let mut records = Vec::new();
        for (k, len) in [100.0, 50.0, 100.0].into_iter().enumerate() {
            if k > 0 {
                records.push(StitchRecord::new(Point::new(0.0, 0.0), StitchKind::ColorChange));
            }
            for i in 0..=10 {
                records.push(StitchRecord::stitch(len * f64::from(i) / 10.0, 0.0));
            }
        }
        records.push(StitchRecord::new(Point::new(0.0, 0.0), StitchKind::End));
        StitchDesign {
            threads: vec![
                t(1, "Red", Rgb(200, 20, 30), "1747"),
                t(2, "Blue", Rgb(0, 0, 200), "1966"),
                t(3, "Red", Rgb(200, 20, 30), "1747"),
            ],
            records,
        }
    }

    #[test]
    fn thread_lengths_join_a_thread_that_sews_two_times() {
        let l = thread_lengths(&design(), 0.0);
        assert_eq!(l.len(), 2);
        // Top thread: 1.6 × the length, and a tail of 15 mm for each block.
        assert!((l[0].1 - (2.0 * (100.0 * 1.6 + 15.0))).abs() < 1e-6);
        assert!((l[1].1 - (50.0 * 1.6 + 15.0)).abs() < 1e-6);
    }

    #[test]
    fn threads_to_buy_for_an_order() {
        let mut stock = ThreadStock::default();
        stock.add(red());
        // 1000 pieces: red needs 350 m, 2 spools of 1000 m: 2 left (one begun), the lowest is 1.
        let buy = threads_to_buy(&stock, &design(), 0.0, 1000, 1000.0);
        assert_eq!(buy.len(), 1, "{buy:?}");
        assert_eq!(buy[0].look.name, "Blue");
        assert_eq!(buy[0].reason, BuyReason::NotInStock);
        assert_eq!(buy[0].spools, 1);
        assert!((buy[0].need_m - 95.0).abs() < 1e-6);
        // 10000 pieces: red needs 3500 m of 2000 m: 2 spools short, and 1 to keep.
        let buy = threads_to_buy(&stock, &design(), 0.0, 10_000, 1000.0);
        let r = buy.iter().find(|b| b.look.name == "Red").unwrap();
        assert_eq!((r.reason, r.spools), (BuyReason::TooLittle, 3));
        // 4000 pieces: 1400 m of 2000 m: 600 m left, one spool begun, the lowest is 2.
        stock.threads[0].min_spools = 2;
        let buy = threads_to_buy(&stock, &design(), 0.0, 4000, 1000.0);
        let r = buy.iter().find(|b| b.look.name == "Red").unwrap();
        assert_eq!((r.reason, r.spools), (BuyReason::UnderTheLowest, 1));
        let csv = buy_csv(&buy);
        assert!(csv.starts_with("brand,code,name,color,spools_to_buy"));
        assert!(csv.contains("Madeira Polyneon,1747,Red,#c8141e,1,1400,2000,under the lowest number after the order"));
    }

    #[test]
    fn use_stock_threads_changes_only_threads_not_in_stock() {
        use crate::{Design, ObjectKind, RunningParams};
        use emb_geom::Path;
        let mut d = Design::new("Badge");
        let a = d.add_thread("Almost red", Rgb(190, 30, 30));
        let b = d.add_thread("Unused", Rgb(0, 200, 0));
        let c = d.add_thread("Red", Rgb(200, 20, 30));
        for t in [a, c] {
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(10.0, 0.0)),
                    params: RunningParams::default(),
                },
            );
        }
        d.set_colorway_look(d.colorway, c, red().look());
        let mut stock = ThreadStock::default();
        stock.add(red());
        let changes = d.use_stock_threads(&stock);
        assert_eq!(
            changes.len(),
            1,
            "the red thread is in stock; the unused one does not change"
        );
        assert_eq!(changes[0].thread, a);
        assert_eq!(changes[0].to.code.as_deref(), Some("1747"));
        assert!(changes[0].delta_e > 0.0);
        assert_eq!(d.thread(a).unwrap().code.as_deref(), Some("1747"));
        assert_eq!(d.thread(b).unwrap().color, Rgb(0, 200, 0));
        // Nothing in stock: no changes.
        assert!(d.use_stock_threads(&ThreadStock::default()).is_empty());
    }

    #[test]
    fn cleaned_values() {
        let mut s = ThreadStock::default();
        s.threads.push(StockThread {
            spools: u32::MAX,
            spool_m: Some(f64::NAN),
            ..red()
        });
        let c = s.cleaned();
        assert_eq!(c.threads[0].spools, MAX_SPOOLS);
        assert_eq!(c.threads[0].spool_m, None);
    }
}

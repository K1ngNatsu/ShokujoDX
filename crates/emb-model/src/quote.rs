//! Quotes and run times of an order (P13 Milestone 1, ADR 0064).
//!
//! - [`PriceList`]: the prices of a shop, for example one list for shirts
//!   and one for caps.
//! - [`quote`]: the price of an order of one design, line by line.
//! - [`run_time`]: the time to sew the order on a machine with one or
//!   more heads, with the time to hoop the pieces.
//!
//! Money is a number with two decimals; the currency is text (the
//! currency of the thread prices, `ThreadPrices::currency`).

use serde::{Deserialize, Serialize};

use crate::{SewTimes, StitchDesign, ThreadLook};

/// From a quantity of pieces, a percent off the embroidery price.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuantityBreak {
    pub from: u32,
    pub percent_off: f64,
}

/// The prices of a shop. A price of 0 adds no line to a quote.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PriceList {
    pub name: String,
    /// The price of 1000 stitches of one piece.
    pub per_1000: f64,
    /// The lowest embroidery price of one piece.
    pub min_piece: f64,
    /// The number of colors that the stitch price includes.
    pub free_colors: u32,
    /// The added price of each color over `free_colors`, for each piece.
    pub per_color: f64,
    /// The price to set up the machine, once for each order.
    pub setup: f64,
    /// The price to digitize a new design, once: a fixed price and a price
    /// of 1000 stitches.
    pub digitizing: f64,
    pub digitizing_per_1000: f64,
    /// Percents off the embroidery price from a quantity.
    pub breaks: Vec<QuantityBreak>,
    /// The added percent of a rush order.
    pub rush_percent: f64,
    /// The tax percent.
    pub tax_percent: f64,
}

impl Default for PriceList {
    fn default() -> Self {
        PriceList {
            name: "Standard".into(),
            per_1000: 0.0,
            min_piece: 0.0,
            free_colors: 6,
            per_color: 0.0,
            setup: 0.0,
            digitizing: 0.0,
            digitizing_per_1000: 0.0,
            breaks: Vec::new(),
            rush_percent: 0.0,
            tax_percent: 0.0,
        }
    }
}

/// The largest money value of a price list, so a typing error cannot make
/// numbers that are too large to show.
pub const MAX_PRICE: f64 = 1.0e7;
/// The largest quantity of an order.
pub const MAX_QUANTITY: u32 = 1_000_000;

impl PriceList {
    /// A price list from TOML (the fields of [`PriceList`]; a field that is
    /// not given has its default value). An unknown field is an error, so a
    /// typing error does not give a price of 0. (The settings file accepts
    /// unknown fields, so an older program can read it.)
    pub fn from_toml_str(s: &str) -> Result<PriceList, String> {
        let table: toml::Table = toml::from_str(s).map_err(|e| e.to_string())?;
        let known = toml::Table::try_from(PriceList::default()).map_err(|e| e.to_string())?;
        if let Some(k) = table.keys().find(|k| !known.contains_key(*k)) {
            return Err(format!("unknown field {k:?} in the price list"));
        }
        table.try_into().map_err(|e: toml::de::Error| e.to_string())
    }

    /// True if the list has a price, so a quote has a value.
    pub fn is_set(&self) -> bool {
        [
            self.per_1000,
            self.min_piece,
            self.per_color,
            self.setup,
            self.digitizing,
            self.digitizing_per_1000,
        ]
        .iter()
        .any(|v| *v > 0.0)
    }

    /// The list with each value in its range (no negative prices, percents
    /// from 0 to 100, the breaks sorted by quantity, one for each quantity).
    pub fn cleaned(&self) -> PriceList {
        let money = |v: f64| if v.is_finite() { v.clamp(0.0, MAX_PRICE) } else { 0.0 };
        let pct = |v: f64| if v.is_finite() { v.clamp(0.0, 100.0) } else { 0.0 };
        let mut breaks: Vec<QuantityBreak> = self
            .breaks
            .iter()
            .map(|b| QuantityBreak {
                from: b.from.clamp(1, MAX_QUANTITY),
                percent_off: pct(b.percent_off),
            })
            .collect();
        breaks.sort_by_key(|b| b.from);
        breaks.dedup_by_key(|b| b.from);
        PriceList {
            name: self.name.clone(),
            per_1000: money(self.per_1000),
            min_piece: money(self.min_piece),
            free_colors: self.free_colors,
            per_color: money(self.per_color),
            setup: money(self.setup),
            digitizing: money(self.digitizing),
            digitizing_per_1000: money(self.digitizing_per_1000),
            breaks,
            rush_percent: pct(self.rush_percent),
            tax_percent: pct(self.tax_percent),
        }
    }

    /// The percent off for `quantity` pieces: the break with the largest
    /// quantity that is not more than `quantity`.
    pub fn percent_off(&self, quantity: u32) -> f64 {
        self.breaks
            .iter()
            .filter(|b| b.from <= quantity)
            .max_by_key(|b| b.from)
            .map_or(0.0, |b| b.percent_off)
    }
}

/// What the customer orders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Order {
    pub quantity: u32,
    /// The design is new, so the order pays for the digitizing.
    pub new_design: bool,
    pub rush: bool,
    /// The price of one garment, if the shop supplies them.
    pub garment_price: f64,
}

impl Default for Order {
    fn default() -> Self {
        Order {
            quantity: 1,
            new_design: false,
            rush: false,
            garment_price: 0.0,
        }
    }
}

/// The numbers of one piece that the price uses.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Piece {
    pub stitches: usize,
    /// The number of different threads.
    pub colors: usize,
}

impl Piece {
    pub fn of(s: &StitchDesign) -> Piece {
        Piece {
            stitches: s.stitch_count(),
            colors: thread_count(s),
        }
    }
}

/// The number of different threads of `s` (a thread that sews two times
/// is one thread, `needles::same_thread`).
pub fn thread_count(s: &StitchDesign) -> usize {
    if s.stitch_count() == 0 {
        return 0;
    }
    let mut seen: Vec<ThreadLook> = Vec::new();
    let mut unnamed = 0;
    for i in 0..s.color_count() {
        match s.threads.get(i) {
            Some(t) => {
                let look = ThreadLook::of(t);
                if !seen.iter().any(|x| crate::needles::same_thread(x, &look)) {
                    seen.push(look);
                }
            }
            None => unnamed += 1,
        }
    }
    seen.len() + unnamed
}

/// The kind of a line of a quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Stitches,
    ExtraColors,
    Minimum,
    QuantityBreak,
    Garment,
    Setup,
    Digitizing,
    Rush,
    Tax,
}

impl LineKind {
    pub const ALL: [LineKind; 9] = [
        LineKind::Stitches,
        LineKind::ExtraColors,
        LineKind::Minimum,
        LineKind::QuantityBreak,
        LineKind::Garment,
        LineKind::Setup,
        LineKind::Digitizing,
        LineKind::Rush,
        LineKind::Tax,
    ];

    /// The name of the line (English; the app translates it).
    pub fn label(self) -> &'static str {
        match self {
            LineKind::Stitches => "Embroidery",
            LineKind::ExtraColors => "Added colors",
            LineKind::Minimum => "To the lowest price for each piece",
            LineKind::QuantityBreak => "Quantity price",
            LineKind::Garment => "Garments",
            LineKind::Setup => "Setup",
            LineKind::Digitizing => "Digitizing",
            LineKind::Rush => "Rush order",
            LineKind::Tax => "Tax",
        }
    }

    /// True for a line with a price for each piece.
    pub fn per_piece(self) -> bool {
        matches!(
            self,
            LineKind::Stitches
                | LineKind::ExtraColors
                | LineKind::Minimum
                | LineKind::QuantityBreak
                | LineKind::Garment
        )
    }
}

/// One line of a quote.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuoteLine {
    pub kind: LineKind,
    /// The pieces of a line for each piece; 1 for a line of the order.
    pub quantity: u32,
    /// The price of one piece (or of the order).
    pub each: f64,
    pub amount: f64,
}

/// The price of an order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Quote {
    pub quantity: u32,
    pub lines: Vec<QuoteLine>,
    /// The embroidery price of one piece, after the quantity price.
    pub embroidery_each: f64,
    /// The sum before the rush price and the tax.
    pub subtotal: f64,
    pub total: f64,
    /// The total divided by the quantity.
    pub each: f64,
}

impl Quote {
    /// The price as one text for a sheet, for example "$12.13 for each
    /// piece, $582.25 for 48 pieces" (English, as the sheets).
    pub fn summary(&self, currency: &str) -> String {
        if self.quantity == 1 {
            return money_text(currency, self.total);
        }
        format!(
            "{} for each piece, {} for {} pieces",
            money_text(currency, self.each),
            money_text(currency, self.total),
            self.quantity
        )
    }
}

/// A money value with two decimals.
pub fn cents(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A money value as text, with the currency before the number.
pub fn money_text(currency: &str, v: f64) -> String {
    let v = cents(v);
    if v < 0.0 {
        format!("-{currency}{:.2}", -v)
    } else {
        format!("{currency}{v:.2}")
    }
}

/// The price of `order` for a design with the numbers `piece`.
///
/// The embroidery price of one piece is the stitch price plus the added
/// colors, at least the lowest price, less the quantity percent. Garments,
/// setup and digitizing are added; the rush percent is on that sum, and
/// the tax on the sum with the rush price. Each line is in cents, so the
/// lines add up to the total.
pub fn quote(list: &PriceList, piece: Piece, order: &Order) -> Quote {
    let list = list.cleaned();
    let qty = order.quantity.clamp(1, MAX_QUANTITY);
    let mut lines = Vec::new();
    let each_line = |lines: &mut Vec<QuoteLine>, kind, each: f64| {
        let each = cents(each);
        if each != 0.0 {
            lines.push(QuoteLine {
                kind,
                quantity: qty,
                each,
                amount: cents(each * f64::from(qty)),
            });
        }
        each
    };
    let k = piece.stitches as f64 / 1000.0;
    let mut emb = each_line(&mut lines, LineKind::Stitches, k * list.per_1000);
    let extra = piece.colors.saturating_sub(list.free_colors as usize) as f64;
    emb += each_line(&mut lines, LineKind::ExtraColors, extra * list.per_color);
    if emb < list.min_piece {
        emb += each_line(&mut lines, LineKind::Minimum, list.min_piece - emb);
    }
    emb += each_line(
        &mut lines,
        LineKind::QuantityBreak,
        -emb * list.percent_off(qty) / 100.0,
    );
    let garment = if order.garment_price.is_finite() {
        order.garment_price.clamp(0.0, MAX_PRICE)
    } else {
        0.0
    };
    each_line(&mut lines, LineKind::Garment, garment);
    let once = |lines: &mut Vec<QuoteLine>, kind, amount: f64| {
        let amount = cents(amount);
        if amount != 0.0 {
            lines.push(QuoteLine {
                kind,
                quantity: 1,
                each: amount,
                amount,
            });
        }
        amount
    };
    once(&mut lines, LineKind::Setup, list.setup);
    if order.new_design {
        once(
            &mut lines,
            LineKind::Digitizing,
            list.digitizing + k * list.digitizing_per_1000,
        );
    }
    let subtotal = cents(lines.iter().map(|l| l.amount).sum());
    let rush = if order.rush {
        once(&mut lines, LineKind::Rush, subtotal * list.rush_percent / 100.0)
    } else {
        0.0
    };
    once(&mut lines, LineKind::Tax, (subtotal + rush) * list.tax_percent / 100.0);
    let total = cents(lines.iter().map(|l| l.amount).sum());
    Quote {
        quantity: qty,
        lines,
        embroidery_each: cents(emb),
        subtotal,
        total,
        each: cents(total / f64::from(qty)),
    }
}

/// The time to sew an order.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RunTime {
    pub quantity: u32,
    pub heads: u32,
    /// The times that the machine sews, one piece on each head.
    pub cycles: u32,
    /// The time from the first hooping to the last frame change.
    pub minutes: f64,
    pub pieces_per_hour: f64,
    /// True if the operator sets the speed: hooping the pieces of all
    /// heads takes longer than sewing one piece.
    pub operator_bound: bool,
}

/// The run time of `quantity` pieces that sew in `piece_minutes` each, on
/// the machine of `t`.
///
/// One cycle sews one piece on each head. The operator hoops the pieces of
/// the first cycle before the start. With `hoop_while_sewing` (a second
/// set of hoops), the operator hoops the pieces of the next cycle while
/// the machine sews, so a cycle takes the longer of the two times; else
/// the times add. Each cycle ends with a frame change.
pub fn run_time(t: &SewTimes, piece_minutes: f64, quantity: u32) -> RunTime {
    let heads = t.heads.clamp(1, crate::machine::MAX_HEADS);
    let qty = quantity.clamp(1, MAX_QUANTITY);
    let cycles = qty.div_ceil(heads);
    let batch = |i: u32| f64::from(heads.min(qty.saturating_sub(i * heads)));
    let sew_s = piece_minutes.max(0.0) * 60.0;
    let hoop_s = t.hoop_s.max(0.0);
    let frame_s = t.frame_change_s.max(0.0);
    let mut total = batch(0) * hoop_s;
    for i in 0..cycles {
        let next = if i + 1 < cycles { batch(i + 1) * hoop_s } else { 0.0 };
        total += if t.hoop_while_sewing {
            sew_s.max(next)
        } else {
            sew_s + next
        } + frame_s;
    }
    let minutes = total / 60.0;
    RunTime {
        quantity: qty,
        heads,
        cycles,
        minutes,
        pieces_per_hour: if total > 0.0 {
            f64::from(qty) * 3600.0 / total
        } else {
            0.0
        },
        operator_bound: cycles > 1 && t.hoop_while_sewing && f64::from(heads) * hoop_s > sew_s,
    }
}

/// A time in minutes as text: "45 min" or "3 h 05 min".
pub fn duration_text(minutes: f64) -> String {
    let m = minutes.max(0.0).round() as u64;
    if m < 60 {
        format!("{m} min")
    } else {
        format!("{} h {:02} min", m / 60, m % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop() -> PriceList {
        PriceList {
            name: "Shirts".into(),
            per_1000: 1.5,
            min_piece: 5.0,
            free_colors: 6,
            per_color: 0.25,
            setup: 20.0,
            digitizing: 30.0,
            digitizing_per_1000: 0.0,
            breaks: vec![
                QuantityBreak {
                    from: 72,
                    percent_off: 20.0,
                },
                QuantityBreak {
                    from: 24,
                    percent_off: 10.0,
                },
            ],
            rush_percent: 50.0,
            tax_percent: 8.0,
        }
    }

    fn amount(q: &Quote, k: LineKind) -> Option<f64> {
        q.lines.iter().find(|l| l.kind == k).map(|l| l.amount)
    }

    #[test]
    fn an_order_with_a_quantity_price() {
        let order = Order {
            quantity: 48,
            new_design: true,
            rush: false,
            garment_price: 4.0,
        };
        let q = quote(
            &shop(),
            Piece {
                stitches: 8000,
                colors: 8,
            },
            &order,
        );
        // 8000 stitches: 12.00; 2 colors over 6: 0.50; 10 % off 12.50: -1.25.
        assert_eq!(amount(&q, LineKind::Stitches), Some(576.0));
        assert_eq!(amount(&q, LineKind::ExtraColors), Some(24.0));
        assert_eq!(amount(&q, LineKind::Minimum), None);
        assert_eq!(amount(&q, LineKind::QuantityBreak), Some(-60.0));
        assert_eq!(amount(&q, LineKind::Garment), Some(192.0));
        assert_eq!(amount(&q, LineKind::Setup), Some(20.0));
        assert_eq!(amount(&q, LineKind::Digitizing), Some(30.0));
        assert_eq!(amount(&q, LineKind::Rush), None);
        assert_eq!(q.embroidery_each, 11.25);
        assert_eq!(q.subtotal, 782.0);
        assert_eq!(amount(&q, LineKind::Tax), Some(62.56));
        assert_eq!(q.total, 844.56);
        assert_eq!(q.each, 17.6);
        assert_eq!(q.summary("$"), "$17.60 for each piece, $844.56 for 48 pieces");
        // From 72 pieces: 20 % off.
        let q = quote(
            &shop(),
            Piece {
                stitches: 8000,
                colors: 8,
            },
            &Order { quantity: 72, ..order },
        );
        assert_eq!(q.embroidery_each, 10.0);
    }

    #[test]
    fn the_lowest_price_and_a_rush_order() {
        let q = quote(
            &shop(),
            Piece {
                stitches: 2000,
                colors: 1,
            },
            &Order {
                quantity: 1,
                rush: true,
                ..Order::default()
            },
        );
        // 3.00 of stitches, 2.00 to the lowest price, setup 20: 25.00; rush 50 %: 12.50; tax 8 % of 37.50.
        assert_eq!(amount(&q, LineKind::Minimum), Some(2.0));
        assert_eq!(q.subtotal, 25.0);
        assert_eq!(amount(&q, LineKind::Rush), Some(12.5));
        assert_eq!(amount(&q, LineKind::Tax), Some(3.0));
        assert_eq!(q.total, 40.5);
        assert_eq!(q.lines.iter().map(|l| l.amount).sum::<f64>(), q.total);
    }

    #[test]
    fn an_empty_list_gives_no_lines() {
        let q = quote(
            &PriceList::default(),
            Piece {
                stitches: 5000,
                colors: 3,
            },
            &Order::default(),
        );
        assert!(q.lines.is_empty());
        assert_eq!(q.total, 0.0);
        assert!(!PriceList::default().is_set());
        assert!(shop().is_set());
    }

    #[test]
    fn bad_values_are_cleaned() {
        let mut l = shop();
        l.per_1000 = -3.0;
        l.tax_percent = 400.0;
        l.setup = f64::NAN;
        l.breaks.push(QuantityBreak {
            from: 24,
            percent_off: 50.0,
        });
        let c = l.cleaned();
        assert_eq!((c.per_1000, c.tax_percent, c.setup), (0.0, 100.0, 0.0));
        assert_eq!(c.breaks.iter().map(|b| b.from).collect::<Vec<_>>(), vec![24, 72]);
        let q = quote(
            &l,
            Piece::default(),
            &Order {
                quantity: 0,
                garment_price: f64::INFINITY,
                ..Order::default()
            },
        );
        assert_eq!(q.quantity, 1);
        assert!(q.total.is_finite());
    }

    fn six_heads() -> SewTimes {
        SewTimes {
            heads: 6,
            hoop_s: 60.0,
            frame_change_s: 20.0,
            ..SewTimes::default()
        }
    }

    #[test]
    fn run_time_of_a_machine_with_six_heads() {
        // 10 minutes a piece; 6 pieces hoop in 6 minutes, while the machine sews.
        let r = run_time(&six_heads(), 10.0, 48);
        assert_eq!(r.cycles, 8);
        // 360 s to hoop the first 6, then 8 cycles of 600 s + 20 s.
        assert!((r.minutes * 60.0 - 5320.0).abs() < 1e-6);
        assert!(!r.operator_bound);
        assert!((r.pieces_per_hour - 48.0 / (5320.0 / 3600.0)).abs() < 1e-9);
        // 4 minutes a piece: the operator needs 6 minutes for the next 6.
        let r = run_time(&six_heads(), 4.0, 48);
        assert!((r.minutes * 60.0 - (360.0 + 7.0 * 380.0 + 260.0)).abs() < 1e-6);
        assert!(r.operator_bound);
        // A last cycle with 4 pieces.
        let r = run_time(&six_heads(), 10.0, 10);
        assert_eq!(r.cycles, 2);
        assert!((r.minutes * 60.0 - 1600.0).abs() < 1e-6);
    }

    #[test]
    fn run_time_with_no_second_set_of_hoops() {
        let t = SewTimes {
            heads: 1,
            hoop_while_sewing: false,
            ..six_heads()
        };
        let r = run_time(&t, 10.0, 2);
        // 60 s to hoop, 600 + 60 + 20, then 600 + 20.
        assert!((r.minutes * 60.0 - 1360.0).abs() < 1e-6);
        assert!(!r.operator_bound);
        assert_eq!(duration_text(r.minutes), "23 min");
        assert_eq!(duration_text(185.0), "3 h 05 min");
    }

    #[test]
    fn price_list_from_toml() {
        let l = PriceList::from_toml_str(
            "name = 'Caps'
per_1000 = 2.0
setup = 15
[[breaks]]
from = 24
percent_off = 10
",
        )
        .unwrap();
        assert_eq!((l.name.as_str(), l.per_1000, l.setup), ("Caps", 2.0, 15.0));
        assert_eq!(l.free_colors, 6, "a field that is not given has its default");
        assert_eq!(l.percent_off(30), 10.0);
        assert!(PriceList::from_toml_str("per_1000 = 'a lot'").is_err());
        assert!(PriceList::from_toml_str("per_thousand = 2.0").is_err());
    }

    #[test]
    fn money_as_text() {
        assert_eq!(money_text("$", 17.595), "$17.60");
        assert_eq!(money_text("", -1.25), "-1.25");
        assert_eq!(money_text("€", 0.0), "€0.00");
    }

    #[test]
    fn threads_are_counted_once() {
        use crate::{Rgb, StitchKind, StitchRecord, Thread, ThreadId};
        use emb_geom::Point;
        let red = |id| Thread {
            id: ThreadId(id),
            name: "Red".into(),
            color: Rgb(200, 0, 0),
            brand: None,
            code: None,
        };
        let mut blue = red(3);
        blue.color = Rgb(0, 0, 200);
        let s = StitchDesign {
            records: vec![
                StitchRecord::stitch(0.0, 0.0),
                StitchRecord::new(Point::new(0.0, 0.0), StitchKind::ColorChange),
                StitchRecord::stitch(1.0, 0.0),
                StitchRecord::new(Point::new(1.0, 0.0), StitchKind::ColorChange),
                StitchRecord::stitch(2.0, 0.0),
                StitchRecord::new(Point::new(2.0, 0.0), StitchKind::End),
            ],
            threads: vec![red(1), blue, red(2)],
        };
        assert_eq!(thread_count(&s), 2);
        assert_eq!(Piece::of(&s).stitches, 3);
    }
}

//! Order details of a design (P13 Milestone 2): the customer, the order
//! number, the garment, the sizes and quantities, the placement and the due
//! date. The worksheet, the approval sheet, the quote and the design
//! properties CSV show them. A design has no order until the user types one
//! (`Design::order`); a template keeps no order.

use serde::{Deserialize, Serialize};

use crate::quote::{MAX_PRICE, MAX_QUANTITY, Order};

/// The pieces of one size, for example "M" and 20.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeCount {
    pub size: String,
    pub count: u32,
}

/// The order of a design.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OrderInfo {
    pub customer: String,
    /// The order number of the shop. The worksheet barcode has it.
    pub number: String,
    /// The garment, for example "Polo shirt".
    pub garment: String,
    pub garment_color: String,
    /// The pieces of each size. With no sizes, [`OrderInfo::quantity`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<SizeCount>,
    /// The number of pieces when the order has no sizes. 0: one piece.
    pub quantity: u32,
    /// Where the design goes, for example "Left chest".
    pub placement: String,
    /// The due date, as the user types it (for example 2026-11-02).
    pub due: String,
    /// The name of the price list of the quote. Empty: the first list.
    pub price_list: String,
    /// The design is new, so the quote has the digitizing price.
    pub new_design: bool,
    pub rush: bool,
    /// The price of one garment, if the shop supplies them.
    pub garment_price: f64,
    /// Notes for this order (the design notes are for all orders).
    pub notes: String,
}

impl OrderInfo {
    /// True if the order has no values, so the design needs no order.
    pub fn is_empty(&self) -> bool {
        *self == OrderInfo::default()
    }

    /// The number of pieces: the sum of the sizes, or the quantity (at
    /// least 1).
    pub fn total_quantity(&self) -> u32 {
        let sizes: u64 = self.sizes.iter().map(|s| u64::from(s.count)).sum();
        let q = if self.sizes.is_empty() {
            u64::from(self.quantity)
        } else {
            sizes
        };
        q.clamp(1, u64::from(MAX_QUANTITY)) as u32
    }

    /// The sizes as text, for example "S 10, M 20, L 5". Empty with no
    /// sizes.
    pub fn sizes_text(&self) -> String {
        self.sizes
            .iter()
            .filter(|s| s.count > 0)
            .map(|s| format!("{} {}", s.size.trim(), s.count))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The order of the quote.
    pub fn quote_order(&self) -> Order {
        let g = self.garment_price;
        Order {
            quantity: self.total_quantity(),
            new_design: self.new_design,
            rush: self.rush,
            garment_price: if g.is_finite() { g.clamp(0.0, MAX_PRICE) } else { 0.0 },
        }
    }

    /// The garment and its color as one text, for example "Polo shirt,
    /// navy".
    pub fn garment_text(&self) -> String {
        [self.garment.trim(), self.garment_color.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The rows of the order for a sheet: a name and a value, only the
    /// values that are set. The names are English (the sheets are in
    /// English).
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        let mut add = |k: &'static str, v: String| {
            if !v.trim().is_empty() {
                rows.push((k, v.trim().to_string()));
            }
        };
        add("Order", self.number.clone());
        add("Customer", self.customer.clone());
        add("Garment", self.garment_text());
        let quantity = if self.sizes.is_empty() {
            if self.quantity > 0 {
                self.quantity.to_string()
            } else {
                String::new()
            }
        } else {
            format!("{} ({})", self.total_quantity(), self.sizes_text())
        };
        add("Quantity", quantity);
        add("Placement", self.placement.clone());
        add("Due", self.due.clone());
        rows
    }

    /// The order with each value in its range: no sizes with no name and no
    /// count, a quantity and a garment price in their ranges.
    pub fn cleaned(&self) -> OrderInfo {
        let mut o = self.clone();
        o.sizes.retain(|s| !s.size.trim().is_empty() || s.count > 0);
        for s in &mut o.sizes {
            s.count = s.count.min(MAX_QUANTITY);
        }
        o.quantity = o.quantity.min(MAX_QUANTITY);
        o.garment_price = if o.garment_price.is_finite() {
            o.garment_price.clamp(0.0, MAX_PRICE)
        } else {
            0.0
        };
        o
    }

    /// A file name part from the order number: the characters that a file
    /// name cannot have become `-`. Empty if the order has no number.
    pub fn file_part(&self) -> String {
        self.number
            .trim()
            .chars()
            .map(|c| {
                if c.is_control() || r#"/\:*?"<>|"#.contains(c) {
                    '-'
                } else {
                    c
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order() -> OrderInfo {
        OrderInfo {
            customer: "Riverside FC".into(),
            number: "A-1042".into(),
            garment: "Polo shirt".into(),
            garment_color: "Navy".into(),
            sizes: vec![
                SizeCount {
                    size: "S".into(),
                    count: 10,
                },
                SizeCount {
                    size: "M".into(),
                    count: 20,
                },
                SizeCount {
                    size: "XL".into(),
                    count: 0,
                },
            ],
            quantity: 3,
            placement: "Left chest".into(),
            due: "2026-11-02".into(),
            garment_price: 4.5,
            rush: true,
            ..OrderInfo::default()
        }
    }

    #[test]
    fn quantity_from_sizes_or_the_quantity() {
        let o = order();
        assert_eq!(o.total_quantity(), 30);
        assert_eq!(o.sizes_text(), "S 10, M 20");
        let q = o.quote_order();
        assert_eq!((q.quantity, q.rush, q.garment_price), (30, true, 4.5));
        let plain = OrderInfo { sizes: Vec::new(), ..o };
        assert_eq!(plain.total_quantity(), 3);
        assert_eq!(OrderInfo::default().total_quantity(), 1);
    }

    #[test]
    fn rows_for_the_sheets() {
        let rows = order().rows();
        let get = |k: &str| rows.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("Order").as_deref(), Some("A-1042"));
        assert_eq!(get("Garment").as_deref(), Some("Polo shirt, Navy"));
        assert_eq!(get("Quantity").as_deref(), Some("30 (S 10, M 20)"));
        assert_eq!(get("Due").as_deref(), Some("2026-11-02"));
        // Only the values that are set.
        let only = OrderInfo {
            customer: "Ann".into(),
            ..OrderInfo::default()
        };
        assert_eq!(only.rows(), vec![("Customer", "Ann".to_string())]);
        assert!(OrderInfo::default().is_empty());
        assert!(!only.is_empty());
    }

    #[test]
    fn cleaned_and_file_part() {
        let mut o = order();
        o.sizes.push(SizeCount {
            size: " ".into(),
            count: 0,
        });
        o.garment_price = f64::NAN;
        o.quantity = u32::MAX;
        let c = o.cleaned();
        assert_eq!(c.sizes.len(), 3, "the empty row goes; XL with 0 stays");
        assert_eq!(c.garment_price, 0.0);
        assert_eq!(c.quantity, MAX_QUANTITY);
        let o = OrderInfo {
            number: " 2026/11:7 ".into(),
            ..OrderInfo::default()
        };
        assert_eq!(o.file_part(), "2026-11-7");
        assert_eq!(OrderInfo::default().file_part(), "");
    }

    #[test]
    fn a_design_with_and_without_an_order() {
        let mut d = crate::Design::new("Badge");
        let s = serde_json::to_string(&d).unwrap();
        assert!(!s.contains("\"order\""), "an old program reads it as before");
        assert!(serde_json::from_str::<crate::Design>(&s).unwrap().order.is_none());
        d.order = Some(order());
        let s = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<crate::Design>(&s).unwrap().order, Some(order()));
    }

    #[test]
    fn json_round_trip() {
        let o = order();
        let s = serde_json::to_string(&o).unwrap();
        assert_eq!(serde_json::from_str::<OrderInfo>(&s).unwrap(), o);
        // An order from a later program with an unknown field still opens.
        let later = s.replacen('{', "{\"tracking\":\"x\",", 1);
        assert_eq!(serde_json::from_str::<OrderInfo>(&later).unwrap(), o);
    }
}

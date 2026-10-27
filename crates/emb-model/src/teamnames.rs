//! Team names (P4 Milestone 6, ADR 0020): one design, many names.
//!
//! Lettering objects of the design are the columns (for example a name
//! and a number). Each row of the list has a value for each column and a
//! quantity. `emb-letter` makes the design of a row (the lettering objects
//! with the values of the row) and a layout of all rows in one design.

use serde::{Deserialize, Serialize};

use crate::ObjectId;

/// A lettering object whose text changes for each row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamColumn {
    pub object: ObjectId,
    /// The column title, for example "Name" or "Number".
    pub title: String,
    /// A longer text gets a smaller height, so that it is at most this wide
    /// (mm). `None`: no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_width_mm: Option<f64>,
}

/// One row: a value for each column, and how many to sew.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamRow {
    pub values: Vec<String>,
    #[serde(default = "one")]
    pub quantity: u32,
}

fn one() -> u32 {
    1
}

impl TeamRow {
    /// The first value that is not empty (to name files).
    pub fn label(&self) -> &str {
        self.values
            .iter()
            .map(|v| v.trim())
            .find(|v| !v.is_empty())
            .unwrap_or("")
    }
}

/// The team names of a design.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TeamNames {
    pub columns: Vec<TeamColumn>,
    pub rows: Vec<TeamRow>,
}

impl TeamNames {
    /// The number of designs to sew (the sum of the quantities).
    pub fn total(&self) -> u32 {
        self.rows.iter().map(|r| r.quantity).sum()
    }

    /// Adds rows from text: one row for each line; the values are separated
    /// by tabs, commas or semicolons (the first of these in the line), and
    /// may be in double quotes. With `header`, the first line is skipped.
    /// Returns the number of rows added.
    pub fn import_text(&mut self, text: &str, header: bool) -> usize {
        let n = self.columns.len().max(1);
        let before = self.rows.len();
        for line in text.lines().skip(usize::from(header)) {
            if line.trim().is_empty() {
                continue;
            }
            let mut values = split_line(line);
            values.resize(n, String::new());
            self.rows.push(TeamRow { values, quantity: 1 });
        }
        self.rows.len() - before
    }

    /// Makes each row have a value for each column.
    pub fn fit_rows(&mut self) {
        let n = self.columns.len().max(1);
        for r in &mut self.rows {
            r.values.resize(n, String::new());
        }
    }
}

/// The values of one line of a CSV or tab-separated list.
pub fn split_line(line: &str) -> Vec<String> {
    let sep = ['\t', ',', ';'].into_iter().find(|c| line.contains(*c));
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if Some(c) == sep && !quoted => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_csv_and_tabs() {
        let mut t = TeamNames {
            columns: vec![
                TeamColumn {
                    object: ObjectId(1),
                    title: "Name".into(),
                    max_width_mm: None,
                },
                TeamColumn {
                    object: ObjectId(2),
                    title: "Number".into(),
                    max_width_mm: None,
                },
            ],
            rows: vec![],
        };
        let n = t.import_text("Name,Number\nAnna,7\n\"O'Brien, Sean\",12\nLee\n", true);
        assert_eq!(n, 3);
        assert_eq!(t.rows[1].values, ["O'Brien, Sean", "12"]);
        assert_eq!(t.rows[2].values, ["Lee", ""]);
        t.import_text("Max\t23", false);
        assert_eq!(t.rows[3].values, ["Max", "23"]);
        assert_eq!(split_line("a;\"b\"\"c\";d"), ["a", "b\"c", "d"]);
        t.rows[0].quantity = 3;
        assert_eq!(t.total(), 6);
        assert_eq!(t.rows[0].label(), "Anna");
    }
}

//! Barcodes for the worksheets (P8 Milestone 7): Code 39 and Code 128
//! (set B). A barcode is a list of modules (`true` is a bar); the page
//! draws each run of bars as a filled rectangle.
//!
//! The patterns are the ones of the public symbol tables of the two codes
//! (ISO/IEC 16388 for Code 39, ISO/IEC 15417 for Code 128).

use emb_geom::{BBox, Point};
use emb_model::Rgb;
use serde::{Deserialize, Serialize};

use crate::page::{Op, Page, text_width_mm};

/// The kind of barcode on the worksheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarcodeKind {
    None,
    /// Capital letters, digits and `- . $ / + %` and space.
    Code39,
    /// All printable ASCII characters.
    #[default]
    Code128,
}

impl BarcodeKind {
    pub const ALL: [BarcodeKind; 3] = [BarcodeKind::None, BarcodeKind::Code39, BarcodeKind::Code128];

    pub fn label(self) -> &'static str {
        match self {
            BarcodeKind::None => "No barcode",
            BarcodeKind::Code39 => "Code 39",
            BarcodeKind::Code128 => "Code 128",
        }
    }

    /// The modules of `text`, or `None` if the code cannot hold the text.
    pub fn encode(self, text: &str) -> Option<Vec<bool>> {
        match self {
            BarcodeKind::None => None,
            BarcodeKind::Code39 => code39(text),
            BarcodeKind::Code128 => code128(text),
        }
    }
}

/// Code 39 characters and their patterns: 9 elements (bar, space, bar, …),
/// `1` for a wide element.
const CODE39: [(char, &str); 44] = [
    ('0', "000110100"),
    ('1', "100100001"),
    ('2', "001100001"),
    ('3', "101100000"),
    ('4', "000110001"),
    ('5', "100110000"),
    ('6', "001110000"),
    ('7', "000100101"),
    ('8', "100100100"),
    ('9', "001100100"),
    ('A', "100001001"),
    ('B', "001001001"),
    ('C', "101001000"),
    ('D', "000011001"),
    ('E', "100011000"),
    ('F', "001011000"),
    ('G', "000001101"),
    ('H', "100001100"),
    ('I', "001001100"),
    ('J', "000011100"),
    ('K', "100000011"),
    ('L', "001000011"),
    ('M', "101000010"),
    ('N', "000010011"),
    ('O', "100010010"),
    ('P', "001010010"),
    ('Q', "000000111"),
    ('R', "100000110"),
    ('S', "001000110"),
    ('T', "000010110"),
    ('U', "110000001"),
    ('V', "011000001"),
    ('W', "111000000"),
    ('X', "010010001"),
    ('Y', "110010000"),
    ('Z', "011010000"),
    ('-', "010000101"),
    ('.', "110000100"),
    (' ', "011000100"),
    ('$', "010101000"),
    ('/', "010100010"),
    ('+', "010001010"),
    ('%', "000101010"),
    ('*', "010010100"),
];

/// Code 39 with the start and stop character `*`, a wide element of 3
/// modules and a narrow space between characters. Small letters become
/// capitals. No check character (it is optional in Code 39).
pub fn code39(text: &str) -> Option<Vec<bool>> {
    let mut out = Vec::new();
    let chars: Vec<char> = std::iter::once('*')
        .chain(text.chars().map(|c| c.to_ascii_uppercase()))
        .chain(std::iter::once('*'))
        .collect();
    for (k, c) in chars.iter().enumerate() {
        if *c == '*' && k != 0 && k != chars.len() - 1 {
            return None;
        }
        let (_, pattern) = CODE39.iter().find(|(x, _)| x == c)?;
        if k > 0 {
            out.push(false);
        }
        for (i, w) in pattern.bytes().enumerate() {
            let n = if w == b'1' { 3 } else { 1 };
            out.extend(std::iter::repeat_n(i % 2 == 0, n));
        }
    }
    Some(out)
}

/// The Code 128 symbols 0 to 106: the widths of bar, space, bar, space,
/// bar, space (the stop symbol has a seventh bar).
const CODE128: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212", "221213", "221312",
    "231212", "112232", "122132", "122231", "113222", "123122", "123221", "223211", "221132", "221231", "213212",
    "223112", "312131", "311222", "321122", "321221", "312212", "322112", "322211", "212123", "212321", "232121",
    "111323", "131123", "131321", "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331",
    "132131", "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131", "311123",
    "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111", "111224", "111422", "121124",
    "121421", "141122", "141221", "112214", "112412", "122114", "122411", "142112", "142211", "241211", "221114",
    "413111", "241112", "134111", "111242", "121142", "121241", "114212", "124112", "124211", "411212", "421112",
    "421211", "212141", "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];
const START_B: usize = 104;
const STOP: usize = 106;

/// Code 128 in set B (start B, the characters, the check symbol, stop).
/// `None` if a character is not printable ASCII.
pub fn code128(text: &str) -> Option<Vec<bool>> {
    let mut symbols = vec![START_B];
    for c in text.chars() {
        let v = c as u32;
        if !(32..127).contains(&v) {
            return None;
        }
        symbols.push((v - 32) as usize);
    }
    let check = symbols
        .iter()
        .enumerate()
        .map(|(i, &s)| if i == 0 { s } else { s * i })
        .sum::<usize>()
        % 103;
    symbols.push(check);
    symbols.push(STOP);
    let mut out = Vec::new();
    for s in symbols {
        for (i, w) in CODE128[s].bytes().enumerate() {
            out.extend(std::iter::repeat_n(i % 2 == 0, usize::from(w - b'0')));
        }
    }
    Some(out)
}

/// Draws the barcode of `text` with its top right corner at `(right, top)`,
/// `height` mm high, with the text under it. Returns `false` (and draws
/// nothing) if the code cannot hold the text. The module is as wide as
/// fits in `max_width` mm, from 0.19 mm (so that a phone reads it) to
/// 0.33 mm.
pub fn draw(page: &mut Page, kind: BarcodeKind, text: &str, right: f64, top: f64, height: f64, max_width: f64) -> bool {
    let Some(modules) = kind.encode(text) else {
        return false;
    };
    // Quiet zones of 10 modules on each side.
    let n = modules.len() + 20;
    let module = (max_width / n as f64).clamp(0.19, 0.33);
    let left = right - n as f64 * module + 10.0 * module;
    let mut i = 0;
    while i < modules.len() {
        if !modules[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < modules.len() && modules[i] {
            i += 1;
        }
        let x0 = left + start as f64 * module;
        page.ops.push(Op::Rect {
            rect: BBox::new(Point::new(x0, top), Point::new(left + i as f64 * module, top + height)),
            fill: Some(Rgb(0, 0, 0)),
            stroke: None,
        });
    }
    let width = modules.len() as f64 * module;
    let label: String = text.chars().take(40).collect();
    let tw = text_width_mm(&label, 7.0);
    page.gray_text(left + (width - tw).max(0.0) / 2.0, top + height + 3.0, 7.0, label);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code39_patterns_have_three_wide_elements() {
        for (c, p) in CODE39 {
            assert_eq!(p.len(), 9, "{c}");
            assert_eq!(p.bytes().filter(|&b| b == b'1').count(), 3, "{c}");
        }
        // Each character is 15 modules, with one narrow space between.
        let m = code39("AB").unwrap();
        assert_eq!(m.len(), 4 * 15 + 3);
        assert_eq!(code39("ab"), code39("AB"));
        assert!(code39("a*b").is_none() && code39("é").is_none());
    }

    #[test]
    fn code128_symbols_are_eleven_modules() {
        for (i, p) in CODE128.iter().enumerate() {
            let sum: u32 = p.bytes().map(|b| u32::from(b - b'0')).sum();
            assert_eq!(sum, if i == STOP { 13 } else { 11 }, "symbol {i}");
        }
        let mut sorted = CODE128.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 107, "each symbol is different");
        // Start B, 'A' (33), check (104 + 33) % 103 = 34, stop.
        let m = code128("A").unwrap();
        assert_eq!(m.len(), 11 * 3 + 13);
        assert!(code128("é").is_none());
    }

    #[test]
    fn draw_puts_bars_and_the_text_on_the_page() {
        let mut p = Page::new(crate::page::Paper::A4);
        assert!(draw(
            &mut p,
            BarcodeKind::Code128,
            "Rose patch",
            195.0,
            15.0,
            10.0,
            60.0
        ));
        let bars = p.ops.iter().filter(|o| matches!(o, Op::Rect { .. })).count();
        assert!(bars > 10);
        for o in &p.ops {
            if let Op::Rect { rect, .. } = o {
                assert!(
                    rect.max.x <= 195.0 + 1e-9 && rect.min.x >= 195.0 - 60.0 - 1e-9,
                    "{rect:?}"
                );
            }
        }
        assert!(!draw(&mut p, BarcodeKind::Code39, "é", 195.0, 15.0, 10.0, 60.0));
        assert!(!draw(&mut p, BarcodeKind::None, "x", 195.0, 15.0, 10.0, 60.0));
    }
}

//! Thread charts: lists of commercial or custom thread colors.
//!
//! A chart is a GIMP palette (`.gpl`) or a CSV file (`code,name,#rrggbb`).
//! The app ships the Brother PEC chart (`data/threads/brother-pec.gpl`) and
//! the 74 brand charts of Ink/Stitch (`data/threads/inkstitch`, GPL-3.0;
//! owner decision, 2026-10-25); users import other charts. See
//! `docs/design/p3-plan.md` and `docs/design/p11-plan.md`.

use emb_geom::math;

use crate::{ModelError, Rgb};

/// One thread of a chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartThread {
    pub code: Option<String>,
    pub name: String,
    pub color: Rgb,
    /// The brand of the thread, if it is not the chart name (the chart of
    /// the threads in stock has threads of more brands, `stock`).
    pub brand: Option<String>,
}

/// A named list of threads, usually of one brand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadChart {
    pub name: String,
    pub threads: Vec<ChartThread>,
}

const PEC_GPL: &str = include_str!("../../../data/threads/brother-pec.gpl");

include!("charts_inkstitch.rs");

impl ThreadChart {
    /// The charts that are part of the program: Brother PEC first, then the
    /// brand charts by name.
    pub fn builtin() -> Vec<ThreadChart> {
        let mut v = vec![ThreadChart::parse_gpl(PEC_GPL, "Brother PEC").expect("built-in chart is valid")];
        v.extend(
            INKSTITCH
                .iter()
                .map(|(name, text)| ThreadChart::parse_gpl(text, name).expect("built-in chart is valid (tested)")),
        );
        v
    }

    /// Reads a chart file. The format comes from the extension (`.gpl` or
    /// `.csv`); the file name is the chart name if the file has none.
    pub fn load(path: &std::path::Path) -> Result<ThreadChart, ModelError> {
        let text = std::fs::read_to_string(path).map_err(|e| ModelError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Chart");
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("csv") => ThreadChart::parse_csv(&text, stem),
            _ => ThreadChart::parse_gpl(&text, stem),
        }
    }

    /// Parses a GIMP palette. A color line is `R G B name`. If the last word
    /// of the name has a digit, it is the thread code.
    pub fn parse_gpl(text: &str, default_name: &str) -> Result<ThreadChart, ModelError> {
        let bad = |line: usize, message: &str| ModelError::Invalid {
            what: "thread chart",
            message: format!("line {line}: {message}"),
        };
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, l)) if l.trim_start_matches('\u{feff}').trim() == "GIMP Palette" => {}
            _ => return Err(bad(1, "the first line must be \"GIMP Palette\"")),
        }
        let mut chart = ThreadChart {
            name: default_name.to_string(),
            threads: Vec::new(),
        };
        for (i, line) in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("Columns:") {
                continue;
            }
            if let Some(name) = line.strip_prefix("Name:") {
                chart.name = name.trim().to_string();
                continue;
            }
            let mut words = line.split_whitespace();
            let mut channel = || -> Result<u8, ModelError> {
                words
                    .next()
                    .and_then(|w| w.parse().ok())
                    .ok_or_else(|| bad(i + 1, "expected three numbers 0 to 255"))
            };
            let color = Rgb(channel()?, channel()?, channel()?);
            let mut rest: Vec<&str> = words.collect();
            let code = match rest.last() {
                Some(w) if rest.len() > 1 && w.chars().any(|c| c.is_ascii_digit()) => rest.pop().map(str::to_string),
                _ => None,
            };
            let name = if rest.is_empty() {
                color.to_hex()
            } else {
                rest.join(" ")
            };
            chart.threads.push(ChartThread {
                code,
                name,
                color,
                brand: None,
            });
        }
        if chart.threads.is_empty() {
            return Err(bad(1, "the chart has no colors"));
        }
        Ok(chart)
    }

    /// Parses `code,name,#rrggbb` lines. A first line that has no color is a header.
    pub fn parse_csv(text: &str, name: &str) -> Result<ThreadChart, ModelError> {
        let mut chart = ThreadChart {
            name: name.to_string(),
            threads: Vec::new(),
        };
        for (i, line) in text.lines().enumerate() {
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() {
                continue;
            }
            let cells: Vec<&str> = line.split(',').map(|c| c.trim().trim_matches('"')).collect();
            let color = cells.get(2).and_then(|c| Rgb::from_hex(c));
            match (color, i) {
                (Some(color), _) => chart.threads.push(ChartThread {
                    code: Some(cells[0].to_string()).filter(|c| !c.is_empty()),
                    name: cells[1].to_string(),
                    color,
                    brand: None,
                }),
                (None, 0) => {}
                (None, _) => {
                    return Err(ModelError::Invalid {
                        what: "thread chart",
                        message: format!("line {}: expected code,name,#rrggbb", i + 1),
                    });
                }
            }
        }
        if chart.threads.is_empty() {
            return Err(ModelError::Invalid {
                what: "thread chart",
                message: "the chart has no colors".into(),
            });
        }
        Ok(chart)
    }

    /// The index of the thread with the nearest color (CIE76).
    pub fn nearest(&self, color: Rgb) -> Option<usize> {
        let lab = Lab::of(color);
        (0..self.threads.len()).min_by(|&a, &b| {
            let da = lab.distance(Lab::of(self.threads[a].color));
            let db = lab.distance(Lab::of(self.threads[b].color));
            da.total_cmp(&db)
        })
    }

    /// The indexes of the threads whose name or code has all the words of
    /// `query` (case is ignored).
    pub fn search(&self, query: &str) -> Vec<usize> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        (0..self.threads.len())
            .filter(|&i| {
                let t = &self.threads[i];
                let text = format!("{} {}", t.name, t.code.as_deref().unwrap_or("")).to_lowercase();
                words.iter().all(|w| text.contains(w.as_str()))
            })
            .collect()
    }
}

/// A color in CIELAB (D65 white point).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

impl Lab {
    pub fn of(c: Rgb) -> Lab {
        let lin = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                math::pow((v + 0.055) / 1.055, 2.4)
            }
        };
        let (r, g, b) = (lin(c.0), lin(c.1), lin(c.2));
        let x = (0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / 0.950_47;
        let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
        let z = (0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b) / 1.088_83;
        let f = |t: f64| {
            if t > 216.0 / 24389.0 {
                math::cbrt(t)
            } else {
                (24389.0 / 27.0 * t + 16.0) / 116.0
            }
        };
        let (fx, fy, fz) = (f(x), f(y), f(z));
        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    /// The CIE76 color difference. About 2.3 is the smallest difference that
    /// a person can see.
    pub fn distance(self, o: Lab) -> f64 {
        ((self.l - o.l).powi(2) + (self.a - o.a).powi(2) + (self.b - o.b).powi(2)).sqrt()
    }
}

/// Hue (degrees, 0 to 360), saturation and value (0 to 1) of a color.
pub fn hsv(c: Rgb) -> (f64, f64, f64) {
    let (r, g, b) = (f64::from(c.0) / 255.0, f64::from(c.1) / 255.0, f64::from(c.2) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

/// The color with hue `h` (degrees), saturation `s` and value `v`.
pub fn from_hsv(h: f64, s: f64, v: f64) -> Rgb {
    let h = h.rem_euclid(360.0) / 60.0;
    let (s, v) = (s.clamp(0.0, 1.0), v.clamp(0.0, 1.0));
    let c = v * s;
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let u = |k: f64| ((k + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb(u(r), u(g), u(b))
}

/// Colors that go with `c` on the color wheel: the same saturation and
/// value at other hues. Complementary (across the wheel), analogous (30°
/// to each side), triad (120° to each side) and split complementary (150°
/// to each side).
pub fn harmonies(c: Rgb) -> Vec<(&'static str, Rgb)> {
    let (h, s, v) = hsv(c);
    [
        ("Complementary", 180.0),
        ("Analogous", -30.0),
        ("Analogous", 30.0),
        ("Triad", -120.0),
        ("Triad", 120.0),
        ("Split complementary", -150.0),
        ("Split complementary", 150.0),
    ]
    .into_iter()
    .map(|(name, turn)| (name, from_hsv(h + turn, s, v)))
    .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn hsv_round_trip_and_harmonies() {
        use super::*;
        for c in [
            Rgb(255, 0, 0),
            Rgb(12, 200, 99),
            Rgb(40, 40, 40),
            Rgb(250, 240, 10),
            Rgb(0, 0, 255),
        ] {
            let (h, s, v) = hsv(c);
            assert_eq!(from_hsv(h, s, v), c, "{c:?}");
        }
        assert_eq!(hsv(Rgb(0, 255, 0)).0, 120.0);
        let h = harmonies(Rgb(255, 0, 0));
        assert_eq!(h[0], ("Complementary", Rgb(0, 255, 255)));
        assert_eq!(h[3].1, Rgb(0, 0, 255));
        assert_eq!(h[4].1, Rgb(0, 255, 0));
        // A gray has no hue: all harmonies are the same gray.
        assert!(harmonies(Rgb(90, 90, 90)).iter().all(|(_, c)| *c == Rgb(90, 90, 90)));
    }

    use super::*;

    #[test]
    fn builtin_pec_chart() {
        let charts = ThreadChart::builtin();
        let pec = &charts[0];
        assert_eq!(pec.name, "Brother PEC");
        assert_eq!(pec.threads.len(), 64);
        assert_eq!(pec.threads[0].name, "Prussian Blue");
        assert_eq!(pec.threads[0].code.as_deref(), Some("1"));
        assert_eq!(pec.threads[0].color, Rgb(14, 31, 124));
    }

    #[test]
    fn the_brand_charts_of_ink_stitch_load() {
        let charts = ThreadChart::builtin();
        assert_eq!(charts.len(), 1 + 74);
        let mut names = std::collections::HashSet::new();
        for c in &charts[1..] {
            assert!(names.insert(c.name.clone()), "{} twice", c.name);
            assert!(!c.name.contains("Ink/Stitch"), "{}", c.name);
            assert!(c.threads.len() >= 10, "{}: {}", c.name, c.threads.len());
            // Nearly all threads have a code (the number of the maker).
            let coded = c.threads.iter().filter(|t| t.code.is_some()).count();
            assert!(
                coded * 10 >= c.threads.len() * 9,
                "{}: {coded} of {}",
                c.name,
                c.threads.len()
            );
        }
        let madeira = charts
            .iter()
            .find(|c| c.name == "Madeira Rayon")
            .expect("Madeira Rayon");
        let black = &madeira.threads[0];
        assert_eq!(
            (black.name.as_str(), black.code.as_deref(), black.color),
            ("Black", Some("1000"), Rgb(40, 41, 42))
        );
        assert!(charts.iter().any(|c| c.name == "Isacord Polyester"));
        assert!(charts.iter().any(|c| c.name == "Robison-Anton Rayon"));
        assert!(charts.iter().any(|c| c.name == "Sulky Rayon"));
    }

    #[test]
    fn gpl_names_and_codes() {
        let text = "GIMP Palette\nName: Test Rayon\nColumns: 4\n# comment\n\
                    255 255 255\tWhite 1001\n0 0 0 Black\n10 20 30\n200 0 0 Red  Dark 1147A\n";
        let c = ThreadChart::parse_gpl(text, "file").unwrap();
        assert_eq!(c.name, "Test Rayon");
        let got: Vec<(&str, Option<&str>)> = c.threads.iter().map(|t| (t.name.as_str(), t.code.as_deref())).collect();
        assert_eq!(
            got,
            [
                ("White", Some("1001")),
                ("Black", None),
                ("#0a141e", None),
                ("Red Dark", Some("1147A"))
            ]
        );
        assert!(ThreadChart::parse_gpl("Palette\n1 2 3 x\n", "f").is_err());
        assert!(ThreadChart::parse_gpl("GIMP Palette\n1 2 x y\n", "f").is_err());
        assert!(ThreadChart::parse_gpl("GIMP Palette\nName: empty\n", "f").is_err());
    }

    #[test]
    fn csv_with_header() {
        let text = "code,name,color\n1001,White,#ffffff\n,No code,#000000\n";
        let c = ThreadChart::parse_csv(text, "Mine").unwrap();
        assert_eq!(c.name, "Mine");
        assert_eq!(c.threads.len(), 2);
        assert_eq!(c.threads[1].code, None);
        assert!(ThreadChart::parse_csv("a,b,#fff\n1,x,#000000\nbad line\n", "x").is_err());
    }

    #[test]
    fn nearest_and_search() {
        let pec = &ThreadChart::builtin()[0];
        let red = pec.nearest(Rgb(236, 20, 30)).unwrap();
        assert_eq!(pec.threads[red].name, "Red");
        let white = pec.nearest(Rgb(250, 250, 250)).unwrap();
        assert_eq!(pec.threads[white].name, "White");
        assert!(pec.search("blue").len() >= 5);
        assert_eq!(pec.search("prussian 1").len(), 1);
        assert_eq!(pec.search(" ").len(), 64);
    }

    #[test]
    fn lab_reference_values() {
        let w = Lab::of(Rgb(255, 255, 255));
        assert!(
            (w.l - 100.0).abs() < 0.01 && w.a.abs() < 0.01 && w.b.abs() < 0.01,
            "{w:?}"
        );
        let r = Lab::of(Rgb(255, 0, 0));
        assert!(
            (r.l - 53.24).abs() < 0.05 && (r.a - 80.09).abs() < 0.1 && (r.b - 67.20).abs() < 0.1,
            "{r:?}"
        );
    }
}

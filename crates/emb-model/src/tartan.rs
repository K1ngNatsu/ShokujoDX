//! The sett of a tartan fill (P11 Milestone 4, ADR 0047).
//!
//! A sett is the list of stripes of one repeat of a tartan, with the
//! number of threads (the count) of each stripe. The usual text form is a
//! color code and a count for each stripe: `K4 R24 K24 Y4`. Most tartans
//! are symmetric: the stripes go out from a pivot and back (`mirror`).
//!
//! The color codes are the common letters of thread counts (K black, R
//! red, G green, and more); the colors of the codes are our own values.

use emb_geom::Point;
use serde::{Deserialize, Serialize};

use crate::Rgb;
use crate::object::{FillParams, FillPattern, Underlay};

/// Most stripes in one sett.
pub const MAX_STRIPES: usize = 200;

/// The stripes of a tartan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sett {
    /// The color code of each color of the tartan (index = color number).
    pub colors: Vec<String>,
    /// The stripes of one repeat: the color number and the count.
    pub stripes: Vec<(u32, f64)>,
    /// The width of one count, in mm.
    pub count_mm: f64,
    /// The stripes go out from the first stripe and back (a symmetric sett).
    #[serde(default = "yes")]
    pub mirror: bool,
}

fn yes() -> bool {
    true
}

impl Default for Sett {
    /// A simple check: black and white stripes 4 mm wide.
    fn default() -> Self {
        Sett {
            colors: vec!["K".into(), "W".into()],
            stripes: vec![(0, 8.0), (1, 8.0)],
            count_mm: 0.5,
            mirror: false,
        }
    }
}

/// Common color codes of thread counts and our colors for them.
pub const CODES: &[(&str, &str, Rgb)] = &[
    ("K", "Black", Rgb(20, 20, 24)),
    ("W", "White", Rgb(240, 240, 236)),
    ("R", "Red", Rgb(190, 20, 30)),
    ("DR", "Dark red", Rgb(120, 10, 20)),
    ("C", "Crimson", Rgb(170, 0, 50)),
    ("M", "Maroon", Rgb(100, 20, 40)),
    ("O", "Orange", Rgb(230, 110, 20)),
    ("Y", "Yellow", Rgb(240, 200, 30)),
    ("GD", "Gold", Rgb(200, 150, 40)),
    ("G", "Green", Rgb(20, 100, 50)),
    ("DG", "Dark green", Rgb(10, 60, 35)),
    ("LG", "Light green", Rgb(110, 170, 90)),
    ("B", "Blue", Rgb(30, 50, 130)),
    ("DB", "Dark blue", Rgb(15, 25, 70)),
    ("LB", "Light blue", Rgb(110, 160, 210)),
    ("A", "Azure", Rgb(80, 150, 200)),
    ("P", "Purple", Rgb(90, 40, 110)),
    ("N", "Grey", Rgb(130, 130, 130)),
    ("T", "Brown", Rgb(110, 70, 40)),
    ("LT", "Tan", Rgb(190, 150, 100)),
];

/// The name and color of a code (case does not matter); unknown codes are grey.
pub fn code_color(code: &str) -> (String, Rgb) {
    let up = code.to_uppercase();
    match CODES.iter().find(|(c, _, _)| *c == up) {
        Some((_, name, rgb)) => (name.to_string(), *rgb),
        None => (format!("Color {up}"), Rgb(150, 150, 150)),
    }
}

/// Why a sett text cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettError {
    /// A part that is not a code and a count, such as `R` or `12`.
    Part(String),
    /// No stripes.
    Empty,
    /// More than [`MAX_STRIPES`] stripes.
    TooMany,
}

impl std::fmt::Display for SettError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettError::Part(p) => write!(f, "\"{p}\" is not a color code and a count (for example R8)"),
            SettError::Empty => write!(f, "The sett has no stripes"),
            SettError::TooMany => write!(f, "The sett has more than {MAX_STRIPES} stripes"),
        }
    }
}

impl Sett {
    /// Reads a sett text such as `K4 R24 K24 Y4` (spaces or commas between
    /// the stripes; a `/` of a pivot count is ignored).
    pub fn parse(text: &str, count_mm: f64, mirror: bool) -> Result<Sett, SettError> {
        let mut colors: Vec<String> = Vec::new();
        let mut stripes = Vec::new();
        for part in text
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|p| !p.is_empty())
        {
            let clean: String = part.chars().filter(|c| *c != '/').collect();
            let split = clean.find(|c: char| c.is_ascii_digit() || c == '.');
            let (code, count) = match split {
                Some(i) if i > 0 => (&clean[..i], &clean[i..]),
                _ => return Err(SettError::Part(part.to_string())),
            };
            let count: f64 = count.parse().map_err(|_| SettError::Part(part.to_string()))?;
            if !count.is_finite() || count <= 0.0 || !code.chars().all(char::is_alphabetic) {
                return Err(SettError::Part(part.to_string()));
            }
            let code = code.to_uppercase();
            let color = match colors.iter().position(|c| *c == code) {
                Some(i) => i,
                None => {
                    colors.push(code);
                    colors.len() - 1
                }
            };
            stripes.push((color as u32, count));
        }
        if stripes.is_empty() {
            return Err(SettError::Empty);
        }
        if stripes.len() > MAX_STRIPES {
            return Err(SettError::TooMany);
        }
        Ok(Sett {
            colors,
            stripes,
            count_mm,
            mirror,
        })
    }

    /// The text form of the stripes.
    pub fn text(&self) -> String {
        self.stripes
            .iter()
            .map(|(c, n)| {
                let code = self.colors.get(*c as usize).map_or("?", String::as_str);
                let n = (n * 100.0).round() / 100.0;
                format!("{code}{n}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The stripes of one full repeat, as (color, width in mm). A mirrored
    /// sett goes out to the last stripe and back.
    pub fn repeat(&self) -> Vec<(u32, f64)> {
        let w = self.count_mm.max(0.01);
        let mut out: Vec<(u32, f64)> = self.stripes.iter().map(|(c, n)| (*c, n.max(0.0) * w)).collect();
        if self.mirror && out.len() > 2 {
            let back: Vec<(u32, f64)> = out[1..out.len() - 1].iter().rev().copied().collect();
            out.extend(back);
        }
        out.retain(|(_, w)| *w > 0.0);
        out
    }

    /// The width of one full repeat, in mm.
    pub fn width_mm(&self) -> f64 {
        self.repeat().iter().map(|(_, w)| w).sum()
    }
}

/// The fill settings of the colors of a tartan made from `base`: one for
/// each color of the sett, in color order. Only the first keeps the
/// underlay of `base` (the colors together cover the shape one time).
pub fn tartan_layers(base: &FillParams, sett: &Sett) -> Vec<FillParams> {
    let first = base.with_pattern(FillPattern::Tartan {
        sett: sett.clone(),
        color: 0,
    });
    (0..sett.colors.len().max(1) as u32)
        .map(|color| FillParams {
            pattern: FillPattern::Tartan {
                sett: sett.clone(),
                color,
            },
            underlay: if color == 0 {
                first.underlay.clone()
            } else {
                Underlay::None
            },
            ..first.clone()
        })
        .collect()
}

/// The fill settings of the colors of a gradient made from `base`, with
/// the colors at the places `stops` (0 to 1). Only the first has the
/// underlay of `base`: the colors together cover the shape.
pub fn gradient_layers(base: &FillParams, line: Option<(Point, Point)>, stops: &[f64]) -> Vec<FillParams> {
    let first = base.with_pattern(FillPattern::Gradient {
        line,
        stops: stops.to_vec(),
        color: 0,
    });
    (0..stops.len().max(1) as u32)
        .map(|color| FillParams {
            pattern: FillPattern::Gradient {
                line,
                stops: stops.to_vec(),
                color,
            },
            underlay: if color == 0 {
                base.underlay.clone()
            } else {
                Underlay::None
            },
            ..first.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sett_text_reads_and_writes() {
        let s = Sett::parse("K4 R/24, k24 Y4", 0.5, true).unwrap();
        assert_eq!(s.colors, ["K", "R", "Y"]);
        assert_eq!(s.stripes, [(0, 4.0), (1, 24.0), (0, 24.0), (2, 4.0)]);
        assert_eq!(s.text(), "K4 R24 K24 Y4");
        // Mirrored: K R K Y K R, so 4 + 24 + 24 + 4 + 24 + 24 counts.
        assert_eq!(s.repeat().len(), 6);
        assert!((s.width_mm() - 52.0).abs() < 1e-9);
        let plain = Sett { mirror: false, ..s };
        assert!((plain.width_mm() - 28.0).abs() < 1e-9);
    }

    #[test]
    fn wrong_sett_texts_say_why() {
        assert_eq!(Sett::parse("", 0.5, true), Err(SettError::Empty));
        assert_eq!(Sett::parse("R8 12", 0.5, true), Err(SettError::Part("12".into())));
        assert_eq!(Sett::parse("R", 0.5, true), Err(SettError::Part("R".into())));
        assert_eq!(Sett::parse("R0", 0.5, true), Err(SettError::Part("R0".into())));
        assert_eq!(Sett::parse("R-2", 0.5, true), Err(SettError::Part("R-2".into())));
        let long = "R2 ".repeat(MAX_STRIPES + 1);
        assert_eq!(Sett::parse(&long, 0.5, true), Err(SettError::TooMany));
    }

    #[test]
    fn the_layers_of_a_tartan_and_a_gradient() {
        let base = FillParams {
            angle_deg: 30.0,
            ..FillParams::default()
        };
        let sett = Sett::parse("K4 R8 G4", 0.5, true).unwrap();
        let t = tartan_layers(&base, &sett);
        assert_eq!(t.len(), 3);
        assert!(t.iter().all(|p| p.angle_deg == 30.0 && p.spacing_mm == 0.8));
        assert_eq!(t[0].underlay, Underlay::Auto);
        assert!(t[1..].iter().all(|p| p.underlay == Underlay::None));
        assert!(matches!(t[2].pattern, FillPattern::Tartan { color: 2, .. }));
        let g = gradient_layers(&base, None, &[0.0, 0.5, 1.0]);
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].underlay, Underlay::Auto);
        // One gradient fill alone has no underlay.
        let alone = base.with_pattern(FillPattern::Gradient {
            line: None,
            stops: vec![0.0, 1.0],
            color: 0,
        });
        assert_eq!(alone.underlay, Underlay::None);
        assert!(matches!(&g[1].pattern, FillPattern::Gradient { color: 1, stops, .. } if stops.len() == 3));
    }

    #[test]
    fn codes_have_colors() {
        assert_eq!(code_color("k").0, "Black");
        assert_eq!(code_color("DG").1, Rgb(10, 60, 35));
        assert_eq!(code_color("XQ").0, "Color XQ");
    }
}

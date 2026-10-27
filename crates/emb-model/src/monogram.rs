//! Monograms: one to three initials in a style, with a border and
//! ornaments (P4 Milestone 5, ADR 0019).
//!
//! `emb-letter` makes the objects of a monogram from [`MonogramSpec`]: a
//! lettering object for the initials, and satin, running and fill objects
//! for the border and the ornaments. The app adds them as one group and
//! keeps a [`MonogramRecord`] in the design, so that the monogram can
//! change later. A [`MonogramTemplate`] is a spec with a name, for new
//! initials.

use emb_geom::Point;
use serde::{Deserialize, Serialize};

use crate::{FontRef, Rgb};

/// How the initials stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonogramStyle {
    /// The first initial only.
    Single,
    /// The initials side by side, all the same size.
    #[default]
    Block,
    /// The middle initial larger, the outer ones smaller.
    Classic,
    /// The first initial large; the other two small, one above the other,
    /// to its right.
    Stacked,
    /// The initials in a diamond shape: tallest in the middle.
    Diamond,
    /// The initials fill an oval: the outer ones shorter.
    Oval,
}

impl MonogramStyle {
    pub const ALL: [MonogramStyle; 6] = [
        MonogramStyle::Single,
        MonogramStyle::Block,
        MonogramStyle::Classic,
        MonogramStyle::Stacked,
        MonogramStyle::Diamond,
        MonogramStyle::Oval,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MonogramStyle::Single => "Single letter",
            MonogramStyle::Block => "Block",
            MonogramStyle::Classic => "Classic (large middle)",
            MonogramStyle::Stacked => "Stacked",
            MonogramStyle::Diamond => "Diamond",
            MonogramStyle::Oval => "Oval",
        }
    }
}

/// The shape of the border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderShape {
    #[default]
    None,
    Circle,
    Oval,
    Square,
    RoundedSquare,
    Diamond,
    /// Two circles, the outer one a running line.
    DoubleCircle,
}

impl BorderShape {
    pub const ALL: [BorderShape; 7] = [
        BorderShape::None,
        BorderShape::Circle,
        BorderShape::Oval,
        BorderShape::Square,
        BorderShape::RoundedSquare,
        BorderShape::Diamond,
        BorderShape::DoubleCircle,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BorderShape::None => "None",
            BorderShape::Circle => "Circle",
            BorderShape::Oval => "Oval",
            BorderShape::Square => "Square",
            BorderShape::RoundedSquare => "Rounded square",
            BorderShape::Diamond => "Diamond",
            BorderShape::DoubleCircle => "Double circle",
        }
    }
}

/// The stitch of the border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderStitch {
    #[default]
    Satin,
    Run,
    /// A triple (bean) run.
    Triple,
}

impl BorderStitch {
    pub const ALL: [BorderStitch; 3] = [BorderStitch::Satin, BorderStitch::Run, BorderStitch::Triple];

    pub fn name(self) -> &'static str {
        match self {
            BorderStitch::Satin => "Satin",
            BorderStitch::Run => "Running stitch",
            BorderStitch::Triple => "Triple run",
        }
    }
}

/// The border round the initials (and the ornaments).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Border {
    pub shape: BorderShape,
    pub stitch: BorderStitch,
    /// The width of a satin border, in mm.
    pub width_mm: f64,
    /// The space between the initials and the border, in mm.
    pub margin_mm: f64,
}

impl Default for Border {
    fn default() -> Self {
        Border {
            shape: BorderShape::None,
            stitch: BorderStitch::Satin,
            width_mm: 2.5,
            margin_mm: 3.0,
        }
    }
}

/// A small decoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrnamentShape {
    #[default]
    None,
    Dot,
    Diamond,
    Star,
    Heart,
    Leaf,
    /// A curled line.
    Scroll,
}

impl OrnamentShape {
    pub const ALL: [OrnamentShape; 7] = [
        OrnamentShape::None,
        OrnamentShape::Dot,
        OrnamentShape::Diamond,
        OrnamentShape::Star,
        OrnamentShape::Heart,
        OrnamentShape::Leaf,
        OrnamentShape::Scroll,
    ];

    pub fn name(self) -> &'static str {
        match self {
            OrnamentShape::None => "None",
            OrnamentShape::Dot => "Dot",
            OrnamentShape::Diamond => "Diamond",
            OrnamentShape::Star => "Star",
            OrnamentShape::Heart => "Heart",
            OrnamentShape::Leaf => "Leaf",
            OrnamentShape::Scroll => "Scroll",
        }
    }
}

/// Where the ornaments go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrnamentPlace {
    /// One on each side of the initials (mirrored).
    #[default]
    Sides,
    /// One above and one below the initials.
    TopBottom,
    /// Both.
    All,
}

impl OrnamentPlace {
    pub const ALL: [OrnamentPlace; 3] = [OrnamentPlace::Sides, OrnamentPlace::TopBottom, OrnamentPlace::All];

    pub fn name(self) -> &'static str {
        match self {
            OrnamentPlace::Sides => "Left and right",
            OrnamentPlace::TopBottom => "Above and below",
            OrnamentPlace::All => "All four sides",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ornaments {
    pub shape: OrnamentShape,
    pub place: OrnamentPlace,
    /// The size of one ornament, in mm.
    pub size_mm: f64,
}

impl Default for Ornaments {
    fn default() -> Self {
        Ornaments {
            shape: OrnamentShape::None,
            place: OrnamentPlace::Sides,
            size_mm: 6.0,
        }
    }
}

/// Everything about a monogram except the initials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonogramSpec {
    pub style: MonogramStyle,
    pub font: FontRef,
    /// The height of the largest initial, in mm.
    pub height_mm: f64,
    #[serde(default)]
    pub border: Border,
    #[serde(default)]
    pub ornaments: Ornaments,
    pub letter_color: Rgb,
    pub border_color: Rgb,
    pub ornament_color: Rgb,
}

impl MonogramSpec {
    /// A block monogram in `font` with no border or ornaments.
    pub fn new(font: FontRef) -> MonogramSpec {
        MonogramSpec {
            style: MonogramStyle::Block,
            font,
            height_mm: 30.0,
            border: Border::default(),
            ornaments: Ornaments::default(),
            letter_color: Rgb(25, 35, 90),
            border_color: Rgb(180, 140, 40),
            ornament_color: Rgb(180, 140, 40),
        }
    }
}

/// A monogram in a design: the group of its objects, the initials and the
/// spec, and the middle of the monogram in the design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonogramRecord {
    pub group: u32,
    pub initials: String,
    pub spec: MonogramSpec,
    pub center: Point,
}

/// A named monogram spec for new initials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonogramTemplate {
    pub name: String,
    pub spec: MonogramSpec,
}

/// The file extension of monogram templates (JSON).
pub const TEMPLATE_EXTENSION: &str = "monogram";

impl MonogramTemplate {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<MonogramTemplate, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    /// The templates that come with the app, in `font` (a bundled
    /// embroidery font).
    pub fn builtin(font: &FontRef) -> Vec<MonogramTemplate> {
        let t = |name: &str, style, border: Border, ornaments: Ornaments| MonogramTemplate {
            name: name.to_string(),
            spec: MonogramSpec {
                style,
                border,
                ornaments,
                ..MonogramSpec::new(font.clone())
            },
        };
        let border = |shape, stitch| Border {
            shape,
            stitch,
            ..Border::default()
        };
        let orn = |shape, place, size_mm| Ornaments { shape, place, size_mm };
        vec![
            t(
                "Classic in a circle",
                MonogramStyle::Classic,
                border(BorderShape::Circle, BorderStitch::Satin),
                Ornaments::default(),
            ),
            t(
                "Diamond with hearts",
                MonogramStyle::Diamond,
                Border::default(),
                orn(OrnamentShape::Heart, OrnamentPlace::Sides, 8.0),
            ),
            t(
                "Block in a square",
                MonogramStyle::Block,
                border(BorderShape::RoundedSquare, BorderStitch::Triple),
                orn(OrnamentShape::Dot, OrnamentPlace::Sides, 4.0),
            ),
            t(
                "Stacked in an oval",
                MonogramStyle::Stacked,
                border(BorderShape::Oval, BorderStitch::Satin),
                Ornaments::default(),
            ),
            t(
                "Oval in a double circle",
                MonogramStyle::Oval,
                border(BorderShape::DoubleCircle, BorderStitch::Satin),
                orn(OrnamentShape::Star, OrnamentPlace::TopBottom, 6.0),
            ),
            t(
                "Single letter with scrolls",
                MonogramStyle::Single,
                border(BorderShape::Diamond, BorderStitch::Run),
                orn(OrnamentShape::Scroll, OrnamentPlace::Sides, 14.0),
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_round_trip() {
        let all = MonogramTemplate::builtin(&FontRef::embroidery("Block Sans", ""));
        assert_eq!(all.len(), 6);
        for t in all {
            assert_eq!(MonogramTemplate::from_json(&t.to_json()).unwrap(), t);
        }
        let min: MonogramSpec = serde_json::from_str(
            r##"{"style":"classic","font":{"family":"F","style":""},"height_mm":20,
                "letter_color":"#000000","border_color":"#000000","ornament_color":"#000000"}"##,
        )
        .unwrap();
        assert_eq!(min.border, Border::default());
    }
}

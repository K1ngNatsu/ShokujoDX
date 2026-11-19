use serde::{Deserialize, Serialize};

/// Stable identifier of a thread in a design palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThreadId(pub u32);

/// An sRGB color. The serialized form is `"#rrggbb"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    pub fn from_hex(s: &str) -> Option<Rgb> {
        let h = s.strip_prefix('#')?;
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Rgb(c(0)?, c(2)?, c(4)?))
    }
}

impl TryFrom<String> for Rgb {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Rgb::from_hex(&s).ok_or_else(|| format!("invalid color {s:?}, expected \"#rrggbb\""))
    }
}

impl From<Rgb> for String {
    fn from(c: Rgb) -> String {
        c.to_hex()
    }
}

/// The kind of a thread (P18 view plan, Milestone 2, ADR 0123): how it
/// takes the light. The view uses it; it does not change the stitches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadKind {
    /// Polyester: it shines. The default: most machine embroidery thread
    /// is polyester or rayon.
    #[default]
    Polyester,
    /// Rayon (viscose): it shines the most.
    Rayon,
    /// Cotton and other matte threads: no shine.
    Cotton,
    /// Metallic thread: small, strong highlights.
    Metallic,
    /// Wool and acrylic: thick and matte.
    Wool,
    /// Thread that glows in the dark: a little shine.
    Glow,
}

impl ThreadKind {
    pub const ALL: [ThreadKind; 6] = [
        ThreadKind::Polyester,
        ThreadKind::Rayon,
        ThreadKind::Cotton,
        ThreadKind::Metallic,
        ThreadKind::Wool,
        ThreadKind::Glow,
    ];

    /// The name of the kind (English; the program shows it translated).
    pub fn name(self) -> &'static str {
        match self {
            ThreadKind::Polyester => "Polyester",
            ThreadKind::Rayon => "Rayon",
            ThreadKind::Cotton => "Cotton (matte)",
            ThreadKind::Metallic => "Metallic",
            ThreadKind::Wool => "Wool or acrylic",
            ThreadKind::Glow => "Glow in the dark",
        }
    }

    pub fn is_default(&self) -> bool {
        *self == ThreadKind::default()
    }
}

/// The weight of a thread that the stitch settings are for (a weight
/// number: the higher the number, the thinner the thread).
pub const DEFAULT_WEIGHT: u16 = 40;
/// The weights that a thread can have.
pub const WEIGHTS: std::ops::RangeInclusive<u16> = 8..=150;
/// The weights that the Threads panel lists.
pub const COMMON_WEIGHTS: [u16; 10] = [12, 15, 20, 30, 35, 40, 50, 60, 75, 100];
/// The width of a 40 weight thread on the fabric (the width of the design
/// picture since P10).
pub const WIDTH_40_MM: f64 = 0.4;

/// The width of a thread of `weight`, as a part of the width of a 40
/// weight thread. A weight number is a length for each mass, so the
/// section of a thread is inversely proportional to its weight, and its
/// width is the square root of the ratio of the weights (a 60 weight
/// thread is 0.82 times as wide as a 40 weight thread).
pub fn width_factor(weight: u16) -> f64 {
    let w = weight.clamp(*WEIGHTS.start(), *WEIGHTS.end());
    (f64::from(DEFAULT_WEIGHT) / f64::from(w)).sqrt()
}

/// The width of a thread of `weight` on the fabric, mm.
pub fn width_mm(weight: u16) -> f64 {
    WIDTH_40_MM * width_factor(weight)
}

fn default_weight() -> u16 {
    DEFAULT_WEIGHT
}

fn is_default_weight(w: &u16) -> bool {
    *w == DEFAULT_WEIGHT
}

/// A thread in the design palette.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: ThreadId,
    pub name: String,
    pub color: Rgb,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// The kind of the thread (ADR 0123).
    #[serde(default, skip_serializing_if = "ThreadKind::is_default")]
    pub kind: ThreadKind,
    /// The weight number of the thread (ADR 0123).
    #[serde(default = "default_weight", skip_serializing_if = "is_default_weight")]
    pub weight: u16,
}

impl Default for Thread {
    /// A black polyester thread of weight 40 with id 0 and no name.
    fn default() -> Self {
        Thread {
            id: ThreadId(0),
            name: String::new(),
            color: Rgb(0, 0, 0),
            brand: None,
            code: None,
            kind: ThreadKind::default(),
            weight: DEFAULT_WEIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thread_of_an_earlier_version_reads_with_the_defaults() {
        let t: Thread = serde_json::from_str(r##"{"id":3,"name":"Red","color":"#c81e2d"}"##).unwrap();
        assert_eq!((t.kind, t.weight), (ThreadKind::Polyester, 40));
        // The defaults are not written, so a file of an earlier design stays the same.
        assert_eq!(
            serde_json::to_string(&t).unwrap(),
            r##"{"id":3,"name":"Red","color":"#c81e2d"}"##
        );
        let fine = Thread {
            kind: ThreadKind::Cotton,
            weight: 60,
            ..t
        };
        let text = serde_json::to_string(&fine).unwrap();
        assert!(
            text.contains(r#""kind":"cotton""#) && text.contains(r#""weight":60"#),
            "{text}"
        );
        assert_eq!(serde_json::from_str::<Thread>(&text).unwrap(), fine);
    }

    #[test]
    fn a_thinner_thread_is_narrower() {
        assert_eq!(width_mm(40), WIDTH_40_MM);
        assert!((width_factor(60) - 0.816).abs() < 0.001);
        assert!((width_factor(12) - 1.826).abs() < 0.001);
        assert!(width_mm(30) > width_mm(40) && width_mm(40) > width_mm(60));
        // Out of the range: the nearest weight.
        assert_eq!(width_factor(0), width_factor(8));
        assert_eq!(width_factor(1000), width_factor(150));
        assert!(COMMON_WEIGHTS.iter().all(|w| WEIGHTS.contains(w)));
        let names: std::collections::BTreeSet<&str> = ThreadKind::ALL.iter().map(|k| k.name()).collect();
        assert_eq!(names.len(), ThreadKind::ALL.len());
    }

    #[test]
    fn hex_round_trip() {
        let c = Rgb(0x12, 0xab, 0xff);
        assert_eq!(c.to_hex(), "#12abff");
        assert_eq!(Rgb::from_hex("#12ABFF"), Some(c));
        assert_eq!(Rgb::from_hex("12abff"), None);
        assert_eq!(Rgb::from_hex("#12ab"), None);
    }
}

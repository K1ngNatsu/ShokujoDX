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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Rgb(0x12, 0xab, 0xff);
        assert_eq!(c.to_hex(), "#12abff");
        assert_eq!(Rgb::from_hex("#12ABFF"), Some(c));
        assert_eq!(Rgb::from_hex("12abff"), None);
        assert_eq!(Rgb::from_hex("#12ab"), None);
    }
}

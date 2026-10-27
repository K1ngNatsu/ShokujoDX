//! Keyboard design collections (P4 Milestone 6, ADR 0020): a design for
//! each key. Typed text places the designs of its characters side by side.
//! Unlike an embroidery font, a key design can have more than one color
//! (for example a letter with an outline, or an appliqué letter).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ObjectKind, Rgb};

/// One object of a key design and its color.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyObject {
    pub color: Rgb,
    pub kind: ObjectKind,
}

/// The design of one key, in mm at the design height of the collection,
/// with the baseline at Y = 0 and the left side at X = 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyDesign {
    /// The distance to the next key design.
    pub advance_mm: f64,
    pub objects: Vec<KeyObject>,
}

/// The file extension of keyboard design collections (JSON).
pub const FILE_EXTENSION: &str = "embkdc";

/// A keyboard design collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyboardCollection {
    pub format: String,
    pub version: u32,
    pub name: String,
    /// The height that the key designs are stored at (their capital height).
    pub height_mm: f64,
    /// The advance of a space.
    pub space_mm: f64,
    /// The key designs, by character.
    pub keys: BTreeMap<String, KeyDesign>,
}

impl KeyboardCollection {
    pub fn new(name: &str, height_mm: f64) -> KeyboardCollection {
        KeyboardCollection {
            format: "emb-keyboard-collection".into(),
            version: 1,
            name: name.to_string(),
            height_mm,
            space_mm: 0.3 * height_mm,
            keys: BTreeMap::new(),
        }
    }

    pub fn key(&self, c: char) -> Option<&KeyDesign> {
        self.keys.get(&c.to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<KeyboardCollection, String> {
        let k: KeyboardCollection = serde_json::from_str(s).map_err(|e| e.to_string())?;
        if k.format != "emb-keyboard-collection" {
            return Err("not a keyboard design collection".into());
        }
        Ok(k)
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;

    #[test]
    fn round_trip() {
        let mut k = KeyboardCollection::new("Test", 20.0);
        k.keys.insert(
            "A".into(),
            KeyDesign {
                advance_mm: 15.0,
                objects: vec![KeyObject {
                    color: Rgb(1, 2, 3),
                    kind: ObjectKind::Fill {
                        outer: Path::rect(BBox::new(Point::new(0.0, -20.0), Point::new(14.0, 0.0))),
                        holes: vec![],
                        params: Default::default(),
                    },
                }],
            },
        );
        let back = KeyboardCollection::from_json(&k.to_json()).unwrap();
        assert_eq!(back, k);
        assert!(back.key('A').is_some() && back.key('B').is_none());
        assert!(
            KeyboardCollection::from_json(
                r#"{"format":"x","version":1,"name":"","height_mm":1,"space_mm":1,"keys":{}}"#
            )
            .is_err()
        );
    }
}

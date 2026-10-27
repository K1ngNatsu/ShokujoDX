//! Hoop presets from `data/hoops.toml`.

use std::collections::BTreeSet;

use emb_geom::{BBox, Path, Point};
use serde::{Deserialize, Serialize};

use crate::ModelError;

/// The built-in presets, from `data/hoops.toml` at build time.
pub const BUILTIN_HOOPS_TOML: &str = include_str!("../../../data/hoops.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoopShape {
    Rectangle,
    RoundedRectangle,
    Oval,
    Circle,
}

/// A hoop preset. The size is the sewable field, not the outer frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoopPreset {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    pub width_mm: f64,
    pub height_mm: f64,
    pub shape: HoopShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_stitch_length_mm: Option<f64>,
    /// The fixed positions of a multi-position hoop (two or more), or none.
    /// The size of the preset is then the field of all positions together.
    #[serde(default, rename = "position", skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<HoopPosition>,
    /// A cap frame (P11 Milestone 7): designs in it sew from the center out
    /// and from the bottom up (Arrange > Sew order > Cap order), so the cap
    /// fabric does not move.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cap: bool,
}

/// One fixed position of a multi-position hoop: the field that the machine
/// sews while the hoop is in this position, and the center of that field
/// from the hoop center (mm, the hoop straight, Y down).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoopPosition {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl HoopPreset {
    /// The field of position `i` as a rectangular preset of its own (for the
    /// checks of one position). `None` if there is no such position.
    pub fn position_preset(&self, i: usize) -> Option<HoopPreset> {
        let p = self.positions.get(i)?;
        Some(HoopPreset {
            id: format!("{}#{}", self.id, i + 1),
            name: format!("{}, position {}", self.name, i + 1),
            brand: self.brand.clone(),
            width_mm: p.width_mm,
            height_mm: p.height_mm,
            shape: HoopShape::Rectangle,
            corner_radius_mm: None,
            max_stitch_length_mm: self.max_stitch_length_mm,
            positions: Vec::new(),
            cap: self.cap,
        })
    }

    /// The sewable field as a box with its center at `center`.
    pub fn field_bbox(&self, center: Point) -> BBox {
        BBox::from_center_size(center, self.width_mm, self.height_mm)
    }

    /// The outline of the sewable field made smaller by `margin`, with its center at `center`.
    /// Returns `None` if the margin removes the full field.
    pub fn outline(&self, center: Point, margin: f64) -> Option<Path> {
        let (w, h) = (self.width_mm - 2.0 * margin, self.height_mm - 2.0 * margin);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let b = BBox::from_center_size(center, w, h);
        Some(match self.shape {
            HoopShape::Rectangle => Path::rect(b),
            HoopShape::RoundedRectangle => {
                Path::rounded_rect(b, (self.corner_radius_mm.unwrap_or(0.0) - margin).max(0.0))
            }
            HoopShape::Oval | HoopShape::Circle => Path::ellipse(center, w / 2.0, h / 2.0),
        })
    }

    /// Returns `true` if `p` (relative to the hoop center) is inside the
    /// sewable field made smaller by `margin` mm on each side.
    ///
    /// For an oval, the margin makes each semi-axis smaller by `margin`.
    /// This is exact for a circle and a close approximation for an oval.
    pub fn contains(&self, p: Point, margin: f64) -> bool {
        let hw = self.width_mm / 2.0 - margin;
        let hh = self.height_mm / 2.0 - margin;
        if hw < 0.0 || hh < 0.0 {
            return false;
        }
        let (x, y) = (p.x.abs(), p.y.abs());
        match self.shape {
            HoopShape::Rectangle => x <= hw && y <= hh,
            HoopShape::RoundedRectangle => {
                if x > hw || y > hh {
                    return false;
                }
                let r = (self.corner_radius_mm.unwrap_or(0.0) - margin).max(0.0);
                let (cx, cy) = (hw - r, hh - r);
                if x > cx && y > cy {
                    Point::new(x - cx, y - cy).length() <= r
                } else {
                    true
                }
            }
            HoopShape::Oval | HoopShape::Circle => {
                if hw == 0.0 || hh == 0.0 {
                    return x == 0.0 && y == 0.0;
                }
                (x / hw).powi(2) + (y / hh).powi(2) <= 1.0
            }
        }
    }

    /// Returns `true` if a centered box of `width` × `height` fits inside the field
    /// made smaller by `margin`.
    pub fn fits(&self, width: f64, height: f64, margin: f64) -> bool {
        self.contains(Point::new(width / 2.0, height / 2.0), margin)
    }

    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(format!("hoop id {id:?} must use only a-z, 0-9, '_' and '-'"));
        }
        if self.name.trim().is_empty() {
            return Err(format!("hoop {id}: name is empty"));
        }
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(self.width_mm) || !positive(self.height_mm) {
            return Err(format!("hoop {id}: width_mm and height_mm must be more than 0"));
        }
        if self.shape == HoopShape::Circle && self.width_mm != self.height_mm {
            return Err(format!("hoop {id}: a circle must have width_mm equal to height_mm"));
        }
        match (self.shape, self.corner_radius_mm) {
            (HoopShape::RoundedRectangle, None) => {
                return Err(format!("hoop {id}: rounded_rectangle needs corner_radius_mm"));
            }
            (HoopShape::RoundedRectangle, Some(r)) => {
                let max = self.width_mm.min(self.height_mm) / 2.0;
                if !positive(r) || r > max {
                    return Err(format!(
                        "hoop {id}: corner_radius_mm must be more than 0 and not more than {max}"
                    ));
                }
            }
            (_, Some(_)) => {
                return Err(format!("hoop {id}: corner_radius_mm is only for rounded_rectangle"));
            }
            _ => {}
        }
        if let Some(l) = self.max_stitch_length_mm
            && !positive(l)
        {
            return Err(format!("hoop {id}: max_stitch_length_mm must be more than 0"));
        }
        if self.positions.len() == 1 {
            return Err(format!("hoop {id}: a multi-position hoop needs two positions or more"));
        }
        for (i, p) in self.positions.iter().enumerate() {
            let n = i + 1;
            if !positive(p.width_mm) || !positive(p.height_mm) || !p.x_mm.is_finite() || !p.y_mm.is_finite() {
                return Err(format!(
                    "hoop {id}: position {n} needs a width and a height more than 0"
                ));
            }
            let tol = 1e-6;
            if p.x_mm.abs() + p.width_mm / 2.0 > self.width_mm / 2.0 + tol
                || p.y_mm.abs() + p.height_mm / 2.0 > self.height_mm / 2.0 + tol
            {
                return Err(format!("hoop {id}: position {n} goes outside the field of the hoop"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoopCatalog {
    #[serde(rename = "hoop", default)]
    pub hoops: Vec<HoopPreset>,
}

impl HoopCatalog {
    /// Parses and validates a hoop catalog in TOML.
    pub fn from_toml_str(s: &str) -> Result<Self, ModelError> {
        let cat: HoopCatalog = toml::from_str(s).map_err(|source| ModelError::Toml {
            what: "hoop catalog",
            source,
        })?;
        cat.validate()?;
        Ok(cat)
    }

    pub fn load(path: &std::path::Path) -> Result<Self, ModelError> {
        let s = std::fs::read_to_string(path).map_err(|source| ModelError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_toml_str(&s)
    }

    /// The presets from `data/hoops.toml` at build time.
    pub fn builtin() -> Self {
        Self::from_toml_str(BUILTIN_HOOPS_TOML).expect("built-in data/hoops.toml is valid (tested)")
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let invalid = |message| ModelError::Invalid {
            what: "hoop catalog",
            message,
        };
        let mut ids = BTreeSet::new();
        for h in &self.hoops {
            h.validate().map_err(invalid)?;
            if !ids.insert(h.id.as_str()) {
                return Err(invalid(format!("hoop id {:?} is used more than once", h.id)));
            }
        }
        Ok(())
    }

    /// The catalog as TOML text, in the form that `from_toml_str` reads.
    pub fn to_toml_string(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    pub fn get(&self, id: &str) -> Option<&HoopPreset> {
        self.hoops.iter().find(|h| h.id == id)
    }

    /// Presets that a centered box of `width` × `height` fits, smallest field first.
    pub fn fitting(&self, width: f64, height: f64, margin: f64) -> Vec<&HoopPreset> {
        let mut v: Vec<&HoopPreset> = self.hoops.iter().filter(|h| h.fits(width, height, margin)).collect();
        v.sort_by(|a, b| (a.width_mm * a.height_mm).total_cmp(&(b.width_mm * b.height_mm)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_catalog_is_valid() {
        let cat = HoopCatalog::builtin();
        let topaz = cat.get("topaz20_crown").unwrap();
        // Upright, as the Topaz 20 shows it: 200 mm left to right.
        assert_eq!((topaz.width_mm, topaz.height_mm), (200.0, 260.0));
    }

    fn preset(shape: HoopShape, w: f64, h: f64, r: Option<f64>) -> HoopPreset {
        HoopPreset {
            id: "t".into(),
            name: "t".into(),
            brand: None,
            width_mm: w,
            height_mm: h,
            shape,
            corner_radius_mm: r,
            max_stitch_length_mm: None,
            positions: Vec::new(),
            cap: false,
        }
    }

    #[test]
    fn positions_read_and_check() {
        let ok = "[[hoop]]\nid='m'\nname='Multi'\nwidth_mm=300\nheight_mm=150\nshape='rectangle'\n\
                  [[hoop.position]]\nx_mm=-75\ny_mm=0\nwidth_mm=150\nheight_mm=150\n\
                  [[hoop.position]]\nx_mm=75\ny_mm=0\nwidth_mm=150\nheight_mm=150\n";
        let cat = HoopCatalog::from_toml_str(ok).unwrap();
        let h = cat.get("m").unwrap();
        assert_eq!(h.positions.len(), 2);
        let p = h.position_preset(1).unwrap();
        assert_eq!(
            (p.width_mm, p.height_mm, p.name.as_str()),
            (150.0, 150.0, "Multi, position 2")
        );
        assert!(h.position_preset(2).is_none());
        // It writes back the same.
        assert_eq!(&HoopCatalog::from_toml_str(&cat.to_toml_string()).unwrap(), &cat);
        // One position only, or a position outside the hoop: refused.
        let one = "[[hoop]]\nid='m'\nname='M'\nwidth_mm=300\nheight_mm=150\nshape='rectangle'\n\
                   [[hoop.position]]\nx_mm=0\ny_mm=0\nwidth_mm=150\nheight_mm=150\n";
        assert!(HoopCatalog::from_toml_str(one).is_err());
        let out = ok.replace("x_mm=75", "x_mm=100");
        assert!(HoopCatalog::from_toml_str(&out).is_err());
    }

    #[test]
    fn rectangle_contains_with_margin() {
        let h = preset(HoopShape::Rectangle, 100.0, 100.0, None);
        assert!(h.contains(Point::new(50.0, -50.0), 0.0));
        assert!(!h.contains(Point::new(50.0, 0.0), 5.0));
        assert!(h.contains(Point::new(45.0, 0.0), 5.0));
    }

    #[test]
    fn rounded_corner() {
        let h = preset(HoopShape::RoundedRectangle, 100.0, 100.0, Some(10.0));
        assert!(!h.contains(Point::new(50.0, 50.0), 0.0));
        assert!(h.contains(Point::new(50.0, 40.0), 0.0));
        // On the arc at 45°.
        let d = 10.0 / 2.0_f64.sqrt();
        assert!(h.contains(Point::new(40.0 + d - 1e-9, 40.0 + d - 1e-9), 0.0));
    }

    #[test]
    fn oval_and_fit() {
        let h = preset(HoopShape::Oval, 200.0, 100.0, None);
        assert!(h.contains(Point::new(100.0, 0.0), 0.0));
        assert!(!h.contains(Point::new(80.0, 40.0), 0.0));
        // A square of 2*35 fits: (35/100)^2 + (35/50)^2 = 0.61.
        assert!(h.fits(70.0, 70.0, 0.0));
        assert!(!h.fits(200.0, 100.0, 0.0));
    }

    #[test]
    fn validation_errors() {
        let bad = [
            "[[hoop]]\nid='A'\nname='x'\nwidth_mm=1\nheight_mm=1\nshape='rectangle'",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=0\nheight_mm=1\nshape='rectangle'",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=2\nheight_mm=1\nshape='circle'",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=2\nheight_mm=2\nshape='rounded_rectangle'",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=2\nheight_mm=2\nshape='rectangle'\ncorner_radius_mm=1",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=2\nheight_mm=2\nshape='rectangle'\ncolor='red'",
            "[[hoop]]\nid='a'\nname='x'\nwidth_mm=2\nheight_mm=2\nshape='rectangle'\n[[hoop]]\nid='a'\nname='y'\nwidth_mm=2\nheight_mm=2\nshape='rectangle'",
        ];
        for s in bad {
            assert!(HoopCatalog::from_toml_str(s).is_err(), "accepted: {s}");
        }
    }

    #[test]
    fn toml_text_reads_back() {
        let cat = HoopCatalog {
            hoops: vec![preset(HoopShape::RoundedRectangle, 150.0, 100.0, Some(12.0))],
        };
        let back = HoopCatalog::from_toml_str(&cat.to_toml_string()).unwrap();
        assert_eq!(back, cat);
    }

    #[test]
    fn fitting_sorts_smallest_first() {
        let cat = HoopCatalog::builtin();
        let fits = cat.fitting(90.0, 90.0, 0.0);
        assert!(fits.len() > 2);
        assert!(
            fits.windows(2)
                .all(|w| w[0].width_mm * w[0].height_mm <= w[1].width_mm * w[1].height_mm)
        );
        assert!(cat.fitting(1000.0, 1000.0, 0.0).is_empty());
    }
}

//! Fabric presets from `data/fabrics.toml`.
//!
//! A fabric changes the satin and fill settings relative to the base
//! settings (a stable woven cotton). To change a design from one fabric to
//! another, the settings are changed by the difference between the two
//! fabrics. Thus the differences that the user made between objects stay,
//! and a change back to the first fabric gives the first settings again.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Design, FillParams, ModelError, ObjectId, ObjectKind, SatinParams, Underlay};

/// The built-in presets, from `data/fabrics.toml` at build time.
pub const BUILTIN_FABRICS_TOML: &str = include_str!("../../../data/fabrics.toml");

/// How much automatic underlay a fabric needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnderlayWeight {
    Light,
    #[default]
    Normal,
    Heavy,
    /// Free-standing lace on water-soluble stabilizer (P11 Milestone 5).
    Lace,
}

impl UnderlayWeight {
    /// The automatic underlay mode of this weight.
    pub fn underlay(self) -> Underlay {
        match self {
            UnderlayWeight::Light => Underlay::Light,
            UnderlayWeight::Normal => Underlay::Auto,
            UnderlayWeight::Heavy => Underlay::Heavy,
            UnderlayWeight::Lace => Underlay::Lace,
        }
    }
}

/// A fabric preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fabric {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The stitch spacing of satin and fill is multiplied by this. Less than 1 is denser.
    #[serde(default = "one")]
    pub spacing_factor: f64,
    /// The fill stitch length is multiplied by this.
    #[serde(default = "one")]
    pub stitch_length_factor: f64,
    /// Added to the pull compensation of satin and fill, in mm.
    #[serde(default)]
    pub pull_mm: f64,
    #[serde(default)]
    pub underlay: UnderlayWeight,
    /// The backing (stabilizer) to use.
    #[serde(default)]
    pub backing: String,
    /// The thickness of the fabric, in mm, for the thread use (each needle
    /// point takes thread through the fabric; `crate::thread_use`).
    #[serde(default = "default_thickness")]
    pub thickness_mm: f64,
    /// The topping to use, if the fabric needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topping: Option<String>,
}

fn default_thickness() -> f64 {
    crate::thread_use::DEFAULT_THICKNESS_MM
}

fn one() -> f64 {
    1.0
}

/// The limits of the stitch settings after a fabric change, in mm.
const MIN_SPACING_MM: f64 = 0.1;
const MAX_SPACING_MM: f64 = 10.0;
const MIN_LENGTH_MM: f64 = 0.5;
const MAX_LENGTH_MM: f64 = 12.0;

impl Fabric {
    /// The base fabric: it changes nothing. A design with no fabric, or with
    /// a fabric that is not in the catalog, has these values.
    pub fn base() -> Fabric {
        Fabric {
            id: String::new(),
            name: "Not set".into(),
            description: String::new(),
            spacing_factor: 1.0,
            stitch_length_factor: 1.0,
            pull_mm: 0.0,
            underlay: UnderlayWeight::Normal,
            backing: String::new(),
            thickness_mm: default_thickness(),
            topping: None,
        }
    }

    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(format!("fabric id {id:?} must use only a-z, 0-9, '_' and '-'"));
        }
        if self.name.trim().is_empty() {
            return Err(format!("fabric {id}: name is empty"));
        }
        let factor = |v: f64| v.is_finite() && (0.5..=2.0).contains(&v);
        if !factor(self.spacing_factor) || !factor(self.stitch_length_factor) {
            return Err(format!(
                "fabric {id}: spacing_factor and stitch_length_factor must be from 0.5 to 2"
            ));
        }
        if !(self.pull_mm.is_finite() && (-0.5..=1.0).contains(&self.pull_mm)) {
            return Err(format!("fabric {id}: pull_mm must be from -0.5 to 1"));
        }
        if !(self.thickness_mm.is_finite() && (0.05..=10.0).contains(&self.thickness_mm)) {
            return Err(format!("fabric {id}: thickness_mm must be from 0.05 to 10"));
        }
        Ok(())
    }
}

/// The change from one fabric to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FabricChange {
    spacing: f64,
    length: f64,
    pull_mm: f64,
    underlay: UnderlayWeight,
}

impl FabricChange {
    pub fn new(from: &Fabric, to: &Fabric) -> FabricChange {
        FabricChange {
            spacing: to.spacing_factor / from.spacing_factor,
            length: to.stitch_length_factor / from.stitch_length_factor,
            pull_mm: to.pull_mm - from.pull_mm,
            underlay: to.underlay,
        }
    }

    /// Automatic underlay gets the new weight. No underlay and custom
    /// underlay layers are the choice of the user and stay.
    fn underlay(&self, u: &mut Underlay) {
        if matches!(u, Underlay::Auto | Underlay::Light | Underlay::Heavy | Underlay::Lace) {
            *u = self.underlay.underlay();
        }
    }

    fn pull(&self, v: f64) -> f64 {
        (v + self.pull_mm).max(0.0)
    }

    pub fn satin(&self, p: &mut SatinParams) {
        p.spacing_mm = (p.spacing_mm * self.spacing).clamp(MIN_SPACING_MM, MAX_SPACING_MM);
        p.pull_compensation_mm = self.pull(p.pull_compensation_mm);
        self.underlay(&mut p.underlay);
    }

    pub fn fill(&self, p: &mut FillParams) {
        p.spacing_mm = (p.spacing_mm * self.spacing).clamp(MIN_SPACING_MM, MAX_SPACING_MM);
        p.stitch_length_mm = (p.stitch_length_mm * self.length).clamp(MIN_LENGTH_MM, MAX_LENGTH_MM);
        p.pull_compensation_mm = self.pull(p.pull_compensation_mm);
        self.underlay(&mut p.underlay);
    }

    /// Changes the settings of a satin or fill object. Returns `false` for
    /// other objects.
    pub fn object(&self, kind: &mut ObjectKind) -> bool {
        match kind {
            ObjectKind::Satin { params, .. } => self.satin(params),
            ObjectKind::Fill { params, .. } => self.fill(params),
            ObjectKind::Applique { params, .. } => self.satin(&mut params.cover_satin),
            ObjectKind::Lettering(l) => {
                self.satin(&mut l.satin);
                self.fill(&mut l.fill);
                l.apply_params();
            }
            _ => return false,
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FabricCatalog {
    #[serde(rename = "fabric", default)]
    pub fabrics: Vec<Fabric>,
}

impl Fabric {
    /// Free-standing lace: the stabilizer washes away, so each part must
    /// hold to the others (`emb_stitch::lace`).
    pub fn is_lace(&self) -> bool {
        self.underlay == UnderlayWeight::Lace
    }
}

impl FabricCatalog {
    /// Parses and validates a fabric catalog in TOML.
    pub fn from_toml_str(s: &str) -> Result<Self, ModelError> {
        let cat: FabricCatalog = toml::from_str(s).map_err(|source| ModelError::Toml {
            what: "fabric catalog",
            source,
        })?;
        cat.validate()?;
        Ok(cat)
    }

    /// The presets from `data/fabrics.toml` at build time.
    pub fn builtin() -> Self {
        Self::from_toml_str(BUILTIN_FABRICS_TOML).expect("built-in data/fabrics.toml is valid (tested)")
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let invalid = |message| ModelError::Invalid {
            what: "fabric catalog",
            message,
        };
        let mut ids = BTreeSet::new();
        for f in &self.fabrics {
            f.validate().map_err(invalid)?;
            if !ids.insert(f.id.as_str()) {
                return Err(invalid(format!("fabric id {:?} is used more than once", f.id)));
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Fabric> {
        self.fabrics.iter().find(|f| f.id == id)
    }

    /// The fabric of a design: the base fabric if none is set or the id is unknown.
    pub fn of(&self, design: &Design) -> Fabric {
        design
            .fabric
            .as_deref()
            .and_then(|id| self.get(id))
            .cloned()
            .unwrap_or_else(Fabric::base)
    }
}

impl Design {
    /// Sets the fabric of the design (`None`: no fabric). The current
    /// properties (for new objects) change with it. If `objects` is
    /// `true`, the satin and fill objects change too. Returns the changed
    /// objects.
    pub fn set_fabric(&mut self, catalog: &FabricCatalog, id: Option<&str>, objects: bool) -> Vec<ObjectId> {
        let from = catalog.of(self);
        let to = id.and_then(|id| catalog.get(id)).cloned().unwrap_or_else(Fabric::base);
        let change = FabricChange::new(&from, &to);
        self.fabric = id.map(str::to_string);
        change.satin(&mut self.current.satin);
        change.fill(&mut self.current.fill);
        change.satin(&mut self.current.applique.cover_satin);
        if !objects {
            return Vec::new();
        }
        let ids: Vec<ObjectId> = self
            .objects()
            .iter()
            .filter(|o| {
                matches!(
                    o.kind,
                    ObjectKind::Satin { .. }
                        | ObjectKind::Fill { .. }
                        | ObjectKind::Applique { .. }
                        | ObjectKind::Lettering(_)
                )
            })
            .map(|o| o.id)
            .collect();
        for id in &ids {
            if let Some(o) = self.object_mut(*id) {
                change.object(&mut o.kind);
            }
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RunningParams, SatinGeometry};
    use emb_geom::{Path, Point};

    #[test]
    fn builtin_fabrics_are_valid() {
        let c = FabricCatalog::builtin();
        assert!(c.fabrics.len() >= 8);
        let cotton = c.get("cotton").unwrap();
        assert_eq!(
            FabricChange::new(&Fabric::base(), cotton),
            FabricChange::new(&Fabric::base(), &Fabric::base())
        );
        assert!(c.get("towel").unwrap().topping.is_some());
    }

    #[test]
    fn bad_fabrics_are_rejected() {
        let bad = [
            "[[fabric]]\nid = \"A\"\nname = \"x\"",
            "[[fabric]]\nid = \"a\"\nname = \" \"",
            "[[fabric]]\nid = \"a\"\nname = \"x\"\nspacing_factor = 3.0",
            "[[fabric]]\nid = \"a\"\nname = \"x\"\npull_mm = 2.0",
            "[[fabric]]\nid = \"a\"\nname = \"x\"\n[[fabric]]\nid = \"a\"\nname = \"y\"",
            "[[fabric]]\nid = \"a\"\nname = \"x\"\ncolor = 1",
        ];
        for s in bad {
            assert!(FabricCatalog::from_toml_str(s).is_err(), "{s}");
        }
    }

    fn design() -> Design {
        let mut d = Design::new("t");
        let t = d.add_thread("red", crate::Rgb(200, 0, 0));
        let line = Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(30.0, 0.0));
        d.add_object(
            t,
            ObjectKind::Satin {
                geometry: SatinGeometry::from_centerline(&line, 3.0),
                params: SatinParams {
                    spacing_mm: 0.35,
                    ..SatinParams::default()
                },
            },
        );
        d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(emb_geom::BBox::new(Point::new(0.0, 5.0), Point::new(20.0, 25.0))),
                holes: Vec::new(),
                params: FillParams {
                    underlay: Underlay::None,
                    ..FillParams::default()
                },
            },
        );
        d.add_object(
            t,
            ObjectKind::Running {
                path: line,
                params: RunningParams::default(),
            },
        );
        d
    }

    fn params(d: &Design) -> (SatinParams, FillParams) {
        let o = d.objects();
        let ObjectKind::Satin { params: s, .. } = &o[0].kind else {
            panic!()
        };
        let ObjectKind::Fill { params: f, .. } = &o[1].kind else {
            panic!()
        };
        (s.clone(), f.clone())
    }

    #[test]
    fn cotton_to_towel_and_back() {
        let c = FabricCatalog::builtin();
        let mut d = design();
        let before = params(&d);
        let run_before = d.objects()[2].kind.clone();
        assert!(d.set_fabric(&c, Some("cotton"), true).len() == 2);
        assert_eq!(params(&d), before, "cotton is the base");
        d.set_fabric(&c, Some("towel"), true);
        let (s, f) = params(&d);
        assert!(
            (s.spacing_mm - 0.35 * 0.9).abs() < 1e-9,
            "the object keeps its own density, made denser"
        );
        assert!((s.pull_compensation_mm - 0.4).abs() < 1e-9);
        assert_eq!(s.underlay, Underlay::Heavy);
        assert_eq!(f.underlay, Underlay::None, "no underlay is the choice of the user");
        assert!((f.pull_compensation_mm - 0.2).abs() < 1e-9);
        assert_eq!(d.objects()[2].kind, run_before, "running stitches do not change");
        assert_eq!(d.fabric.as_deref(), Some("towel"));
        assert!(
            (d.current.fill.spacing_mm - 0.36).abs() < 1e-9,
            "new objects get the towel settings"
        );
        d.set_fabric(&c, Some("cotton"), true);
        let (s, f) = params(&d);
        assert!((s.spacing_mm - before.0.spacing_mm).abs() < 1e-9);
        assert!((s.pull_compensation_mm - before.0.pull_compensation_mm).abs() < 1e-9);
        assert_eq!(s.underlay, Underlay::Auto);
        assert!((f.spacing_mm - before.1.spacing_mm).abs() < 1e-9);
    }

    #[test]
    fn new_objects_only() {
        let c = FabricCatalog::builtin();
        let mut d = design();
        let before = params(&d);
        assert!(d.set_fabric(&c, Some("leather"), false).is_empty());
        assert_eq!(params(&d), before);
        assert_eq!(d.current.satin.underlay, Underlay::Light);
        assert!(d.current.fill.stitch_length_mm > FillParams::default().stitch_length_mm);
        // An unknown fabric id is the base fabric.
        d.fabric = Some("unknown".into());
        assert_eq!(c.of(&d), Fabric::base());
    }
}

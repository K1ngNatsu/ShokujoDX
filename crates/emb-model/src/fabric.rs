//! Fabric presets from `data/fabrics.toml`.
//!
//! A fabric changes the satin and fill settings relative to the base
//! settings (a stable woven cotton). To change a design from one fabric to
//! another, the settings are changed by the difference between the two
//! fabrics. Thus the differences that the user made between objects stay,
//! and a change back to the first fabric gives the first settings again.

use emb_geom::text;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Design, FillParams, ModelError, ObjectId, ObjectKind, SatinParams, Underlay};

/// The built-in presets, from `data/fabrics.toml` at build time.
pub static BUILTIN_FABRICS_TOML: &str = include_str!("../../../data/fabrics.toml");

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

/// The surface of a fabric in the 3D view (P18 view plan, Milestone 6, ADR
/// 0127): what its threads, loops or grain look like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weave {
    /// Plain weave: each thread goes over one thread and under the next one
    /// (cotton, silk).
    #[default]
    Plain,
    /// Twill: each thread of the warp goes over three threads and under one,
    /// so the weave has diagonal lines (denim, caps).
    Twill,
    /// Knit: columns of V-shaped loops (T-shirts).
    Knit,
    /// Piqué: a knit with small raised cells (polo shirts).
    Pique,
    /// Terry: loops of thread that stand up (towels).
    Terry,
    /// Fleece: soft hair that stands up (sweatshirts).
    Fleece,
    /// Felt: pressed fibers, almost flat.
    Felt,
    /// Leather and vinyl: a fine grain.
    Leather,
    /// Aida: an even weave of blocks of threads with a hole at each corner
    /// of a block, for cross stitch.
    Aida,
}

impl Weave {
    /// All weaves.
    pub const ALL: [Weave; 9] = [
        Weave::Plain,
        Weave::Twill,
        Weave::Knit,
        Weave::Pique,
        Weave::Terry,
        Weave::Fleece,
        Weave::Felt,
        Weave::Leather,
        Weave::Aida,
    ];

    /// The usual size of the weave, mm: the distance from one thread to
    /// the next one, or the size of a loop, a cell or a grain.
    pub fn default_mm(self) -> f64 {
        match self {
            Weave::Plain => 0.25,
            Weave::Twill => 0.45,
            Weave::Knit => 0.7,
            Weave::Pique => 1.0,
            Weave::Terry => 1.0,
            Weave::Fleece => 0.5,
            Weave::Felt => 0.4,
            Weave::Leather => 0.7,
            // 14 blocks to the inch.
            Weave::Aida => 1.814,
        }
    }

    /// The id of the weave in `data/fabrics.toml` (data, not shown).
    pub fn id(self) -> &'static str {
        match self {
            Weave::Plain => "plain",
            Weave::Twill => "twill",
            Weave::Knit => "knit",
            Weave::Pique => "pique",
            Weave::Terry => "terry",
            Weave::Fleece => "fleece",
            Weave::Felt => "felt",
            Weave::Leather => "leather",
            Weave::Aida => "aida",
        }
    }

    /// The name that the program shows (translated where it shows).
    pub fn label(self) -> &'static str {
        match self {
            Weave::Plain => "Plain weave",
            Weave::Twill => "Twill",
            Weave::Knit => "Knit",
            Weave::Pique => "Piqué",
            Weave::Terry => "Terry",
            Weave::Fleece => "Fleece",
            Weave::Felt => "Felt",
            Weave::Leather => "Leather and vinyl",
            Weave::Aida => "Aida (even weave)",
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
    /// The surface of the fabric in the 3D view.
    #[serde(default)]
    pub weave: Weave,
    /// The size of the weave, mm; `None`: the usual size of the weave
    /// ([`Weave::default_mm`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weave_mm: Option<f64>,
    /// The loft of the batting under the fabric (a quilt), mm: in the 3D
    /// view the fabric is this much higher between the lines of stitches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loft_mm: Option<f64>,
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
            weave: Weave::Plain,
            weave_mm: None,
            loft_mm: None,
        }
    }

    /// The size of the weave, mm.
    pub fn weave_size(&self) -> f64 {
        self.weave_mm.unwrap_or_else(|| self.weave.default_mm())
    }

    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(text::fill(
                "fabric id {} must use only a-z, 0-9, '_' and '-'",
                &[&format!("{:?}", id)],
            ));
        }
        if self.name.trim().is_empty() {
            return Err(text::fill("fabric {}: name is empty", &[&id]));
        }
        let factor = |v: f64| v.is_finite() && (0.5..=2.0).contains(&v);
        if !factor(self.spacing_factor) || !factor(self.stitch_length_factor) {
            return Err(text::fill(
                "fabric {}: spacing_factor and stitch_length_factor must be from 0.5 to 2",
                &[&id],
            ));
        }
        if !(self.pull_mm.is_finite() && (-0.5..=1.0).contains(&self.pull_mm)) {
            return Err(text::fill("fabric {}: pull_mm must be from -0.5 to 1", &[&id]));
        }
        if !(self.thickness_mm.is_finite() && (0.05..=10.0).contains(&self.thickness_mm)) {
            return Err(text::fill("fabric {}: thickness_mm must be from 0.05 to 10", &[&id]));
        }
        if self
            .weave_mm
            .is_some_and(|v| !(v.is_finite() && (0.05..=5.0).contains(&v)))
        {
            return Err(text::fill("fabric {}: weave_mm must be from 0.05 to 5", &[&id]));
        }
        if self
            .loft_mm
            .is_some_and(|v| !(v.is_finite() && (0.0..=10.0).contains(&v)))
        {
            return Err(text::fill("fabric {}: loft_mm must be from 0 to 10", &[&id]));
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

    /// The topping of the sew-out of `design` on this fabric: the topping
    /// of the fabric, if the design does not say that it has none
    /// (`Design::no_topping`). The sheets and the 3D view use it.
    pub fn topping_for(&self, design: &Design) -> Option<&str> {
        if design.no_topping {
            return None;
        }
        self.topping.as_deref()
    }
}

impl FabricCatalog {
    /// Parses and validates a fabric catalog in TOML.
    pub fn from_toml_str(s: &str) -> Result<Self, ModelError> {
        let cat: FabricCatalog = toml::from_str(s).map_err(|source| ModelError::Toml {
            what: text::tr("fabric catalog"),
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
            what: text::tr("fabric catalog"),
            message,
        };
        let mut ids = BTreeSet::new();
        for f in &self.fabrics {
            f.validate().map_err(invalid)?;
            if !ids.insert(f.id.as_str()) {
                return Err(invalid(text::fill(
                    "fabric id {} is used more than once",
                    &[&format!("{:?}", f.id)],
                )));
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
        self.set_fabric_inner(catalog, id, objects)
    }

    /// The change from the base settings to the fabric of the design. New
    /// lettering and monograms start from the base settings, not from
    /// `current`: with this change they suit the fabric as the objects that
    /// the tools draw (ADR 0163).
    pub fn fabric_change(&self, catalog: &FabricCatalog) -> FabricChange {
        FabricChange::new(&Fabric::base(), &catalog.of(self))
    }

    fn set_fabric_inner(&mut self, catalog: &FabricCatalog, id: Option<&str>, objects: bool) -> Vec<ObjectId> {
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
    fn each_fabric_has_a_weave() {
        let c = FabricCatalog::builtin();
        assert_eq!(c.get("towel").unwrap().weave, Weave::Terry);
        assert_eq!(c.get("denim").unwrap().weave, Weave::Twill);
        assert_eq!(c.get("leather").unwrap().weave, Weave::Leather);
        assert!((c.get("silk").unwrap().weave_size() - 0.15).abs() < 1e-12);
        assert_eq!(Fabric::base().weave_size(), Weave::Plain.default_mm());
        // The ids are the names in the file.
        #[derive(Deserialize)]
        struct W {
            weave: Weave,
        }
        for w in Weave::ALL {
            let r: W = toml::from_str(&format!("weave = \"{}\"", w.id())).unwrap();
            assert_eq!(r.weave, w);
        }
        // A size out of its range is refused.
        let bad = BUILTIN_FABRICS_TOML.replace("weave_mm = 0.25", "weave_mm = 9.0");
        assert!(FabricCatalog::from_toml_str(&bad).is_err());
    }

    #[test]
    fn a_design_can_have_no_topping() {
        let c = FabricCatalog::builtin();
        let mut d = Design::new("towel");
        d.set_fabric(&c, Some("towel"), false);
        assert!(c.of(&d).topping_for(&d).is_some());
        d.no_topping = true;
        assert_eq!(c.of(&d).topping_for(&d), None);
        // The project file keeps it, and the second color of the fabric.
        d.fabric_mix = Some(crate::Rgb(250, 250, 240));
        let back = crate::project::from_str(&crate::project::to_string(&d)).unwrap();
        assert!(back.no_topping);
        assert_eq!(back.fabric_mix, d.fabric_mix);
        // A colorway keeps the second color.
        d.ensure_colorways();
        d.add_colorway("Two");
        d.fabric_mix = None;
        assert!(d.switch_colorway(0));
        assert_eq!(d.fabric_mix, Some(crate::Rgb(250, 250, 240)));
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

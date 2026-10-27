//! Machine profiles from `data/machines.toml`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{HoopCatalog, ModelError};

/// The built-in profiles, from `data/machines.toml` at build time.
pub const BUILTIN_MACHINES_TOML: &str = include_str!("../../../data/machines.toml");

/// File format ids that a profile can name. Lowercase file extensions.
pub const KNOWN_FORMATS: &[&str] = &[
    "vp3", "dst", "pes", "pec", "exp", "jef", "hus", "vip", "tbf", "tap", "dsb", "dsz", "10o", "100", "u01", "dat",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineProfile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    /// Format that the export dialog selects first.
    pub default_format: String,
    /// Formats that the machine reads.
    pub formats: Vec<String>,
    /// Hoop preset ids that fit the machine.
    pub hoops: Vec<String>,
    pub max_stitch_length_mm: f64,
    pub min_stitch_length_mm: f64,
    /// Used for the estimated sew time.
    pub stitches_per_minute: u32,
    /// Gaps between objects of one color up to this length get a travel
    /// stitch; longer gaps get a trim. If not set, the engine default (2 mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_gap_mm: Option<f64>,
    /// DST and TAP: the number of jump records that make a trim (the
    /// machine cuts the thread after so many jumps). Default 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_jumps: Option<u32>,
    /// DST and TAP: the move of each jump of a trim, mm. Default 0.4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_jump_mm: Option<f64>,
    /// The number of needles of a multi-needle machine (ADR 0063). `None`:
    /// one needle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needles: Option<u8>,
    /// The number of heads of a machine that sews more pieces at a time
    /// (P13 Milestone 1). `None`: one head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heads: Option<u8>,
}

/// The most heads of a machine.
pub const MAX_HEADS: u32 = 50;

/// The values that machine files can have for [`MachineProfile::trim_jumps`].
pub const TRIM_JUMPS_RANGE: std::ops::RangeInclusive<u32> = 1..=9;
/// The values for [`MachineProfile::trim_jump_mm`].
pub const TRIM_JUMP_MM_RANGE: std::ops::RangeInclusive<f64> = 0.2..=6.0;

impl MachineProfile {
    fn validate(&self) -> Result<(), String> {
        let id = &self.id;
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(format!("machine id {id:?} must use only a-z, 0-9, '_' and '-'"));
        }
        if self.formats.is_empty() {
            return Err(format!("machine {id}: formats is empty"));
        }
        for f in &self.formats {
            if !KNOWN_FORMATS.contains(&f.as_str()) {
                return Err(format!("machine {id}: unknown format {f:?}"));
            }
        }
        if self.trim_jumps.is_some_and(|n| !TRIM_JUMPS_RANGE.contains(&n)) {
            return Err(format!("machine {id}: trim_jumps must be 1 to 9"));
        }
        if self.trim_jump_mm.is_some_and(|v| !TRIM_JUMP_MM_RANGE.contains(&v)) {
            return Err(format!("machine {id}: trim_jump_mm must be 0.2 to 6.0"));
        }
        if self
            .needles
            .is_some_and(|n| !crate::needles::NEEDLES_RANGE.contains(&n))
        {
            return Err(format!(
                "machine {id}: needles must be 1 to {}",
                crate::needles::MAX_NEEDLES
            ));
        }
        if self.heads.is_some_and(|n| n == 0 || u32::from(n) > MAX_HEADS) {
            return Err(format!("machine {id}: heads must be 1 to {MAX_HEADS}"));
        }
        if !self.formats.contains(&self.default_format) {
            return Err(format!("machine {id}: default_format must be in formats"));
        }
        if self.hoops.is_empty() {
            return Err(format!("machine {id}: hoops is empty"));
        }
        let (min, max) = (self.min_stitch_length_mm, self.max_stitch_length_mm);
        if !(min.is_finite() && max.is_finite() && min > 0.0 && min < max) {
            return Err(format!(
                "machine {id}: stitch lengths must be more than 0, with min less than max"
            ));
        }
        if self.stitches_per_minute == 0 {
            return Err(format!("machine {id}: stitches_per_minute must be more than 0"));
        }
        if let Some(g) = self.trim_gap_mm
            && !(g.is_finite() && g >= 0.0)
        {
            return Err(format!("machine {id}: trim_gap_mm must be 0 or more"));
        }
        Ok(())
    }
}

/// The times for the sew-time estimate of a machine. The user can change
/// them for each machine (Design > Sew time).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SewTimes {
    pub stitches_per_minute: u32,
    /// Seconds for a color change: the operator changes the thread on a
    /// single-needle machine; a multi-needle machine changes by itself.
    pub color_change_s: f64,
    pub trim_s: f64,
    /// Seconds for a stop (for example to put on appliqué fabric).
    pub stop_s: f64,
    /// The number of heads: the pieces that the machine sews at a time
    /// (P13 Milestone 1).
    pub heads: u32,
    /// Seconds to hoop one piece and to take it out of the hoop after it
    /// sews.
    pub hoop_s: f64,
    /// Seconds to take the hoops off the machine and put the next hoops on.
    pub frame_change_s: f64,
    /// True if the operator hoops the next pieces while the machine sews
    /// (a second set of hoops).
    pub hoop_while_sewing: bool,
}

impl Default for SewTimes {
    fn default() -> Self {
        SewTimes {
            stitches_per_minute: 600,
            color_change_s: 30.0,
            trim_s: 5.0,
            stop_s: 30.0,
            heads: 1,
            hoop_s: 60.0,
            frame_change_s: 20.0,
            hoop_while_sewing: true,
        }
    }
}

impl SewTimes {
    /// The sew time of the stitches `s` in minutes, with `extra_stops` more
    /// stops (for example the thread changes of ADR 0063).
    pub fn minutes_of(&self, s: &crate::StitchDesign, extra_stops: usize) -> f64 {
        let stops = s.records.iter().filter(|r| r.kind == crate::StitchKind::Stop).count();
        self.minutes(
            s.stitch_count(),
            s.color_count().saturating_sub(1),
            s.trim_count(),
            stops + extra_stops,
        )
    }

    /// The sew time in minutes.
    pub fn minutes(&self, stitches: usize, color_changes: usize, trims: usize, stops: usize) -> f64 {
        stitches as f64 / f64::from(self.stitches_per_minute.max(1))
            + (color_changes as f64 * self.color_change_s + trims as f64 * self.trim_s + stops as f64 * self.stop_s)
                / 60.0
    }
}

impl MachineProfile {
    /// The default times of the machine: its speed, and general times for
    /// color changes, trims and stops.
    pub fn sew_times(&self) -> SewTimes {
        SewTimes {
            stitches_per_minute: self.stitches_per_minute,
            heads: self.heads.map_or(1, u32::from),
            ..SewTimes::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineCatalog {
    #[serde(rename = "machine", default)]
    pub machines: Vec<MachineProfile>,
}

impl MachineCatalog {
    /// Parses and validates a machine catalog in TOML. Use
    /// [`MachineCatalog::validate_hoops`] to check the hoop ids.
    pub fn from_toml_str(s: &str) -> Result<Self, ModelError> {
        let cat: MachineCatalog = toml::from_str(s).map_err(|source| ModelError::Toml {
            what: "machine catalog",
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

    /// The profiles from `data/machines.toml` at build time.
    pub fn builtin() -> Self {
        Self::from_toml_str(BUILTIN_MACHINES_TOML).expect("built-in data/machines.toml is valid (tested)")
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        let invalid = |message| ModelError::Invalid {
            what: "machine catalog",
            message,
        };
        let mut ids = BTreeSet::new();
        for m in &self.machines {
            m.validate().map_err(invalid)?;
            if !ids.insert(m.id.as_str()) {
                return Err(invalid(format!("machine id {:?} is used more than once", m.id)));
            }
        }
        Ok(())
    }

    /// Checks that each hoop id in each profile is in the hoop catalog.
    pub fn validate_hoops(&self, hoops: &HoopCatalog) -> Result<(), ModelError> {
        for m in &self.machines {
            for h in &m.hoops {
                if hoops.get(h).is_none() {
                    return Err(ModelError::Invalid {
                        what: "machine catalog",
                        message: format!("machine {}: hoop {h:?} is not in the hoop catalog", m.id),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&MachineProfile> {
        self.machines.iter().find(|m| m.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sew_minutes() {
        let t = SewTimes {
            stitches_per_minute: 600,
            color_change_s: 30.0,
            trim_s: 6.0,
            stop_s: 60.0,
            ..SewTimes::default()
        };
        // 1 min of stitches, 2 color changes (1 min), 10 trims (1 min), 1 stop (1 min).
        assert!((t.minutes(600, 2, 10, 1) - 4.0).abs() < 1e-9);
        let topaz = MachineCatalog::builtin();
        let m = topaz.get("husqvarna_topaz20").unwrap();
        assert_eq!(m.sew_times().stitches_per_minute, m.stitches_per_minute);
    }

    #[test]
    fn builtin_catalog_is_valid() {
        let cat = MachineCatalog::builtin();
        cat.validate_hoops(&HoopCatalog::builtin()).unwrap();
        let topaz = cat.get("husqvarna_topaz20").unwrap();
        assert_eq!(topaz.default_format, "vp3");
        // The 360 x 200 hoop does not fit the Topaz 20.
        assert!(!topaz.hoops.iter().any(|h| h == "extra_large_360x200"));
        for id in ["tajima", "happy", "toyota", "ricoma", "bai", "highland"] {
            let m = cat.get(id).unwrap_or_else(|| panic!("no profile {id}"));
            assert!(m.hoops.iter().any(|h| h == "field_360x500"), "{id}");
        }
        assert_eq!(cat.get("happy").unwrap().default_format, "tap");
        for id in ["tajima", "happy", "toyota", "ricoma", "bai", "highland", "barudan"] {
            assert_eq!(cat.get(id).unwrap().needles, Some(15), "{id}");
        }
        assert_eq!(topaz.needles, None);
        assert_eq!(topaz.sew_times().heads, 1);
        let two = MachineCatalog::from_toml_str(
            "[[machine]]
id='m'
name='M'
default_format='dst'
formats=['dst']
hoops=['small_100x100']
             max_stitch_length_mm=12.0
min_stitch_length_mm=0.3
stitches_per_minute=600
heads=6",
        )
        .unwrap();
        assert_eq!(two.machines[0].sew_times().heads, 6);
    }

    #[test]
    fn rejects_bad_profiles() {
        let base = |extra: &str| {
            format!(
                "[[machine]]\nid='m'\nname='M'\ndefault_format='dst'\nformats=['dst']\nhoops=['small_100x100']\n\
                 max_stitch_length_mm=12.0\nmin_stitch_length_mm=0.3\nstitches_per_minute=600\n{extra}"
            )
        };
        assert!(MachineCatalog::from_toml_str(&base("")).is_ok());
        let bad = [
            base("").replace("default_format='dst'", "default_format='pes'"),
            base("").replace("formats=['dst']", "formats=['xyz']"),
            base("").replace("min_stitch_length_mm=0.3", "min_stitch_length_mm=20.0"),
            base("").replace("stitches_per_minute=600", "stitches_per_minute=0"),
            base("speed=1"),
            base("needles=16"),
            base("needles=0"),
            base("heads=0"),
            base("heads=51"),
        ];
        for s in &bad {
            assert!(MachineCatalog::from_toml_str(s).is_err(), "accepted: {s}");
        }
        let unknown_hoop = MachineCatalog::from_toml_str(&base("").replace("small_100x100", "nope")).unwrap();
        assert!(unknown_hoop.validate_hoops(&HoopCatalog::builtin()).is_err());
    }
}

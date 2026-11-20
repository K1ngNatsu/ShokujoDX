//! Content families (P17 Milestone 8, ADR 0118): a motif, a program-split
//! pattern, a carving stamp or a clipart item that is made from a small
//! description with parameters.
//!
//! A family is a rule that makes an item from some numbers: the family
//! "Waves" makes a motif from the number of the waves and their height.
//! One family gives many items, so the libraries are large with little
//! data, and the user can make an item of his own.
//!
//! The description of an item is a text: the name of the family and the
//! values of its parameters, for example `Waves: 2, 0.3`
//! ([`Family::description`], [`from_description`]). The built-in items of
//! the families are descriptions with a name ([`presets`]).
//!
//! The item itself (the points of a motif, the rows of a pattern) is kept
//! in the object that uses it, so a design opens with no family.
//!
//! All families are our own designs.

mod clipart;
mod motifs;
mod tiles;

use emb_geom::Point;

use crate::{Design, Motif, SplitPattern, Stamp};

pub(crate) const TAU: f64 = std::f64::consts::TAU;
pub(crate) const PI: f64 = std::f64::consts::PI;

/// A parameter of a family.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// The value is a whole number (a count).
    pub whole: bool,
}

/// A parameter that is a count.
pub(crate) const fn count(name: &'static str, min: u32, max: u32, default: u32) -> Param {
    Param {
        name,
        min: min as f64,
        max: max as f64,
        default: default as f64,
        whole: true,
    }
}

/// A parameter that is a size or a part.
pub(crate) const fn size(name: &'static str, min: f64, max: f64, default: f64) -> Param {
    Param {
        name,
        min,
        max,
        default,
        whole: false,
    }
}

/// What a family makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Motif,
    Split,
    Stamp,
    Clipart,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Motif, Kind::Split, Kind::Stamp, Kind::Clipart];

    /// The name of the kind (English; shown translated).
    pub fn name(self) -> &'static str {
        match self {
            Kind::Motif => "Motif",
            Kind::Split => "Split pattern",
            Kind::Stamp => "Carving stamp",
            Kind::Clipart => "Clipart",
        }
    }
}

/// An item that a family made.
#[derive(Debug, Clone)]
pub enum Item {
    Motif(Motif),
    Split(SplitPattern),
    Stamp(Stamp),
    /// A clipart item: a small design (its notes are the keywords).
    Clipart(Box<Design>),
}

impl PartialEq for Item {
    fn eq(&self, other: &Item) -> bool {
        match (self, other) {
            (Item::Motif(a), Item::Motif(b)) => a == b,
            (Item::Split(a), Item::Split(b)) => a == b,
            (Item::Stamp(a), Item::Stamp(b)) => a == b,
            // Two designs are the same if their objects (the kinds, in
            // order) and their names are the same.
            (Item::Clipart(a), Item::Clipart(b)) => {
                a.name == b.name
                    && a.notes == b.notes
                    && a.objects().len() == b.objects().len()
                    && a.objects().iter().zip(b.objects()).all(|(x, y)| x.kind == y.kind)
            }
            _ => false,
        }
    }
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::Motif(m) => &m.name,
            Item::Split(p) => &p.name,
            Item::Stamp(s) => &s.name,
            Item::Clipart(d) => &d.name,
        }
    }
}

/// The rule of a family: from the values of the parameters (all there, in
/// their ranges) to the item with no name.
#[derive(Clone, Copy)]
pub(crate) enum Make {
    /// The needle points of a motif from (0, 0) to (1, 0).
    Motif(fn(&[f64]) -> Vec<(f64, f64)>),
    Split(fn(&[f64]) -> SplitPattern),
    /// The lines of a stamp in the unit box.
    Stamp(fn(&[f64]) -> Vec<Vec<Point>>),
    Clipart(fn(&[f64]) -> Design),
}

/// A built-in item of a family: its name and the values of the parameters.
pub(crate) type Preset = (&'static str, &'static [f64]);

/// A family.
pub struct Family {
    /// The name of the family (English; it is in the descriptions).
    pub name: &'static str,
    pub params: &'static [Param],
    pub(crate) make: Make,
    pub(crate) presets: &'static [Preset],
    /// The keywords of the clipart items of the family.
    pub(crate) keywords: &'static str,
}

impl std::fmt::Debug for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Family({:?} {})", self.kind(), self.name)
    }
}

impl Family {
    pub fn kind(&self) -> Kind {
        match self.make {
            Make::Motif(_) => Kind::Motif,
            Make::Split(_) => Kind::Split,
            Make::Stamp(_) => Kind::Stamp,
            Make::Clipart(_) => Kind::Clipart,
        }
    }

    /// The keywords of the clipart items of the family (empty for the
    /// other kinds).
    pub fn keywords(&self) -> &'static str {
        self.keywords
    }

    /// The default values of the parameters.
    pub fn defaults(&self) -> Vec<f64> {
        self.params.iter().map(|p| p.default).collect()
    }

    /// `values` as the family uses them: one value for each parameter (the
    /// default where a value is not there), each in its range, counts as
    /// whole numbers.
    pub fn values(&self, values: &[f64]) -> Vec<f64> {
        self.params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let v = values.get(i).copied().filter(|v| v.is_finite()).unwrap_or(p.default);
                let v = v.clamp(p.min, p.max);
                if p.whole { v.round() } else { v }
            })
            .collect()
    }

    /// The item with these values of the parameters and this name.
    pub fn item(&self, name: &str, values: &[f64]) -> Item {
        let v = self.values(values);
        match self.make {
            Make::Motif(f) => {
                let mut points: Vec<Point> = f(&v).into_iter().map(|(x, y)| Point::new(x, y)).collect();
                points.dedup_by(|a, b| a.distance(*b) < 1e-9);
                Item::Motif(Motif {
                    name: name.to_string(),
                    points,
                })
            }
            Make::Split(f) => Item::Split(SplitPattern {
                name: name.to_string(),
                ..f(&v)
            }),
            Make::Stamp(f) => Item::Stamp(Stamp {
                name: name.to_string(),
                lines: f(&v),
            }),
            Make::Clipart(f) => {
                let mut d = f(&v);
                d.name = name.to_string();
                d.notes = self.keywords.to_string();
                Item::Clipart(Box::new(d))
            }
        }
    }

    /// The description of the item with these values: the name of the
    /// family and the values, for example `Waves: 2, 0.3`.
    pub fn description(&self, values: &[f64]) -> String {
        let v = self.values(values);
        let texts: Vec<String> = self
            .params
            .iter()
            .zip(&v)
            .map(|(p, v)| {
                if p.whole {
                    format!("{v:.0}")
                } else {
                    let t = format!("{v:.2}");
                    t.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            })
            .collect();
        if texts.is_empty() {
            self.name.to_string()
        } else {
            format!("{}: {}", self.name, texts.join(", "))
        }
    }

    /// The built-in items of the family, with their names.
    pub fn presets(&self) -> Vec<Item> {
        self.presets.iter().map(|(name, v)| self.item(name, v)).collect()
    }

    /// The names of the built-in items of the family and their values.
    pub fn preset_values(&self) -> Vec<(&'static str, Vec<f64>)> {
        self.presets.iter().map(|(name, v)| (*name, self.values(v))).collect()
    }
}

/// All families: the families of motifs, of split patterns, of stamps and
/// of clipart, each kind in menu order.
pub fn families() -> Vec<&'static Family> {
    motifs::FAMILIES
        .iter()
        .chain(tiles::SPLITS)
        .chain(tiles::STAMPS)
        .chain(clipart::FAMILIES)
        .collect()
}

/// The families of one kind.
pub fn families_of(kind: Kind) -> Vec<&'static Family> {
    families().into_iter().filter(|f| f.kind() == kind).collect()
}

/// The family of a kind with this name.
pub fn family(kind: Kind, name: &str) -> Option<&'static Family> {
    families_of(kind)
        .into_iter()
        .find(|f| f.name.eq_ignore_ascii_case(name.trim()))
}

/// The family and the values of a description (see
/// [`Family::description`]). Values that are not there are the defaults.
pub fn parse(kind: Kind, description: &str) -> Option<(&'static Family, Vec<f64>)> {
    let (name, rest) = match description.split_once(':') {
        Some((name, rest)) => (name, rest),
        None => (description, ""),
    };
    let f = family(kind, name)?;
    let mut values = Vec::new();
    for t in rest.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        values.push(t.parse::<f64>().ok()?);
    }
    Some((f, f.values(&values)))
}

/// The item of a description; its name is the description.
pub fn from_description(kind: Kind, description: &str) -> Option<Item> {
    let (f, values) = parse(kind, description)?;
    Some(f.item(&f.description(&values), &values))
}

/// The built-in items of the families of a kind, in menu order.
pub fn presets(kind: Kind) -> Vec<Item> {
    families_of(kind).into_iter().flat_map(Family::presets).collect()
}

/// The built-in motifs of the families.
pub fn preset_motifs() -> Vec<Motif> {
    presets(Kind::Motif)
        .into_iter()
        .filter_map(|i| match i {
            Item::Motif(m) => Some(m),
            _ => None,
        })
        .collect()
}

/// The built-in split patterns of the families.
pub fn preset_splits() -> Vec<SplitPattern> {
    presets(Kind::Split)
        .into_iter()
        .filter_map(|i| match i {
            Item::Split(p) => Some(p),
            _ => None,
        })
        .collect()
}

/// The built-in stamps of the families.
pub fn preset_stamps() -> Vec<Stamp> {
    presets(Kind::Stamp)
        .into_iter()
        .filter_map(|i| match i {
            Item::Stamp(s) => Some(s),
            _ => None,
        })
        .collect()
}

/// `n + 1` points of the curve `f(t)` for t from 0 to 1.
pub(crate) fn curve(n: usize, f: impl Fn(f64) -> (f64, f64)) -> Vec<(f64, f64)> {
    let n = n.max(1);
    (0..=n).map(|i| f(i as f64 / n as f64)).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn each_family_has_parameters_in_order_and_presets() {
        let mut names = BTreeSet::new();
        for f in families() {
            assert!(names.insert((f.kind(), f.name)), "{f:?} is there two times");
            assert!(!f.name.contains(':') && !f.name.contains(','), "{f:?}");
            for p in f.params {
                assert!(
                    p.min < p.max && (p.min..=p.max).contains(&p.default),
                    "{f:?} {}",
                    p.name
                );
                assert!(
                    !p.whole || (p.min.fract() == 0.0 && p.max.fract() == 0.0),
                    "{f:?} {}",
                    p.name
                );
            }
            assert!(!f.presets.is_empty(), "{f:?} has no built-in item");
            for (name, v) in f.presets {
                assert_eq!(v.len(), f.params.len(), "{f:?} {name}");
                assert_eq!(&f.values(v), v, "{f:?} {name}: a value is out of its range");
            }
        }
        // The names of the built-in items of a kind are different.
        for kind in Kind::ALL {
            let items = presets(kind);
            let names: BTreeSet<&str> = items.iter().map(Item::name).collect();
            assert_eq!(names.len(), items.len(), "{kind:?}");
            assert!(families_of(kind).len() >= 8, "{kind:?}");
        }
    }

    #[test]
    fn a_description_gives_the_item_again() {
        for f in families() {
            for values in [
                f.defaults(),
                f.params.iter().map(|p| p.min).collect(),
                f.params.iter().map(|p| p.max).collect(),
            ] {
                let text = f.description(&values);
                let (g, v) = parse(f.kind(), &text).unwrap_or_else(|| panic!("{text}"));
                assert!(std::ptr::eq(f, g), "{text}");
                // The description has two decimal places.
                for (a, b) in v.iter().zip(&f.values(&values)) {
                    assert!((a - b).abs() < 0.006, "{text}: {a} {b}");
                }
                let item = from_description(f.kind(), &text).unwrap();
                assert_eq!(item.name(), text);
                assert_eq!(item, f.item(&text, &v), "{text}");
            }
        }
        // Values that are not there are the defaults; values out of the
        // range go to the range.
        let waves = family(Kind::Motif, "waves").unwrap();
        assert_eq!(parse(Kind::Motif, "Waves").unwrap().1, waves.defaults());
        assert_eq!(parse(Kind::Motif, "Waves: 99").unwrap().1[0], waves.params[0].max);
        assert_eq!(waves.description(&[2.0, 0.3]), "Waves: 2, 0.3");
        assert!(parse(Kind::Motif, "No such family: 1").is_none());
        assert!(parse(Kind::Motif, "Waves: x").is_none());
        assert!(parse(Kind::Stamp, "Waves: 2, 0.3").is_none_or(|(f, _)| f.kind() == Kind::Stamp));
    }

    #[test]
    fn a_parameter_changes_the_item() {
        for f in families() {
            let base = f.item("x", &f.defaults());
            for (i, p) in f.params.iter().enumerate() {
                let mut v = f.defaults();
                v[i] = if p.default < p.max { p.max } else { p.min };
                assert_ne!(f.item("x", &v), base, "{f:?}: {} changes nothing", p.name);
            }
        }
    }
}

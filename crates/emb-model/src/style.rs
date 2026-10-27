//! Current properties and styles.
//!
//! - The current properties of a design are the stitch settings that new
//!   objects get. "Make properties current" copies the settings of an
//!   object to them; "Apply current properties" copies them to objects.
//! - A style is a named set of stitch settings. "Apply style" copies the
//!   settings to objects of the same type. A style is a copy: a later change
//!   to the style does not change the objects.

use serde::{Deserialize, Serialize};

use crate::{AppliqueParams, Design, DesignObject, FillParams, ObjectId, ObjectKind, RunningParams, SatinParams};

/// The stitch settings for each object type.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CurrentProperties {
    pub running: RunningParams,
    pub satin: SatinParams,
    pub fill: FillParams,
    pub applique: AppliqueParams,
}

impl CurrentProperties {
    pub fn is_default(&self) -> bool {
        *self == CurrentProperties::default()
    }

    /// Copies the settings of `kind` for its type. Returns `false` if the
    /// object has no stitch settings (stitches from a machine file).
    pub fn take_from(&mut self, kind: &ObjectKind) -> bool {
        match kind {
            ObjectKind::Running { params, .. } => self.running = params.clone(),
            ObjectKind::Satin { params, .. } => self.satin = params.clone(),
            ObjectKind::Fill { params, .. } => self.fill = params.clone(),
            ObjectKind::Applique { params, .. } => self.applique = params.clone(),
            // Lettering has its own settings (Object Properties).
            ObjectKind::Stitches { .. }
            | ObjectKind::Lettering(_)
            | ObjectKind::Vector { .. }
            | ObjectKind::CrossStitch(_)
            | ObjectKind::Bling(_)
            | ObjectKind::Sketch(_) => return false,
        }
        true
    }

    /// Gives `kind` the settings for its type. Returns `true` if they changed.
    pub fn apply_to(&self, kind: &mut ObjectKind) -> bool {
        set_params(
            kind,
            Some(&self.running),
            Some(&self.satin),
            Some(&self.fill),
            Some(&self.applique),
        )
    }
}

fn set_params(
    kind: &mut ObjectKind,
    running: Option<&RunningParams>,
    satin: Option<&SatinParams>,
    fill: Option<&FillParams>,
    applique: Option<&AppliqueParams>,
) -> bool {
    fn set<T: Clone + PartialEq>(dst: &mut T, src: Option<&T>) -> bool {
        match src {
            Some(s) if s != dst => {
                *dst = s.clone();
                true
            }
            _ => false,
        }
    }
    match kind {
        ObjectKind::Running { params, .. } => set(params, running),
        ObjectKind::Satin { params, .. } => set(params, satin),
        ObjectKind::Fill { params, .. } => set(params, fill),
        ObjectKind::Applique { params, .. } => set(params, applique),
        ObjectKind::Stitches { .. }
        | ObjectKind::Lettering(_)
        | ObjectKind::Vector { .. }
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => false,
    }
}

/// A named set of stitch settings. Each type is optional: a style made from
/// a satin object has only satin settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<RunningParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub satin: Option<SatinParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<FillParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applique: Option<AppliqueParams>,
}

impl Style {
    /// A style with the settings of the first object of each type. `None`
    /// if no object has stitch settings.
    pub fn from_objects<'a>(
        name: impl Into<String>,
        objects: impl IntoIterator<Item = &'a DesignObject>,
    ) -> Option<Style> {
        let mut s = Style {
            name: name.into(),
            running: None,
            satin: None,
            fill: None,
            applique: None,
        };
        for o in objects {
            match &o.kind {
                ObjectKind::Running { params, .. } if s.running.is_none() => s.running = Some(params.clone()),
                ObjectKind::Satin { params, .. } if s.satin.is_none() => s.satin = Some(params.clone()),
                ObjectKind::Fill { params, .. } if s.fill.is_none() => s.fill = Some(params.clone()),
                ObjectKind::Applique { params, .. } if s.applique.is_none() => s.applique = Some(params.clone()),
                _ => {}
            }
        }
        (!s.is_empty()).then_some(s)
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_none() && self.satin.is_none() && self.fill.is_none() && self.applique.is_none()
    }

    /// The object types that the style has settings for, for example "satin, fill".
    pub fn types(&self) -> String {
        let mut v = Vec::new();
        if self.running.is_some() {
            v.push("running");
        }
        if self.satin.is_some() {
            v.push("satin");
        }
        if self.fill.is_some() {
            v.push("fill");
        }
        if self.applique.is_some() {
            v.push("appliqué");
        }
        v.join(", ")
    }

    /// Gives `kind` the settings of the style for its type. Returns `true` if they changed.
    pub fn apply_to(&self, kind: &mut ObjectKind) -> bool {
        set_params(
            kind,
            self.running.as_ref(),
            self.satin.as_ref(),
            self.fill.as_ref(),
            self.applique.as_ref(),
        )
    }

    /// Takes the settings of `objects` for the types they have; the other types stay.
    pub fn update_from<'a>(&mut self, objects: impl IntoIterator<Item = &'a DesignObject>) {
        if let Some(new) = Style::from_objects("", objects) {
            self.running = new.running.or(self.running.take());
            self.satin = new.satin.or(self.satin.take());
            self.fill = new.fill.or(self.fill.take());
            self.applique = new.applique.or(self.applique.take());
        }
    }
}

impl Design {
    /// Copies the settings of the objects to the current properties (the
    /// first object of each type). Returns the number of types that changed.
    pub fn make_current(&mut self, ids: &[ObjectId]) -> usize {
        let before = self.current.clone();
        let mut seen = [false; 4];
        for id in ids {
            let Some(o) = self.object(*id) else { continue };
            let i = match o.kind {
                ObjectKind::Running { .. } => 0,
                ObjectKind::Satin { .. } => 1,
                ObjectKind::Fill { .. } => 2,
                ObjectKind::Applique { .. } => 3,
                ObjectKind::Stitches { .. }
                | ObjectKind::Lettering(_)
                | ObjectKind::Vector { .. }
                | ObjectKind::CrossStitch(_)
                | ObjectKind::Bling(_)
                | ObjectKind::Sketch(_) => continue,
            };
            if !seen[i] {
                seen[i] = true;
                let kind = o.kind.clone();
                self.current.take_from(&kind);
            }
        }
        let c = &self.current;
        usize::from(c.running != before.running)
            + usize::from(c.satin != before.satin)
            + usize::from(c.fill != before.fill)
            + usize::from(c.applique != before.applique)
    }

    /// Gives the objects the current properties. Returns the changed objects.
    pub fn apply_current(&mut self, ids: &[ObjectId]) -> Vec<ObjectId> {
        let current = self.current.clone();
        self.change_params(ids, |k| current.apply_to(k))
    }

    /// Gives the objects the settings of style `index`. Returns the changed objects.
    pub fn apply_style(&mut self, index: usize, ids: &[ObjectId]) -> Vec<ObjectId> {
        let Some(style) = self.styles.get(index).cloned() else {
            return Vec::new();
        };
        self.change_params(ids, |k| style.apply_to(k))
    }

    /// Changes the objects whose settings `f` would change. Objects that
    /// stay the same keep their revision (no new stitches are needed).
    fn change_params(&mut self, ids: &[ObjectId], f: impl Fn(&mut ObjectKind) -> bool) -> Vec<ObjectId> {
        let mut changed = Vec::new();
        for id in ids {
            let Some(mut kind) = self.object(*id).map(|o| o.kind.clone()) else {
                continue;
            };
            if f(&mut kind)
                && let Some(o) = self.object_mut(*id)
            {
                o.kind = kind;
                changed.push(*id);
            }
        }
        changed
    }

    /// The index of the style with this name.
    pub fn style_index(&self, name: &str) -> Option<usize> {
        self.styles.iter().position(|s| s.name == name)
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Path, Point};

    use super::*;
    use crate::{Rgb, SatinGeometry, Underlay};

    fn design() -> (Design, Vec<ObjectId>) {
        let mut d = Design::new("t");
        let t = d.add_thread("red", Rgb(200, 0, 0));
        let mut ids = Vec::new();
        for i in 0..3 {
            let y = f64::from(i) * 10.0;
            let line = Path::new().move_to(Point::new(0.0, y)).line_to(Point::new(30.0, y));
            ids.push(d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: SatinGeometry::from_centerline(&line, 3.0),
                    params: SatinParams::default(),
                },
            ));
        }
        ids.push(d.add_object(
            t,
            ObjectKind::Fill {
                outer: Path::rect(BBox::new(Point::new(0.0, 40.0), Point::new(20.0, 60.0))),
                holes: Vec::new(),
                params: FillParams {
                    angle_deg: 0.0,
                    ..FillParams::default()
                },
            },
        ));
        (d, ids)
    }

    fn satin(d: &Design, id: ObjectId) -> SatinParams {
        match &d.object(id).unwrap().kind {
            ObjectKind::Satin { params, .. } => params.clone(),
            _ => panic!(),
        }
    }

    #[test]
    fn apply_a_style_to_three_objects() {
        let (mut d, ids) = design();
        let bold = SatinParams {
            spacing_mm: 0.3,
            pull_compensation_mm: 0.35,
            underlay: Underlay::Heavy,
            ..SatinParams::default()
        };
        d.styles.push(Style {
            name: "Bold border".into(),
            running: None,
            satin: Some(bold.clone()),
            fill: None,
            applique: None,
        });
        let rev = d.object(ids[3]).unwrap().revision;
        let changed = d.apply_style(0, &ids);
        assert_eq!(changed, ids[..3], "the fill has no setting in the style");
        for id in &ids[..3] {
            assert_eq!(satin(&d, *id), bold);
        }
        assert_eq!(
            d.object(ids[3]).unwrap().revision,
            rev,
            "an object that does not change keeps its revision"
        );
        assert!(d.apply_style(0, &ids).is_empty(), "a second time changes nothing");
        assert!(d.apply_style(5, &ids).is_empty());
    }

    #[test]
    fn style_from_and_update_from_objects() {
        let (d, ids) = design();
        let objs: Vec<&DesignObject> = ids.iter().filter_map(|i| d.object(*i)).collect();
        let s = Style::from_objects("Mixed", objs.iter().copied()).unwrap();
        assert_eq!(s.types(), "satin, fill");
        assert_eq!(s.fill.as_ref().unwrap().angle_deg, 0.0);
        let mut only_satin = Style::from_objects("Satin", [objs[0]]).unwrap();
        assert!(only_satin.fill.is_none());
        only_satin.update_from([objs[3]]);
        assert!(
            only_satin.fill.is_some() && only_satin.satin.is_some(),
            "update keeps the other types"
        );
        assert!(Style::from_objects("x", []).is_none());
        let json = serde_json::to_string(&only_satin).unwrap();
        assert_eq!(serde_json::from_str::<Style>(&json).unwrap(), only_satin);
    }

    #[test]
    fn make_current_and_apply_current() {
        let (mut d, ids) = design();
        assert_eq!(d.make_current(&ids[3..]), 1);
        assert_eq!(d.current.fill.angle_deg, 0.0);
        assert_eq!(d.make_current(&ids[3..]), 0);
        d.current.satin.spacing_mm = 0.5;
        let changed = d.apply_current(&ids);
        assert_eq!(changed, ids[..3]);
        assert_eq!(satin(&d, ids[1]).spacing_mm, 0.5);
    }
}

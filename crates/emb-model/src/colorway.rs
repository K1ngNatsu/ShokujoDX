//! Colorways: several color versions of one design.
//!
//! `Design::threads` always holds the looks of the current colorway, so the
//! stitch plan, export and views need nothing else. Each colorway stores the
//! look of each thread and the background (fabric) color. See
//! `docs/design/p3-plan.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Design, ObjectId, Rgb, Thread, ThreadChart, ThreadId};

/// The look of a thread: everything except its id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadLook {
    pub name: String,
    pub color: Rgb,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ThreadLook {
    pub fn of(t: &Thread) -> ThreadLook {
        ThreadLook {
            name: t.name.clone(),
            color: t.color,
            brand: t.brand.clone(),
            code: t.code.clone(),
        }
    }

    fn apply(&self, t: &mut Thread) {
        t.name.clone_from(&self.name);
        t.color = self.color;
        t.brand.clone_from(&self.brand);
        t.code.clone_from(&self.code);
    }
}

/// One color version of the design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Colorway {
    pub name: String,
    pub looks: BTreeMap<ThreadId, ThreadLook>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgb>,
}

impl Design {
    /// Makes sure there is at least one colorway (the current colors).
    pub fn ensure_colorways(&mut self) {
        if self.colorways.is_empty() {
            self.colorways.push(Colorway {
                name: "Colorway 1".into(),
                looks: BTreeMap::new(),
                background: None,
            });
            self.colorway = 0;
            self.store_colorway();
        }
    }

    /// Writes the current thread looks and background into the current colorway.
    fn store_colorway(&mut self) {
        let looks = self.threads.iter().map(|t| (t.id, ThreadLook::of(t))).collect();
        let background = self.background;
        if let Some(c) = self.colorways.get_mut(self.colorway) {
            c.looks = looks;
            c.background = background;
        }
    }

    /// Makes colorway `i` the current one. Returns `false` if it does not exist.
    pub fn switch_colorway(&mut self, i: usize) -> bool {
        self.ensure_colorways();
        if i >= self.colorways.len() {
            return false;
        }
        self.store_colorway();
        self.colorway = i;
        let c = &self.colorways[i];
        for t in &mut self.threads {
            if let Some(look) = c.looks.get(&t.id) {
                look.apply(t);
            }
        }
        self.background = c.background;
        true
    }

    /// Adds a copy of the current colorway and makes it current. Returns its index.
    pub fn add_colorway(&mut self, name: impl Into<String>) -> usize {
        self.ensure_colorways();
        self.store_colorway();
        let mut copy = self.colorways[self.colorway].clone();
        copy.name = name.into();
        self.colorways.push(copy);
        self.colorway = self.colorways.len() - 1;
        self.colorway
    }

    /// Removes colorway `i`. The last colorway cannot be removed.
    pub fn remove_colorway(&mut self, i: usize) -> bool {
        if self.colorways.len() < 2 || i >= self.colorways.len() {
            return false;
        }
        let current = self.colorway;
        self.colorways.remove(i);
        if i == current {
            // Show the colorway that took its place.
            let next = i.min(self.colorways.len() - 1);
            self.colorway = next;
            let c = self.colorways[next].clone();
            for t in &mut self.threads {
                if let Some(look) = c.looks.get(&t.id) {
                    look.apply(t);
                }
            }
            self.background = c.background;
        } else if i < current {
            self.colorway = current - 1;
        }
        true
    }

    /// Moves colorway `i` one place up (`up`) or down the list. The same
    /// colorway stays current. Returns `false` if it cannot move.
    pub fn move_colorway(&mut self, i: usize, up: bool) -> bool {
        let j = match (up, i) {
            (true, 0) => return false,
            (true, _) => i - 1,
            (false, _) => i + 1,
        };
        if j >= self.colorways.len() || i >= self.colorways.len() {
            return false;
        }
        self.colorways.swap(i, j);
        if self.colorway == i {
            self.colorway = j;
        } else if self.colorway == j {
            self.colorway = i;
        }
        true
    }

    /// The name of the current colorway, if the design has colorways.
    pub fn colorway_name(&self) -> Option<&str> {
        self.colorways.get(self.colorway).map(|c| c.name.as_str())
    }

    /// The look of each thread in colorway `i` (the current looks for the
    /// current colorway).
    pub fn colorway_looks(&self, i: usize) -> Vec<(ThreadId, ThreadLook)> {
        self.threads
            .iter()
            .map(|t| {
                let stored = (i != self.colorway)
                    .then(|| self.colorways.get(i).and_then(|c| c.looks.get(&t.id)))
                    .flatten();
                (t.id, stored.cloned().unwrap_or_else(|| ThreadLook::of(t)))
            })
            .collect()
    }

    /// Sets the look of a thread in colorway `i`.
    pub fn set_colorway_look(&mut self, i: usize, id: ThreadId, look: ThreadLook) {
        if i == self.colorway || self.colorways.is_empty() {
            if let Some(t) = self.threads.iter_mut().find(|t| t.id == id) {
                look.apply(t);
            }
        } else if let Some(c) = self.colorways.get_mut(i) {
            c.looks.insert(id, look);
        }
    }

    /// The threads that objects use, in the order of their first use.
    pub fn used_threads(&self) -> Vec<ThreadId> {
        let mut used = Vec::new();
        for o in &self.objects {
            if !used.contains(&o.thread) {
                used.push(o.thread);
            }
        }
        used
    }

    /// Removes the threads that no object uses. Returns how many.
    pub fn remove_unused_threads(&mut self) -> usize {
        let used = self.used_threads();
        let before = self.threads.len();
        self.threads.retain(|t| used.contains(&t.id));
        for c in &mut self.colorways {
            c.looks.retain(|id, _| used.contains(id));
        }
        before - self.threads.len()
    }

    /// Moves the looks of the used threads one step: each thread gets the
    /// look of the next used thread, and the last gets the look of the first.
    /// Returns `false` if fewer than two threads are used.
    pub fn cycle_used_colors(&mut self) -> bool {
        let used = self.used_threads();
        if used.len() < 2 {
            return false;
        }
        let looks: Vec<ThreadLook> = used
            .iter()
            .filter_map(|id| self.thread(*id).map(ThreadLook::of))
            .collect();
        for (k, id) in used.iter().enumerate() {
            let look = looks[(k + 1) % looks.len()].clone();
            if let Some(t) = self.threads.iter_mut().find(|t| t.id == *id) {
                look.apply(t);
            }
        }
        true
    }

    /// Gives the objects a new thread with the look of the thread of the
    /// first object, so their color can change alone. Returns the new thread.
    pub fn split_thread(&mut self, ids: &[ObjectId]) -> Option<ThreadId> {
        let first = self.object(*ids.first()?)?.thread;
        let look = ThreadLook::of(self.thread(first)?);
        let new = self.add_thread(look.name.clone(), look.color);
        if let Some(t) = self.threads.iter_mut().find(|t| t.id == new) {
            look.apply(t);
        }
        for c in &mut self.colorways {
            if let Some(l) = c.looks.get(&first).cloned() {
                c.looks.insert(new, l);
            }
        }
        for id in ids {
            if let Some(o) = self.object_mut(*id) {
                o.thread = new;
            }
        }
        Some(new)
    }

    /// Gives each thread in `ids` the look of the nearest chart thread.
    /// The brand is the chart name.
    pub fn match_threads(&mut self, chart: &ThreadChart, ids: &[ThreadId]) {
        for t in &mut self.threads {
            if !ids.contains(&t.id) {
                continue;
            }
            if let Some(k) = chart.nearest(t.color) {
                chart_look(chart, k).apply(t);
            }
        }
    }
}

/// The look of chart thread `k`. The brand is the brand of the thread, else
/// the chart name.
pub fn chart_look(chart: &ThreadChart, k: usize) -> ThreadLook {
    let c = &chart.threads[k];
    ThreadLook {
        name: c.name.clone(),
        color: c.color,
        brand: Some(c.brand.clone().unwrap_or_else(|| chart.name.clone())),
        code: c.code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::tests::line;

    #[test]
    fn move_colorway_keeps_the_current_one() {
        let (mut d, red, _) = two_colors();
        d.add_colorway("B");
        d.add_colorway("C");
        d.switch_colorway(1);
        d.threads[0].color = Rgb(1, 2, 3);
        assert!(d.move_colorway(1, true));
        let names: Vec<&str> = d.colorways.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["B", "Colorway 1", "C"]);
        assert_eq!(d.colorway_name(), Some("B"));
        assert!(!d.move_colorway(0, true), "the first cannot go up");
        assert!(!d.move_colorway(2, false), "the last cannot go down");
        // Another colorway moves past the current one.
        assert!(d.move_colorway(1, true));
        assert_eq!(d.colorway_name(), Some("B"));
        assert_eq!(d.colorway, 1);
        // The looks stay with their colorway.
        d.switch_colorway(0);
        d.switch_colorway(1);
        assert_eq!(d.thread(red).unwrap().color, Rgb(1, 2, 3));
    }

    fn two_colors() -> (Design, ThreadId, ThreadId) {
        let mut d = Design::new("t");
        let red = d.add_thread("Red", Rgb(200, 0, 0));
        let blue = d.add_thread("Blue", Rgb(0, 0, 200));
        d.add_object(red, line(0.0, 10.0));
        d.add_object(blue, line(0.0, 20.0));
        (d, red, blue)
    }

    #[test]
    fn colorways_switch_and_keep_their_colors() {
        let (mut d, red, _) = two_colors();
        d.background = Some(Rgb(255, 255, 255));
        assert_eq!(d.add_colorway("Dark"), 1);
        assert_eq!(d.colorways.len(), 2);
        d.threads[0].color = Rgb(10, 10, 10);
        d.threads[0].name = "Black".into();
        d.background = Some(Rgb(0, 0, 0));

        d.switch_colorway(0);
        assert_eq!(d.thread(red).unwrap().color, Rgb(200, 0, 0));
        assert_eq!(d.thread(red).unwrap().name, "Red");
        assert_eq!(d.background, Some(Rgb(255, 255, 255)));
        // The other colorway, read without a switch.
        let dark = d.colorway_looks(1);
        assert_eq!(dark[0].1.color, Rgb(10, 10, 10));

        d.switch_colorway(1);
        assert_eq!(d.thread(red).unwrap().color, Rgb(10, 10, 10));
        assert_eq!(d.background, Some(Rgb(0, 0, 0)));
        assert_eq!(d.colorway_name(), Some("Dark"));
        assert!(!d.switch_colorway(5));
    }

    #[test]
    fn a_new_thread_keeps_its_look_in_old_colorways() {
        let (mut d, _, _) = two_colors();
        d.add_colorway("B");
        let green = d.add_thread("Green", Rgb(0, 200, 0));
        d.switch_colorway(0);
        assert_eq!(d.thread(green).unwrap().color, Rgb(0, 200, 0));
    }

    #[test]
    fn set_look_in_another_colorway() {
        let (mut d, red, _) = two_colors();
        d.add_colorway("B");
        let look = ThreadLook {
            name: "Pink".into(),
            color: Rgb(255, 150, 150),
            brand: None,
            code: None,
        };
        d.set_colorway_look(0, red, look);
        assert_eq!(d.thread(red).unwrap().name, "Red", "colorway 1 is current");
        d.switch_colorway(0);
        assert_eq!(d.thread(red).unwrap().name, "Pink");
    }

    #[test]
    fn remove_colorway() {
        let (mut d, red, _) = two_colors();
        d.add_colorway("B");
        d.threads[0].color = Rgb(1, 2, 3);
        assert!(d.remove_colorway(1));
        assert_eq!(d.colorway, 0);
        assert_eq!(d.thread(red).unwrap().color, Rgb(200, 0, 0));
        assert!(!d.remove_colorway(0), "the last colorway stays");
    }

    #[test]
    fn cycle_split_and_remove_unused() {
        let (mut d, red, blue) = two_colors();
        let spare = d.add_thread("Spare", Rgb(1, 1, 1));
        assert!(d.cycle_used_colors());
        assert_eq!(d.thread(red).unwrap().name, "Blue");
        assert_eq!(d.thread(blue).unwrap().name, "Red");
        assert_eq!(d.thread(spare).unwrap().name, "Spare", "unused threads do not change");

        let first = d.objects()[0].id;
        let new = d.split_thread(&[first]).unwrap();
        assert_eq!(d.object(first).unwrap().thread, new);
        assert_eq!(d.thread(new).unwrap().color, d.thread(red).unwrap().color);

        // Red is now unused, and Spare too.
        assert_eq!(d.remove_unused_threads(), 2);
        assert_eq!(d.threads.len(), 2);
    }

    #[test]
    fn match_to_chart() {
        let (mut d, red, blue) = two_colors();
        let pec = &ThreadChart::builtin()[0];
        d.match_threads(pec, &[red]);
        let t = d.thread(red).unwrap();
        assert_eq!(t.brand.as_deref(), Some("Brother PEC"));
        assert!(t.code.is_some());
        assert_eq!(d.thread(blue).unwrap().name, "Blue", "not in the list");
    }
}

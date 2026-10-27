//! Search, sort and group of the library entries.
//!
//! The search text has words and filters, with spaces between them. Each
//! word must be in the name, file, folder, notes, thread names, machine or
//! format of the design (capital letters do not matter); `"two words"`
//! is one word. A filter is `key:value`:
//!
//! - `format:vp3` (also `format:vp3,dst`), `folder:caps`, `name:rose`,
//!   `machine:topaz`: text in that field only.
//! - `colors:`, `stitches:`, `trims:`, `width:`, `height:` (mm), `kb:`
//!   (file size): a number `5`, `<5`, `<=5`, `>5`, `>=5` or a range
//!   `2-5`.
//! - `fits:<hoop>`: the design fits the hoop (its id, or a part of its
//!   name), inside the safety margin, straight or turned 90°.
//! - `error:yes` (or `error:no`): files that cannot be read.

use std::cmp::Ordering;

use emb_model::{HoopCatalog, HoopPreset};

use crate::{Entry, Index};

/// The safety margin of `fits:` and of the hoop groups, in mm.
pub const FIT_MARGIN_MM: f64 = 5.0;

/// A number field of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Colors,
    Stitches,
    Trims,
    Width,
    Height,
    Kb,
}

impl Field {
    fn value(self, e: &Entry) -> f64 {
        match self {
            Field::Colors => e.colors as f64,
            Field::Stitches => e.stitches as f64,
            Field::Trims => e.trims as f64,
            Field::Width => e.width_mm,
            Field::Height => e.height_mm,
            Field::Kb => e.bytes as f64 / 1024.0,
        }
    }
}

/// A text field of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextField {
    Format,
    Folder,
    Name,
    Machine,
}

/// One end of a range: the number, and `true` if the number is in the range.
pub type Bound = Option<(f64, bool)>;

/// One filter of the search text.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    /// The field has one of the texts.
    Text(TextField, Vec<String>),
    /// The number is from `min` to `max` (each one included if its flag is set).
    Range { field: Field, min: Bound, max: Bound },
    /// The design fits this hoop.
    Fits(Box<HoopPreset>),
    /// The file has an error (`true`) or not.
    Error(bool),
    /// A filter that nothing matches (for example a hoop that is not known).
    Nothing(String),
}

/// A parsed search text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Query {
    /// Lower-case words; each one must be in the entry.
    pub words: Vec<String>,
    pub filters: Vec<Filter>,
}

impl Query {
    /// Parses the search text. Hoops for `fits:` come from `hoops`.
    pub fn parse(text: &str, hoops: &HoopCatalog) -> Query {
        let mut q = Query::default();
        for token in tokens(text) {
            let Some((key, value)) = token.split_once(':').filter(|(k, v)| !k.is_empty() && !v.is_empty()) else {
                q.words.push(token.to_lowercase());
                continue;
            };
            let value = value.to_lowercase();
            let texts = || value.split(',').map(str::to_string).filter(|s| !s.is_empty()).collect();
            let field = match key.to_lowercase().as_str() {
                "format" | "type" => Some(Ok(TextField::Format)),
                "folder" => Some(Ok(TextField::Folder)),
                "name" => Some(Ok(TextField::Name)),
                "machine" => Some(Ok(TextField::Machine)),
                "colors" | "colours" => Some(Err(Field::Colors)),
                "stitches" => Some(Err(Field::Stitches)),
                "trims" => Some(Err(Field::Trims)),
                "width" => Some(Err(Field::Width)),
                "height" => Some(Err(Field::Height)),
                "kb" => Some(Err(Field::Kb)),
                "fits" | "hoop" => {
                    q.filters.push(match find_hoop(hoops, &value) {
                        Some(h) => Filter::Fits(Box::new(h.clone())),
                        None => Filter::Nothing(format!("no hoop {value:?}")),
                    });
                    None
                }
                "error" | "errors" => {
                    q.filters
                        .push(Filter::Error(!matches!(value.as_str(), "no" | "false" | "0")));
                    None
                }
                _ => {
                    q.words.push(token.to_lowercase());
                    None
                }
            };
            match field {
                Some(Ok(t)) => q.filters.push(Filter::Text(t, texts())),
                Some(Err(f)) => q.filters.push(match range(&value) {
                    Some((min, max)) => Filter::Range { field: f, min, max },
                    None => Filter::Nothing(format!("{key}: needs a number, not {value:?}")),
                }),
                None => {}
            }
        }
        q
    }

    /// `true` if the search text has nothing in it.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.filters.is_empty()
    }

    /// The problems in the search text (filters that match nothing).
    pub fn problems(&self) -> Vec<String> {
        self.filters
            .iter()
            .filter_map(|f| match f {
                Filter::Nothing(why) => Some(why.clone()),
                _ => None,
            })
            .collect()
    }

    /// `true` if the entry has all words and passes all filters.
    pub fn matches(&self, e: &Entry) -> bool {
        if !self.words.is_empty() {
            let hay = haystack(e);
            if !self.words.iter().all(|w| hay.contains(w.as_str())) {
                return false;
            }
        }
        self.filters.iter().all(|f| passes(f, e))
    }

    /// The indices of the entries that match, in index order.
    pub fn run(&self, index: &Index) -> Vec<usize> {
        (0..index.entries.len())
            .filter(|&i| self.matches(&index.entries[i]))
            .collect()
    }
}

/// Words with spaces between them; a quoted part is one word.
fn tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in text.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `5`, `<5`, `<=5`, `>5`, `>=5`, `2-5` (or `2..5`).
fn range(v: &str) -> Option<(Bound, Bound)> {
    let num = |s: &str| s.trim().parse::<f64>().ok();
    if let Some(r) = v.strip_prefix("<=") {
        return Some((None, Some((num(r)?, true))));
    }
    if let Some(r) = v.strip_prefix(">=") {
        return Some((Some((num(r)?, true)), None));
    }
    if let Some(r) = v.strip_prefix('<') {
        return Some((None, Some((num(r)?, false))));
    }
    if let Some(r) = v.strip_prefix('>') {
        return Some((Some((num(r)?, false)), None));
    }
    if let Some((a, b)) = v.split_once("..").or_else(|| v.split_once('-')) {
        return Some((Some((num(a)?, true)), Some((num(b)?, true))));
    }
    let n = num(v)?;
    Some((Some((n, true)), Some((n, true))))
}

fn find_hoop<'a>(hoops: &'a HoopCatalog, v: &str) -> Option<&'a HoopPreset> {
    hoops
        .hoops
        .iter()
        .find(|h| h.id.to_lowercase() == v)
        .or_else(|| hoops.hoops.iter().find(|h| h.name.to_lowercase().contains(v)))
}

/// The text that the words of a search look in, in lower case.
fn haystack(e: &Entry) -> String {
    let mut s = String::with_capacity(64 + e.notes.len());
    for part in [&e.name, &e.file, &e.folder, &e.notes, &e.format] {
        s.push_str(part);
        s.push('\n');
    }
    if let Some(m) = &e.machine {
        s.push_str(m);
        s.push('\n');
    }
    for t in &e.threads {
        s.push_str(&t.name);
        s.push('\n');
    }
    s.to_lowercase()
}

/// `true` if a design of `w` × `h` mm fits the hoop, straight or turned.
pub fn fits(hoop: &HoopPreset, w: f64, h: f64) -> bool {
    hoop.positions.is_empty() && (hoop.fits(w, h, FIT_MARGIN_MM) || hoop.fits(h, w, FIT_MARGIN_MM))
}

fn passes(f: &Filter, e: &Entry) -> bool {
    match f {
        Filter::Text(field, texts) => {
            let v = match field {
                TextField::Format => e.format.clone(),
                TextField::Folder => e.folder.to_lowercase(),
                TextField::Name => e.name.to_lowercase(),
                TextField::Machine => e.machine.clone().unwrap_or_default().to_lowercase(),
            };
            match field {
                TextField::Format => texts.iter().any(|t| v == t.trim_start_matches('.')),
                _ => texts.iter().any(|t| v.contains(t.as_str())),
            }
        }
        Filter::Range { field, min, max } => {
            let v = field.value(e);
            let lo = min.is_none_or(|(m, incl)| if incl { v >= m } else { v > m });
            let hi = max.is_none_or(|(m, incl)| if incl { v <= m } else { v < m });
            e.error.is_none() && lo && hi
        }
        Filter::Fits(hoop) => e.error.is_none() && e.stitches > 0 && fits(hoop, e.width_mm, e.height_mm),
        Filter::Error(want) => e.error.is_some() == *want,
        Filter::Nothing(_) => false,
    }
}

/// The order of a sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Name,
    Folder,
    /// The date of the last change.
    Date,
    /// The area (width × height).
    Size,
    Stitches,
    Colors,
    Format,
}

impl SortKey {
    pub const ALL: [SortKey; 7] = [
        SortKey::Name,
        SortKey::Folder,
        SortKey::Date,
        SortKey::Size,
        SortKey::Stitches,
        SortKey::Colors,
        SortKey::Format,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => "Name",
            SortKey::Folder => "Folder",
            SortKey::Date => "Date",
            SortKey::Size => "Size",
            SortKey::Stitches => "Stitches",
            SortKey::Colors => "Colors",
            SortKey::Format => "Format",
        }
    }

    /// The key for a command-line word (`name`, `date`, …).
    pub fn from_word(w: &str) -> Option<SortKey> {
        SortKey::ALL.into_iter().find(|k| k.label().eq_ignore_ascii_case(w))
    }
}

fn by_name(a: &Entry, b: &Entry) -> Ordering {
    a.name
        .to_lowercase()
        .cmp(&b.name.to_lowercase())
        .then_with(|| a.path.cmp(&b.path))
}

/// Sorts the entry indices `items` by `key` (a tie goes by name, then path).
pub fn sort(index: &Index, items: &mut [usize], key: SortKey, descending: bool) {
    let e = |i: usize| &index.entries[i];
    items.sort_by(|&a, &b| {
        let (a, b) = (e(a), e(b));
        let o = match key {
            SortKey::Name => Ordering::Equal,
            SortKey::Folder => a.folder.to_lowercase().cmp(&b.folder.to_lowercase()),
            SortKey::Date => a.modified.cmp(&b.modified),
            SortKey::Size => (a.width_mm * a.height_mm).total_cmp(&(b.width_mm * b.height_mm)),
            SortKey::Stitches => a.stitches.cmp(&b.stitches),
            SortKey::Colors => a.colors.cmp(&b.colors),
            SortKey::Format => a.format.cmp(&b.format),
        };
        let o = o.then_with(|| by_name(a, b));
        if descending { o.reverse() } else { o }
    });
}

/// How the found entries are grouped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupKey {
    #[default]
    None,
    Folder,
    Format,
    Colors,
    /// The smallest of the given hoops that the design fits.
    Hoop,
}

impl GroupKey {
    pub const ALL: [GroupKey; 5] = [
        GroupKey::None,
        GroupKey::Folder,
        GroupKey::Format,
        GroupKey::Colors,
        GroupKey::Hoop,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GroupKey::None => "No groups",
            GroupKey::Folder => "Folder",
            GroupKey::Format => "Format",
            GroupKey::Colors => "Colors",
            GroupKey::Hoop => "Smallest hoop",
        }
    }
}

/// A group of entries and its title.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub label: String,
    pub items: Vec<usize>,
}

/// Puts the (sorted) entry indices into groups. The groups are in order of
/// their label (hoops: smallest first); the items keep their order. For
/// [`GroupKey::Hoop`] the hoops are `hoops`.
pub fn group(index: &Index, items: &[usize], key: GroupKey, hoops: &[HoopPreset]) -> Vec<Group> {
    if key == GroupKey::None {
        return vec![Group {
            label: String::new(),
            items: items.to_vec(),
        }];
    }
    let mut by_area: Vec<&HoopPreset> = hoops.iter().filter(|h| h.positions.is_empty()).collect();
    by_area.sort_by(|a, b| (a.width_mm * a.height_mm).total_cmp(&(b.width_mm * b.height_mm)));
    // (order, label) for each item.
    let label = |e: &Entry| -> (usize, String) {
        if e.error.is_some() {
            return (usize::MAX, "Files that cannot be read".into());
        }
        match key {
            GroupKey::Folder => {
                let root = index
                    .roots
                    .get(e.root)
                    .and_then(|r| r.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let l = if e.folder.is_empty() {
                    root
                } else {
                    format!("{root}/{}", e.folder)
                };
                (0, l)
            }
            GroupKey::Format => (0, e.format.to_uppercase()),
            GroupKey::Colors => (
                e.colors,
                if e.colors == 1 {
                    "1 color".into()
                } else {
                    format!("{} colors", e.colors)
                },
            ),
            GroupKey::Hoop => match by_area.iter().position(|h| fits(h, e.width_mm, e.height_mm)) {
                Some(k) => (k, by_area[k].name.clone()),
                None => (usize::MAX - 1, "Larger than the hoops".into()),
            },
            GroupKey::None => unreachable!(),
        }
    };
    let mut groups: Vec<(usize, Group)> = Vec::new();
    for &i in items {
        let (order, l) = label(&index.entries[i]);
        match groups.iter_mut().find(|(_, g)| g.label == l) {
            Some((_, g)) => g.items.push(i),
            None => groups.push((
                order,
                Group {
                    label: l,
                    items: vec![i],
                },
            )),
        }
    }
    groups.sort_by(|(oa, a), (ob, b)| {
        oa.cmp(ob)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    groups.into_iter().map(|(_, g)| g).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::index::EntryThread;
    use emb_model::Rgb;

    pub(crate) fn entry(name: &str, folder: &str, format: &str, w: f64, h: f64, colors: usize) -> Entry {
        Entry {
            path: PathBuf::from(format!("/lib/{folder}/{name}.{format}")),
            root: 0,
            folder: folder.into(),
            file: format!("{name}.{format}"),
            name: name.into(),
            format: format.into(),
            bytes: 2048,
            modified: 1_700_000_000,
            width_mm: w,
            height_mm: h,
            stitches: (w * h) as usize,
            colors,
            trims: colors,
            threads: vec![EntryThread {
                name: "Navy".into(),
                color: Rgb(0, 0, 100),
            }],
            machine: Some("husqvarna_topaz20".into()),
            notes: String::new(),
            thumb: None,
            error: None,
        }
    }

    fn sample() -> Index {
        let mut broken = entry("broken", "old", "vp3", 0.0, 0.0, 0);
        broken.error = Some("bad file".into());
        let mut rose = entry("Rose", "flowers", "vp3", 90.0, 80.0, 4);
        rose.notes = "For the spring show".into();
        Index {
            version: crate::INDEX_VERSION,
            roots: vec![PathBuf::from("/lib")],
            entries: vec![
                rose,
                entry("Tulip", "flowers", "dst", 60.0, 120.0, 2),
                entry("Team logo", "caps/2026", "pes", 110.0, 50.0, 3),
                entry("Big eagle", "jackets", "vp3", 240.0, 180.0, 6),
                broken,
            ],
        }
    }

    fn names(index: &Index, items: &[usize]) -> Vec<String> {
        items.iter().map(|&i| index.entries[i].name.clone()).collect()
    }

    fn find(text: &str) -> Vec<String> {
        let idx = sample();
        let q = Query::parse(text, &HoopCatalog::builtin());
        names(&idx, &q.run(&idx))
    }

    #[test]
    fn words_and_filters() {
        assert_eq!(find("").len(), 5);
        assert_eq!(find("flowers"), ["Rose", "Tulip"]);
        assert_eq!(find("SPRING"), ["Rose"], "notes, any case");
        assert_eq!(find("navy tulip"), ["Tulip"], "thread names; all words");
        assert_eq!(find("\"team logo\""), ["Team logo"]);
        assert_eq!(find("format:vp3"), ["Rose", "Big eagle", "broken"]);
        assert_eq!(find("format:dst,pes"), ["Tulip", "Team logo"]);
        assert_eq!(find("folder:caps"), ["Team logo"]);
        assert_eq!(find("colors:<4"), ["Tulip", "Team logo"]);
        assert_eq!(find("colors:<=4"), ["Rose", "Tulip", "Team logo"]);
        assert_eq!(find("colors:3-6 width:>100"), ["Team logo", "Big eagle"]);
        assert_eq!(find("colors:6"), ["Big eagle"]);
        assert_eq!(find("error:yes"), ["broken"]);
        assert_eq!(find("error:no").len(), 4);
        // The Topaz crown hoop: 200 × 260, inside a 5 mm margin, turned too.
        assert_eq!(find("fits:topaz20_crown"), ["Rose", "Tulip", "Team logo", "Big eagle"]);
        // The square hoop (120 × 120): 110 × 110 inside the margin.
        assert_eq!(find("fits:splendid"), ["Rose", "Team logo"]);
        // An unknown key is a word; a filter with no number finds nothing.
        assert!(find("size:big").is_empty());
        let q = Query::parse("colors:many fits:nohoop", &HoopCatalog::builtin());
        assert_eq!(q.problems().len(), 2);
        assert!(q.run(&sample()).is_empty());
    }

    #[test]
    fn sort_and_group() {
        let idx = sample();
        let mut all: Vec<usize> = (0..idx.entries.len()).collect();
        sort(&idx, &mut all, SortKey::Name, false);
        assert_eq!(names(&idx, &all), ["Big eagle", "broken", "Rose", "Team logo", "Tulip"]);
        sort(&idx, &mut all, SortKey::Size, true);
        assert_eq!(names(&idx, &all)[0], "Big eagle");
        assert_eq!(names(&idx, &all)[4], "broken");
        sort(&idx, &mut all, SortKey::Colors, false);
        let g = group(&idx, &all, GroupKey::Colors, &[]);
        let labels: Vec<&str> = g.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "2 colors",
                "3 colors",
                "4 colors",
                "6 colors",
                "Files that cannot be read"
            ]
        );
        let g = group(&idx, &all, GroupKey::Folder, &[]);
        assert_eq!(g[0].label, "lib/caps/2026");
        assert_eq!(names(&idx, &g[1].items), ["Tulip", "Rose"]);
        // Hoops of the Topaz 20: the square hoop first, then the crown hoop.
        let cat = HoopCatalog::builtin();
        let topaz: Vec<HoopPreset> = ["topaz20_crown", "topaz20_splendid_square"]
            .iter()
            .map(|id| cat.get(id).unwrap().clone())
            .collect();
        sort(&idx, &mut all, SortKey::Name, false);
        let g = group(&idx, &all, GroupKey::Hoop, &topaz);
        let labels: Vec<&str> = g.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels.len(), 3, "{labels:?}");
        assert!(labels[0].contains("Splendid") && labels[1].contains("Crown"));
        assert_eq!(names(&idx, &g[0].items), ["Rose", "Team logo"]);
        assert_eq!(names(&idx, &g[1].items), ["Big eagle", "Tulip"]);
    }
}

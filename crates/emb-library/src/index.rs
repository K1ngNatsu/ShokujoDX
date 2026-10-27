//! The library index: an entry for each design file in the library
//! folders, and the scan that keeps it up to date.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use emb_model::{MachineCatalog, Rgb, StitchDesign, StitchKind, project};
use emb_stitch::{PlanOptions, StitchCache, build_plan};
use serde::{Deserialize, Serialize};

/// The version of the index file. A new version (a change of the entry or
/// of the stitch engine) makes the next scan read all files again.
pub const INDEX_VERSION: u32 = 3;

/// The longest side of a thumbnail, in pixels.
pub const THUMB_PX: f64 = 160.0;

/// A thread of a design, as the index keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryThread {
    pub name: String,
    pub color: Rgb,
}

/// The index entry of one design file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub path: PathBuf,
    /// The library folder of the file (an index into `Index::roots`).
    pub root: usize,
    /// The folder of the file relative to its library folder, with `/`
    /// between the parts; empty in the library folder itself.
    pub folder: String,
    /// The file name with its extension.
    pub file: String,
    /// The design name: the name in a project file, else the file name
    /// without its extension.
    pub name: String,
    /// The lower-case file extension.
    pub format: String,
    pub bytes: u64,
    /// The time of the last change of the file, in seconds from 1970.
    pub modified: u64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub stitches: usize,
    pub colors: usize,
    pub trims: usize,
    pub threads: Vec<EntryThread>,
    pub machine: Option<String>,
    pub notes: String,
    /// The file name of the thumbnail in the thumbnail folder.
    pub thumb: Option<String>,
    /// Why the file could not be read.
    pub error: Option<String>,
}

/// The index of a library.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    pub roots: Vec<PathBuf>,
    pub entries: Vec<Entry>,
}

impl Index {
    /// Reads an index file. A missing, damaged or old file gives an empty
    /// index (the next scan reads all files).
    pub fn load(path: &Path) -> Index {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Index>(&b).ok())
            .filter(|i| i.version == INDEX_VERSION)
            .unwrap_or_default()
    }

    /// Writes the index file (and its folder).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        std::fs::write(path, bytes)
    }

    /// The entry of a file, if the index has it.
    pub fn entry(&self, path: &Path) -> Option<&Entry> {
        self.entries.iter().find(|e| e.path == path)
    }
}

/// A design file that a scan found.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanFile {
    pub root: usize,
    pub path: PathBuf,
    pub bytes: u64,
    pub modified: u64,
}

/// What a scan did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanReport {
    /// Files read (new or changed).
    pub read: usize,
    /// Entries kept from the old index (the file did not change).
    pub kept: usize,
    /// Entries removed (the file is gone).
    pub removed: usize,
    /// Files that could not be read.
    pub failed: usize,
    /// The scan stopped before the end.
    pub stopped: bool,
}

/// The design files in the library folders and their subfolders, sorted
/// by path. Folders whose name starts with `.` are left out.
pub fn find_files(roots: &[PathBuf]) -> Vec<ScanFile> {
    let mut out = Vec::new();
    for (root, dir) in roots.iter().enumerate() {
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            let Ok(items) = std::fs::read_dir(&d) else { continue };
            for item in items.flatten() {
                let path = item.path();
                let Ok(meta) = item.metadata() else { continue };
                let hidden = item.file_name().to_string_lossy().starts_with('.');
                if meta.is_dir() {
                    if !hidden {
                        stack.push(path);
                    }
                } else if meta.is_file() && crate::is_design_file(&extension(&path)) {
                    let modified = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs());
                    out.push(ScanFile {
                        root,
                        path,
                        bytes: meta.len(),
                        modified,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Brings `old` up to date with the files in `roots`. Unchanged files keep
/// their entries; new and changed files are read (see [`read_entry`]).
/// Thumbnails go to `thumbs` (none if `None`); the thumbnails of removed
/// and changed entries are deleted. `progress(done, total)` is called
/// before each file is read; when it returns `false` the scan stops and
/// the files not read yet keep their old entries.
pub fn scan(
    roots: &[PathBuf],
    old: &Index,
    thumbs: Option<&Path>,
    mut progress: impl FnMut(usize, usize) -> bool,
) -> (Index, ScanReport) {
    let files = find_files(roots);
    let same_roots = old.roots == roots;
    let mut report = ScanReport::default();
    let mut entries = Vec::with_capacity(files.len());
    let total = files.len();
    for (k, f) in files.iter().enumerate() {
        let prev = same_roots.then(|| old.entry(&f.path)).flatten();
        if let Some(p) = prev.filter(|p| p.bytes == f.bytes && p.modified == f.modified) {
            entries.push(p.clone());
            report.kept += 1;
            continue;
        }
        if report.stopped || !progress(k, total) {
            report.stopped = true;
            if let Some(p) = prev {
                entries.push(p.clone());
            }
            continue;
        }
        if let (Some(p), Some(dir)) = (prev, thumbs) {
            remove_thumb(dir, p);
        }
        let e = read_entry(roots, f, thumbs);
        report.read += 1;
        if e.error.is_some() {
            report.failed += 1;
        }
        entries.push(e);
    }
    for gone in old.entries.iter().filter(|o| !entries.iter().any(|e| e.path == o.path)) {
        report.removed += 1;
        if let Some(dir) = thumbs {
            remove_thumb(dir, gone);
        }
    }
    let index = Index {
        version: INDEX_VERSION,
        roots: roots.to_vec(),
        entries,
    };
    (index, report)
}

fn remove_thumb(dir: &Path, e: &Entry) {
    if let Some(t) = &e.thumb {
        let _ = std::fs::remove_file(dir.join(t));
    }
}

/// Reads one design file and makes its entry (and its thumbnail in
/// `thumbs`). A project file gets its stitch plan with the machine of the
/// project. A file that cannot be read gets an entry with the error.
pub fn read_entry(roots: &[PathBuf], f: &ScanFile, thumbs: Option<&Path>) -> Entry {
    let format = extension(&f.path);
    let file = f
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = f
        .path
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = roots
        .get(f.root)
        .and_then(|r| f.path.parent()?.strip_prefix(r).ok())
        .map(|rel| {
            rel.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default();
    let mut e = Entry {
        path: f.path.clone(),
        root: f.root,
        folder,
        file,
        name: stem.clone(),
        format: format.clone(),
        bytes: f.bytes,
        modified: f.modified,
        width_mm: 0.0,
        height_mm: 0.0,
        stitches: 0,
        colors: 0,
        trims: 0,
        threads: Vec::new(),
        machine: None,
        notes: String::new(),
        thumb: None,
        error: None,
    };
    let (stitches, design) = match load_stitches(&f.path, &format) {
        Ok((s, design)) => {
            if let Some(d) = &design {
                if !d.name.trim().is_empty() {
                    e.name = d.name.clone();
                }
                e.machine = d.machine.clone();
                e.notes = d.notes.clone();
            }
            (s, design)
        }
        Err(err) => {
            e.error = Some(err);
            return e;
        }
    };
    // Some extensions (.dat, .new, .max, …) are also used for other files:
    // a machine file with no stitches is not a design.
    if format != project::FILE_EXTENSION && stitches.stitch_count() == 0 {
        e.error = Some("No stitches: not an embroidery design".into());
        return e;
    }
    // The areas of the picture: the appliqué fabric and the rhinestones.
    let areas = design
        .as_ref()
        .map(emb_render::sheet::picture_areas)
        .unwrap_or_default();
    let sewn = emb_geom::BBox::from_points(
        stitches
            .records
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .map(|r| r.pos)
            .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied())),
    );
    if let Some(b) = sewn {
        e.width_mm = b.width();
        e.height_mm = b.height();
    }
    e.stitches = stitches.stitch_count();
    e.colors = stitches.color_count();
    e.trims = stitches.trim_count();
    e.threads = stitches
        .threads
        .iter()
        .map(|t| EntryThread {
            name: t.name.clone(),
            color: t.color,
        })
        .collect();
    if let Some(dir) = thumbs {
        let name = format!("{:016x}-{}.png", fnv(&f.path.to_string_lossy()), f.modified);
        let png = thumbnail_with(&stitches, &areas);
        if std::fs::create_dir_all(dir).is_ok() && std::fs::write(dir.join(&name), png).is_ok() {
            e.thumb = Some(name);
        }
    }
    e
}

/// The stitches of a design file (the stitch plan of a project file).
pub fn read_stitches(path: &Path) -> Result<StitchDesign, String> {
    load_stitches(path, &extension(path)).map(|(s, _)| s)
}

/// The stitches of a design file, and the design of a project file.
fn load_stitches(path: &Path, ext: &str) -> Result<(StitchDesign, Option<emb_model::Design>), String> {
    if ext == project::FILE_EXTENSION {
        let d = project::load(path).map_err(|e| e.to_string())?;
        let machines = MachineCatalog::builtin();
        let opts = d
            .machine
            .as_deref()
            .and_then(|id| machines.get(id))
            .map_or_else(PlanOptions::default, PlanOptions::for_machine);
        let plan = build_plan(&d, &opts, &mut StitchCache::default());
        return Ok((plan.stitches, Some(d)));
    }
    read_machine_file(path).map(|s| (s, None))
}

/// Reads a machine file. A format with no colors (DST, EXP, …) gets the
/// colors of a thread color file of the same name next to it (`.inf`,
/// `.edr` or `.col`), if there is one.
pub fn read_machine_file(path: &Path) -> Result<StitchDesign, String> {
    let ext = extension(path);
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut s = emb_formats::read_machine(&ext, &bytes)?;
    if s.threads.is_empty() {
        for c in emb_formats::colors::EXTENSIONS {
            let side = path.with_extension(c);
            if let Some(threads) = std::fs::read(&side).ok().and_then(|b| emb_formats::colors::read(c, &b))
                && !threads.is_empty()
            {
                s.threads = threads;
                break;
            }
        }
    }
    Ok(s)
}

/// A PNG picture of the stitches, at most [`THUMB_PX`] on its longest side
/// (transparent background).
pub fn thumbnail(s: &StitchDesign) -> Vec<u8> {
    thumbnail_with(s, &[])
}

/// As [`thumbnail`], with areas under the stitches (the appliqué fabric
/// and the rhinestones of a project, `emb_render::sheet::picture_areas`).
pub fn thumbnail_with(s: &StitchDesign, areas: &emb_render::raster::Areas) -> Vec<u8> {
    let segments = emb_render::segments(s);
    let b = s.extents().into_iter().chain(emb_geom::BBox::from_points(
        areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied()),
    ));
    let size = b
        .reduce(emb_geom::BBox::union)
        .map_or(1.0, |b| b.width().max(b.height()))
        .max(1.0);
    let margin = 1.0;
    let px_per_mm = THUMB_PX / (size + 2.0 * margin);
    // Thin threads at a small scale still show the colors.
    let thread = (1.2 / px_per_mm).max(0.4);
    emb_render::raster::picture(&segments, areas, px_per_mm, thread, margin, None).png()
}

fn extension(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// FNV-1a: a short, stable name for a thumbnail.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use emb_geom::Point;
    use emb_model::{StitchRecord, Thread, ThreadId};

    #[test]
    fn a_project_of_stones_has_a_picture_of_the_stones() {
        let dir = std::env::temp_dir().join(format!("emb-lib-stones-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut d = emb_model::Design::new("stones");
        let t = d.add_thread("Stones", Rgb(0, 0, 0));
        d.add_object(
            t,
            emb_model::ObjectKind::Bling(Box::new(emb_model::Bling::new(
                emb_model::Stone::default(),
                emb_model::BlingLayout::Stones {
                    points: vec![Point::new(0.0, 0.0), Point::new(20.0, 0.0)],
                },
            ))),
        );
        let path = dir.join("stones.embproj");
        project::save(&d, &path).unwrap();
        let f = ScanFile {
            root: 0,
            path: path.clone(),
            bytes: 1,
            modified: 1,
        };
        let thumbs = dir.join("thumbs");
        let e = read_entry(std::slice::from_ref(&dir), &f, Some(&thumbs));
        assert!(e.error.is_none(), "{:?}", e.error);
        assert!(e.width_mm > 20.0, "the size has the stones: {}", e.width_mm);
        let png = std::fs::read(thumbs.join(e.thumb.unwrap())).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert!(img.pixels().any(|p| p[3] > 200), "the stones show");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A temporary folder for one test.
    pub(crate) fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-library-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A square of `size` mm in `colors` colors, as a stitch design.
    pub(crate) fn square(size: f64, colors: usize) -> StitchDesign {
        let mut s = StitchDesign::default();
        for c in 0..colors {
            s.threads.push(Thread {
                id: ThreadId(c as u32),
                name: format!("Color {c}"),
                color: Rgb(40 * c as u8, 100, 200),
                brand: None,
                code: None,
            });
            if c > 0 {
                s.records.push(StitchRecord::new(Point::ZERO, StitchKind::ColorChange));
            }
            let h = size / 2.0;
            for p in [(-h, -h), (h, -h), (h, h), (-h, h), (-h, -h)] {
                s.records
                    .push(StitchRecord::new(Point::new(p.0, p.1), StitchKind::Stitch));
            }
        }
        s.records.push(StitchRecord::new(Point::ZERO, StitchKind::End));
        s
    }

    pub(crate) fn write_dst(path: &Path, s: &StitchDesign) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            emb_formats::write_machine("dst", s, "t", "20260101000000").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn a_scan_reads_designs_in_subfolders_and_keeps_unchanged_ones() {
        let root = tmp("scan");
        let thumbs = root.join(".thumbs");
        write_dst(&root.join("a.dst"), &square(50.0, 2));
        write_dst(&root.join("caps/b.dst"), &square(30.0, 1));
        std::fs::write(root.join("notes.txt"), "not a design").unwrap();
        std::fs::write(root.join("broken.vp3"), "not a VP3 file").unwrap();
        // A .dat file that is not a design (the extension is used by other programs).
        std::fs::write(root.join("data.dat"), [0u8; 400]).unwrap();
        let mut d = emb_model::Design::new("Patch name");
        d.notes = "for the club".into();
        let t = d.add_thread("Red", Rgb(200, 0, 0));
        d.add_object(
            t,
            emb_model::ObjectKind::Running {
                path: emb_geom::Path::new()
                    .move_to(Point::new(0.0, 0.0))
                    .line_to(Point::new(40.0, 0.0)),
                params: Default::default(),
            },
        );
        project::save(&d, &root.join("caps/patch.embproj")).unwrap();

        let roots = vec![root.clone()];
        let (index, report) = scan(&roots, &Index::default(), Some(&thumbs), |_, _| true);
        assert_eq!(index.entries.len(), 5, "{:?}", index.entries);
        assert_eq!((report.read, report.kept, report.failed), (5, 0, 2));
        assert!(
            index
                .entry(&root.join("data.dat"))
                .unwrap()
                .error
                .as_deref()
                .unwrap()
                .contains("No stitches")
        );
        let a = index.entry(&root.join("a.dst")).unwrap();
        assert_eq!((a.folder.as_str(), a.colors, a.format.as_str()), ("", 2, "dst"));
        assert!(
            (a.width_mm - 50.0).abs() < 0.2 && (a.height_mm - 50.0).abs() < 0.2,
            "{a:?}"
        );
        assert!(thumbs.join(a.thumb.as_ref().unwrap()).is_file());
        let p = index.entry(&root.join("caps/patch.embproj")).unwrap();
        assert_eq!((p.name.as_str(), p.folder.as_str()), ("Patch name", "caps"));
        assert_eq!(p.notes, "for the club");
        assert!(p.stitches > 10 && (p.width_mm - 40.0).abs() < 1.0, "{p:?}");
        assert!(index.entry(&root.join("broken.vp3")).unwrap().error.is_some());

        // Saved and read back; a second scan reads nothing again.
        let file = root.join(".index/index.json");
        index.save(&file).unwrap();
        let back = Index::load(&file);
        assert_eq!(back, index);
        let (again, report) = scan(&roots, &back, Some(&thumbs), |_, _| true);
        assert_eq!((report.read, report.kept), (0, 5));
        assert_eq!(again, index);

        // A changed file is read again (new thumbnail); a removed file goes.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        write_dst(&root.join("a.dst"), &square(80.0, 3));
        std::fs::remove_file(root.join("caps/b.dst")).unwrap();
        let old_thumb = thumbs.join(a.thumb.as_ref().unwrap());
        let (after, report) = scan(&roots, &again, Some(&thumbs), |_, _| true);
        assert_eq!((report.read, report.kept, report.removed), (1, 3, 1), "{report:?}");
        let a2 = after.entry(&root.join("a.dst")).unwrap();
        assert_eq!(a2.colors, 3);
        assert!(!old_thumb.is_file(), "the old thumbnail is deleted");
        assert!(thumbs.join(a2.thumb.as_ref().unwrap()).is_file());
        let pngs = std::fs::read_dir(&thumbs).unwrap().count();
        assert_eq!(pngs, 2, "a.dst and the project; the broken file has none");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_stopped_scan_keeps_the_old_entries() {
        let root = tmp("stop");
        for k in 0..4 {
            write_dst(&root.join(format!("d{k}.dst")), &square(10.0 + k as f64, 1));
        }
        let roots = vec![root.clone()];
        let (index, report) = scan(&roots, &Index::default(), None, |done, _| done < 2);
        assert!(report.stopped);
        assert_eq!(index.entries.len(), 2);
        let (full, report) = scan(&roots, &index, None, |_, _| true);
        assert_eq!((report.read, report.kept), (2, 2));
        assert_eq!(full.entries.len(), 4);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_old_or_damaged_index_file_is_empty() {
        let root = tmp("old");
        let f = root.join("index.json");
        std::fs::write(&f, "{ not json").unwrap();
        assert_eq!(Index::load(&f), Index::default());
        let old = Index {
            version: INDEX_VERSION + 1,
            ..Index::default()
        };
        old.save(&f).unwrap();
        assert!(Index::load(&f).entries.is_empty() && Index::load(&f).version == 0);
        std::fs::remove_dir_all(&root).ok();
    }
}

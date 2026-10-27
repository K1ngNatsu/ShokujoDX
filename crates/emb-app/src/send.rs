//! Send to USB (P8 Milestone 3, ADR 0036): the design goes in the format
//! of the machine profile to the USB stick of the machine, in one action.
//!
//! The first send asks for the stick (a list of the removable drives) and
//! a folder on it; the program keeps them for the machine. The next sends
//! go there at once while the stick is in. Each file is read back after it
//! is written, to check it.

use std::io::Write;
use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// A removable drive.
#[derive(Debug, Clone, PartialEq)]
pub struct Drive {
    /// Where the drive is (`E:\` on Windows).
    pub mount: PathBuf,
    /// The name of the drive (the volume label); can be empty.
    pub label: String,
    /// Free space in bytes.
    pub free: u64,
    pub total: u64,
}

impl Drive {
    /// "KINGSTON (E:\) · 14.2 GB free".
    pub fn title(&self) -> String {
        let name = if self.label.is_empty() {
            "USB drive".to_string()
        } else {
            self.label.clone()
        };
        format!("{name} ({}) · {} free", self.mount.display(), size_text(self.free))
    }
}

/// A byte count as "14.2 GB" or "512 KB".
pub fn size_text(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1e9 {
        format!("{:.1} GB", b / 1e9)
    } else if b >= 1e6 {
        format!("{:.1} MB", b / 1e6)
    } else {
        format!("{:.0} KB", (b / 1e3).ceil())
    }
}

/// The removable drives that can be written.
pub fn removable_drives() -> Vec<Drive> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut out: Vec<Drive> = disks
        .list()
        .iter()
        .filter(|d| d.is_removable() && !d.is_read_only())
        .map(|d| Drive {
            mount: d.mount_point().to_path_buf(),
            label: d.name().to_string_lossy().trim().to_string(),
            free: d.available_space(),
            total: d.total_space(),
        })
        .collect();
    out.sort_by(|a, b| a.mount.cmp(&b.mount));
    out.dedup_by(|a, b| a.mount == b.mount);
    out
}

/// Where the designs of a machine go: the stick (by its name, else by its
/// place) and a folder on it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SendTarget {
    pub label: String,
    pub mount: PathBuf,
    /// The folder on the stick, with `/` or `\` between its parts; empty:
    /// the top folder.
    pub folder: String,
}

impl SendTarget {
    /// The drive of the target among `drives`: the one with the same name,
    /// else the one at the same place.
    pub fn find<'a>(&self, drives: &'a [Drive]) -> Option<&'a Drive> {
        let by_label = (!self.label.is_empty())
            .then(|| drives.iter().find(|d| d.label == self.label))
            .flatten();
        by_label.or_else(|| {
            drives
                .iter()
                .find(|d| d.mount == self.mount && (self.label.is_empty() || d.label.is_empty()))
        })
    }

    pub fn dir(&self, drive: &Drive) -> PathBuf {
        let mut p = drive.mount.clone();
        for part in self.folder.split(['/', '\\']).filter(|s| !s.trim().is_empty()) {
            p.push(part.trim());
        }
        p
    }
}

/// A file to send, and what the check after the write must find in it.
#[derive(Debug, Clone, PartialEq)]
pub struct SendFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub stitches: usize,
    pub colors: usize,
}

impl SendFile {
    /// The machine file of `stitches` in the format `ext`, with the needles
    /// of a multi-needle machine (ADR 0063). The numbers for the check come
    /// from the file itself (a format can split long stitches).
    pub fn of(
        stitches: &emb_model::StitchDesign,
        name: &str,
        ext: &str,
        machine: Option<&emb_model::MachineProfile>,
        needles: Option<&emb_model::NeedlePlan>,
    ) -> Result<SendFile, String> {
        let label = name.trim_end_matches(&format!(".{ext}")[..]).to_string();
        let stamp = crate::output::local_stamp();
        let bytes = emb_formats::write_machine_with(ext, stitches, &label, &stamp, machine, needles)
            .ok_or_else(|| format!("The program cannot write .{ext} files."))?;
        let s = emb_formats::read_machine(ext, &bytes)?;
        Ok(SendFile {
            name: name.to_string(),
            bytes,
            stitches: s.stitch_count(),
            colors: s.color_count(),
        })
    }
}

/// A file name with no characters that Windows or the machine cannot use.
pub fn file_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_end_matches('.').to_string();
    if clean.is_empty() { "design".into() } else { clean }
}

/// What a send did.
#[derive(Debug, Clone, PartialEq)]
pub struct Sent {
    pub path: PathBuf,
    /// A file of the same name was there before.
    pub replaced: bool,
}

/// Writes the files into `dir` and reads each one back. The data goes to
/// the stick before the function returns (so the user can take it out
/// after Eject). An error if the space is too small, a write fails, or a
/// file does not read back with the same stitches and colors.
pub fn write_and_check(dir: &Path, files: &[SendFile], free: Option<u64>, ext: &str) -> Result<Vec<Sent>, String> {
    let need: u64 = files.iter().map(|f| f.bytes.len() as u64).sum();
    if let Some(free) = free
        && need > free
    {
        return Err(format!(
            "The stick has {} free; the files need {}.",
            size_text(free),
            size_text(need)
        ));
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut out = Vec::new();
    for f in files {
        let path = dir.join(&f.name);
        let replaced = path.exists();
        let write = || -> std::io::Result<()> {
            let mut file = std::fs::File::create(&path)?;
            file.write_all(&f.bytes)?;
            file.sync_all()
        };
        write().map_err(|e| format!("{}: {e}", path.display()))?;
        let back = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = emb_formats::read_machine(ext, &back).map_err(|e| format!("{}: {e}", path.display()))?;
        if back != f.bytes || s.stitch_count() != f.stitches || s.color_count() != f.colors {
            return Err(format!(
                "{} does not read back as it was written ({} stitches, {} colors; {} and {} were sent). \
                 The stick can be full or damaged.",
                path.display(),
                s.stitch_count(),
                s.color_count(),
                f.stitches,
                f.colors
            ));
        }
        out.push(Sent { path, replaced });
    }
    Ok(out)
}

/// The Send to USB window.
#[derive(Debug, Clone, Default)]
pub struct SendDialog {
    pub drives: Vec<Drive>,
    pub chosen: Option<usize>,
    pub folder: String,
    /// Files from the library (`None`: the open design).
    pub library: Option<Vec<PathBuf>>,
    pub message: String,
}

impl AppState {
    /// The removable drives (in tests: the drives that the test gives).
    pub fn drives(&self) -> Vec<Drive> {
        #[cfg(test)]
        {
            self.test_drives.clone()
        }
        #[cfg(not(test))]
        {
            removable_drives()
        }
    }

    /// The machine profile id of the open design ("" if none).
    fn send_key(&self) -> String {
        self.doc.design().machine.clone().unwrap_or_default()
    }

    /// The format of the machine of the open design (DST if it has none).
    pub fn send_format(&self) -> String {
        self.doc
            .design()
            .machine
            .as_deref()
            .and_then(|m| self.machines.get(m))
            .map(|m| m.default_format.clone())
            .filter(|f| emb_formats::WRITE_EXTENSIONS.contains(&f.as_str()))
            .unwrap_or_else(|| "dst".into())
    }

    /// The file name of the open design, with no extension.
    fn send_name(&self) -> String {
        let stem = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned());
        file_name(&stem.unwrap_or_else(|| self.doc.design().name.clone()))
    }

    /// The machine files of the open design: one file, or one for each
    /// hooping. An error if the design cannot sew (as for Export).
    pub fn design_send_files(&mut self) -> Result<Vec<SendFile>, String> {
        let ext = self.send_format();
        let name = self.send_name();
        let machine = self.machine_for_files(self.doc.design().machine.as_deref());
        let hoopings = self.doc.design().hooping.as_ref().is_some_and(|l| l.hoopings.len() > 1);
        if hoopings {
            let views = self.hooping_views();
            return views
                .iter()
                .enumerate()
                .map(|(k, v)| {
                    let needles = self.catalogs().needle_plan(&v.out.design, &v.plan.stitches);
                    SendFile::of(
                        &v.plan.stitches,
                        &format!("{name}_{}.{ext}", k + 1),
                        &ext,
                        machine.as_ref(),
                        needles.as_ref(),
                    )
                })
                .collect();
        }
        let report = self.export_report();
        if let Some(e) = report.errors.first() {
            return Err(e.text.clone());
        }
        let plan = self.plan_view().plan.clone();
        let needles = self.catalogs().needle_plan(self.doc.design(), &plan.stitches);
        Ok(vec![SendFile::of(
            &plan.stitches,
            &format!("{name}.{ext}"),
            &ext,
            machine.as_ref(),
            needles.as_ref(),
        )?])
    }

    /// The machine files of library designs, in the format of the open
    /// design's machine.
    pub fn library_send_files(&self, paths: &[PathBuf]) -> Result<Vec<SendFile>, String> {
        let ext = self.send_format();
        let machine = self.machine_for_files(self.doc.design().machine.as_deref());
        let mut names = std::collections::BTreeSet::new();
        paths
            .iter()
            .map(|p| {
                let s = emb_library::read_stitches(p).map_err(|e| format!("{}: {e}", p.display()))?;
                let stem = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut name = format!("{}.{ext}", file_name(&stem));
                // Two designs with one name (for example rose.dst and rose.pes).
                let mut k = 2;
                while !names.insert(name.to_lowercase()) {
                    name = format!("{} ({k}).{ext}", file_name(&stem));
                    k += 1;
                }
                // A library design has no pins: the needles of the open
                // design's machine.
                let needles = machine.as_ref().and_then(|m| {
                    let setup = self.settings.needles.get(&m.id);
                    emb_model::needles::plan_for(&emb_model::Design::new(""), &s, m, setup)
                });
                SendFile::of(&s, &name, &ext, machine.as_ref(), needles.as_ref())
            })
            .collect()
    }

    /// File > Send to USB: sends the open design to the stick of its
    /// machine, or opens the window to choose the stick.
    pub fn send_to_usb(&mut self) {
        self.send_now(None);
    }

    /// Sends the selected library designs.
    pub fn send_library_selection(&mut self) {
        let paths: Vec<PathBuf> = self
            .library
            .index
            .entries
            .iter()
            .filter(|e| self.library.is_selected(&e.path))
            .map(|e| e.path.clone())
            .collect();
        if !paths.is_empty() {
            self.send_now(Some(paths));
        }
    }

    /// Sends at once if the stick of the machine is in, else opens the window.
    fn send_now(&mut self, library: Option<Vec<PathBuf>>) {
        let drives = self.drives();
        let target = self.settings.send_targets.get(&self.send_key()).cloned();
        match target
            .as_ref()
            .and_then(|t| t.find(&drives).map(|d| (t.clone(), d.clone())))
        {
            Some((t, d)) => {
                self.send_files_to(&t, &d, library.as_deref());
            }
            None => self.open_send_dialog(library),
        }
    }

    /// Opens the Send to USB window (to choose the stick and the folder).
    pub fn open_send_dialog(&mut self, library: Option<Vec<PathBuf>>) {
        let drives = self.drives();
        let target = self
            .settings
            .send_targets
            .get(&self.send_key())
            .cloned()
            .unwrap_or_default();
        let chosen = target
            .find(&drives)
            .and_then(|d| drives.iter().position(|x| x == d))
            .or((drives.len() == 1).then_some(0));
        self.send_dialog = Some(SendDialog {
            folder: target.folder,
            drives,
            chosen,
            library,
            message: String::new(),
        });
    }

    /// Makes the files and queues the send to the drive (the target is
    /// kept for the machine when the send ends well). `true` if queued.
    pub fn send_files_to(&mut self, target: &SendTarget, drive: &Drive, library: Option<&[PathBuf]>) -> bool {
        let files = match library {
            Some(paths) => self.library_send_files(paths),
            None => self.design_send_files(),
        };
        let files = match files {
            Ok(f) if !f.is_empty() => f,
            Ok(_) => return false,
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot send the design".into(),
                    lines: vec![e],
                });
                return false;
            }
        };
        let ext = self.send_format();
        let dir = target.dir(drive);
        let keep = SendTarget {
            label: drive.label.clone(),
            mount: drive.mount.clone(),
            folder: target.folder.clone(),
        };
        let key = self.send_key();
        let what = match files.as_slice() {
            [one] => one.name.clone(),
            many => format!("{} files", many.len()),
        };
        let (d, free) = (dir.clone(), drive.free);
        let work: crate::jobs::Work = Box::new(move || {
            let sent = write_and_check(&d, &files, Some(free), &ext)?;
            let names: Vec<String> = sent
                .iter()
                .map(|x| {
                    let n = x
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if x.replaced { format!("{n} (replaced)") } else { n }
                })
                .collect();
            Ok(crate::jobs::JobOutput {
                message: format!(
                    "Sent {} to {} and read back. Eject the stick before you take it out.",
                    names.join(", "),
                    d.display()
                ),
                files: sent.into_iter().map(|x| x.path).collect(),
                lines: Vec::new(),
                then: Some(crate::jobs::Then::SendTarget(key, keep)),
            })
        });
        self.queue_job("Send to USB", &what, Some(dir.clone()), None, work);
        self.message = crate::i18n::fill("Sending {} to {}…", &[&what, &(dir.display())]);
        true
    }
}

/// The Send to USB window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.send_dialog.take() else {
        return;
    };
    let mut open = true;
    let mut send = false;
    let machine = st
        .doc
        .design()
        .machine
        .as_deref()
        .and_then(|m| st.machines.get(m))
        .map_or_else(|| "no machine".to_string(), |m| m.name.clone());
    let ext = st.send_format();
    egui::Window::new(tr("Send to USB"))
        .id(egui::Id::new("Send to USB"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            match &dlg.library {
                Some(p) => ui.label(crate::i18n::fill("{} designs from the library", &[&(p.len())])),
                None => ui.label(crate::i18n::fill("The design \"{}\"", &[&(st.doc.design().name)])),
            };
            ui.weak(crate::i18n::fill(
                "Format: {} (machine: {})",
                &[&(ext.to_uppercase()), &machine],
            ));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.strong(tr("USB stick"));
                if ui.small_button(tr("Look again")).clicked() {
                    dlg.drives = st.drives();
                    dlg.chosen = (dlg.drives.len() == 1).then_some(0);
                }
            });
            if dlg.drives.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "No USB stick. Put one in, then click Look again.",
                );
            }
            for (i, d) in dlg.drives.iter().enumerate() {
                ui.radio_value(&mut dlg.chosen, Some(i), d.title());
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(tr("Folder on the stick"));
                ui.add(
                    egui::TextEdit::singleline(&mut dlg.folder)
                        .hint_text(tr("(top folder)"))
                        .desired_width(180.0),
                );
            });
            ui.weak(tr(
                "The program keeps the stick and the folder for this machine: the next Send to USB goes there at once.",
            ));
            if !dlg.message.is_empty() {
                ui.colored_label(ui.visuals().warn_fg_color, &dlg.message);
            }
            ui.add_space(6.0);
            if ui
                .add_enabled(dlg.chosen.is_some(), egui::Button::new(tr("Send")))
                .clicked()
            {
                send = true;
            }
        });
    if send && let Some(d) = dlg.chosen.and_then(|i| dlg.drives.get(i)).cloned() {
        let target = SendTarget {
            label: d.label.clone(),
            mount: d.mount.clone(),
            folder: dlg.folder.trim().to_string(),
        };
        if st.send_files_to(&target, &d, dlg.library.as_deref()) {
            return;
        }
    }
    if open {
        st.send_dialog = Some(dlg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    /// A folder that stands in for a USB stick.
    fn stick(name: &str, label: &str) -> Drive {
        let mount = std::env::temp_dir().join(format!("emb-send-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mount);
        std::fs::create_dir_all(&mount).unwrap();
        Drive {
            mount,
            label: label.into(),
            free: 1 << 30,
            total: 1 << 31,
        }
    }

    fn topaz_harness() -> Harness {
        let mut h = Harness::new();
        h.st.doc
            .edit("Machine", |d| d.machine = Some("husqvarna_topaz20".into()));
        h
    }

    #[test]
    fn the_first_send_asks_and_the_next_one_goes_at_once() {
        let mut h = topaz_harness();
        let d = stick("first", "KINGSTON");
        h.st.test_drives = vec![d.clone()];
        h.st.send_to_usb();
        // No target yet: the window, with the only stick chosen.
        let dlg = h.st.send_dialog.clone().expect("the window opens");
        assert_eq!(dlg.chosen, Some(0));
        let target = SendTarget {
            label: d.label.clone(),
            mount: d.mount.clone(),
            folder: "Designs/Club".into(),
        };
        assert!(h.st.send_files_to(&target, &d, None));
        h.st.wait_jobs();
        let file = d.mount.join("Designs").join("Club").join("Untitled.vp3");
        assert!(file.is_file(), "{}", h.st.message);
        assert!(h.st.message.contains("read back"), "{}", h.st.message);
        let bytes = std::fs::read(&file).unwrap();
        assert!(emb_formats::vp3::read(&bytes).unwrap().stitch_count() > 100);
        // The stick has a new place (another drive letter): found by its name.
        h.st.send_dialog = None;
        let moved = Drive {
            mount: d.mount.join("Designs"),
            ..d.clone()
        };
        h.st.test_drives = vec![stick("other", "OTHER"), moved.clone()];
        h.st.send_to_usb();
        h.st.wait_jobs();
        assert!(h.st.send_dialog.is_none(), "one action: no window");
        assert!(moved.mount.join("Designs/Club/Untitled.vp3").is_file());
        assert!(h.st.message.contains("Untitled.vp3"), "{}", h.st.message);
        // Again to the same place: the file is replaced.
        h.st.send_to_usb();
        h.st.wait_jobs();
        assert!(h.st.message.contains("(replaced)"), "{}", h.st.message);
        // No stick: the window says so.
        h.st.test_drives = Vec::new();
        h.st.send_to_usb();
        assert!(h.st.send_dialog.as_ref().unwrap().drives.is_empty());
        std::fs::remove_dir_all(&d.mount).ok();
    }

    #[test]
    fn a_design_outside_the_hoop_is_not_sent() {
        let mut h = topaz_harness();
        h.st.doc.edit("Move", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                emb_model::ObjectKind::Running {
                    path: emb_geom::Path::new()
                        .move_to(emb_geom::Point::new(150.0, 0.0))
                        .line_to(emb_geom::Point::new(180.0, 0.0)),
                    params: Default::default(),
                },
            );
        });
        let d = stick("outside", "STICK");
        let target = SendTarget {
            label: d.label.clone(),
            mount: d.mount.clone(),
            folder: String::new(),
        };
        assert!(!h.st.send_files_to(&target, &d, None));
        assert!(h.st.notice.as_ref().unwrap().lines[0].contains("outside"));
        assert_eq!(std::fs::read_dir(&d.mount).unwrap().count(), 0);
        std::fs::remove_dir_all(&d.mount).ok();
    }

    /// The real list of the computer (it writes nothing): each drive is there.
    #[test]
    fn the_removable_drives_of_this_computer() {
        for d in removable_drives() {
            assert!(d.mount.exists(), "{d:?}");
            assert!(d.free <= d.total, "{d:?}");
        }
    }

    #[test]
    fn too_little_space_and_names() {
        let s = emb_model::StitchDesign::default();
        let f = SendFile::of(&s, "a.dst", "dst", None, None).unwrap();
        let dir = std::env::temp_dir().join(format!("emb-send-space-{}", std::process::id()));
        let e = write_and_check(&dir, &[f], Some(10), "dst").unwrap_err();
        assert!(e.contains("free"), "{e}");
        assert_eq!(file_name("Rose: \"big\"/v2."), "Rose_ _big__v2");
        assert_eq!(file_name("  "), "design");
        assert_eq!(size_text(14_200_000_000), "14.2 GB");
    }

    #[test]
    fn library_designs_go_in_the_machine_format() {
        let mut h = topaz_harness();
        let src = std::env::temp_dir().join(format!("emb-send-lib-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&src);
        std::fs::create_dir_all(&src).unwrap();
        let s = h.st.plan_view().plan.stitches.clone();
        for ext in ["dst", "pes"] {
            let b = emb_formats::write_machine(ext, &s, "rose", "20260101000000").unwrap();
            std::fs::write(src.join(format!("rose.{ext}")), b).unwrap();
        }
        let files =
            h.st.library_send_files(&[src.join("rose.dst"), src.join("rose.pes")])
                .unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["rose.vp3", "rose (2).vp3"]);
        assert!(
            files[0].stitches >= s.stitch_count(),
            "{} {}",
            files[0].stitches,
            s.stitch_count()
        );
        std::fs::remove_dir_all(&src).ok();
    }
}

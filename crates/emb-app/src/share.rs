//! Work with other programs (P10 Milestone 6):
//!
//! - Open a file or a folder with the program of the system.
//! - Edit the backdrop picture in another program: the program writes the
//!   picture to a file and opens it; when the file changes, the backdrop
//!   changes (same place and width).
//! - Send by email: the machine file, the project and a picture in one zip
//!   file next to the design; the mail program opens with a message, and
//!   the folder shows the zip file to attach (a mail link cannot attach it).
//! - Zip files (stored or deflated) for the email and the asset packages.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

/// Opens a file, a folder or a link with the program that the system uses for it.
pub fn open_with_system(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut c = {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("cmd");
        c.raw_arg(start_command_line(path.as_os_str()));
        c
    };
    #[cfg(not(windows))]
    let mut c = if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    c.spawn().map(|_| ())
}

/// The arguments of `cmd` that open `target` with the program of the system
/// (Windows). The target is in quotes, so that `&`, `^` and spaces stay part
/// of it: Rust puts no quotes round a name with no space, and `cmd` reads
/// `&` as the start of a second command (a file `a&b.png`, or the `&body=`
/// of a mail link). A Windows file name cannot have a `"` in it.
#[cfg(any(windows, test))]
pub(crate) fn start_command_line(target: &std::ffi::OsStr) -> std::ffi::OsString {
    let mut s = std::ffi::OsString::from("/C start \"\" \"");
    s.push(target);
    s.push("\"");
    s
}

/// Shows a file in its folder (selected where the system can do that).
pub fn show_in_folder(path: &Path) -> std::io::Result<()> {
    if cfg!(windows) {
        let mut arg = std::ffi::OsString::from("/select,");
        arg.push(path);
        std::process::Command::new("explorer").arg(arg).spawn().map(|_| ())
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map(|_| ())
    } else {
        open_with_system(path.parent().unwrap_or(path))
    }
}

/// A `mailto:` link with a subject and a text (percent-encoded).
pub fn mailto(subject: &str, body: &str) -> String {
    let enc = |s: &str| -> String {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    };
    format!("mailto:?subject={}&body={}", enc(subject), enc(body))
}

// ---- Zip files ----

/// A zip file of the files (name with `/` for folders, bytes), deflated.
pub fn zip(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(data).expect("writing to memory");
        let packed = enc.finish().expect("writing to memory");
        let crc = crc32fast::hash(data);
        let offset = out.len() as u32;
        let name_bytes = name.as_bytes();
        // Version 2.0, UTF-8 names, deflate, no date (1980-01-01).
        let head = |sig: u32, central: bool| -> Vec<u8> {
            let mut h = Vec::new();
            h.extend(sig.to_le_bytes());
            if central {
                h.extend(20u16.to_le_bytes());
            }
            h.extend(20u16.to_le_bytes());
            h.extend(0x0800u16.to_le_bytes());
            h.extend(8u16.to_le_bytes());
            h.extend(0u16.to_le_bytes());
            h.extend(0x0021u16.to_le_bytes());
            h.extend(crc.to_le_bytes());
            h.extend((packed.len() as u32).to_le_bytes());
            h.extend((data.len() as u32).to_le_bytes());
            h.extend((name_bytes.len() as u16).to_le_bytes());
            h.extend(0u16.to_le_bytes());
            if central {
                // Comment, disk, internal and external attributes, offset.
                h.extend(0u16.to_le_bytes());
                h.extend(0u16.to_le_bytes());
                h.extend(0u16.to_le_bytes());
                h.extend(0u32.to_le_bytes());
                h.extend(offset.to_le_bytes());
            }
            h.extend(name_bytes);
            h
        };
        out.extend(head(0x0403_4b50, false));
        out.extend(&packed);
        central.extend(head(0x0201_4b50, true));
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend(central);
    out.extend(0x0605_4b50u32.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend(cd_size.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out
}

/// The largest file in a zip file that the program reads (a damaged or a
/// wrong zip file can say that a file has 4 GB).
const MAX_UNZIPPED: usize = 256 << 20;

/// The files of a zip file (stored or deflated entries; folders are left out).
pub fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let bad = || "not a zip file, or a damaged one".to_string();
    let u16_at = |p: usize| bytes.get(p..p + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |p: usize| {
        bytes
            .get(p..p + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    // The end record is in the last 64 kB (after it, only a comment).
    let from = bytes.len().saturating_sub(22 + 65_535);
    let end = (from..=bytes.len().saturating_sub(22))
        .rev()
        .find(|&p| u32_at(p) == Some(0x0605_4b50))
        .ok_or_else(bad)?;
    let count = u16_at(end + 10).ok_or_else(bad)?;
    let mut p = u32_at(end + 16).ok_or_else(bad)?;
    let mut out = Vec::new();
    for _ in 0..count {
        if u32_at(p) != Some(0x0201_4b50) {
            return Err(bad());
        }
        let method = u16_at(p + 10).ok_or_else(bad)?;
        let crc = u32_at(p + 16).ok_or_else(bad)? as u32;
        let packed = u32_at(p + 20).ok_or_else(bad)?;
        let size = u32_at(p + 24).ok_or_else(bad)?;
        let (n, x, c) = (
            u16_at(p + 28).ok_or_else(bad)?,
            u16_at(p + 30).ok_or_else(bad)?,
            u16_at(p + 32).ok_or_else(bad)?,
        );
        let local = u32_at(p + 42).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(bytes.get(p + 46..p + 46 + n).ok_or_else(bad)?).into_owned();
        p += 46 + n + x + c;
        if name.ends_with('/') {
            continue;
        }
        if u32_at(local) != Some(0x0403_4b50) {
            return Err(bad());
        }
        let start = local + 30 + u16_at(local + 26).ok_or_else(bad)? + u16_at(local + 28).ok_or_else(bad)?;
        let raw = bytes.get(start..start + packed).ok_or_else(bad)?;
        let data = match method {
            0 => raw.to_vec(),
            8 => {
                let mut d = Vec::with_capacity(size.min(MAX_UNZIPPED));
                flate2::read::DeflateDecoder::new(raw)
                    .take(MAX_UNZIPPED as u64 + 1)
                    .read_to_end(&mut d)
                    .map_err(|e| format!("{name}: {e}"))?;
                if d.len() > MAX_UNZIPPED {
                    return Err(format!("{name}: the file is too large"));
                }
                d
            }
            m => return Err(format!("{name}: compression method {m} is not supported")),
        };
        if crc32fast::hash(&data) != crc {
            return Err(format!("{name}: the data is damaged"));
        }
        out.push((name, data));
    }
    Ok(out)
}

// ---- Edit the backdrop in another program ----

/// The backdrop picture that another program edits.
#[derive(Debug, Clone)]
pub struct OutsideEdit {
    pub path: PathBuf,
    modified: Option<SystemTime>,
    checked: Instant,
}

/// The time between two checks of the edited file.
const CHECK_EVERY: Duration = Duration::from_millis(800);

/// The file extension of a picture file (from its bytes).
fn picture_ext(data: &[u8]) -> &'static str {
    match image::guess_format(data) {
        Ok(image::ImageFormat::Jpeg) => "jpg",
        Ok(image::ImageFormat::Bmp) => "bmp",
        Ok(image::ImageFormat::Tiff) => "tif",
        Ok(image::ImageFormat::Gif) => "gif",
        _ => "png",
    }
}

impl AppState {
    /// Writes the backdrop picture to a file and opens it in the program of
    /// the system. When the file changes, the backdrop changes.
    pub fn edit_backdrop_outside(&mut self) {
        let Some(b) = self.doc.design().backdrop.clone() else {
            self.message = tr("Load a backdrop picture first.").into();
            return;
        };
        let dir = std::env::temp_dir().join(crate::icons::APP_ID);
        let stem = Path::new(&b.name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "backdrop".into());
        let path = dir.join(format!("{stem} (edit).{}", picture_ext(&b.data)));
        let r = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&path, b.data.as_slice()))
            .and_then(|_| open_with_system(&path));
        match r {
            Ok(()) => {
                let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.outside_edit = Some(OutsideEdit {
                    path: path.clone(),
                    modified,
                    checked: Instant::now(),
                });
                self.message = crate::i18n::fill(
                    "The picture is open in another program ({}). Save it there: the backdrop changes here.",
                    &[&(path.display())],
                );
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: "Cannot open the picture in another program".into(),
                    lines: vec![path.display().to_string(), e.to_string()],
                })
            }
        }
    }

    /// Checks the picture that another program edits; a changed picture
    /// becomes the backdrop (one undo step). Returns `true` while it is
    /// checked, so the window keeps checking.
    pub fn poll_outside_edit(&mut self) -> bool {
        let Some(e) = &self.outside_edit else { return false };
        if self.doc.design().backdrop.is_none() {
            self.outside_edit = None;
            return false;
        }
        if e.checked.elapsed() < CHECK_EVERY {
            return true;
        }
        let path = e.path.clone();
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        let changed = modified.is_some() && modified != e.modified;
        if let Some(e) = &mut self.outside_edit {
            e.checked = Instant::now();
            if changed {
                e.modified = modified;
            }
        }
        if changed && let Ok(bytes) = std::fs::read(&path) {
            self.take_outside_picture(bytes);
        }
        true
    }

    /// Puts a picture from another program in place of the backdrop: the
    /// same place and width; the height follows the picture.
    pub fn take_outside_picture(&mut self, bytes: Vec<u8>) {
        // A full decode: the other program may not have finished the file yet.
        if emb_art::decode_fit(&bytes, 64, crate::resources::decode_budget()).is_err() {
            return;
        }
        let Ok(pixels) = emb_art::picture_size(&bytes) else {
            return;
        };
        self.doc.edit("Backdrop changed in another program", |d| {
            if let Some(b) = d.backdrop.as_mut() {
                b.data = std::sync::Arc::new(bytes);
                b.pixels = pixels;
            }
        });
        self.message = tr("The backdrop picture changed in the other program.").into();
    }
}

// ---- Send by email ----

impl AppState {
    /// The files to send: the machine file, the project and a picture.
    pub fn email_files(&mut self) -> Result<Vec<(String, Vec<u8>)>, String> {
        let report = self.export_report();
        if let Some(e) = report.errors.first() {
            return Err(e.text.clone());
        }
        let d = self.doc.design().clone();
        let name = self
            .file_path
            .as_deref()
            .and_then(Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| d.name.clone());
        let machine = self.machine_for_files(d.machine.as_deref());
        let ext = machine.as_ref().map_or("dst".to_string(), |m| m.default_format.clone());
        let plan = self.plan_view().plan.clone();
        let date = crate::output::local_stamp();
        let needles = self.catalogs().needle_plan(&d, &plan.stitches);
        let machine_file =
            emb_formats::write_machine_with(&ext, &plan.stitches, &name, &date, machine.as_ref(), needles.as_ref())
                .ok_or_else(|| format!("The program cannot write .{ext} files."))?;
        let project = emb_model::project::to_string(&d).into_bytes();
        let segments = emb_render::segments(&plan.stitches);
        let look = self.settings.look.picture_look(d.fabric.as_deref());
        let pic = emb_render::raster::picture_look(&segments, &[], 10.0, 0.4, 2.0, d.background, &look);
        Ok(vec![
            (format!("{name}.{ext}"), machine_file),
            (format!("{name}.{}", emb_model::project::FILE_EXTENSION), project),
            (format!("{name}.png"), pic.png()),
        ])
    }

    /// Send by email: writes `<name> (email).zip` next to the design (or in
    /// the documents folder), opens the mail program and shows the zip file.
    pub fn send_by_email(&mut self) {
        let files = match self.email_files() {
            Ok(f) => f,
            Err(e) => {
                self.message = e;
                return;
            }
        };
        let stem = files[0]
            .0
            .rsplit_once('.')
            .map_or(files[0].0.clone(), |(s, _)| s.to_string());
        let dir = self
            .file_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .map(PathBuf::from)
            })
            .unwrap_or_else(std::env::temp_dir);
        let path = dir.join(format!("{stem} (email).zip"));
        if let Err(e) = std::fs::write(&path, zip(&files)) {
            self.notice = Some(Notice {
                title: "Cannot write the zip file".into(),
                lines: vec![path.display().to_string(), e.to_string()],
            });
            return;
        }
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        let body = format!(
            "The embroidery design \"{stem}\" is in the attached file {} ({}).",
            path.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            names.join(", ")
        );
        let opened = open_with_system(Path::new(&mailto(&stem, &body))).is_ok();
        let _ = show_in_folder(&path);
        self.message = if opened {
            format!("Attach {} to the message (the folder shows it).", path.display())
        } else {
            format!("Wrote {}. Attach it to an email.", path.display())
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn zip_round_trip() {
        let files = vec![
            ("a.txt".to_string(), b"hello hello hello hello".to_vec()),
            (
                "fonts/b.bin".to_string(),
                (0..=255u8).cycle().take(5000).collect::<Vec<u8>>(),
            ),
            ("empty".to_string(), Vec::new()),
        ];
        let z = zip(&files);
        assert_eq!(unzip(&z).unwrap(), files);
        assert!(unzip(b"not a zip").is_err());
        // A damaged file.
        let mut bad = z.clone();
        bad[40] ^= 0xff;
        assert!(unzip(&bad).is_err());
    }

    #[test]
    fn the_target_of_cmd_is_in_quotes() {
        let line = |t: &str| {
            start_command_line(std::ffi::OsStr::new(t))
                .to_string_lossy()
                .into_owned()
        };
        assert_eq!(line(r"C:\x\a&b.png"), r#"/C start "" "C:\x\a&b.png""#);
        let mail = mailto("Rose", "a b");
        assert_eq!(
            line(&mail),
            format!(r#"/C start "" "{mail}""#),
            "the &body= stays in the link"
        );
    }

    #[test]
    fn large_files_in_a_zip_are_refused() {
        // A deflated file of zeros, larger than the limit.
        let z = zip(&[("big.bin".to_string(), vec![0u8; MAX_UNZIPPED + 1])]);
        assert!(z.len() < MAX_UNZIPPED / 100, "zeros pack small");
        assert!(unzip(&z).unwrap_err().contains("too large"));
    }

    #[test]
    fn mail_links_are_encoded() {
        assert_eq!(
            mailto("Rose & co", "a b\nc"),
            "mailto:?subject=Rose%20%26%20co&body=a%20b%0Ac"
        );
    }

    #[test]
    fn a_changed_picture_replaces_the_backdrop() {
        let mut h = Harness::new();
        let img = image::RgbaImage::from_pixel(20, 10, image::Rgba([200, 0, 0, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        h.st.import_bitmap_bytes(png.into_inner(), "logo.png");
        let before = h.st.doc.design().backdrop.clone().unwrap();
        let img = image::RgbaImage::from_pixel(40, 40, image::Rgba([0, 0, 200, 255]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        h.st.take_outside_picture(png.into_inner());
        let after = h.st.doc.design().backdrop.clone().unwrap();
        assert_eq!(after.pixels, (40, 40));
        assert_eq!((after.origin, after.width_mm), (before.origin, before.width_mm));
        // A file that is not finished yet: no change.
        h.st.take_outside_picture(b"half a file".to_vec());
        assert_eq!(h.st.doc.design().backdrop.as_ref().unwrap().pixels, (40, 40));
    }

    #[test]
    fn the_email_has_the_machine_file_the_project_and_a_picture() {
        let mut h = Harness::new();
        let files = h.st.email_files().unwrap();
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names[0].ends_with(".vp3") || names[0].ends_with(".dst"), "{names:?}");
        assert!(names[1].ends_with(".embproj") && names[2].ends_with(".png"));
        assert!(emb_model::project::from_str(std::str::from_utf8(&files[1].1).unwrap()).is_ok());
    }
}

//! The file dialogs. On the desktop, `FileDialog` is the dialog of the
//! system (`rfd::FileDialog`). In the browser trial (P15 step 1) a dialog
//! of the system is not there, and this `FileDialog` has the same methods:
//!
//! - A file to save is a download: `save_file` gives the name, and
//!   [`write`] gives the file to the browser.
//! - A folder to write to is one zip file: [`folder_to_write`] gives a
//!   folder of that name, and the files that [`write`] puts in it go as one
//!   download at the end of the frame (`web::poll`).
//! - A file to open is an upload: [`open_then`] asks the browser for the
//!   file and goes on with it in a later frame, and [`read`] gives its bytes.
//! - Other dialogs (`pick_file` alone, several files, `pick_folder`: a
//!   folder to read or keep) give nothing and say that the command needs
//!   the desktop.
//!
//! The owner, 2026-11-25: the commands with a file dialog (Import Artwork as
//! vectors first) must work in the browser where they can.

use std::path::{Path, PathBuf};

use crate::app::AppState;

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

/// The bytes of a file that a dialog gave: from the disk, or in the
/// browser from the upload.
pub fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    if let Some(bytes) = crate::web::uploaded_file(path) {
        return Ok(bytes);
    }
    std::fs::read(path)
}

/// The text of a file that a dialog gave (see [`read`]).
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    String::from_utf8(read(path)?).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Writes a file that a save dialog or a folder dialog gave: to the disk,
/// or in the browser as a download (in the zip file of a folder).
pub fn write(path: &Path, bytes: impl AsRef<[u8]>) -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    return crate::web::write_download(path, bytes.as_ref()).map_err(std::io::Error::other);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write(path, bytes)
}

/// Asks with `dialog` for a folder for new files (see [`write`]). In the
/// browser: the folder of one zip file, with no question.
pub fn folder_to_write(dialog: FileDialog) -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    return dialog.pick_folder();
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dialog;
        Some(PathBuf::from(ZIP_FOLDER))
    }
}

/// Asks with `dialog` for a file to open, and gives its path to `then`. On
/// the desktop at once; in the browser after the upload, in a later frame
/// (and [`read`] gives the bytes of that path).
pub fn open_then(dialog: FileDialog, st: &mut AppState, then: impl FnOnce(&mut AppState, PathBuf) + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(p) = dialog.pick_file() {
        then(st, p);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = st;
        crate::web::upload_then(dialog.filters, Box::new(then));
    }
}

/// A file dialog of the browser (see the module).
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Default, Clone)]
pub struct FileDialog {
    /// The filters: a name and the extensions.
    filters: Vec<(String, Vec<String>)>,
    title: String,
    file_name: Option<String>,
}

/// The folder of [`folder_to_write`] in the browser: its files go in one
/// zip file with this name.
#[cfg(target_arch = "wasm32")]
pub const ZIP_FOLDER: &str = "ShokujoDX files.zip";

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn new() -> Self {
        FileDialog::default()
    }

    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[impl ToString]) -> Self {
        self.filters
            .push((name.into(), extensions.iter().map(ToString::to_string).collect()));
        self
    }

    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn set_directory<P: AsRef<Path>>(self, _path: P) -> Self {
        self
    }

    pub fn set_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// A file to open needs [`open_then`] in the browser.
    pub fn pick_file(self) -> Option<PathBuf> {
        crate::web::not_in_the_browser();
        None
    }

    pub fn pick_files(self) -> Option<Vec<PathBuf>> {
        crate::web::not_in_the_browser();
        None
    }

    /// A folder to write to needs [`folder_to_write`] in the browser.
    pub fn pick_folder(self) -> Option<PathBuf> {
        crate::web::not_in_the_browser();
        None
    }

    /// The name of the download: the file name of the dialog, with the
    /// extension of the first filter if it has none.
    pub fn save_file(self) -> Option<PathBuf> {
        Some(PathBuf::from(download_name(
            self.file_name.as_deref(),
            &self.title,
            &self.filters,
        )))
    }
}

/// The name of a download (see `FileDialog::save_file`).
#[cfg(any(target_arch = "wasm32", test))]
fn download_name(file_name: Option<&str>, title: &str, filters: &[(String, Vec<String>)]) -> String {
    let mut name = file_name
        .filter(|n| !n.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if title.is_empty() {
                "file".into()
            } else {
                title.replace([':', '/', '\\'], "")
            }
        });
    if Path::new(&name).extension().is_none()
        && let Some(ext) = filters.first().and_then(|(_, e)| e.first())
    {
        name = format!("{name}.{ext}");
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_has_the_name_of_the_dialog_and_an_extension() {
        let pdf = vec![("PDF".to_string(), vec!["pdf".to_string()])];
        assert_eq!(download_name(Some("Rose chart.pdf"), "Chart", &pdf), "Rose chart.pdf");
        assert_eq!(download_name(Some("Rose chart"), "Chart", &pdf), "Rose chart.pdf");
        assert_eq!(download_name(None, "Quote: sheet", &pdf), "Quote sheet.pdf");
        assert_eq!(download_name(None, "", &[]), "file");
    }

    /// On the desktop, a file to save is written, and [`read`] reads it.
    #[test]
    fn write_and_read_go_to_the_disk_on_the_desktop() {
        let p = std::env::temp_dir().join(format!("emb-dialog-{}.txt", std::process::id()));
        write(&p, "text").unwrap();
        assert_eq!(read_to_string(&p).unwrap(), "text");
        assert_eq!(read(&p).unwrap(), b"text");
        std::fs::remove_file(&p).ok();
    }
}

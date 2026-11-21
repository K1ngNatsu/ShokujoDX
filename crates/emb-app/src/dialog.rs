//! The file dialogs. On the desktop, `FileDialog` is the dialog of the
//! system (`rfd::FileDialog`). In the browser trial (P15 step 1) a dialog
//! of the system is not there: this `FileDialog` has the same methods and
//! gives no file, and the browser opens and saves files through
//! `crate::web` (an upload and a download).

#[cfg(not(target_arch = "wasm32"))]
pub use rfd::FileDialog;

/// A file dialog that gives no file (the browser).
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Default, Clone)]
pub struct FileDialog;

#[cfg(target_arch = "wasm32")]
impl FileDialog {
    pub fn new() -> Self {
        FileDialog
    }

    pub fn add_filter(self, _name: impl Into<String>, _extensions: &[impl ToString]) -> Self {
        self
    }

    pub fn set_title(self, _title: impl Into<String>) -> Self {
        self
    }

    pub fn set_directory<P: AsRef<std::path::Path>>(self, _path: P) -> Self {
        self
    }

    pub fn set_file_name(self, _name: impl Into<String>) -> Self {
        self
    }

    pub fn pick_file(self) -> Option<std::path::PathBuf> {
        crate::web::not_in_the_browser();
        None
    }

    pub fn pick_files(self) -> Option<Vec<std::path::PathBuf>> {
        crate::web::not_in_the_browser();
        None
    }

    pub fn pick_folder(self) -> Option<std::path::PathBuf> {
        crate::web::not_in_the_browser();
        None
    }

    pub fn save_file(self) -> Option<std::path::PathBuf> {
        crate::web::not_in_the_browser();
        None
    }
}

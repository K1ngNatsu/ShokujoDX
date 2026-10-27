//! The design library (ADR 0035): the design files in the library
//! folders, with an index entry and a thumbnail for each one.
//!
//! - [`index`]: the entries, the index file and the scan of the folders.
//! - [`search`]: the search text (words and filters), sort and group.
//! - [`csv`]: the design list as CSV; [`catalog`]: the catalog pages.
//!
//! This crate has no UI dependency. The app and `emb-cli` use it.
#![forbid(unsafe_code)]

pub mod catalog;
pub mod csv;
pub mod index;
pub mod preview;
pub mod search;

pub use index::{
    Entry, INDEX_VERSION, Index, ScanFile, ScanReport, find_files, read_entry, read_machine_file, read_stitches, scan,
};
pub use search::{Group, GroupKey, Query, SortKey, group, sort};

/// The file extensions that the library shows: project files and the
/// machine formats that the program reads.
pub fn is_design_file(ext: &str) -> bool {
    ext == emb_model::project::FILE_EXTENSION || emb_formats::READ_EXTENSIONS.contains(&ext)
}

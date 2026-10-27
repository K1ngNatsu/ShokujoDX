//! The helper of the macOS Quick Look extension (P11 Milestone 3, ADR 0046):
//! it reads the bytes of a design file on standard input and writes its
//! picture (PNG) to standard output. The extension runs in a sandbox and
//! gives the bytes, so the helper opens no file.
//!
//! Use:  emb-thumbnail-helper <size> [<extension>] < design > picture.png
#![forbid(unsafe_code)]

use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let size: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(256);
    let ext = args.get(1).map(String::as_str);
    let mut bytes = Vec::new();
    let read = std::io::stdin()
        .take(emb_thumbnail::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > emb_thumbnail::MAX_FILE_BYTES {
        std::process::exit(2);
    }
    let Some(picture) = emb_library::preview::picture(&bytes, ext, size) else {
        std::process::exit(1);
    };
    if std::io::stdout().write_all(&picture.png()).is_err() {
        std::process::exit(2);
    }
}

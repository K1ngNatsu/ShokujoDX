//! The Windows thumbnail handler (P11 Milestone 3, ADR 0046): Explorer shows
//! the picture of project and machine files.
//!
//! This crate is the one place of the program with `unsafe` code: Explorer
//! calls the handler through COM, and Rust cannot check calls that come from
//! outside the program (an exception to rule 9). To make it robust:
//!
//! - The handler takes the file as a stream (`IInitializeWithStream`), so
//!   Windows runs it in a separate process: an error cannot stop Explorer.
//! - The `unsafe` code only reads the stream, makes the bitmap and answers
//!   COM. The picture comes from `emb_library::preview` (no `unsafe`).
//! - Each call from Windows catches a panic and gives an error code.
//! - A file larger than [`MAX_FILE_BYTES`] gets no picture.
//!
//! On other systems the crate is empty.

// The MSVC linker says that the COM exports "should be PRIVATE" (out of the
// import library). Nothing links to this DLL, so the note does not matter.
#![allow(linker_messages)]

/// The class id of the handler (the installer registers it).
pub const CLSID_TEXT: &str = "{F1F7156E-ACB6-4891-9AC2-7E32E078B242}";

/// The largest file that gets a picture.
pub const MAX_FILE_BYTES: usize = 64 << 20;

/// The picture of the bytes of a design file, as BGRA pixels with the color
/// multiplied by the alpha (as Windows bitmaps with alpha have them), row by
/// row from the top: (width, height, pixels).
pub fn bgra_picture(bytes: &[u8], px: u32) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.len() > MAX_FILE_BYTES {
        return None;
    }
    let p = emb_library::preview::picture(bytes, None, px)?;
    let mut out = p.rgba;
    for c in out.as_chunks_mut::<4>().0 {
        let a = u16::from(c[3]);
        let m = |v: u8| ((u16::from(v) * a + 127) / 255) as u8;
        let (r, g, b) = (m(c[0]), m(c[1]), m(c[2]));
        c[0] = b;
        c[1] = g;
        c[2] = r;
    }
    Some((p.width, p.height, out))
}

#[cfg(windows)]
mod handler;

#[cfg(windows)]
pub use handler::{CLSID, DllCanUnloadNow, DllGetClassObject};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_is_bgra_with_the_alpha_multiplied() {
        let d = emb_model::design_from_stitches(
            &emb_model::StitchDesign {
                threads: vec![emb_model::Thread {
                    id: emb_model::ThreadId(0),
                    name: "Red".into(),
                    color: emb_model::Rgb(220, 20, 30),
                    brand: None,
                    code: None,
                }],
                records: (0..=40)
                    .flat_map(|i| {
                        let y = f64::from(i) * 0.5;
                        (0..=10).map(move |k| {
                            let x = f64::from(if i % 2 == 0 { k } else { 10 - k }) * 2.0;
                            emb_model::StitchRecord::stitch(x, y)
                        })
                    })
                    .collect(),
            },
            "square",
        );
        let bytes = emb_model::project::to_string(&d).into_bytes();
        let (w, h, px) = bgra_picture(&bytes, 96).expect("a picture");
        assert!(w.max(h) <= 96 && px.len() == (w * h * 4) as usize);
        // Red is the third byte; no color is more than its alpha.
        assert!(px.chunks(4).any(|c| c[3] > 200 && c[2] > c[0]));
        assert!(px.chunks(4).all(|c| c[0] <= c[3] && c[1] <= c[3] && c[2] <= c[3]));
        assert!(bgra_picture(b"not a design", 96).is_none());
        assert!(bgra_picture(&vec![b' '; MAX_FILE_BYTES + 1], 96).is_none());
    }
}

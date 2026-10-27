//! Limits for computers with little memory or a weak graphics card (ADR
//! 0077): the memory that one picture decode can use, the size of a
//! texture, and the stack of a background job.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::i18n::tr;

/// One picture decode uses at most half of the free memory, but not less
/// than this (a computer that tells no free memory)...
const DECODE_MIN_BYTES: u64 = 256 * 1024 * 1024;
/// ...and not more than this.
const DECODE_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The stack of a thread that runs engine work in the background. The
/// memory is reserved, not used, until the work needs it.
pub const JOB_STACK: usize = 64 * 1024 * 1024;

/// The memory that the decoder of one picture can use, in bytes: half of
/// the free memory now, in a range.
pub fn decode_budget() -> u64 {
    let mut s = sysinfo::System::new();
    s.refresh_memory();
    budget_of(s.available_memory())
}

fn budget_of(free: u64) -> u64 {
    (free / 2).clamp(DECODE_MIN_BYTES, DECODE_MAX_BYTES)
}

/// The text of a picture decode error, in the language of the program.
pub fn decode_error(e: &str) -> String {
    if e == emb_art::TOO_LARGE {
        // The same text as `emb_art::TOO_LARGE` (a test checks it).
        tr("The picture is too large for the free memory. Make it smaller in a picture program, or close other programs.")
            .into()
    } else {
        e.into()
    }
}

/// The largest side of a texture that the graphics card takes, in pixels.
pub fn max_texture_side(ctx: &egui::Context) -> usize {
    ctx.input(|i| i.max_texture_side).max(1)
}

/// `img` made smaller if a side is longer than `max_side` (the graphics
/// card cannot make a texture of it, and the program stops).
pub fn fit_texture(img: ColorImage, max_side: usize) -> ColorImage {
    let [w, h] = img.size;
    if w.max(h) <= max_side || w == 0 || h == 0 {
        return img;
    }
    let k = max_side as f64 / w.max(h) as f64;
    let (nw, nh) = (
        ((w as f64 * k).round() as u32).clamp(1, max_side as u32),
        ((h as f64 * k).round() as u32).clamp(1, max_side as u32),
    );
    // The pixels of a `ColorImage` are premultiplied: they average well.
    let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
    let Some(full) = image::RgbaImage::from_raw(w as u32, h as u32, raw) else {
        return ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT]);
    };
    let small = image::imageops::resize(&full, nw, nh, image::imageops::FilterType::Triangle);
    ColorImage::from_rgba_premultiplied([nw as usize, nh as usize], small.as_raw())
}

/// Loads a texture, made smaller first if the graphics card needs it.
pub fn load_texture(
    ctx: &egui::Context,
    name: impl Into<String>,
    img: ColorImage,
    opts: TextureOptions,
) -> TextureHandle {
    ctx.load_texture(name, fit_texture(img, max_texture_side(ctx)), opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_texture_larger_than_the_card_takes_is_made_smaller() {
        let img = ColorImage::new([9000, 30], vec![egui::Color32::RED; 9000 * 30]);
        let small = fit_texture(img, 8192);
        assert_eq!(small.size, [8192, 27]);
        assert_eq!(small.pixels[100], egui::Color32::RED);
        let tall = fit_texture(ColorImage::new([3, 20000], vec![egui::Color32::BLUE; 60000]), 2048);
        assert_eq!(tall.size, [1, 2048]);
        let ok = ColorImage::new([20, 10], vec![egui::Color32::GREEN; 200]);
        assert_eq!(fit_texture(ok.clone(), 2048), ok);
        // The context of the tests gives the default of egui.
        let ctx = egui::Context::default();
        let t = load_texture(
            &ctx,
            "t",
            ColorImage::new([5000, 1], vec![egui::Color32::WHITE; 5000]),
            TextureOptions::LINEAR,
        );
        assert!(t.size()[0] <= max_texture_side(&ctx));
    }

    #[test]
    fn the_decode_budget_follows_the_free_memory() {
        assert_eq!(decode_error(emb_art::TOO_LARGE), emb_art::TOO_LARGE);
        assert_eq!(decode_error("bad data"), "bad data");
        assert_eq!(budget_of(0), DECODE_MIN_BYTES);
        assert_eq!(budget_of(2_000_000_000), 1_000_000_000);
        assert_eq!(budget_of(64_000_000_000), DECODE_MAX_BYTES);
        let b = decode_budget();
        assert!((DECODE_MIN_BYTES..=DECODE_MAX_BYTES).contains(&b));
    }
}

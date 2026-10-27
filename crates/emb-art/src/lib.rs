//! Bitmap artwork for digitizing (roadmap P6; ADR 0027).
//!
//! - `adjust`: brightness, contrast and sharpness.
//! - `crop`: cut a picture to a rectangle, an ellipse or a polygon.
//! - `reduce`: color reduction ("prepare colors"): a small number of
//!   colors, no small specks, clean edges; and the thread nearest to each
//!   color.
//! - `autotrace`: the color areas of a reduced picture as smooth vector
//!   shapes (ADR 0028).
//! - `click`: embroidery objects from one color area (click-to-stitch).
//! - `background`: background removal (a color under a click, or the
//!   color at the edges), and trimming of clear edges.
//! - `cutout`: cut out the subject from a box and marks (GrabCut with a
//!   graph cut, `maxflow`; ADR 0031).
//! - `photo`: photo stitch (rows, color rows, scribble; ADR 0030).
//! - `recognize`: the area that the stitches of a stitch object cover
//!   (recognize outlines; ADR 0030).
//! - `cross`: picture to cross stitch (P9 Milestone 2).
//! - `smart`: picture to design (the steps of the guided wizard: detail,
//!   colors, stitch types, sew order; ADR 0029).
//!
//! The functions are pure and deterministic (ADR 0003): the same picture
//! and settings always give the same result. Later P6 milestones add
//! auto-digitizing here.

#![forbid(unsafe_code)]

pub mod adjust;
pub mod autotrace;
pub mod background;
pub mod click;
pub mod crop;
pub mod cross;
pub mod cutout;
mod maxflow;
pub mod photo;
pub mod recognize;
pub mod reduce;
pub mod smart;

pub use image::{Rgba, RgbaImage};

/// The memory that the decoder of one picture can use when the caller gives
/// no budget, in bytes (the default of the `image` crate).
pub const DECODE_BYTES: u64 = 512 * 1024 * 1024;

/// The error of a decode that needs more memory than its budget.
pub const TOO_LARGE: &str =
    "The picture is too large for the free memory. Make it smaller in a picture program, or close other programs.";

/// Decodes a picture file (PNG, JPEG, BMP and the other formats of the
/// `image` crate), upright: the EXIF orientation of the file (a photo from
/// a phone) is applied (P12 Milestone 1).
pub fn decode(data: &[u8]) -> Result<RgbaImage, String> {
    decode_image(data).map(|i| i.to_rgba8())
}

/// As [`decode`], in the color type of the file.
pub fn decode_image(data: &[u8]) -> Result<image::DynamicImage, String> {
    let (img, orientation) = decode_raw(data, DECODE_BYTES)?;
    let mut img = img;
    img.apply_orientation(orientation);
    Ok(img)
}

/// Decodes a picture file upright, at most `max_side` pixels on its long
/// side, and the decoder uses at most `max_bytes` of memory (else the error
/// is [`TOO_LARGE`]). ADR 0077: a large picture is made smaller in the
/// color type of the file, before the conversion to RGBA and before the
/// EXIF turn, so only one full-size copy is in memory (3 bytes a pixel for
/// a JPEG photo). A box filter makes it smaller, with no large work buffer.
/// Colors are averaged with their transparency (premultiplied), so a clear
/// pixel does not bleed its color into the edge; the alpha stays as it is
/// averaged. A picture that is not larger than `max_side` is as [`decode`]
/// gives it.
pub fn decode_fit(data: &[u8], max_side: u32, max_bytes: u64) -> Result<RgbaImage, String> {
    decode_fit_info(data, max_side, max_bytes, false).map(|(img, _)| img)
}

/// As [`decode_fit`]; `true` if the picture was made smaller. `exact`: with
/// the triangle filter of `smart::resample` (the same pixels as before
/// ADR 0077; a work buffer of 12 bytes for each pixel of the full width
/// and the new height).
pub(crate) fn decode_fit_info(
    data: &[u8],
    max_side: u32,
    max_bytes: u64,
    exact: bool,
) -> Result<(RgbaImage, bool), String> {
    use image::metadata::Orientation;
    let (img, orientation) = decode_raw(data, max_bytes)?;
    let turned = matches!(
        orientation,
        Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH
    );
    let (uw, uh) = if turned {
        (img.height(), img.width())
    } else {
        (img.width(), img.height())
    };
    let long = uw.max(uh);
    let max_side = max_side.max(1);
    let (out, smaller) = if long <= max_side || uw == 0 || uh == 0 {
        (img.into_rgba8(), false)
    } else {
        // The size as `smart::resample` gives it for the upright picture.
        let tw = ((f64::from(uw) * f64::from(max_side) / f64::from(long)).round() as u32).max(1);
        let th = ((f64::from(uh) * f64::from(tw) / f64::from(uw)).round() as u32).max(1);
        let (sw, sh) = if turned { (th, tw) } else { (tw, th) };
        (shrink(img, sw, sh, exact), true)
    };
    let mut out = image::DynamicImage::ImageRgba8(out);
    out.apply_orientation(orientation);
    Ok((out.into_rgba8(), smaller))
}

/// The picture of a file in its own color type, and its EXIF orientation;
/// the decoder uses at most `max_bytes`.
fn decode_raw(data: &[u8], max_bytes: u64) -> Result<(image::DynamicImage, image::metadata::Orientation), String> {
    use image::ImageDecoder;
    let mut r = reader(data)?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(max_bytes);
    r.limits(limits);
    let mut decoder = r.into_decoder().map_err(decode_error)?;
    // The decoders do not all keep to the limit for the pixel buffer.
    if decoder.total_bytes() > max_bytes {
        return Err(TOO_LARGE.to_string());
    }
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let img = image::DynamicImage::from_decoder(decoder).map_err(decode_error)?;
    Ok((img, orientation))
}

fn decode_error(e: image::ImageError) -> String {
    match e {
        image::ImageError::Limits(_) => TOO_LARGE.to_string(),
        e => e.to_string(),
    }
}

/// `img` made smaller to `w` × `h` (a triangle filter if `exact`, else a
/// box filter), as RGBA. An RGB picture (a photo) is made smaller as it is;
/// another one is converted first and its colors averaged premultiplied.
fn shrink(img: image::DynamicImage, w: u32, h: u32, exact: bool) -> RgbaImage {
    use image::imageops::{FilterType, resize, thumbnail};
    if let image::DynamicImage::ImageRgb8(rgb) = &img {
        let small = if exact {
            resize(rgb, w, h, FilterType::Triangle)
        } else {
            thumbnail(rgb, w, h)
        };
        drop(img);
        return RgbaImage::from_fn(w, h, |x, y| {
            let p = small.get_pixel(x, y);
            Rgba([p[0], p[1], p[2], 255])
        });
    }
    let mut rgba = img.into_rgba8();
    for p in rgba.pixels_mut() {
        let a = u16::from(p[3]);
        for c in 0..3 {
            p[c] = ((u16::from(p[c]) * a + 127) / 255) as u8;
        }
    }
    let mut out = if exact {
        resize(&rgba, w, h, FilterType::Triangle)
    } else {
        thumbnail(&rgba, w, h)
    };
    drop(rgba);
    for p in out.pixels_mut() {
        // A clear pixel (alpha 0) gets no color.
        let a = u16::from(p[3]);
        for c in 0..3 {
            p[c] = (u16::from(p[c]) * 255 + a / 2)
                .checked_div(a)
                .map_or(0, |v| v.min(255) as u8);
        }
    }
    out
}

/// The pixel size of a picture file, upright (as [`decode`] gives it),
/// without a full decode.
pub fn picture_size(data: &[u8]) -> Result<(u32, u32), String> {
    use image::ImageDecoder;
    use image::metadata::Orientation;
    let mut decoder = reader(data)?.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    let turned = matches!(
        decoder.orientation(),
        Ok(Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH)
    );
    Ok(if turned { (h, w) } else { (w, h) })
}

fn reader(data: &[u8]) -> Result<image::ImageReader<std::io::Cursor<&[u8]>>, String> {
    image::ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| e.to_string())
}

/// Encodes a picture as PNG.
pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trip() {
        let mut img = RgbaImage::new(3, 2);
        img.put_pixel(1, 1, image::Rgba([10, 20, 30, 255]));
        let back = decode(&encode_png(&img).unwrap()).unwrap();
        assert_eq!(back, img);
        assert!(decode(b"no picture").is_err());
        assert_eq!(picture_size(&encode_png(&img).unwrap()).unwrap(), (3, 2));
    }

    /// A JPEG of `img` with the EXIF orientation `o` (1 to 8).
    fn jpeg_with_orientation(img: &RgbaImage, o: u8) -> Vec<u8> {
        use image::ImageEncoder;
        // A TIFF header (little-endian) with one entry: orientation, a SHORT.
        let exif = [
            b'I', b'I', 42, 0, 8, 0, 0, 0, // header, the first entry list at 8
            1, 0, // one entry
            0x12, 0x01, 3, 0, 1, 0, 0, 0, o, 0, 0, 0, // 0x0112 orientation
            0, 0, 0, 0, // no next entry list
        ];
        let mut out = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95);
        enc.set_exif_metadata(exif.to_vec()).unwrap();
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        enc.write_image(rgb.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    #[test]
    fn a_phone_photo_comes_in_upright() {
        // 40 x 20 px: red on the left half, blue on the right.
        let img = RgbaImage::from_fn(40, 20, |x, _| {
            if x < 20 {
                image::Rgba([220, 20, 20, 255])
            } else {
                image::Rgba([20, 20, 220, 255])
            }
        });
        let red = |p: &image::Rgba<u8>| p[0] > 150 && p[2] < 100;
        // 1: as it is.
        let plain = jpeg_with_orientation(&img, 1);
        assert_eq!(picture_size(&plain).unwrap(), (40, 20));
        assert!(red(decode(&plain).unwrap().get_pixel(5, 10)));
        // 6: the camera was turned; the picture turns 90° clockwise, so the
        // red left half is at the top.
        let turned = jpeg_with_orientation(&img, 6);
        assert_eq!(picture_size(&turned).unwrap(), (20, 40));
        let up = decode(&turned).unwrap();
        assert_eq!(up.dimensions(), (20, 40));
        assert!(red(up.get_pixel(10, 5)) && !red(up.get_pixel(10, 35)));
        // 3: upside down; the red is on the right.
        let down = decode(&jpeg_with_orientation(&img, 3)).unwrap();
        assert!(red(down.get_pixel(35, 10)) && !red(down.get_pixel(5, 10)));
        // Made smaller, the turned photo is upright too.
        let small = decode_fit(&turned, 10, DECODE_BYTES).unwrap();
        assert_eq!(small.dimensions(), (5, 10));
        assert!(red(small.get_pixel(2, 1)) && !red(small.get_pixel(2, 8)));
    }

    #[test]
    fn a_large_picture_is_made_smaller_as_the_wizard_did_it() {
        // A photo (RGB) and a picture with clear parts (RGBA), 1500 x 1000.
        let rgba = RgbaImage::from_fn(1500, 1000, |x, y| {
            let a = if x < 40 {
                0
            } else if x < 60 {
                100
            } else {
                255
            };
            Rgba([(x % 256) as u8, (y * 7 % 256) as u8, ((x + y) % 256) as u8, a])
        });
        let opaque = RgbaImage::from_fn(1500, 1000, |x, y| {
            let p = rgba.get_pixel(x, y);
            Rgba([p[0], p[1], p[2], 255])
        });
        let png = |img: &RgbaImage| {
            let mut out = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(img.clone())
                .write_to(&mut out, image::ImageFormat::Png)
                .unwrap();
            out.into_inner()
        };
        let rgb_png = {
            let mut out = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(opaque.clone())
                .to_rgb8()
                .write_to(&mut out, image::ImageFormat::Png)
                .unwrap();
            out.into_inner()
        };
        // The work picture has the same pixels as the full decode and
        // `smart::work_picture`.
        for (data, full) in [(png(&rgba), &rgba), (rgb_png, &opaque)] {
            let work = smart::decode_work(&data, DECODE_BYTES).unwrap();
            assert_eq!(work.dimensions(), (smart::WORK_PX, 800));
            assert_eq!(work, smart::work_picture(full));
            // The box filter: near the same, with soft alpha at the edge.
            let small = decode_fit(&data, 150, DECODE_BYTES).unwrap();
            assert_eq!(small.dimensions(), (150, 100));
            let p = small.get_pixel(100, 50);
            let q = smart::resample(full, 150);
            let q = q.get_pixel(100, 50);
            assert!((0..4).all(|c| p[c].abs_diff(q[c]) <= 12), "{p:?} {q:?}");
            assert_eq!(small.get_pixel(1, 1)[3], full.get_pixel(10, 10)[3]);
        }
        // Not larger than the side: as `decode`.
        assert_eq!(decode_fit(&png(&rgba), 1500, DECODE_BYTES).unwrap(), rgba);
    }

    #[test]
    fn a_picture_over_the_memory_budget_is_an_error_not_a_crash() {
        let img = RgbaImage::from_pixel(400, 300, Rgba([1, 2, 3, 255]));
        let data = encode_png(&img).unwrap();
        assert_eq!(decode_fit(&data, 100, 10_000).unwrap_err(), TOO_LARGE);
        assert!(decode_fit(&data, 100, DECODE_BYTES).is_ok());
        // Odd data is an error too.
        for bad in [&b""[..], b"\x89PNG\r\n\x1a\n", &data[..data.len() / 2]] {
            assert!(decode_fit(bad, 100, DECODE_BYTES).is_err());
        }
    }
}

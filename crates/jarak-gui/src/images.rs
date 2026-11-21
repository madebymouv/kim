//! Images from the clipboard and from files (`JARAK-FR-042`, `JARAK-FR-044`, `JARAK-SEC-004`).
//!
//! Pasted images are untrusted input:
//!
//! - **formats**: the registered `PNG` clipboard format (kept as it is after checking it), the
//!   bitmap formats `CF_DIBV5` and `CF_DIB` (24- and 32-bit, bottom-up or top-down, `BI_RGB` or
//!   `BI_BITFIELDS`; converted to PNG), and image files copied in Explorer (`CF_HDROP`: PNG, JPEG,
//!   GIF, BMP, WebP recognised by their first bytes, copied as they are);
//! - **bounded**: at most [`MAX_BYTES`] of data and [`MAX_SIDE`] pixels on a side and
//!   [`MAX_PIXELS`] in all, checked from the header before anything large is allocated;
//! - decoding for the preview uses the pure-Rust `image` decoders (png, jpeg, gif, bmp) with the
//!   same limits.

use std::path::{Path, PathBuf};

/// The most bytes one pasted image may have.
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
/// The most pixels on a side.
pub const MAX_SIDE: u32 = 16_384;
/// The most pixels in all (an 8K screenshot fits).
pub const MAX_PIXELS: u64 = 40_000_000;

/// An image on the clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipImage {
    /// PNG bytes (the `PNG` clipboard format).
    Png(Vec<u8>),
    /// A packed DIB (`CF_DIBV5` or `CF_DIB`: header, optional masks, pixels).
    Dib(Vec<u8>),
    /// Image files copied in Explorer.
    Files(Vec<PathBuf>),
}

/// An image file kind recognised by its first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Webp,
}

impl Kind {
    pub fn ext(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpg",
            Kind::Gif => "gif",
            Kind::Bmp => "bmp",
            Kind::Webp => "webp",
        }
    }
}

/// The image kind of `data` from its signature (never from a file name).
pub fn sniff(data: &[u8]) -> Option<Kind> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(Kind::Png)
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Kind::Jpeg)
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(Kind::Gif)
    } else if data.starts_with(b"BM") && data.len() > 26 {
        Some(Kind::Bmp)
    } else if data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        Some(Kind::Webp)
    } else {
        None
    }
}

/// Checks a picture size against the limits.
pub fn check_size(w: u32, h: u32) -> Result<(), String> {
    if w == 0 || h == 0 {
        return Err("the image is empty".into());
    }
    if w > MAX_SIDE || h > MAX_SIDE || u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(format!(
            "the image is {w} × {h} pixels; Jarak takes up to {MAX_SIDE} on a side and {MAX_PIXELS} in all"
        ));
    }
    Ok(())
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn i32_at(b: &[u8], i: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// A packed DIB as straight RGBA: `(width, height, pixels)`.
pub fn dib_to_rgba(dib: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let bad = || "the clipboard bitmap is malformed".to_string();
    let header = u32_at(dib, 0).ok_or_else(bad)? as usize;
    if !(40..=124).contains(&header) || dib.len() < header {
        return Err(bad());
    }
    let w = i32_at(dib, 4).ok_or_else(bad)?;
    let h = i32_at(dib, 8).ok_or_else(bad)?;
    let bits = u16_at(dib, 14).ok_or_else(bad)?;
    let compression = u32_at(dib, 16).ok_or_else(bad)?;
    let colors_used = u32_at(dib, 32).ok_or_else(bad)?;
    if w <= 0 || h == 0 || h == i32::MIN {
        return Err(bad());
    }
    let (width, top_down) = (w.unsigned_abs(), h < 0);
    let height = h.unsigned_abs();
    check_size(width, height)?;
    if !matches!(bits, 24 | 32) {
        return Err(format!(
            "{bits}-bit clipboard bitmaps are not supported (24 and 32 are)"
        ));
    }
    // BI_RGB = 0, BI_BITFIELDS = 3; anything else (RLE, JPEG, PNG inside a DIB) is refused.
    let (mut rmask, mut gmask, mut bmask, mut amask) =
        (0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF, 0);
    let mut offset = header;
    match compression {
        0 => {}
        3 if bits == 32 => {
            if header >= 52 {
                rmask = u32_at(dib, 40).ok_or_else(bad)?;
                gmask = u32_at(dib, 44).ok_or_else(bad)?;
                bmask = u32_at(dib, 48).ok_or_else(bad)?;
                if header >= 56 {
                    amask = u32_at(dib, 52).ok_or_else(bad)?;
                }
            } else {
                // A BITMAPINFOHEADER is followed by the three masks.
                rmask = u32_at(dib, 40).ok_or_else(bad)?;
                gmask = u32_at(dib, 44).ok_or_else(bad)?;
                bmask = u32_at(dib, 48).ok_or_else(bad)?;
                offset += 12;
            }
        }
        _ => return Err("this clipboard bitmap compression is not supported".into()),
    }
    // A colour table (only for palettes; skipped when a 24/32-bit DIB still carries one).
    offset += colors_used as usize * 4;
    let stride = (width as usize * bits as usize).div_ceil(32) * 4;
    let pixels = stride * height as usize;
    // Windows' synthesized CF_DIBV5 (from a CF_BITMAP or CF_DIB) also puts the three BI_BITFIELDS
    // masks after the V4/V5 header; then exactly 12 bytes sit before the pixels.
    if compression == 3 && header > 40 && dib.len() == offset + 12 + pixels {
        offset += 12;
    }
    let need = offset + pixels;
    if dib.len() < need {
        return Err(bad());
    }
    let shift = |m: u32| if m == 0 { 0 } else { m.trailing_zeros() };
    let take = |px: u32, m: u32| -> u8 {
        if m == 0 {
            return 255;
        }
        let v = (px & m) >> shift(m);
        let max = m >> shift(m);
        if max == 255 {
            v as u8
        } else {
            (v * 255 / max.max(1)) as u8
        }
    };
    let mut out = vec![0u8; width as usize * height as usize * 4];
    let mut any_alpha = false;
    for y in 0..height as usize {
        let src_row = if top_down { y } else { height as usize - 1 - y };
        let row = &dib[offset + src_row * stride..offset + src_row * stride + stride];
        for x in 0..width as usize {
            let o = (y * width as usize + x) * 4;
            if bits == 24 {
                let p = &row[x * 3..x * 3 + 3];
                out[o..o + 4].copy_from_slice(&[p[2], p[1], p[0], 255]);
            } else {
                let px = u32::from_le_bytes(row[x * 4..x * 4 + 4].try_into().unwrap_or([0; 4]));
                let a = if compression == 0 {
                    (px >> 24) as u8
                } else {
                    take(px, amask)
                };
                any_alpha |= a != 0;
                out[o..o + 4].copy_from_slice(&[
                    take(px, rmask),
                    take(px, gmask),
                    take(px, bmask),
                    a,
                ]);
            }
        }
    }
    // Most programs leave the fourth byte of a 32-bit bitmap zero: that means opaque, not invisible.
    if bits == 32 && !any_alpha {
        for px in out.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }
    Ok((width, height, out))
}

/// RGBA pixels as PNG bytes (8-bit RGBA, no metadata).
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    check_size(w, h)?;
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// The size of a PNG from its header (and whether the header is sane).
pub fn png_size(data: &[u8]) -> Result<(u32, u32), String> {
    if sniff(data) != Some(Kind::Png) || data.len() < 24 {
        return Err("not a PNG image".into());
    }
    let w = u32::from_be_bytes(data[16..20].try_into().map_err(|_| "bad PNG header")?);
    let h = u32::from_be_bytes(data[20..24].try_into().map_err(|_| "bad PNG header")?);
    check_size(w, h)?;
    Ok((w, h))
}

/// What a paste of `img` writes as an asset: `(extension, bytes)`. A DIB becomes PNG; PNG and
/// image files are kept as they are after their signature and size are checked.
pub fn asset_bytes(img: &ClipImage) -> Result<Vec<(String, Vec<u8>)>, String> {
    match img {
        ClipImage::Png(b) => {
            if b.len() > MAX_BYTES {
                return Err("the image is too large".into());
            }
            png_size(b)?;
            Ok(vec![("png".into(), b.clone())])
        }
        ClipImage::Dib(b) => {
            if b.len() > MAX_BYTES {
                return Err("the image is too large".into());
            }
            let (w, h, rgba) = dib_to_rgba(b)?;
            Ok(vec![("png".into(), encode_png(w, h, &rgba)?)])
        }
        ClipImage::Files(files) => {
            let mut out = Vec::new();
            for f in files {
                let meta = std::fs::metadata(f).map_err(|e| format!("{}: {e}", f.display()))?;
                if meta.len() as usize > MAX_BYTES {
                    return Err(format!("{} is too large", f.display()));
                }
                let b = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
                let kind = sniff(&b)
                    .ok_or_else(|| format!("{} is not an image Jarak knows", f.display()))?;
                out.push((kind.ext().to_string(), b));
            }
            if out.is_empty() {
                return Err("no image files".into());
            }
            Ok(out)
        }
    }
}

/// Decodes an image file for the preview (PNG, JPEG, GIF's first frame, BMP) within the limits.
pub fn decode_file(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() as usize > MAX_BYTES {
        return Err("the image file is too large".into());
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    decode(&data)
}

/// Decodes image bytes (PNG, JPEG, GIF's first frame, BMP) within the limits.
pub fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let format = format_of(data)?;
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_PIXELS * 8);
    reader.limits(limits);
    let img = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader.decode()))
        .map_err(|_| "the image could not be decoded".to_string())?
        .map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    check_size(w, h)?;
    Ok((w, h, rgba.into_raw()))
}

/// The decoder for `data` by its first bytes.
fn format_of(data: &[u8]) -> Result<image::ImageFormat, String> {
    match sniff(data) {
        Some(Kind::Png) => Ok(image::ImageFormat::Png),
        Some(Kind::Jpeg) => Ok(image::ImageFormat::Jpeg),
        Some(Kind::Gif) => Ok(image::ImageFormat::Gif),
        Some(Kind::Bmp) => Ok(image::ImageFormat::Bmp),
        Some(Kind::Webp) => Ok(image::ImageFormat::WebP),
        None => Err("not an image Jarak knows".into()),
    }
}

/// A picture's size from its header (Step 91: the preview reserves its place before decoding).
pub fn dimensions(path: &Path) -> Result<[u32; 2], String> {
    use std::io::Read;
    let mut head = Vec::new();
    std::fs::File::open(path)
        .and_then(|f| f.take(64 * 1024).read_to_end(&mut head))
        .map_err(|e| e.to_string())?;
    let format = format_of(&head)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = image::ImageReader::with_format(std::io::BufReader::new(file), format);
    let (w, h) =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| reader.into_dimensions()))
            .map_err(|_| "the image could not be read".to_string())?
            .map_err(|e| e.to_string())?;
    check_size(w, h)?;
    Ok([w, h])
}

/// `rgba` (`w` × `h`) scaled down to at most `max` pixels on its longer side (unchanged when it
/// fits): textures stay within what every graphics card takes and within the cache's budget.
pub fn fit(w: u32, h: u32, rgba: Vec<u8>, max: u32) -> (u32, u32, Vec<u8>) {
    if w.max(h) <= max {
        return (w, h, rgba);
    }
    let Some(img) = image::RgbaImage::from_raw(w, h, rgba) else {
        return (0, 0, Vec::new());
    };
    let s = f64::from(max) / f64::from(w.max(h));
    let (nw, nh) = (
        ((f64::from(w) * s).round() as u32).max(1),
        ((f64::from(h) * s).round() as u32).max(1),
    );
    let out = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    (nw, nh, out.into_raw())
}

/// The UI audit's and the e2e's picture document (Step 91): see [`write_samples`].
pub const SAMPLE_DOC: &str = "# Pictures\n\nA wide picture, scaled down to the column:\n\n![wide](assets/wide.png)\n\nSmall ones at their size, PNG and WebP: ![small](assets/small.png) ![webp](assets/small.webp)\n\n![gone](assets/missing.png)\n\n![remote](https://example.com/r.png)\n";

/// Writes the pictures [`SAMPLE_DOC`] refers to into `dir`: `wide.png` (1600 × 400),
/// `small.png` (160 × 90) and `small.webp` (200 × 120); files that exist are kept.
pub fn write_samples(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let rgba = |w: u32, h: u32| dib_to_rgba(&sample_dib(w, h)).map(|(_, _, px)| px);
    for (name, w, h) in [("wide.png", 1600, 400), ("small.png", 160, 90)] {
        let p = dir.join(name);
        if !p.exists() {
            std::fs::write(&p, encode_png(w, h, &rgba(w, h)?)?).map_err(|e| e.to_string())?;
        }
    }
    let p = dir.join("small.webp");
    if !p.exists() {
        let mut out = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut out)
            .encode(&rgba(200, 120)?, 200, 120, image::ExtendedColorType::Rgba8)
            .map_err(|e| e.to_string())?;
        std::fs::write(&p, out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// A small packed 24-bit bitmap with a colour gradient (what a screenshot tool puts on the
/// clipboard as `CF_DIB`): for tests and the UI audit's `paste-image` view.
pub fn sample_dib(w: u32, h: u32) -> Vec<u8> {
    let stride = (w as usize * 3).div_ceil(4) * 4;
    let mut b = Vec::with_capacity(40 + stride * h as usize);
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(w as i32).to_le_bytes());
    b.extend_from_slice(&(h as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    for y in 0..h {
        let mut row = vec![0u8; stride];
        for x in 0..w as usize {
            let (r, g, bl) = (
                (x * 255 / w.max(1) as usize) as u8,
                (y * 255 / h.max(1)) as u8,
                0x9B,
            );
            row[x * 3..x * 3 + 3].copy_from_slice(&[bl, g, r]);
        }
        b.extend_from_slice(&row);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A packed `BITMAPINFOHEADER` DIB of `w` × `h` with `bits` bits per pixel.
    fn dib(w: i32, h: i32, bits: u16, compression: u32, pixels: &[u8], masks: &[u32]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&w.to_le_bytes());
        b.extend_from_slice(&h.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(&compression.to_le_bytes());
        b.extend_from_slice(&[0u8; 20]);
        for m in masks {
            b.extend_from_slice(&m.to_le_bytes());
        }
        b.extend_from_slice(pixels);
        b
    }

    #[test]
    fn a_24_bit_bottom_up_bitmap_becomes_rgba_top_down() {
        // 2 × 2: bottom row blue, red; top row green, white (rows padded to 8 bytes).
        let px = [
            255, 0, 0, 0, 0, 255, 0, 0, // bottom: blue (BGR 255,0,0), red (0,0,255)
            0, 255, 0, 255, 255, 255, 0, 0, // top: green, white
        ];
        let (w, h, rgba) = dib_to_rgba(&dib(2, 2, 24, 0, &px, &[])).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&rgba[0..4], &[0, 255, 0, 255], "top-left green");
        assert_eq!(&rgba[4..8], &[255, 255, 255, 255]);
        assert_eq!(&rgba[8..12], &[0, 0, 255, 255], "bottom-left blue");
        assert_eq!(&rgba[12..16], &[255, 0, 0, 255]);
    }

    #[test]
    fn a_32_bit_top_down_bitmap_keeps_alpha_and_zero_alpha_means_opaque() {
        let px = [10, 20, 30, 128, 40, 50, 60, 255];
        let (_, _, rgba) = dib_to_rgba(&dib(2, -1, 32, 0, &px, &[])).unwrap();
        assert_eq!(rgba, vec![30, 20, 10, 128, 60, 50, 40, 255]);
        let px = [10, 20, 30, 0, 40, 50, 60, 0];
        let (_, _, rgba) = dib_to_rgba(&dib(2, -1, 32, 0, &px, &[])).unwrap();
        assert_eq!(rgba[3], 255, "an all-zero fourth byte is opaque");
        // BI_BITFIELDS with the masks after the header.
        let px = [1, 2, 3, 4];
        let (_, _, rgba) = dib_to_rgba(&dib(
            1,
            1,
            32,
            3,
            &px,
            &[0x00FF_0000, 0x0000_FF00, 0x0000_00FF],
        ))
        .unwrap();
        assert_eq!(rgba, vec![3, 2, 1, 255]);
    }

    #[test]
    fn a_synthesized_dibv5_with_masks_after_the_header_is_not_shifted() {
        // A 124-byte V5 header (BI_BITFIELDS, masks inside it), then the masks again, then 2 × 1
        // pixels: red, blue.
        let mut b = vec![0u8; 124];
        b[0..4].copy_from_slice(&124u32.to_le_bytes());
        b[4..8].copy_from_slice(&2i32.to_le_bytes());
        b[8..12].copy_from_slice(&1i32.to_le_bytes());
        b[12..14].copy_from_slice(&1u16.to_le_bytes());
        b[14..16].copy_from_slice(&32u16.to_le_bytes());
        b[16..20].copy_from_slice(&3u32.to_le_bytes());
        for (i, m) in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF]
            .iter()
            .enumerate()
        {
            b[40 + i * 4..44 + i * 4].copy_from_slice(&m.to_le_bytes());
        }
        for m in [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF] {
            b.extend_from_slice(&m.to_le_bytes());
        }
        b.extend_from_slice(&[0, 0, 255, 0, 255, 0, 0, 0]);
        let (_, _, rgba) = dib_to_rgba(&b).unwrap();
        assert_eq!(
            rgba,
            vec![255, 0, 0, 255, 0, 0, 255, 255],
            "red then blue, no shift"
        );
    }

    #[test]
    fn malformed_and_huge_bitmaps_are_refused() {
        assert!(dib_to_rgba(b"short").is_err());
        assert!(
            dib_to_rgba(&dib(2, 2, 24, 0, &[0; 4], &[])).is_err(),
            "too few pixels"
        );
        assert!(
            dib_to_rgba(&dib(2, 2, 8, 0, &[0; 64], &[])).is_err(),
            "8-bit palettes"
        );
        assert!(
            dib_to_rgba(&dib(2, 2, 32, 1, &[0; 64], &[])).is_err(),
            "RLE"
        );
        let e = dib_to_rgba(&dib(100_000, 100_000, 32, 0, &[], &[])).unwrap_err();
        assert!(e.contains("pixels"), "{e}");
    }

    #[test]
    fn png_round_trip_signatures_and_asset_bytes() {
        let rgba = vec![255u8, 0, 0, 255, 0, 0, 255, 128];
        let png = encode_png(2, 1, &rgba).unwrap();
        assert_eq!(sniff(&png), Some(Kind::Png));
        assert_eq!(png_size(&png).unwrap(), (2, 1));
        assert_eq!(decode(&png).unwrap(), (2, 1, rgba));
        // A DIB becomes PNG; PNG stays as it is.
        let d = dib(1, 1, 24, 0, &[0, 0, 255, 0], &[]);
        let out = asset_bytes(&ClipImage::Dib(d)).unwrap();
        assert_eq!(out[0].0, "png");
        assert_eq!(decode(&out[0].1).unwrap().2, vec![255, 0, 0, 255]);
        assert_eq!(asset_bytes(&ClipImage::Png(png.clone())).unwrap()[0].1, png);
        assert!(asset_bytes(&ClipImage::Png(b"not png".to_vec())).is_err());
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), Some(Kind::Jpeg));
        assert_eq!(sniff(b"GIF89a.."), Some(Kind::Gif));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some(Kind::Webp));
        assert_eq!(sniff(b"hello"), None);
    }

    #[test]
    fn large_pictures_are_scaled_down_to_the_texture_limit_keeping_the_aspect() {
        let (w, h, px) = fit(5000, 10, vec![7u8; 5000 * 10 * 4], 4096);
        assert_eq!((w, h, px.len()), (4096, 8, 4096 * 8 * 4));
        let (w, h, _) = fit(30, 20, vec![0u8; 30 * 20 * 4], 4096);
        assert_eq!((w, h), (30, 20), "small pictures are kept as they are");
    }

    #[test]
    fn webp_pictures_decode_and_their_size_is_read_from_the_header() {
        let rgba: Vec<u8> = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 255, 1, 2, 3, 255, 4, 5, 6,
            255,
        ];
        let mut out = Vec::new();
        image::codecs::webp::WebPEncoder::new_lossless(&mut out)
            .encode(&rgba, 3, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(sniff(&out), Some(Kind::Webp));
        assert_eq!(decode(&out).unwrap(), (3, 2, rgba));
        let p = std::env::temp_dir().join(format!("jarak-webp-{}.webp", std::process::id()));
        std::fs::write(&p, &out).unwrap();
        assert_eq!(dimensions(&p).unwrap(), [3, 2]);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn the_sample_bitmap_is_a_valid_dib() {
        let (w, h, rgba) = dib_to_rgba(&sample_dib(5, 3)).unwrap();
        assert_eq!((w, h, rgba.len()), (5, 3, 60));
        assert!(rgba.chunks(4).all(|p| p[3] == 255));
    }

    #[test]
    fn copied_files_are_recognised_by_their_bytes_not_their_names() {
        let d = std::env::temp_dir().join(format!("jarak-images-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let real = d.join("photo.dat");
        std::fs::write(&real, encode_png(1, 1, &[1, 2, 3, 255]).unwrap()).unwrap();
        let fake = d.join("fake.png");
        std::fs::write(&fake, b"just text").unwrap();
        let out = asset_bytes(&ClipImage::Files(vec![real])).unwrap();
        assert_eq!(out[0].0, "png", "named .dat, but it is a PNG");
        assert!(asset_bytes(&ClipImage::Files(vec![fake])).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}

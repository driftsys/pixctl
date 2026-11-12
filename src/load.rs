//! Decode an image file to RGBA8.

use std::path::Path;

use anyhow::{bail, Context, Result};
use image::RgbaImage;

/// A decoded image in RGBA8 form.
pub struct Loaded {
    pub rgba: RgbaImage,
    /// True when at least one pixel has alpha below 255.
    pub has_alpha: bool,
}

/// Decode the image at `path` to RGBA8.
pub fn load(path: &Path) -> Result<Loaded> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if matches!(ext.as_deref(), Some("emf" | "wmf")) {
        bail!("unsupported image format (EMF/WMF): {}", path.display());
    }
    let rgba = image::open(path)
        .with_context(|| format!("cannot decode image {}", path.display()))?
        .to_rgba8();
    let has_alpha = rgba.pixels().any(|p| p.0[3] < 255);
    Ok(Loaded { rgba, has_alpha })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, GrayImage, ImageBuffer, Rgb, RgbImage, Rgba};
    use std::path::PathBuf;

    fn dims(l: &Loaded) -> (u32, u32) {
        (l.rgba.width(), l.rgba.height())
    }

    fn tmp(name: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(name);
        (dir, p)
    }

    fn rgb(w: u32, h: u32) -> RgbImage {
        RgbImage::from_pixel(w, h, Rgb([10, 120, 200]))
    }

    #[test]
    fn loads_png_with_alpha() {
        let (_d, p) = tmp("a.png");
        let mut img = image::RgbaImage::from_pixel(4, 3, Rgba([1, 2, 3, 255]));
        img.put_pixel(0, 0, Rgba([1, 2, 3, 0]));
        img.save(&p).unwrap();
        let l = load(&p).unwrap();
        assert_eq!((l.rgba.width(), l.rgba.height(), l.has_alpha), (4, 3, true));
    }

    #[test]
    fn opaque_rgba_png_has_no_alpha() {
        let (_d, p) = tmp("a.png");
        image::RgbaImage::from_pixel(4, 3, Rgba([1, 2, 3, 255]))
            .save(&p)
            .unwrap();
        assert!(!load(&p).unwrap().has_alpha);
    }

    #[test]
    fn loads_jpeg() {
        let (_d, p) = tmp("a.jpg");
        rgb(5, 4).save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (5, 4));
    }

    #[test]
    fn loads_tiff() {
        let (_d, p) = tmp("a.tiff");
        rgb(5, 4).save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (5, 4));
    }

    #[test]
    fn loads_bmp() {
        let (_d, p) = tmp("a.bmp");
        rgb(5, 4).save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (5, 4));
    }

    #[test]
    fn loads_webp_lossless() {
        let (_d, p) = tmp("a.webp");
        rgb(5, 4).save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (5, 4));
    }

    #[test]
    fn loads_16bit_png() {
        let (_d, p) = tmp("a16.png");
        let img: ImageBuffer<Rgba<u16>, Vec<u16>> =
            ImageBuffer::from_pixel(6, 2, Rgba([1000, 2000, 3000, 65535]));
        DynamicImage::ImageRgba16(img).save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (6, 2));
    }

    #[test]
    fn loads_palette_gif() {
        let (_d, p) = tmp("a.gif");
        let mut img = rgb(4, 4);
        img.put_pixel(0, 0, Rgb([255, 0, 0]));
        img.save(&p).unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (4, 4));
    }

    #[test]
    fn loads_greyscale_jpeg() {
        let (_d, p) = tmp("g.jpg");
        GrayImage::from_pixel(7, 3, image::Luma([128]))
            .save(&p)
            .unwrap();
        assert_eq!(dims(&load(&p).unwrap()), (7, 3));
    }

    #[test]
    fn emf_is_unsupported() {
        let (_d, p) = tmp("a.emf");
        std::fs::write(&p, b"").unwrap();
        let err = load(&p).err().expect("emf must fail");
        assert!(format!("{err:#}").contains("unsupported"));
    }

    #[test]
    fn wmf_and_upper_case_extensions_are_unsupported() {
        for name in ["a.wmf", "b.EMF", "c.Wmf"] {
            let (_d, p) = tmp(name);
            std::fs::write(&p, b"").unwrap();
            let err = load(&p).err().expect("emf/wmf must fail");
            assert!(format!("{err:#}").contains("unsupported"), "{name}");
        }
    }

    #[test]
    fn missing_file_error_names_the_path() {
        let (_d, p) = tmp("nope.png");
        let err = load(&p).err().expect("missing file must fail");
        assert!(format!("{err:#}").contains("nope.png"));
    }

    #[test]
    fn truncated_png_errors() {
        let (_d, p) = tmp("full.png");
        rgb(8, 8).save(&p).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        let (_d2, cut) = tmp("cut.png");
        std::fs::write(&cut, &bytes[..20]).unwrap();
        assert!(load(&cut).is_err());
    }
}

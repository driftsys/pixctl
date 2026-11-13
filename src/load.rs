//! Decode an image file to RGBA8.

use std::path::Path;

use crate::output::Failure;
use image::{GrayImage, Luma, RgbaImage};

/// A decoded image in RGBA8 form.
pub struct Loaded {
    pub rgba: RgbaImage,
    /// True when at least one pixel has alpha below 255.
    pub has_alpha: bool,
}

/// Decode the image at `path` to RGBA8.
///
/// Fails with `unsupported_type` for EMF/WMF and `read_failed` when the file
/// cannot be opened or decoded.
pub fn load(path: &Path) -> Result<Loaded, Failure> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    if matches!(ext.as_deref(), Some("emf" | "wmf")) {
        return Err(Failure {
            code: "unsupported_type",
            message: format!("unsupported image format (EMF/WMF): {}", path.display()),
            file: Some(path.to_string_lossy().into_owned()),
            hint: None,
            with: None,
        });
    }
    let rgba = image::open(path)
        .map_err(|e| Failure {
            code: "read_failed",
            message: format!("cannot decode image {}: {e}", path.display()),
            file: Some(path.to_string_lossy().into_owned()),
            hint: None,
            with: None,
        })?
        .to_rgba8();
    let has_alpha = rgba.pixels().any(|p| p.0[3] < 255);
    Ok(Loaded { rgba, has_alpha })
}

/// Composites RGBA onto white, then converts to Rec. 601 luma.
pub fn flatten_to_gray(rgba: &RgbaImage) -> GrayImage {
    GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let a = a as f32 / 255.0;
        let over_white = |c: u8| c as f32 * a + 255.0 * (1.0 - a);
        let luma = 0.299 * over_white(r) + 0.587 * over_white(g) + 0.114 * over_white(b);
        Luma([luma.round().clamp(0.0, 255.0) as u8])
    })
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
    fn load_failures_carry_codes() {
        let (_d, p) = tmp("a.emf");
        std::fs::write(&p, b"").unwrap();
        let f = load(&p).err().expect("emf must fail");
        assert_eq!(f.code, "unsupported_type");
        assert_eq!(f.file.as_deref(), Some(p.to_str().unwrap()));
        let (_d, p) = tmp("nope.png");
        let f = load(&p).err().expect("missing file must fail");
        assert_eq!(f.code, "read_failed");
        assert_eq!(f.file.as_deref(), Some(p.to_str().unwrap()));
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

    #[test]
    fn flatten_composites_onto_white() {
        let mut img = RgbaImage::new(3, 1);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
        img.put_pixel(1, 0, Rgba([0, 0, 0, 255]));
        img.put_pixel(2, 0, Rgba([0, 0, 0, 128]));
        let gray = flatten_to_gray(&img);
        assert_eq!(gray.get_pixel(0, 0).0[0], 255);
        assert_eq!(gray.get_pixel(1, 0).0[0], 0);
        assert_eq!(gray.get_pixel(2, 0).0[0], 127);
    }
}

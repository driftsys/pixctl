//! The pure part of `fit`: render an image to the display contract and
//! encode it as a PNG.

use crate::contract::{FRAME, FRAME_COLOUR, PADDING};
use crate::rule::{Plan, Rule};
use anyhow::{Context, Result};
use image::codecs::png::PngEncoder;
use image::imageops::{self, FilterType};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, RgbaImage};

/// Flattens `rgba` onto white and scales it by the plan's factor. Unless the
/// rule is `Icon`, it then adds the padding and the frame.
pub fn render(rgba: &RgbaImage, plan: &Plan) -> RgbImage {
    let flat = flatten(rgba);
    let scaled = if plan.factor == 1.0 {
        flat
    } else {
        let filter = if plan.factor == 2.0 {
            FilterType::Nearest
        } else {
            FilterType::Lanczos3
        };
        let w = (flat.width() as f32 * plan.factor).round() as u32;
        let h = (flat.height() as f32 * plan.factor).round() as u32;
        imageops::resize(&flat, w, h, filter)
    };
    if plan.rule == Rule::Icon {
        return scaled;
    }
    let band = PADDING + FRAME;
    let mut out = RgbImage::from_pixel(
        scaled.width() + 2 * band,
        scaled.height() + 2 * band,
        Rgb([255, 255, 255]),
    );
    imageops::replace(&mut out, &scaled, band as i64, band as i64);
    let (w, h) = out.dimensions();
    for (x, y, p) in out.enumerate_pixels_mut() {
        if x < FRAME || y < FRAME || x >= w - FRAME || y >= h - FRAME {
            *p = Rgb(FRAME_COLOUR);
        }
    }
    out
}

/// Composites RGBA onto white.
fn flatten(rgba: &RgbaImage) -> RgbImage {
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let a = a as f32 / 255.0;
        let over_white = |c: u8| (c as f32 * a + 255.0 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        Rgb([over_white(r), over_white(g), over_white(b)])
    })
}

/// Encodes `img` as a PNG with a fixed oxipng preset, no metadata and no
/// interlacing, so the same image always gives the same bytes.
pub fn encode_png(img: &RgbImage) -> Result<Vec<u8>> {
    let mut raw = Vec::new();
    PngEncoder::new(&mut raw)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgb8,
        )
        .context("cannot encode PNG")?;
    let mut opts = oxipng::Options::from_preset(2);
    opts.strip = oxipng::StripChunks::All;
    opts.interlace = Some(false);
    oxipng::optimize_from_memory(&raw, &opts).context("cannot optimise PNG")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule;
    use image::Rgba;

    fn white(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]))
    }

    fn plan_of(rule: Rule, factor: f32, w: u32) -> Plan {
        Plan {
            rule,
            factor,
            output_width: w,
        }
    }

    #[test]
    fn unchanged_adds_border() {
        let img = white(300, 200);
        let out = render(&img, &rule::plan(300, 200, None));
        assert_eq!(out.dimensions(), (334, 234));
        assert_eq!(out.get_pixel(0, 0).0, FRAME_COLOUR);
        assert_eq!(out.get_pixel(1, 1).0, [255, 255, 255]);
    }

    #[test]
    fn upscale_2x_nearest() {
        let mut img = white(300, 200);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        let out = render(&img, &plan_of(Rule::Upscale, 2.0, 634));
        assert_eq!(out.dimensions(), (634, 434));
        for (x, y) in [(17, 17), (18, 17), (17, 18), (18, 18)] {
            assert_eq!(out.get_pixel(x, y).0, [0, 0, 0], "({x},{y})");
        }
    }

    #[test]
    fn icon_only_flattens() {
        let mut img = white(32, 32);
        img.put_pixel(3, 4, Rgba([0, 0, 0, 0]));
        let out = render(&img, &rule::plan(32, 32, None));
        assert_eq!(out.dimensions(), (32, 32));
        assert_eq!(out.get_pixel(3, 4).0, [255, 255, 255]);
    }

    #[test]
    fn alpha_flattens_onto_white() {
        let mut img = white(32, 32);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 128]));
        let out = render(&img, &rule::plan(32, 32, None));
        let v = out.get_pixel(0, 0).0[0];
        assert!(v == 127 || v == 128, "{v}");
    }

    #[test]
    fn encode_is_deterministic() {
        let mut img = RgbImage::from_pixel(40, 30, Rgb([255, 255, 255]));
        img.put_pixel(5, 5, Rgb([1, 2, 3]));
        let a = encode_png(&img).unwrap();
        let b = encode_png(&img).unwrap();
        assert_eq!(a, b);
        let back = image::load_from_memory(&a).unwrap().to_rgb8();
        assert_eq!(back, img);
        for chunk in [b"tEXt", b"iTXt", b"zTXt", b"tIME"] {
            assert!(!a.windows(4).any(|w| w == chunk), "{chunk:?}");
        }
    }
}

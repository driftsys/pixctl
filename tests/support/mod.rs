//! Synthetic image fixtures for integration tests.
//!
//! Each test crate includes this module with `mod support;`, and uses only a
//! subset of the helpers, so the items are allowed to be unused.
#![allow(dead_code)]

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{ExtendedColorType, GrayImage, Luma};

const DEJAVU: &[u8] = include_bytes!("../fonts/DejaVuSans.ttf");
const LIBERATION: &[u8] = include_bytes!("../fonts/LiberationSans-Regular.ttf");
const NOTO: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// The text drawn on every line of a rendered fixture.
pub const PANGRAM: &str = "The quick brown fox jumps over the lazy dog 0123456789";

#[derive(Clone, Copy, Debug)]
pub enum Face {
    DejaVu,
    Liberation,
    Noto,
}

impl Face {
    fn bytes(self) -> &'static [u8] {
        match self {
            Face::DejaVu => DEJAVU,
            Face::Liberation => LIBERATION,
            Face::Noto => NOTO,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextSpec {
    pub face: Face,
    /// CSS-like em size in pixels (the font's units-per-em, scaled).
    pub em_px: f32,
    pub lines: usize,
    /// White text on a black background instead of black on white.
    pub invert: bool,
}

/// Renders `spec.lines` identical lines of [`PANGRAM`].
///
/// The margin is `2 * em_px` on every side and the line pitch is
/// `1.4 * em_px`.
pub fn render_text(spec: &TextSpec) -> GrayImage {
    let font = FontRef::try_from_slice(spec.face.bytes()).expect("embedded font parses");
    // `PxScale` is the font height (ascent - descent) in pixels, not the em
    // size. Convert so that `em_px` maps to one em.
    let units_per_em = font.units_per_em().expect("font has units-per-em");
    let scale = PxScale::from(spec.em_px * font.height_unscaled() / units_per_em);
    let scaled = font.as_scaled(scale);

    let margin = 2.0 * spec.em_px;
    let pitch = 1.4 * spec.em_px;
    let lines = spec.lines.max(1);

    // Glyph positions along one line: pen x, with kerning between neighbours.
    let mut glyphs = Vec::new();
    let mut pen = 0.0_f32;
    let mut previous = None;
    for ch in PANGRAM.chars() {
        let id = scaled.glyph_id(ch);
        if let Some(prev) = previous {
            pen += scaled.kern(prev, id);
        }
        glyphs.push((id, pen));
        pen += scaled.h_advance(id);
        previous = Some(id);
    }
    let text_width = pen;

    let width = (text_width + 2.0 * margin).ceil() as u32;
    let height = (2.0 * margin + (lines - 1) as f32 * pitch + scaled.height()).ceil() as u32;

    let (background, foreground) = if spec.invert {
        (0.0, 255.0)
    } else {
        (255.0, 0.0)
    };
    let mut img = GrayImage::from_pixel(width, height, Luma([background as u8]));

    for line in 0..lines {
        let baseline = margin + scaled.ascent() + line as f32 * pitch;
        for &(id, x) in &glyphs {
            let glyph = id.with_scale_and_position(scale, point(margin + x, baseline));
            let Some(outline) = font.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i64 + gx as i64;
                let py = bounds.min.y as i64 + gy as i64;
                if px < 0 || py < 0 || px >= width as i64 || py >= height as i64 {
                    return;
                }
                let pixel = img.get_pixel_mut(px as u32, py as u32);
                // Overlapping glyph boxes keep the stronger coverage.
                let current = (pixel.0[0] as f32 - background) / (foreground - background);
                let c = coverage.clamp(0.0, 1.0).max(current);
                pixel.0[0] = (background + (foreground - background) * c).round() as u8;
            });
        }
    }
    img
}

/// Encodes `img` as JPEG at `quality` and decodes it again.
pub fn jpeg_roundtrip(img: &GrayImage, quality: u8) -> GrayImage {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::L8,
        )
        .expect("jpeg encode");
    image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
        .expect("jpeg decode")
        .to_luma8()
}

/// Scales both dimensions by `factor` with a Lanczos3 filter.
pub fn resize(img: &GrayImage, factor: f32) -> GrayImage {
    let w = ((img.width() as f32 * factor).round() as u32).max(1);
    let h = ((img.height() as f32 * factor).round() as u32).max(1);
    imageops::resize(img, w, h, FilterType::Lanczos3)
}

/// Deterministic linear congruential generator (Knuth MMIX constants).
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    /// Next value in `[0, 1)`, from the high 24 bits of the state.
    fn next_unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Uniform per-pixel noise, fully determined by `seed`.
pub fn noise(w: u32, h: u32, seed: u64) -> GrayImage {
    let mut rng = Lcg::new(seed);
    GrayImage::from_fn(w, h, |_, _| Luma([(rng.next_unit() * 256.0) as u8]))
}

/// Smooth low-frequency gradients plus mild noise, with no text-like detail.
pub fn photo_like(w: u32, h: u32, seed: u64) -> GrayImage {
    let mut rng = Lcg::new(seed);
    // Each wave: (cycles across x, cycles across y, phase).
    let waves: Vec<(f32, f32, f32)> = (0..3)
        .map(|_| {
            (
                0.3 + 1.2 * rng.next_unit(),
                0.3 + 1.2 * rng.next_unit(),
                std::f32::consts::TAU * rng.next_unit(),
            )
        })
        .collect();
    GrayImage::from_fn(w, h, |x, y| {
        let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
        let smooth: f32 = waves
            .iter()
            .map(|&(fx, fy, phase)| (std::f32::consts::TAU * (fx * u + fy * v) + phase).sin() / 3.0)
            .sum();
        let value = 128.0 + 90.0 * smooth + 6.0 * (rng.next_unit() - 0.5);
        Luma([value.clamp(0.0, 255.0) as u8])
    })
}

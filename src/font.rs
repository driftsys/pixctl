//! Estimates the font size of the text in a raster image.
//!
//! The estimate is the median height of the text lines, scaled by [`K`].

use image::{GrayImage, Luma};
use imageproc::contrast::otsu_level;
use imageproc::region_labelling::{connected_components, Connectivity};
use serde::Serialize;

/// Ratio of the CSS em size to the measured line height.
///
/// Calibrated on rendered pangram fixtures (6 lines, normal and inverted,
/// em sizes 11, 14, 20 and 24 px). Ratio of median line height to em size:
///
/// | face       | min    | median | max    |
/// |------------|--------|--------|--------|
/// | DejaVu     | 0.9583 | 1.0000 | 1.0000 |
/// | Liberation | 0.9000 | 0.9435 | 0.9583 |
/// | Noto       | 1.0000 | 1.0000 | 1.0455 |
/// | all        | 0.9000 | 1.0000 | 1.0455 |
///
/// The line height spans from the ascender top to the descender bottom,
/// which is close to one em for these faces. Liberation lines are about 5 %
/// shorter than the other two faces.
///
/// Rule: `K` is the midpoint of the interval of `K` values that keeps every
/// one of these fixtures within 1.5 px of its em size, rounded to 3
/// decimals. The lower bound is 18.5 / 18 = 1.0278 (Liberation at 20 px,
/// line height 18 px). The upper bound is 25.5 / 24 = 1.0625 (Noto at 24
/// px, line height 24 px). The
/// midpoint is 1.0451.
pub const K: f32 = 1.045;
/// Fewest text lines needed for a high-confidence estimate.
pub const MIN_LINES: usize = 3;
/// Fewest glyph-like components needed for a high-confidence estimate.
pub const MIN_GLYPHS: usize = 20;
/// Largest coefficient of variation of the line heights for a
/// high-confidence estimate.
pub const MAX_CV: f32 = 0.35;

/// How much the estimate can be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Low,
}

/// Result of [`estimate`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FontEstimate {
    /// Estimated em size in pixels, or `None` when no text line was found.
    pub px: Option<f32>,
    pub confidence: Confidence,
    /// Number of text lines found.
    pub lines: usize,
    /// Number of glyph-like components found.
    pub glyphs: usize,
}

/// Vertical extent of a glyph or line: first and last row, inclusive.
#[derive(Clone, Copy)]
struct Span {
    top: u32,
    bottom: u32,
}

/// Estimates the font size of the text in `img`.
pub fn estimate(img: &GrayImage) -> FontEstimate {
    let glyphs = glyph_spans(img);
    let lines = group_lines(&glyphs);
    let mut heights: Vec<f32> = lines
        .iter()
        .map(|l| (l.bottom - l.top + 1) as f32)
        .collect();
    let Some(median) = median(&mut heights) else {
        return FontEstimate {
            px: None,
            confidence: Confidence::Low,
            lines: 0,
            glyphs: glyphs.len(),
        };
    };
    let confident = lines.len() >= MIN_LINES
        && glyphs.len() >= MIN_GLYPHS
        && coefficient_of_variation(&heights) <= MAX_CV;
    FontEstimate {
        px: Some(median * K),
        confidence: if confident {
            Confidence::High
        } else {
            Confidence::Low
        },
        lines: lines.len(),
        glyphs: glyphs.len(),
    }
}

/// Binarises `img` with Otsu's threshold and returns the vertical spans of
/// the glyph-like 8-connected components of the minority class.
fn glyph_spans(img: &GrayImage) -> Vec<Span> {
    let level = otsu_level(img);
    let above = img.pixels().filter(|p| p.0[0] > level).count();
    // The majority class is the background.
    let dark_text = above * 2 >= img.pixels().len();
    let binary = GrayImage::from_fn(img.width(), img.height(), |x, y| {
        let is_above = img.get_pixel(x, y).0[0] > level;
        Luma([if is_above != dark_text { 255 } else { 0 }])
    });
    let labels = connected_components(&binary, Connectivity::Eight, Luma([0u8]));

    // Per label: min x, min y, max x, max y, area.
    let mut boxes: Vec<(u32, u32, u32, u32, u32)> = Vec::new();
    for (x, y, label) in labels.enumerate_pixels() {
        let id = label.0[0] as usize;
        if id == 0 {
            continue;
        }
        if boxes.len() < id {
            boxes.resize(id, (u32::MAX, u32::MAX, 0, 0, 0));
        }
        let b = &mut boxes[id - 1];
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
        b.4 += 1;
    }
    boxes
        .into_iter()
        .filter(|b| b.4 >= 6)
        .filter_map(|(x0, y0, x1, y1, _)| {
            let w = (x1 - x0 + 1) as f32;
            let h = y1 - y0 + 1;
            let aspect = w / h as f32;
            ((4..=200).contains(&h) && (0.1..=3.0).contains(&aspect)).then_some(Span {
                top: y0,
                bottom: y1,
            })
        })
        .collect()
}

/// Groups glyphs whose vertical spans overlap into lines.
fn group_lines(glyphs: &[Span]) -> Vec<Span> {
    let mut sorted = glyphs.to_vec();
    sorted.sort_by_key(|s| s.top);
    let mut lines: Vec<Span> = Vec::new();
    for g in sorted {
        match lines.last_mut() {
            Some(line) if g.top <= line.bottom => line.bottom = line.bottom.max(g.bottom),
            _ => lines.push(g),
        }
    }
    lines
}

fn median(values: &mut [f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

fn coefficient_of_variation(values: &[f32]) -> f32 {
    let n = values.len() as f32;
    let mean = values.iter().sum::<f32>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    variance.sqrt() / mean
}

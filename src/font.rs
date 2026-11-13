//! Estimates the font size of the text in a raster image.
//!
//! The estimate is the median height of the text lines, scaled by [`K`].

use image::{GrayImage, Luma};
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
/// | Liberation | 0.9000 | 0.9416 | 0.9583 |
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
/// line height 18 px). The upper bound is 25.5 / 24 = 1.0625 (Noto at
/// 24 px, line height 24 px). The midpoint is 1.0451.
pub const K: f32 = 1.045;
/// Fewest text lines needed for a high-confidence estimate.
pub const MIN_LINES: usize = 3;
/// Fewest glyph-like components needed for a high-confidence estimate.
pub const MIN_GLYPHS: usize = 20;
/// Coefficient of variation of the line heights that a high-confidence
/// estimate must stay below.
pub const MAX_CV: f32 = 0.35;

/// How much the estimate can be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Low,
}

/// The font size that may drive a decision: the estimate when its confidence
/// is high, else `None`. A low-confidence estimate is never used.
pub fn usable_px(px: Option<f32>, confidence: Confidence) -> Option<f32> {
    match confidence {
        Confidence::High => px,
        Confidence::Low => None,
    }
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
        && coefficient_of_variation(&heights) < MAX_CV;
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

/// Otsu threshold of `img`, with the same result as
/// `imageproc::contrast::otsu_level`: pixels above the level form one class.
fn otsu_level(img: &GrayImage) -> u8 {
    let mut hist = [0u64; 256];
    for p in img.pixels() {
        hist[p.0[0] as usize] += 1;
    }
    otsu_from_histogram(&hist)
}

/// Otsu threshold of a 256-bin histogram. Counts are `u64` and sums are
/// `f64`, so no image size overflows the computation.
fn otsu_from_histogram(hist: &[u64; 256]) -> u8 {
    let total: u64 = hist.iter().sum();
    let total_sum: f64 = hist
        .iter()
        .enumerate()
        .map(|(t, &h)| t as f64 * h as f64)
        .sum();
    let mut background_weight = 0u64;
    let mut background_sum = 0f64;
    let mut largest_variance = 0f64;
    let mut best = 0u8;
    for (t, &h) in hist.iter().enumerate() {
        background_weight += h;
        if background_weight == 0 {
            continue;
        }
        let foreground_weight = total - background_weight;
        if foreground_weight == 0 {
            break;
        }
        background_sum += t as f64 * h as f64;
        let background_mean = background_sum / background_weight as f64;
        let foreground_mean = (total_sum - background_sum) / foreground_weight as f64;
        let variance = background_weight as f64
            * foreground_weight as f64
            * (background_mean - foreground_mean).powi(2);
        if variance > largest_variance {
            largest_variance = variance;
            best = t as u8;
        }
    }
    best
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

#[cfg(test)]
mod tests {
    #[test]
    fn usable_px_drops_a_low_confidence_estimate() {
        assert_eq!(usable_px(Some(9.0), Confidence::High), Some(9.0));
        assert_eq!(usable_px(Some(9.0), Confidence::Low), None);
        assert_eq!(usable_px(None, Confidence::High), None);
    }

    use super::*;

    #[test]
    fn otsu_matches_imageproc_on_normal_images() {
        let bimodal = GrayImage::from_fn(97, 61, |x, y| {
            let dark = (x / 7 + y / 5) % 3 == 0;
            Luma([if dark {
                40 + (x % 9) as u8
            } else {
                210 - (y % 13) as u8
            }])
        });
        let gradient = GrayImage::from_fn(256, 3, |x, y| Luma([(x as u8).wrapping_add(y as u8)]));
        for img in [bimodal, gradient] {
            assert_eq!(otsu_level(&img), imageproc::contrast::otsu_level(&img));
        }
    }

    /// White image with black rectangles `(x, y, w, h)`.
    fn boxes(w: u32, h: u32, rects: &[(u32, u32, u32, u32)]) -> GrayImage {
        let mut img = GrayImage::from_pixel(w, h, Luma([255]));
        for &(x, y, rw, rh) in rects {
            for yy in y..y + rh {
                for xx in x..x + rw {
                    img.put_pixel(xx, yy, Luma([0]));
                }
            }
        }
        img
    }

    /// `rows` lines of `per_row` 6 x 10 px blocks.
    fn grid(rows: u32, per_row: u32) -> GrayImage {
        let rects: Vec<_> = (0..rows)
            .flat_map(|r| (0..per_row).map(move |c| (4 + c * 10, 4 + r * 20, 6, 10)))
            .collect();
        boxes(4 + per_row * 10 + 4, 4 + rows * 20 + 4, &rects)
    }

    #[test]
    fn fewer_than_20_glyphs_is_low() {
        let est = estimate(&grid(3, 6));
        assert_eq!((est.lines, est.glyphs), (3, 18));
        assert_eq!(est.confidence, Confidence::Low);
        let est = estimate(&grid(3, 7));
        assert_eq!((est.lines, est.glyphs), (3, 21));
        assert_eq!(est.confidence, Confidence::High);
    }

    #[test]
    fn three_lines_is_enough() {
        let est = estimate(&grid(2, 12));
        assert_eq!((est.lines, est.confidence), (2, Confidence::Low));
        let est = estimate(&grid(3, 12));
        assert_eq!((est.lines, est.confidence), (3, Confidence::High));
    }

    #[test]
    fn glyph_filters() {
        // Each case: one component that is (true) or is not (false) a glyph.
        for (rect, kept) in [
            ((10, 10, 6, 10), true),    // plain glyph
            ((10, 10, 2, 4), true),     // area 8, height 4
            ((10, 10, 1, 5), false),    // area 5
            ((10, 10, 3, 3), false),    // height 3
            ((10, 10, 21, 200), true),  // height 200
            ((10, 10, 21, 201), false), // height 201
            ((10, 10, 12, 4), true),    // aspect 3
            ((10, 10, 13, 4), false),   // aspect 3.25
            ((10, 10, 1, 10), true),    // aspect 0.1
            ((10, 10, 1, 11), false),   // aspect 0.09
        ] {
            let est = estimate(&boxes(60, 230, &[rect]));
            assert_eq!(est.glyphs, usize::from(kept), "{rect:?}");
        }
    }

    #[test]
    fn touching_glyphs_share_a_line() {
        // The second glyph starts on the last row of the first one.
        let est = estimate(&boxes(40, 40, &[(4, 4, 6, 10), (20, 13, 6, 10)]));
        assert_eq!((est.glyphs, est.lines), (2, 1));
    }

    #[test]
    fn otsu_handles_counts_above_u32() {
        // 17 million white pixels: the sum of intensity x count is
        // 255 x 17e6 = 4.3e9, above u32::MAX.
        let mut hist = [0u64; 256];
        hist[255] = 17_000_000;
        hist[10] = 1_000_000;
        let level = otsu_from_histogram(&hist);
        assert!((10..255).contains(&level), "{level}");
    }
}

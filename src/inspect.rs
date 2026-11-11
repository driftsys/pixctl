//! The `inspect` command: size, alpha, font estimate and rule per image.

use crate::font::{self, Confidence};
use crate::load;
use crate::rule::{self, Rule};
use anyhow::Result;
use image::{GrayImage, Luma};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSource {
    Estimate,
    Override,
}

#[derive(Debug, Serialize)]
pub struct FontReport {
    pub px: Option<f32>,
    pub confidence: Confidence,
    pub lines: usize,
    pub glyphs: usize,
    pub source: FontSource,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub alpha: bool,
    pub font: FontReport,
    pub rule: Rule,
    pub factor: f32,
    pub output_width: u32,
}

/// Inspects every image. Returns `0`, or `2` if any image failed to load.
///
/// Errors go to `err` with the path; the other images are still reported.
pub fn run(
    images: &[PathBuf],
    json: bool,
    font_px: Option<f32>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut reports = Vec::new();
    let mut code = 0;
    for path in images {
        match inspect(path, font_px) {
            Ok(report) => reports.push(report),
            Err(e) => {
                let _ = writeln!(err, "pixctl: {e:#}");
                code = 2;
            }
        }
    }
    let written = if json {
        serde_json::to_writer_pretty(&mut *out, &reports)
            .map_err(anyhow::Error::from)
            .and_then(|()| writeln!(out).map_err(anyhow::Error::from))
    } else {
        reports
            .iter()
            .try_for_each(|r| writeln!(out, "{}", plain_line(r)))
            .map_err(anyhow::Error::from)
    };
    if let Err(e) = written {
        let _ = writeln!(err, "pixctl: {e:#}");
        code = 2;
    }
    code
}

fn inspect(path: &Path, font_px: Option<f32>) -> Result<Report> {
    let loaded = load::load(path)?;
    let (width, height) = loaded.rgba.dimensions();
    let font = match font_px {
        Some(px) => FontReport {
            px: Some(px),
            confidence: Confidence::High,
            lines: 0,
            glyphs: 0,
            source: FontSource::Override,
        },
        None => {
            let e = font::estimate(&flatten_to_gray(&loaded.rgba));
            FontReport {
                px: e.px,
                confidence: e.confidence,
                lines: e.lines,
                glyphs: e.glyphs,
                source: FontSource::Estimate,
            }
        }
    };
    // A low-confidence estimate never drives the rule.
    let usable = match font.confidence {
        Confidence::High => font.px,
        Confidence::Low => None,
    };
    let plan = rule::plan(width, height, usable);
    Ok(Report {
        path: path.to_path_buf(),
        width,
        height,
        alpha: loaded.has_alpha,
        font,
        rule: plan.rule,
        factor: plan.factor,
        output_width: plan.output_width,
    })
}

/// Composites RGBA onto white, then converts to Rec. 601 luma.
fn flatten_to_gray(rgba: &image::RgbaImage) -> GrayImage {
    GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let a = a as f32 / 255.0;
        let over_white = |c: u8| c as f32 * a + 255.0 * (1.0 - a);
        let luma = 0.299 * over_white(r) + 0.587 * over_white(g) + 0.114 * over_white(b);
        Luma([luma.round().clamp(0.0, 255.0) as u8])
    })
}

fn plain_line(r: &Report) -> String {
    let font = match r.font.px {
        Some(px) => format!("{px:.1}px"),
        None => "?px".to_string(),
    };
    let confidence = match r.font.confidence {
        Confidence::High => "high",
        Confidence::Low => "low",
    };
    let rule = serde_json::to_value(r.rule)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    format!(
        "{}  {}x{}  alpha={}  font={} ({})  rule={} x{} -> {}px",
        r.path.display(),
        r.width,
        r.height,
        r.alpha,
        font,
        confidence,
        rule,
        r.factor,
        r.output_width
    )
}

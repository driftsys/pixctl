//! The `fit` command: writes one fitted PNG per input image.

use crate::font;
use crate::{fit, load, rule};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Fits every image into `out_dir`. Returns `0`, or `2` if any image failed.
///
/// Errors go to `err` as `pixctl: <reason>`, and the reason names the path.
/// The other images are still processed. If the output directory cannot be created, the error is
/// reported once and no image is processed.
pub fn run(
    images: &[PathBuf],
    out_dir: &Path,
    font_px: Option<f32>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        let _ = writeln!(err, "pixctl: cannot create {}: {e}", out_dir.display());
        return 2;
    }
    let mut code = 0;
    let mut written = HashSet::new();
    for path in images {
        match fit_one(path, images, out_dir, font_px, &mut written) {
            Ok(line) => {
                if let Err(e) = writeln!(out, "{line}") {
                    let _ = writeln!(err, "pixctl: {e:#}");
                    code = 2;
                }
            }
            Err(e) => {
                let _ = writeln!(err, "pixctl: {e:#}");
                code = 2;
            }
        }
    }
    code
}

/// Loads one image and writes its fitted PNG. A load error already names the
/// path; every other error, including a missing file stem, is prefixed with it.
fn fit_one(
    path: &Path,
    inputs: &[PathBuf],
    out_dir: &Path,
    font_px: Option<f32>,
    written: &mut HashSet<PathBuf>,
) -> Result<String> {
    let loaded = load::load(path)?;
    let (w, h) = loaded.rgba.dimensions();
    let plan = rule::plan(w, h, chosen_font(&loaded.rgba, font_px));
    let stem = path
        .file_stem()
        .with_context(|| format!("{}: the file name has no stem", path.display()))?;
    let target = {
        let mut name = stem.to_os_string();
        name.push(".png");
        out_dir.join(name)
    };
    (|| -> Result<String> {
        guard_target(&target, out_dir, inputs, written)?;
        let img = fit::render(&loaded.rgba, &plan);
        let bytes = fit::encode_png(&img)?;
        std::fs::write(&target, bytes)
            .with_context(|| format!("cannot write {}", target.display()))?;
        Ok(format!(
            "FIT  [{}] {}  {} ({}×{})",
            rule_name(plan.rule),
            path.display(),
            target.display(),
            img.width(),
            img.height()
        ))
    })()
    .with_context(|| path.display().to_string())
}

/// Refuses a target that would overwrite an input, reached directly, through
/// a hard link or through a symbolic link, or that has the same name (ignoring
/// case) as an output already written in this run.
fn guard_target(
    target: &Path,
    out_dir: &Path,
    inputs: &[PathBuf],
    written: &mut HashSet<PathBuf>,
) -> Result<()> {
    if target.symlink_metadata().is_ok() {
        // `exists` follows links, so it is false for a dangling link.
        if !target.exists() {
            bail!(
                "the output {} is a link to a missing file",
                target.display()
            );
        }
        if inputs
            .iter()
            .any(|input| same_file::is_same_file(input, target).unwrap_or(false))
        {
            bail!("the output {} would overwrite an input", target.display());
        }
    }
    // The directory exists, so it can be resolved to a real path.
    let key = fold_case(
        &out_dir
            .canonicalize()?
            .join(target.file_name().unwrap_or_default()),
    );
    if !written.insert(key) {
        bail!(
            "the output {} was already written for another input",
            target.display()
        );
    }
    Ok(())
}

/// Lowercases the file name, so that names that differ only in case compare
/// equal. Case-insensitive file systems (macOS, Windows) map them to one file.
fn fold_case(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(name.to_lowercase())
}

/// The font size that drives the rule: the override, else a high-confidence
/// estimate, else `None`.
fn chosen_font(rgba: &image::RgbaImage, font_px: Option<f32>) -> Option<f32> {
    if font_px.is_some() {
        return font_px;
    }
    let e = font::estimate(&load::flatten_to_gray(rgba));
    font::usable_px(e.px, e.confidence)
}

fn rule_name(rule: rule::Rule) -> String {
    serde_json::to_value(rule)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// Black blocks, `h` px high, on white: `rows` lines of `per_row` blocks.
    fn text_like(rows: u32, per_row: u32, h: u32) -> RgbaImage {
        let mut img =
            RgbaImage::from_pixel(8 + per_row * 10, 8 + rows * 30, Rgba([255, 255, 255, 255]));
        for r in 0..rows {
            for c in 0..per_row {
                for y in 0..h {
                    for x in 0..6 {
                        img.put_pixel(4 + c * 10 + x, 4 + r * 30 + y, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn low_confidence_estimate_is_not_used() {
        // Two lines are too few: the estimate has a size but low confidence.
        let low = text_like(2, 12, 5);
        let e = font::estimate(&load::flatten_to_gray(&low));
        assert!(e.px.is_some_and(|px| px < 11.0), "{e:?}");
        assert_eq!(e.confidence, font::Confidence::Low);
        assert_eq!(chosen_font(&low, None), None);
    }

    #[test]
    fn high_confidence_estimate_is_used_and_override_wins() {
        let high = text_like(3, 12, 5);
        let px = chosen_font(&high, None).expect("a high-confidence size");
        assert!(px < 11.0, "{px}");
        assert_eq!(chosen_font(&high, Some(7.0)), Some(7.0));
    }
}

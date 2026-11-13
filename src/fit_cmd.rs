//! The `fit` command: writes one fitted PNG per input image.

use crate::font;
use crate::output::{Envelope, Failure};
use crate::rule::{self, Rule};
use crate::{fit, load};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One image that was written.
#[derive(Debug, Serialize)]
struct FitResult {
    /// Serialised lossily, so that a non-UTF-8 path cannot stop the output.
    #[serde(serialize_with = "lossy_path")]
    path: PathBuf,
    #[serde(serialize_with = "lossy_path")]
    out: PathBuf,
    rule: Rule,
    factor: f32,
    width: u32,
    height: u32,
}

/// Serialises a path as a string; invalid UTF-8 becomes U+FFFD.
fn lossy_path<S: serde::Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy())
}

/// The totals of a `fit` run.
#[derive(Debug, Serialize)]
struct Summary {
    written: usize,
}

impl FitResult {
    /// The text line: `FIT  [rule] path  output (W×H)`.
    fn line(&self) -> String {
        format!(
            "FIT  [{}] {}  {} ({}×{})",
            rule_name(self.rule),
            self.path.display(),
            self.out.display(),
            self.width,
            self.height
        )
    }
}

/// Fits every image into `out_dir`. Returns `0`, or `2` if any image failed.
///
/// Each error goes to `err` as `pixctl: <message>` and, with `json`, into the
/// envelope on `out`. The other images are still processed. If the output
/// directory cannot be created, that one error is reported and no image is
/// processed. Without `json`, each written image gets a `FIT` line on `err`
/// and `out` stays empty.
pub fn run(
    images: &[PathBuf],
    out_dir: &Path,
    font_px: Option<f32>,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut results = Vec::new();
    let mut errors: Vec<Failure> = Vec::new();
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        errors.push(Failure {
            code: "out_dir_failed",
            message: format!("cannot create {}: {e}", out_dir.display()),
            file: Some(out_dir.to_string_lossy().into_owned()),
            hint: None,
            with: None,
        });
    } else {
        // Maps each output name already written to the input that owns it.
        let mut written = HashMap::new();
        for path in images {
            match fit_one(path, images, out_dir, font_px, &mut written) {
                Ok(result) => {
                    if !json {
                        if let Err(e) = writeln!(err, "{}", result.line()) {
                            errors.push(io_failure(path, &e));
                        }
                    }
                    results.push(result);
                }
                Err(failure) => errors.push(failure),
            }
        }
    }
    let mut code = if errors.is_empty() { 0 } else { 2 };
    for failure in &errors {
        // A failed write to `err` is already in `errors`, and cannot be told.
        let _ = writeln!(err, "pixctl: {failure}");
    }
    if json {
        let envelope = Envelope {
            schema: "pixctl/fit/v1".to_string(),
            summary: Summary {
                written: results.len(),
            },
            results,
            errors,
        };
        let written = serde_json::to_writer_pretty(&mut *out, &envelope)
            .map_err(anyhow::Error::from)
            .and_then(|()| writeln!(out).map_err(anyhow::Error::from));
        if let Err(e) = written {
            let _ = writeln!(err, "pixctl: {e:#}");
            code = 2;
        }
    }
    code
}

/// A failure to write a status line.
fn io_failure(path: &Path, e: &std::io::Error) -> Failure {
    Failure {
        code: "write_failed",
        message: format!("{}: cannot write the output line: {e}", path.display()),
        file: Some(path.to_string_lossy().into_owned()),
        hint: None,
        with: None,
    }
}

/// Builds a per-image failure. The message names the input path.
fn failure(code: &'static str, path: &Path, reason: String, with: Option<&Path>) -> Failure {
    Failure {
        code,
        message: format!("{}: {reason}", path.display()),
        file: Some(path.to_string_lossy().into_owned()),
        hint: None,
        with: with.map(|p| p.to_string_lossy().into_owned()),
    }
}

/// Loads one image and writes its fitted PNG. A load error already names the
/// path; every other error is prefixed with it.
fn fit_one(
    path: &Path,
    inputs: &[PathBuf],
    out_dir: &Path,
    font_px: Option<f32>,
    written: &mut HashMap<PathBuf, PathBuf>,
) -> Result<FitResult, Failure> {
    let loaded = load::load(path)?;
    let (w, h) = loaded.rgba.dimensions();
    let plan = rule::plan(w, h, chosen_font(&loaded.rgba, font_px));
    let stem = path.file_stem().ok_or_else(|| {
        failure(
            "read_failed",
            path,
            "the file name has no stem".to_string(),
            None,
        )
    })?;
    let target = {
        let mut name = stem.to_os_string();
        name.push(".png");
        out_dir.join(name)
    };
    guard_target(path, &target, out_dir, inputs, written)?;
    let img = fit::render(&loaded.rgba, &plan);
    let bytes =
        fit::encode_png(&img).map_err(|e| failure("write_failed", path, format!("{e:#}"), None))?;
    std::fs::write(&target, bytes).map_err(|e| {
        failure(
            "write_failed",
            path,
            format!("cannot write {}: {e}", target.display()),
            None,
        )
    })?;
    Ok(FitResult {
        path: path.to_path_buf(),
        out: target,
        rule: plan.rule,
        factor: plan.factor,
        width: img.width(),
        height: img.height(),
    })
}

/// Refuses a target that would overwrite an input, reached directly, through
/// a hard link or through a symbolic link, or that has the same name (ignoring
/// case) as an output already written in this run. `path` is the input being
/// fitted.
fn guard_target(
    path: &Path,
    target: &Path,
    out_dir: &Path,
    inputs: &[PathBuf],
    written: &mut HashMap<PathBuf, PathBuf>,
) -> Result<(), Failure> {
    if target.symlink_metadata().is_ok() {
        // `exists` follows links, so it is false for a dangling link.
        if !target.exists() {
            return Err(failure(
                "write_failed",
                path,
                format!(
                    "the output {} is a link to a missing file",
                    target.display()
                ),
                None,
            ));
        }
        if inputs
            .iter()
            .any(|input| same_file::is_same_file(input, target).unwrap_or(false))
        {
            return Err(failure(
                "same_file",
                path,
                format!("the output {} would overwrite an input", target.display()),
                None,
            ));
        }
    }
    // The directory exists, so it can be resolved to a real path.
    let dir = out_dir
        .canonicalize()
        .map_err(|e| failure("write_failed", path, e.to_string(), None))?;
    let key = fold_case(&dir.join(target.file_name().unwrap_or_default()));
    if let Some(earlier) = written.get(&key) {
        return Err(failure(
            "name_collision",
            path,
            format!(
                "the output {} was already written for another input",
                target.display()
            ),
            Some(earlier),
        ));
    }
    written.insert(key, path.to_path_buf());
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

fn rule_name(rule: Rule) -> String {
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

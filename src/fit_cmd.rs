//! The `fit` command: writes one fitted PNG per input image.

use crate::font;
use crate::output::{self, lossy_path, Envelope, Failure};
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
        let failure = Failure {
            code: "out_dir_failed",
            message: format!("cannot create {}: {e}", out_dir.display()),
            file: Some(out_dir.to_string_lossy().into_owned()),
            hint: None,
            with: None,
        };
        let _ = writeln!(err, "pixctl: {failure}");
        errors.push(failure);
    } else {
        // Maps each output name to the input that claimed it first.
        let mut written = HashMap::new();
        for path in images {
            match fit_one(path, images, out_dir, font_px, &mut written) {
                Ok(result) => {
                    if !json {
                        if let Err(failure) = write_status_line(err, &result, path) {
                            // `err` is broken, so this line cannot be told.
                            errors.push(failure);
                        }
                    }
                    results.push(result);
                }
                Err(failure) => {
                    let _ = writeln!(err, "pixctl: {failure}");
                    errors.push(failure);
                }
            }
        }
    }
    let mut code = if errors.is_empty() { 0 } else { 2 };
    if json {
        let envelope = Envelope {
            schema: "pixctl/fit/v1".to_string(),
            summary: Summary {
                written: results.len(),
            },
            results,
            errors,
        };
        if let Err(e) = output::write_envelope(out, &envelope) {
            let _ = writeln!(err, "pixctl: {e:#}");
            code = 2;
        }
    }
    code
}

/// Writes the `FIT` line of `result`; a write error is a `write_failed`.
fn write_status_line(err: &mut impl Write, result: &FitResult, path: &Path) -> Result<(), Failure> {
    writeln!(err, "{}", result.line()).map_err(|e| io_failure(path, &e))
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
    let hint = match code {
        "same_file" => Some("choose a different --out directory"),
        "name_collision" => Some("rename one of the inputs"),
        _ => None,
    };
    Failure {
        code,
        message: format!("{}: {reason}", path.display()),
        file: Some(path.to_string_lossy().into_owned()),
        hint: hint.map(str::to_string),
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
/// case) as an output already claimed in this run. `path` is the input being
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
                "the output {} is already claimed by another input",
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

    /// A writer that always fails.
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("broken pipe"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A temporary directory holding one input image.
    fn one_input() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let input = dir.path().join("a.png");
        RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]))
            .save(&input)
            .unwrap();
        (dir, input)
    }

    #[test]
    fn broken_stderr_makes_the_status_line_write_failed() {
        let (dir, input) = one_input();
        let mut out = Vec::new();
        let code = run(
            std::slice::from_ref(&input),
            &dir.path().join("out"),
            None,
            false,
            &mut out,
            &mut Broken,
        );
        assert_eq!(code, 2);
        assert!(out.is_empty());
        // The failure itself cannot be reported on the broken stderr, so the
        // code is checked on the value that `run` records.
        let result = FitResult {
            path: input.clone(),
            out: dir.path().join("out").join("a.png"),
            rule: Rule::Unchanged,
            factor: 1.0,
            width: 1,
            height: 1,
        };
        let failure = write_status_line(&mut Broken, &result, &input).unwrap_err();
        assert_eq!(failure.code, "write_failed");
    }

    /// A writer that appends to a buffer shared with its clones.
    #[derive(Clone, Default)]
    struct Shared(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn errors_are_printed_when_they_happen() {
        let (dir, good) = one_input();
        let missing = dir.path().join("missing.png");
        let shared = Shared::default();
        let code = run(
            &[missing, good],
            &dir.path().join("out"),
            None,
            false,
            &mut shared.clone(),
            &mut shared.clone(),
        );
        assert_eq!(code, 2);
        let text = String::from_utf8(shared.0.borrow().clone()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].starts_with("pixctl: "), "{text}");
        assert!(lines[1].starts_with("FIT  "), "{text}");
    }

    #[test]
    fn a_name_stays_claimed_when_its_write_fails() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut inputs = Vec::new();
        for sub in ["one", "two"] {
            let d = dir.path().join(sub);
            std::fs::create_dir(&d).unwrap();
            let p = d.join("a.png");
            RgbaImage::from_pixel(100, 100, Rgba([255, 255, 255, 255]))
                .save(&p)
                .unwrap();
            inputs.push(p);
        }
        let out_dir = dir.path().join("out");
        // A directory at the target makes the first write fail.
        std::fs::create_dir_all(out_dir.join("a.png")).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(&inputs, &out_dir, None, true, &mut out, &mut err);
        assert_eq!(code, 2);
        let doc: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(doc["errors"][0]["code"], "write_failed");
        assert_eq!(doc["errors"][1]["code"], "name_collision");
    }

    #[test]
    fn status_line_write_failure_is_coded_write_failed() {
        let failure = io_failure(Path::new("a.png"), &std::io::Error::other("broken pipe"));
        assert_eq!(failure.code, "write_failed");
        assert_eq!(failure.file.as_deref(), Some("a.png"));
        assert!(failure.message.contains("broken pipe"));
    }

    #[test]
    fn broken_stdout_makes_the_json_envelope_exit_2() {
        let (dir, input) = one_input();
        let mut err = Vec::new();
        let code = run(
            &[input],
            &dir.path().join("out"),
            None,
            true,
            &mut Broken,
            &mut err,
        );
        assert_eq!(code, 2);
        assert!(String::from_utf8(err).unwrap().contains("broken pipe"));
    }

    #[test]
    fn same_file_and_name_collision_carry_a_hint() {
        let same = failure("same_file", Path::new("a.png"), "r".into(), None);
        assert_eq!(
            same.hint.as_deref(),
            Some("choose a different --out directory")
        );
        let clash = failure("name_collision", Path::new("a.png"), "r".into(), None);
        assert_eq!(clash.hint.as_deref(), Some("rename one of the inputs"));
        for code in ["read_failed", "write_failed", "out_dir_failed"] {
            assert_eq!(failure(code, Path::new("a"), "r".into(), None).hint, None);
        }
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

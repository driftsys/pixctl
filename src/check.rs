//! The `check` command: runs the display checks on each image.

use crate::checks::{self, Outcome, Status};
use crate::load;
use crate::output::{self, Envelope, Failure};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// The checks of one image and the worst status among them.
#[derive(Debug, Serialize)]
struct ImageResult {
    /// Serialised lossily, so that a non-UTF-8 path cannot stop the output.
    #[serde(serialize_with = "lossy_path")]
    path: PathBuf,
    status: Status,
    checks: Vec<Outcome>,
}

impl ImageResult {
    fn new(path: &Path, checks: Vec<Outcome>) -> Self {
        // fail > review > pass
        let status = checks
            .iter()
            .map(|c| c.status)
            .fold(Status::Pass, |worst, s| match (worst, s) {
                (Status::Fail, _) | (_, Status::Fail) => Status::Fail,
                (Status::Review, _) | (_, Status::Review) => Status::Review,
                _ => Status::Pass,
            });
        ImageResult {
            path: path.to_path_buf(),
            status,
            checks,
        }
    }
}

/// Serialises a path as a string; invalid UTF-8 becomes U+FFFD.
fn lossy_path<S: serde::Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy())
}

/// The totals of a `check` run. `images` counts the results, not the inputs.
#[derive(Debug, Default, Serialize)]
struct Summary {
    images: usize,
    pass: usize,
    fail: usize,
    review: usize,
}

impl Summary {
    fn of(results: &[ImageResult]) -> Self {
        let mut s = Summary {
            images: results.len(),
            ..Summary::default()
        };
        for r in results {
            match r.status {
                Status::Pass => s.pass += 1,
                Status::Fail => s.fail += 1,
                Status::Review => s.review += 1,
            }
        }
        s
    }

    fn line(&self) -> String {
        format!(
            "{}: {} passed, {} failed, {} review",
            output::count(self.images, "image"),
            self.pass,
            self.fail,
            self.review
        )
    }
}

/// Checks every image. Returns `2` if any image failed to load or the output
/// could not be written; otherwise `1` if any check failed (or needs review
/// under `strict`); otherwise `0`.
///
/// Each load error goes to `err` as one `pixctl: <message>` line and, with
/// `json`, into the envelope. The other images are still checked. `color`
/// paints the status tags in text mode.
pub fn run(
    images: &[PathBuf],
    strict: bool,
    font_px: Option<f32>,
    json: bool,
    color: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut results = Vec::new();
    let mut errors: Vec<Failure> = Vec::new();
    for path in images {
        match load::load(path) {
            Ok(loaded) => results.push(ImageResult::new(
                path,
                checks::run_all(&loaded.rgba, font_px),
            )),
            Err(failure) => {
                let _ = writeln!(err, "pixctl: {failure}");
                errors.push(failure);
            }
        }
    }
    let mut code = if errors.is_empty() { 0 } else { 2 };
    let summary = Summary::of(&results);
    let failed = results.iter().any(|r| match r.status {
        Status::Fail => true,
        Status::Review => strict,
        Status::Pass => false,
    });
    let written = if json {
        let envelope = Envelope {
            schema: "pixctl/check/v1".to_string(),
            results,
            summary,
            errors,
        };
        serde_json::to_writer_pretty(&mut *out, &envelope)
            .map_err(anyhow::Error::from)
            .and_then(|()| writeln!(out).map_err(anyhow::Error::from))
    } else {
        results
            .iter()
            .flat_map(|r| r.checks.iter().map(move |c| (&r.path, c)))
            .try_for_each(|(path, c)| writeln!(out, "{}", plain_line(path, c, color)))
            .and_then(|()| {
                if results.is_empty() {
                    Ok(())
                } else {
                    writeln!(out, "{}", summary.line())
                }
            })
            .map_err(anyhow::Error::from)
    };
    if let Err(e) = written {
        let _ = writeln!(err, "pixctl: {e:#}");
        code = 2;
    }
    if code == 0 {
        code = i32::from(failed);
    }
    code
}

fn plain_line(path: &Path, outcome: &Outcome, color: bool) -> String {
    let tag = match outcome.status {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Review => "REVIEW",
    };
    // The two spaces after the tag stay outside the escape codes.
    format!(
        "{}  [{}] {}  {}",
        output::paint(tag, color),
        outcome.name,
        path.display(),
        outcome.detail
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn output_write_failure_exits_2() {
        let mut err = Vec::new();
        assert_eq!(run(&[], false, None, true, false, &mut Broken, &mut err), 2);
        assert!(String::from_utf8(err).unwrap().contains("broken pipe"));
    }
}

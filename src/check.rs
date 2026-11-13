//! The `check` command: runs the display checks on each image.

use crate::checks::{self, Outcome, Status};
use crate::load;
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One line of output: an outcome tied to the image it belongs to.
#[derive(Debug, Serialize)]
struct Row {
    /// Serialised lossily, so that a non-UTF-8 path cannot stop the output.
    #[serde(serialize_with = "lossy_path")]
    path: PathBuf,
    #[serde(flatten)]
    outcome: Outcome,
}

/// Serialises a path as a string; invalid UTF-8 becomes U+FFFD.
fn lossy_path<S: serde::Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy())
}

/// Checks every image. Returns `2` if any image failed to load or the output
/// could not be written; otherwise `1` if any check failed (or needs review
/// under `strict`); otherwise `0`.
///
/// Errors go to `err` with the path; the other images are still checked.
pub fn run(
    images: &[PathBuf],
    strict: bool,
    font_px: Option<f32>,
    json: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut rows = Vec::new();
    let mut code = 0;
    for path in images {
        match load::load(path) {
            Ok(loaded) => rows.extend(checks::run_all(&loaded.rgba, font_px).into_iter().map(
                |outcome| Row {
                    path: path.clone(),
                    outcome,
                },
            )),
            Err(e) => {
                let _ = writeln!(err, "pixctl: {e:#}");
                code = 2;
            }
        }
    }
    let written = if json {
        serde_json::to_writer_pretty(&mut *out, &rows)
            .map_err(anyhow::Error::from)
            .and_then(|()| writeln!(out).map_err(anyhow::Error::from))
    } else {
        rows.iter()
            .try_for_each(|r| writeln!(out, "{}", plain_line(r)))
            .map_err(anyhow::Error::from)
    };
    if let Err(e) = written {
        let _ = writeln!(err, "pixctl: {e:#}");
        code = 2;
    }
    if code == 0 {
        let failed = rows.iter().any(|r| match r.outcome.status {
            Status::Fail => true,
            Status::Review => strict,
            Status::Pass => false,
        });
        code = i32::from(failed);
    }
    code
}

fn plain_line(r: &Row) -> String {
    let tag = match r.outcome.status {
        Status::Pass => "PASS",
        Status::Fail => "FAIL",
        Status::Review => "REVIEW",
    };
    format!(
        "{tag}  [{}] {}  {}",
        r.outcome.name,
        r.path.display(),
        r.outcome.detail
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
        assert_eq!(run(&[], false, None, true, &mut Broken, &mut err), 2);
        assert!(String::from_utf8(err).unwrap().contains("broken pipe"));
    }
}

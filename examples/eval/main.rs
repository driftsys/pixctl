//! Private evaluation tool, run by `just eval DIR [MANIFEST]`.
//!
//! It prints totals only: never a file name, a path or a per-image line.

mod error;
mod manifest;
mod metrics;
mod survey;

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use pixctl::font::{estimate, Confidence, K};
use pixctl::load::{flatten_to_gray, load};

use crate::error::EvalError;
use crate::metrics::{accuracy, k_interval, Sample};

const USAGE: &str = "usage: just eval DIR [MANIFEST]";

/// Runs the tool and returns the exit code: 0 limit met, 1 limit not met,
/// 2 operational error.
fn run(args: &[OsString], out: &mut impl Write, err: &mut impl Write) -> i32 {
    let Some(dir) = args.first().map(Path::new) else {
        let _ = writeln!(err, "{USAGE}");
        return 2;
    };
    // The guard runs before anything is read.
    match guard(dir) {
        Ok(()) => {}
        Err(e) => return fail(err, &e),
    }
    let Some(manifest) = args.get(1).map(Path::new) else {
        return match survey::collect(dir) {
            Ok((paths, skipped_svg)) => {
                let mut s = survey::run(&paths, err);
                s.skipped_svg = skipped_svg;
                let _ = writeln!(out, "{}", survey::report(&s));
                0
            }
            Err(e) => fail(err, &e),
        };
    };
    match labelled(dir, manifest) {
        Ok((text, met)) => {
            let _ = writeln!(out, "{text}");
            if met {
                0
            } else {
                1
            }
        }
        Err(e) => fail(err, &e),
    }
}

fn fail(err: &mut impl Write, e: &EvalError) -> i32 {
    let _ = writeln!(err, "pixctl eval: {e}");
    2
}

/// Refuses a DIR that cannot be read or that is inside the repository.
fn guard(dir: &Path) -> Result<(), EvalError> {
    if !dir.is_dir() || dir.canonicalize().is_err() {
        return Err(EvalError::CannotReadDir);
    }
    if inside_repository(dir) {
        return Err(EvalError::DirInsideRepository);
    }
    Ok(())
}

/// True when `dir` is the pixctl repository or a folder inside it. A path that
/// cannot be canonicalized is not inside.
fn inside_repository(dir: &Path) -> bool {
    let (Ok(dir), Ok(root)) = (
        dir.canonicalize(),
        Path::new(env!("CARGO_MANIFEST_DIR")).canonicalize(),
    ) else {
        return false;
    };
    dir.starts_with(root)
}

/// Measures every labelled image and returns the report and the verdict.
fn labelled(dir: &Path, manifest: &Path) -> Result<(String, bool), EvalError> {
    let text = std::fs::read_to_string(manifest).map_err(|_| EvalError::CannotReadManifest)?;
    let entries = manifest::parse(&text)?;
    let mut samples = Vec::with_capacity(entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let cannot_read = EvalError::CannotReadImage { index: i + 1 };
        // A panic in a decoder is the same operational error as a load failure.
        let sample = survey::guarded(|| measure(&dir.join(&entry.file), entry.label_px))
            .and_then(Result::ok)
            .ok_or(cannot_read)?;
        samples.push(sample);
    }
    let acc = accuracy(&samples);
    let ki = k_interval(&samples, K);
    Ok(metrics::report(&acc, &ki, K))
}

/// Estimates the font size of one labelled image. A load failure is `Err`.
fn measure(path: &Path, label_px: f32) -> Result<Sample, ()> {
    let loaded = load(path).map_err(|_| ())?;
    let est = estimate(&flatten_to_gray(&loaded.rgba));
    Ok(Sample {
        label_px,
        est_px: est.px,
        confident: est.confidence == Confidence::High && est.px.is_some(),
    })
}

fn main() {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // A panic message names a source path. Both modes catch a panic with
    // `survey::guarded` and report a fixed text instead.
    std::panic::set_hook(Box::new(|_| {}));
    let code = run(&args, &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}

#[cfg(test)]
#[path = "../../tests/support/mod.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use support::{render_text, Face, TextSpec};
    use tempfile::TempDir;

    /// A folder outside the repository with `n` rendered images and a
    /// manifest that labels each one `label` px.
    fn fixture(n: usize, label: f32) -> (TempDir, OsString) {
        let dir = TempDir::new().unwrap();
        let mut manifest = String::new();
        for i in 0..n {
            let img = render_text(&TextSpec {
                face: Face::DejaVu,
                em_px: 14.0,
                lines: 3,
                invert: false,
            });
            img.save(dir.path().join(format!("shot{i}.png"))).unwrap();
            manifest.push_str(&format!(
                "[[image]]\nfile = \"shot{i}.png\"\npx = {label}\n\n"
            ));
        }
        let path = dir.path().join("manifest.toml");
        fs::write(&path, manifest).unwrap();
        (dir, path.into_os_string())
    }

    fn go(args: &[OsString]) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = run(args, &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn labelled_run_prints_totals_and_exits_0() {
        let (dir, manifest) = fixture(4, 14.0);
        let (code, out, err) = go(&[dir.path().into(), manifest]);
        assert_eq!(code, 0, "out: {out} err: {err}");
        assert!(out.contains("images: 4"), "{out}");
        assert!(out.contains("PASS"), "{out}");
        let root = dir.path().to_string_lossy().into_owned();
        for text in [&out, &err] {
            assert!(!text.contains("png"));
            assert!(!text.contains(&root));
        }
    }

    #[test]
    fn confidently_wrong_labels_exit_1() {
        let (dir, manifest) = fixture(4, 30.0);
        let (code, out, _) = go(&[dir.path().into(), manifest]);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("FAIL"), "{out}");
    }

    #[test]
    fn missing_image_error_names_the_index_only() {
        let (dir, _) = fixture(1, 14.0);
        let manifest = dir.path().join("two.toml");
        fs::write(
            &manifest,
            "[[image]]\nfile = \"shot0.png\"\npx = 14.0\n\n\
             [[image]]\nfile = \"nope.png\"\npx = 14.0\n",
        )
        .unwrap();
        let (code, out, err) = go(&[dir.path().into(), manifest.into()]);
        assert_eq!(code, 2);
        assert_eq!(out, "");
        assert_eq!(err, "pixctl eval: entry 2: cannot read image\n");
    }

    #[test]
    fn bad_manifest_exits_2() {
        let (dir, _) = fixture(1, 14.0);
        let manifest = dir.path().join("bad.toml");
        fs::write(&manifest, "[[image]\nfile = \"secret-name.png\"\n").unwrap();
        let (code, _, err) = go(&[dir.path().into(), manifest.into()]);
        assert_eq!(code, 2);
        assert!(
            err.starts_with("pixctl eval: manifest: invalid TOML"),
            "{err}"
        );
        assert!(!err.contains("secret-name"));

        let (code, _, err) = go(&[dir.path().into(), dir.path().join("absent.toml").into()]);
        assert_eq!(code, 2);
        assert_eq!(err, "pixctl eval: manifest: cannot be read\n");
    }

    #[test]
    fn dir_inside_the_repository_is_refused() {
        let want = format!("pixctl eval: {}\n", error::EvalError::DirInsideRepository);
        for dir in [env!("CARGO_MANIFEST_DIR"), ".", "src"] {
            let (code, out, err) = go(&[dir.into(), "whatever.toml".into()]);
            assert_eq!(code, 2, "{dir}");
            assert_eq!(out, "");
            assert_eq!(err, want, "{dir}");
        }
        let (dir, _) = fixture(1, 14.0);
        assert!(!inside_repository(dir.path()));
    }

    #[test]
    fn missing_dir_argument_prints_usage_and_exits_2() {
        let (code, out, err) = go(&[]);
        assert_eq!((code, out.as_str()), (2, ""));
        assert_eq!(err, "usage: just eval DIR [MANIFEST]\n");
    }

    #[test]
    fn run_without_manifest_surveys_and_exits_0() {
        let (dir, _) = fixture(2, 14.0);
        let (code, out, err) = go(&[dir.path().into()]);
        assert_eq!(code, 0, "out: {out} err: {err}");
        assert!(out.contains("images: 2 (skipped SVG: 0)"), "{out}");
        assert_eq!(err, "\r2/2\n");
        let root = dir.path().to_string_lossy().into_owned();
        for text in [&out, &err] {
            assert!(!text.contains("png"));
            assert!(!text.contains(&root));
        }
    }

    #[test]
    fn panic_in_labelled_work_is_cannot_read_image() {
        // The same helper and mapping as in `labelled`, with a panicking body.
        let got = survey::guarded(|| -> Result<Sample, ()> { panic!("decoder bug") })
            .and_then(Result::ok)
            .ok_or(EvalError::CannotReadImage { index: 1 });
        assert_eq!(got.unwrap_err().to_string(), "entry 1: cannot read image");
    }

    #[test]
    fn missing_dir_is_cannot_read_dir() {
        let (dir, _) = fixture(1, 14.0);
        let (code, _, err) = go(&[dir.path().join("gone").into(), "m.toml".into()]);
        assert_eq!(code, 2);
        assert_eq!(err, "pixctl eval: DIR cannot be read\n");
    }
}

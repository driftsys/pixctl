//! The label-free survey: totals over a folder of images, nothing per image.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use pixctl::font::{estimate, usable_px, Confidence};
use pixctl::load::{flatten_to_gray, load};
use pixctl::rule::{plan, Rule};
use rayon::prelude::*;

use crate::error::EvalError;

const EXTENSIONS: [&str; 10] = [
    "png", "jpg", "jpeg", "gif", "bmp", "tif", "tiff", "webp", "emf", "wmf",
];

/// Images per progress step.
const CHUNK: usize = 100;

/// The totals of a survey. The rule counts are in the order icon, unchanged,
/// upscale, too-small, oversize.
#[derive(Debug, Default)]
pub struct Survey {
    pub images: usize,
    pub confident: usize,
    pub rules: [usize; 5],
    pub failed: BTreeMap<&'static str, usize>,
    pub skipped_svg: usize,
}

/// What one image adds to the totals.
enum Outcome {
    Measured { confident: bool, rule: usize },
    Failed(&'static str),
}

/// Lists the image paths below `dir` and counts the `.svg` files. Symbolic
/// links are not followed. An unreadable `dir` is an error; an unreadable
/// entry below it is skipped.
pub fn collect(dir: &Path) -> Result<(Vec<PathBuf>, usize), EvalError> {
    let mut paths = Vec::new();
    let mut svg = 0;
    let mut pending = vec![dir.to_path_buf()];
    let mut root = true;
    while let Some(current) = pending.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) if root => return Err(EvalError::CannotReadDir),
            Err(_) => continue,
        };
        root = false;
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                if ext == "svg" {
                    svg += 1;
                } else if EXTENSIONS.contains(&ext.as_str()) {
                    paths.push(path);
                }
            }
        }
    }
    Ok((paths, svg))
}

/// Measures every path in parallel, in chunks. After each chunk and at the
/// end it writes `\r{done}/{total}` to `progress`, then a newline. The
/// progress is a count only.
pub fn run(paths: &[PathBuf], progress: &mut impl Write) -> Survey {
    let mut s = Survey::default();
    let total = paths.len();
    let mut done = 0;
    for chunk in paths.chunks(CHUNK) {
        let outcomes: Vec<Outcome> = chunk
            .par_iter()
            .map(|p| guarded(|| measure(p)).unwrap_or(Outcome::Failed("panicked")))
            .collect();
        for outcome in outcomes {
            s.images += 1;
            match outcome {
                Outcome::Measured { confident, rule } => {
                    s.confident += usize::from(confident);
                    s.rules[rule] += 1;
                }
                Outcome::Failed(code) => *s.failed.entry(code).or_default() += 1,
            }
        }
        done += chunk.len();
        let _ = write!(progress, "\r{done}/{total}");
    }
    if total == 0 {
        let _ = write!(progress, "\r0/0");
    }
    let _ = writeln!(progress);
    s
}

/// Runs `f` and returns `None` when it panics (for example in a decoder), so
/// one image cannot stop the run. The caller maps `None` to its own failure.
pub fn guarded<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

fn measure(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(failure) => return Outcome::Failed(failure.code),
    };
    let (w, h) = loaded.rgba.dimensions();
    let est = estimate(&flatten_to_gray(&loaded.rgba));
    let rule = match plan(w, h, usable_px(est.px, est.confidence)).rule {
        Rule::Icon => 0,
        Rule::Unchanged => 1,
        Rule::Upscale => 2,
        Rule::TooSmall => 3,
        Rule::Oversize => 4,
    };
    Outcome::Measured {
        confident: est.confidence == Confidence::High && est.px.is_some(),
        rule,
    }
}

/// The totals as text: counts and percentages only.
pub fn report(s: &Survey) -> String {
    let pct = if s.images == 0 {
        0.0
    } else {
        100.0 * s.confident as f64 / s.images as f64
    };
    let failed = if s.failed.is_empty() {
        "0".to_string()
    } else {
        s.failed
            .iter()
            .map(|(code, n)| format!("{code} {n}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let [a, b, c, d, e] = s.rules;
    format!(
        "images: {} (skipped SVG: {})\nconfident: {} ({pct:.1} %)\n\
         rules: icon {a}, unchanged {b}, upscale {c}, too-small {d}, oversize {e}\n\
         failed: {failed}",
        s.images, s.skipped_svg, s.confident
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{render_text, Face, TextSpec};
    use std::fs;
    use tempfile::TempDir;

    fn text_png(path: &Path) {
        render_text(&TextSpec {
            face: Face::DejaVu,
            em_px: 14.0,
            lines: 3,
            invert: false,
        })
        .save(path)
        .unwrap();
    }

    #[test]
    fn survey_walks_nested_folders_and_skips_svg() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        fs::write(dir.path().join("a/b/x.png"), b"").unwrap();
        fs::write(dir.path().join("a/y.PNG"), b"").unwrap();
        fs::write(dir.path().join("z.svg"), b"").unwrap();
        fs::write(dir.path().join("notes.txt"), b"").unwrap();
        let (paths, svg) = collect(dir.path()).unwrap();
        assert_eq!((paths.len(), svg), (2, 1));
    }

    #[cfg(unix)]
    #[test]
    fn survey_does_not_follow_symlinks() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/x.png"), b"").unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("sub/loop")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("sub/x.png"), dir.path().join("l.png")).unwrap();
        let (paths, _) = collect(dir.path()).unwrap();
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn unreadable_root_is_cannot_read_dir() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            collect(&dir.path().join("gone")).unwrap_err(),
            EvalError::CannotReadDir
        );
    }

    #[test]
    fn survey_counts_rules_and_failures() {
        let dir = TempDir::new().unwrap();
        text_png(&dir.path().join("t.png"));
        image::RgbaImage::new(32, 32)
            .save(dir.path().join("i.png"))
            .unwrap();
        fs::write(dir.path().join("junk.png"), b"not an image").unwrap();
        let (paths, _) = collect(dir.path()).unwrap();
        let mut progress = Vec::new();
        let s = run(&paths, &mut progress);
        assert_eq!(s.images, 3);
        assert_eq!(s.confident, 1);
        assert_eq!(s.rules[0], 1);
        assert_eq!(s.rules.iter().sum::<usize>(), 2);
        assert_eq!(s.failed["read_failed"], 1);
        assert_eq!(String::from_utf8(progress).unwrap(), "\r3/3\n");
    }

    #[test]
    fn progress_is_written_every_100_images() {
        let dir = TempDir::new().unwrap();
        let paths: Vec<PathBuf> = (0..150)
            .map(|i| {
                let p = dir.path().join(format!("{i}.png"));
                fs::write(&p, b"x").unwrap();
                p
            })
            .collect();
        let mut progress = Vec::new();
        run(&paths, &mut progress);
        assert_eq!(String::from_utf8(progress).unwrap(), "\r100/150\r150/150\n");
    }

    #[test]
    fn panicking_image_is_counted_as_failed() {
        let outcome = guarded(|| -> Outcome { panic!("decoder bug") });
        assert!(outcome.is_none());
        let outcome = guarded(|| measure(Path::new("/nonexistent/x.png")));
        assert!(matches!(outcome, Some(Outcome::Failed(_))));
    }

    #[test]
    fn empty_survey_prints_zero() {
        let mut progress = Vec::new();
        let text = report(&run(&[], &mut progress));
        assert!(text.contains("images: 0"), "{text}");
        assert!(text.contains("(0.0 %)"), "{text}");
        assert!(text.contains("failed: 0"), "{text}");
    }

    #[test]
    fn report_has_the_fixed_lines() {
        let mut s = Survey {
            images: 4,
            confident: 1,
            rules: [1, 2, 0, 0, 1],
            skipped_svg: 2,
            ..Survey::default()
        };
        s.failed.insert("unsupported_type", 1);
        s.failed.insert("read_failed", 3);
        assert_eq!(
            report(&s),
            "images: 4 (skipped SVG: 2)\n\
             confident: 1 (25.0 %)\n\
             rules: icon 1, unchanged 2, upscale 0, too-small 0, oversize 1\n\
             failed: read_failed 3, unsupported_type 1"
        );
    }

    #[test]
    fn report_has_no_paths() {
        let dir = TempDir::new().unwrap();
        text_png(&dir.path().join("t.png"));
        let (paths, svg) = collect(dir.path()).unwrap();
        let mut s = run(&paths, &mut Vec::new());
        s.skipped_svg = svg;
        let text = report(&s);
        assert!(!text.contains("png") && !text.contains('/'), "{text}");
    }
}

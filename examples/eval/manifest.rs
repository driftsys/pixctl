//! The manifest: a TOML list of `[[image]]` entries that give the label size
//! of the text in each image.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::EvalError;

/// One labelled image.
#[derive(Debug, PartialEq)]
pub struct Entry {
    /// Path of the image, relative to the images directory.
    pub file: PathBuf,
    /// Label font size in pixels.
    pub label_px: f32,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    image: Vec<RawEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    file: String,
    px: Option<f32>,
    pt: Option<f32>,
    scale: Option<f32>,
}

fn positive(value: f32) -> bool {
    value.is_finite() && value > 0.0
}

fn bad(index: usize, reason: &'static str) -> EvalError {
    EvalError::BadEntry { index, reason }
}

/// Parses the manifest text. Unknown keys are ignored. Indexes in errors are
/// 1-based.
pub fn parse(text: &str) -> Result<Vec<Entry>, EvalError> {
    let raw: Raw = toml::from_str(text).map_err(|e| EvalError::BadToml {
        line: e
            .span()
            .map(|s| text[..s.start.min(text.len())].matches('\n').count() + 1),
    })?;
    if raw.image.is_empty() {
        return Err(EvalError::EmptyManifest);
    }
    raw.image
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let index = i + 1;
            let path = Path::new(&e.file);
            if path.is_absolute() || path.has_root() {
                return Err(bad(index, "file must be a relative path"));
            }
            let label_px = match (e.px, e.pt, e.scale) {
                (Some(px), None, None) => px,
                (None, Some(pt), Some(scale)) => {
                    if !positive(scale) {
                        return Err(bad(index, "label size must be positive"));
                    }
                    pt * 96.0 / 72.0 * scale
                }
                (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
                    return Err(bad(index, "px cannot be combined with pt or scale"))
                }
                (None, Some(_), None) => return Err(bad(index, "pt needs scale")),
                (None, None, Some(_)) => return Err(bad(index, "scale needs pt")),
                (None, None, None) => return Err(bad(index, "label size is missing")),
            };
            if !positive(label_px) {
                return Err(bad(index, "label size must be positive"));
            }
            Ok(Entry {
                file: PathBuf::from(e.file),
                label_px,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(body: &str) -> Result<Vec<Entry>, EvalError> {
        parse(&format!("[[image]]\nfile = \"a.png\"\n{body}\n"))
    }

    fn is_bad_entry(r: Result<Vec<Entry>, EvalError>, want: usize) -> bool {
        matches!(r, Err(EvalError::BadEntry { index, .. }) if index == want)
    }

    #[test]
    fn px_entry_is_used_as_is() {
        let e = one("px = 13.0").unwrap();
        assert_eq!(e[0].label_px, 13.0);
        assert_eq!(e[0].file, PathBuf::from("a.png"));
    }

    #[test]
    fn pt_and_scale_give_px() {
        let e = one("pt = 9.0\nscale = 1.25").unwrap();
        assert!((e[0].label_px - 15.0).abs() < 1e-4);
    }

    #[test]
    fn unknown_keys_are_allowed() {
        assert!(one("px = 13.0\nsource = \"x\"").is_ok());
    }

    #[test]
    fn px_with_pt_is_an_error() {
        assert!(is_bad_entry(one("px = 13.0\npt = 9.0"), 1));
        assert!(is_bad_entry(one("px = 13.0\nscale = 1.0"), 1));
    }

    #[test]
    fn pt_without_scale_is_an_error() {
        assert!(is_bad_entry(one("pt = 9.0"), 1));
    }

    #[test]
    fn scale_without_pt_is_an_error() {
        assert!(is_bad_entry(one("scale = 1.25"), 1));
    }

    #[test]
    fn non_positive_or_non_finite_is_an_error() {
        assert!(is_bad_entry(one("px = 0.0"), 1));
        assert!(is_bad_entry(one("px = -1.0"), 1));
        assert!(is_bad_entry(one("pt = nan\nscale = 1.0"), 1));
        assert!(is_bad_entry(one("pt = 9.0\nscale = 0.0"), 1));
    }

    #[test]
    fn absolute_file_is_an_error() {
        let r = parse("[[image]]\nfile = \"/etc/x.png\"\npx = 13.0\n");
        assert!(is_bad_entry(r, 1));
        #[cfg(windows)]
        {
            let r = parse("[[image]]\nfile = \"C:\\\\x.png\"\npx = 13.0\n");
            assert!(is_bad_entry(r, 1));
        }
    }

    #[test]
    fn empty_manifest_is_an_error() {
        assert_eq!(parse("").unwrap_err(), EvalError::EmptyManifest);
    }

    #[test]
    fn bad_toml_reports_the_line_only() {
        let err = parse("[[image]]\nsecret-name.png = = 1\n").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("line 2"), "{text}");
        assert!(!text.contains("secret"), "{text}");
    }

    #[test]
    fn second_entry_error_names_index_2() {
        let r = parse("[[image]]\nfile = \"a.png\"\npx = 13.0\n[[image]]\nfile = \"b.png\"\n");
        assert!(is_bad_entry(r, 2));
    }
}

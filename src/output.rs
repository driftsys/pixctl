//! Shared output helpers: the JSON envelope, coded errors, colour and counts.

use serde::Serialize;
use std::fmt;
use std::io::Write;
use std::path::Path;

/// A coded error, reported in the JSON envelope and as one stderr line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Failure {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
}

impl fmt::Display for Failure {
    /// The message alone. The `pixctl: ` prefix is added when it is printed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Failure {}

/// The single JSON document that a command writes with `--json`.
#[derive(Debug, Serialize)]
pub struct Envelope<R, S> {
    pub schema: String,
    pub results: Vec<R>,
    pub summary: S,
    pub errors: Vec<Failure>,
}

/// Serialises a path as a string; invalid UTF-8 becomes U+FFFD.
pub fn lossy_path<S: serde::Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&path.to_string_lossy())
}

/// Writes the envelope as pretty JSON followed by a newline.
pub fn write_envelope<R: Serialize, S: Serialize>(
    out: &mut impl Write,
    envelope: &Envelope<R, S>,
) -> anyhow::Result<()> {
    serde_json::to_writer_pretty(&mut *out, envelope)?;
    writeln!(out)?;
    Ok(())
}

/// Decides whether to colour the output.
///
/// A non-empty `NO_COLOR` turns colour off and wins over everything else. A
/// non-empty `CLICOLOR_FORCE` turns it on. Otherwise colour follows the TTY.
pub fn color_enabled(is_tty: bool, no_color: Option<&str>, clicolor_force: Option<&str>) -> bool {
    if no_color.is_some_and(|v| !v.is_empty()) {
        return false;
    }
    if clicolor_force.is_some_and(|v| !v.is_empty()) {
        return true;
    }
    is_tty
}

/// Wraps a status tag in an ANSI colour when `enabled`. The visible text is the
/// same either way.
pub fn paint(tag: &str, enabled: bool) -> String {
    let code = match tag {
        "PASS" => "32",
        "FAIL" => "31",
        "REVIEW" => "33",
        _ => return tag.to_string(),
    };
    if enabled {
        format!("\x1b[{code}m{tag}\x1b[0m")
    } else {
        tag.to_string()
    }
}

/// Formats a count with its noun: `1 image`, `3 images`.
pub fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_enabled_covers_all_combinations() {
        for tty in [false, true] {
            for no_color in [None, Some("1")] {
                for force in [None, Some("1")] {
                    let expected = if no_color.is_some() {
                        false
                    } else {
                        tty || force.is_some()
                    };
                    assert_eq!(
                        color_enabled(tty, no_color, force),
                        expected,
                        "tty={tty} no_color={no_color:?} force={force:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn empty_no_color_counts_as_unset() {
        assert!(color_enabled(true, Some(""), None));
        assert!(color_enabled(false, Some(""), Some("1")));
        assert!(!color_enabled(false, Some(""), Some("")));
    }

    #[test]
    fn paint_emits_exact_escape_bytes() {
        assert_eq!(paint("PASS", true), "\x1b[32mPASS\x1b[0m");
        assert_eq!(paint("FAIL", true), "\x1b[31mFAIL\x1b[0m");
        assert_eq!(paint("REVIEW", true), "\x1b[33mREVIEW\x1b[0m");
    }

    #[test]
    fn paint_is_plain_when_off() {
        for tag in ["PASS", "FAIL", "REVIEW"] {
            assert_eq!(paint(tag, false), tag);
        }
    }

    #[test]
    fn count_pluralises() {
        assert_eq!(count(0, "image"), "0 images");
        assert_eq!(count(1, "image"), "1 image");
        assert_eq!(count(3, "image"), "3 images");
    }

    #[test]
    fn failure_omits_absent_fields() {
        let f = Failure {
            code: "read_failed",
            message: "cannot decode image a.png".into(),
            file: None,
            hint: None,
            with: None,
        };
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            r#"{"code":"read_failed","message":"cannot decode image a.png"}"#
        );
    }

    #[test]
    fn failure_serialises_present_fields_in_order() {
        let f = Failure {
            code: "name_collision",
            message: "m".into(),
            file: Some("b.png".into()),
            hint: Some("h".into()),
            with: Some("a.png".into()),
        };
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            r#"{"code":"name_collision","message":"m","file":"b.png","hint":"h","with":"a.png"}"#
        );
    }

    #[test]
    fn failure_displays_the_message_alone() {
        let f = Failure {
            code: "x",
            message: "boom".into(),
            file: Some("f".into()),
            hint: None,
            with: None,
        };
        assert_eq!(f.to_string(), "boom");
    }

    #[test]
    fn envelope_key_order() {
        let e = Envelope::<u8, u8> {
            schema: "pixctl/x/v1".into(),
            results: vec![1],
            summary: 2,
            errors: vec![],
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"schema":"pixctl/x/v1","results":[1],"summary":2,"errors":[]}"#
        );
    }
}

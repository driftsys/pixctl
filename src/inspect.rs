//! The `inspect` command: size, alpha, font estimate and rule per image.

use crate::font::{self, Confidence};
use crate::load;
use crate::output::{self, lossy_path, Envelope, Failure};
use crate::rule::{self, Rule};
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
    /// Serialised lossily, so that a non-UTF-8 path cannot stop the output.
    #[serde(serialize_with = "lossy_path")]
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub alpha: bool,
    pub font: FontReport,
    pub rule: Rule,
    pub factor: f32,
    pub output_width: u32,
}

/// The totals of an `inspect` run. `images` counts the reports, not the inputs.
#[derive(Debug, Default, Serialize)]
struct Summary {
    images: usize,
    icon: usize,
    unchanged: usize,
    upscale: usize,
    too_small: usize,
    oversize: usize,
}

impl Summary {
    fn of(reports: &[Report]) -> Self {
        let mut s = Summary {
            images: reports.len(),
            ..Summary::default()
        };
        for r in reports {
            match r.rule {
                Rule::Icon => s.icon += 1,
                Rule::Unchanged => s.unchanged += 1,
                Rule::Upscale => s.upscale += 1,
                Rule::TooSmall => s.too_small += 1,
                Rule::Oversize => s.oversize += 1,
            }
        }
        s
    }

    fn line(&self) -> String {
        format!(
            "{}: {} icon, {} unchanged, {} upscale, {} too-small, {} oversize",
            output::count(self.images, "image"),
            self.icon,
            self.unchanged,
            self.upscale,
            self.too_small,
            self.oversize
        )
    }
}

/// Inspects every image. Returns `0`, or `2` if any image failed to load or
/// the output could not be written.
///
/// Each error goes to `err` as one `pixctl: <message>` line and, with `json`,
/// into the envelope. The other images are still reported.
pub fn run(
    images: &[PathBuf],
    json: bool,
    font_px: Option<f32>,
    out: &mut impl Write,
    err: &mut impl Write,
) -> i32 {
    let mut reports = Vec::new();
    let mut errors = Vec::new();
    for path in images {
        match inspect(path, font_px) {
            Ok(report) => reports.push(report),
            Err(failure) => {
                let _ = writeln!(err, "pixctl: {failure}");
                errors.push(failure);
            }
        }
    }
    let mut code = if errors.is_empty() { 0 } else { 2 };
    let summary = Summary::of(&reports);
    let written = if json {
        let envelope = Envelope {
            schema: "pixctl/inspect/v1".to_string(),
            results: reports,
            summary,
            errors,
        };
        output::write_envelope(out, &envelope)
    } else {
        reports
            .iter()
            .try_for_each(|r| writeln!(out, "{}", plain_line(r)))
            .and_then(|()| {
                if reports.is_empty() {
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
    code
}

fn inspect(path: &Path, font_px: Option<f32>) -> Result<Report, Failure> {
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
            let e = font::estimate(&load::flatten_to_gray(&loaded.rgba));
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
    let usable = font::usable_px(font.px, font.confidence);
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
        assert_eq!(run(&[], true, None, &mut Broken, &mut err), 2);
        assert!(String::from_utf8(err).unwrap().contains("broken pipe"));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_serialises_lossily() {
        use std::os::unix::ffi::OsStrExt;
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(b"bad\xff.png"));
        let report = Report {
            path,
            width: 1,
            height: 1,
            alpha: false,
            font: FontReport {
                px: None,
                confidence: Confidence::Low,
                lines: 0,
                glyphs: 0,
                source: FontSource::Estimate,
            },
            rule: Rule::Icon,
            factor: 1.0,
            output_width: 1,
        };
        let json = serde_json::to_string(&[report]).expect("serialises");
        assert!(json.contains("\"bad\u{FFFD}.png\""), "{json}");
    }
}

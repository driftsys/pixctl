//! End-to-end tests for `pixctl inspect`.

mod support;

use assert_cmd::Command;
use image::{imageops, GrayImage, Luma, Rgba, RgbaImage};
use serde_json::Value;
use std::path::{Path, PathBuf};
use support::{noise, render_text, Face, TextSpec};
use tempfile::TempDir;

fn pixctl() -> Command {
    Command::cargo_bin("pixctl").unwrap()
}

/// DejaVu text at 14 px placed on a white canvas 700 px wide.
///
/// The width stays under the content width (766 px) so that the rule is not
/// `oversize`.
fn text_png(dir: &Path, name: &str) -> PathBuf {
    let text = render_text(&TextSpec {
        face: Face::DejaVu,
        em_px: 14.0,
        lines: 12,
        invert: false,
    });
    let mut canvas = GrayImage::from_pixel(700, text.height().max(400), Luma([255]));
    imageops::overlay(&mut canvas, &text, 0, 0);
    let path = dir.join(name);
    canvas.save(&path).unwrap();
    path
}

fn blank_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let path = dir.join(name);
    GrayImage::from_pixel(w, h, Luma([255]))
        .save(&path)
        .unwrap();
    path
}

fn json_of(output: &[u8]) -> Vec<Value> {
    serde_json::from_slice::<Value>(output)
        .expect("stdout is JSON")
        .as_array()
        .expect("JSON array")
        .clone()
}

#[test]
fn inspect_json_reports_all_fields() {
    let dir = TempDir::new().unwrap();
    let path = text_png(dir.path(), "text.png");
    let out = pixctl()
        .arg("inspect")
        .arg(&path)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let reports = json_of(&out.stdout);
    assert_eq!(reports.len(), 1);
    let r = &reports[0];
    for key in [
        "path",
        "width",
        "height",
        "alpha",
        "font",
        "rule",
        "factor",
        "output_width",
    ] {
        assert!(r.get(key).is_some(), "missing key {key}");
    }
    for key in ["px", "confidence", "lines", "glyphs", "source"] {
        assert!(r["font"].get(key).is_some(), "missing font key {key}");
    }
    assert_eq!(r["width"], 700);
    assert_eq!(r["font"]["source"], "estimate");
    let px = r["font"]["px"].as_f64().expect("font px is a number");
    assert!((px - 14.0).abs() <= 1.5, "px {px}");
    assert_eq!(r["rule"], "unchanged");
}

#[test]
fn font_px_override() {
    let dir = TempDir::new().unwrap();
    let path = blank_png(dir.path(), "small.png", 300, 200);
    let out = pixctl()
        .args(["inspect", "--json", "--font-px", "8"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let r = &json_of(&out.stdout)[0];
    assert_eq!(r["font"]["source"], "override");
    assert_eq!(r["font"]["confidence"], "high");
    assert_eq!(r["font"]["lines"], 0);
    assert_eq!(r["font"]["glyphs"], 0);
    assert_eq!(r["rule"], "upscale");
    assert_eq!(r["factor"].as_f64(), Some(1.5));
}

#[test]
fn plain_output_has_one_line_per_image() {
    let dir = TempDir::new().unwrap();
    let a = blank_png(dir.path(), "a.png", 300, 200);
    let b = blank_png(dir.path(), "b.png", 40, 30);
    let out = pixctl().arg("inspect").arg(&a).arg(&b).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].contains(&a.display().to_string()));
    assert!(lines[0].contains("300x200"));
    assert!(lines[0].contains("unchanged"));
    assert!(lines[1].contains(&b.display().to_string()));
    assert!(lines[1].contains("40x30"));
    assert!(lines[1].contains("icon"));
}

#[test]
fn fully_transparent_png() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("clear.png");
    RgbaImage::from_pixel(300, 200, Rgba([0, 0, 0, 0]))
        .save(&path)
        .unwrap();
    let out = pixctl()
        .args(["inspect", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let r = &json_of(&out.stdout)[0];
    assert_eq!(r["alpha"], true);
    assert!(r["font"]["px"].is_null());
    assert_eq!(r["font"]["confidence"], "low");
}

#[test]
fn bad_file_among_good_ones() {
    let dir = TempDir::new().unwrap();
    let good = blank_png(dir.path(), "good.png", 300, 200);
    let corrupt = dir.path().join("corrupt.png");
    std::fs::write(&corrupt, b"not a png").unwrap();
    let missing = dir.path().join("missing.png");
    let emf = dir.path().join("a.emf");
    std::fs::write(&emf, b"emf").unwrap();
    let good2 = blank_png(dir.path(), "good2.png", 200, 100);
    let out = pixctl()
        .args(["inspect", "--json"])
        .args([&good, &corrupt, &missing, &emf, &good2])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(json_of(&out.stdout).len(), 2);
    let stderr = String::from_utf8(out.stderr).unwrap();
    for name in ["corrupt.png", "missing.png", "a.emf"] {
        assert!(stderr.contains(name), "stderr lacks {name}: {stderr}");
    }
}

#[test]
fn font_px_rejects_zero_negative_and_text() {
    for value in ["0", "-3", "abc", "NaN", "inf"] {
        let out = pixctl()
            .args(["inspect", "a.png", "--font-px", value])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "value {value}");
        assert!(!out.stderr.is_empty(), "value {value}");
    }
}

#[test]
fn needs_review_still_exits_zero() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("noise.png");
    noise(300, 200, 7).save(&path).unwrap();
    let out = pixctl()
        .args(["inspect", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let r = &json_of(&out.stdout)[0];
    assert_eq!(r["font"]["confidence"], "low");
    assert_eq!(r["rule"], "unchanged");
}

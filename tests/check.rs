//! End-to-end tests for `pixctl check`.

use assert_cmd::Command;
use image::{Rgba, RgbaImage};
use pixctl::contract::FRAME_COLOUR;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn pixctl() -> Command {
    Command::cargo_bin("pixctl").unwrap()
}

/// A white PNG; with `framed`, it has a one pixel frame ring.
fn png(dir: &Path, name: &str, w: u32, h: u32, framed: bool) -> PathBuf {
    let [r, g, b] = FRAME_COLOUR;
    let mut img = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    if framed {
        for (x, y, p) in img.enumerate_pixels_mut() {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                *p = Rgba([r, g, b, 255]);
            }
        }
    }
    let path = dir.join(name);
    img.save(&path).unwrap();
    path
}

#[test]
fn clean_framed_image_exits_0() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 4, "{stdout}");
    assert!(lines.iter().all(|l| l.starts_with("PASS  [")), "{stdout}");
}

#[test]
fn fail_exits_1() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "bare.png", 300, 200, false);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("FAIL  [framed]"));
}

#[test]
fn review_exits_0_strict_exits_1() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "blank.png", 300, 200, true);
    let out = pixctl().arg("check").arg(&path).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("REVIEW  [min-font-size]"));
    let out = pixctl()
        .args(["check", "--strict"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("REVIEW  [min-font-size]"));
}

#[test]
fn json_is_array_of_outcomes() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--json", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    let items = serde_json::from_slice::<Value>(&out.stdout).unwrap();
    let items = items.as_array().expect("JSON array");
    assert_eq!(items.len(), 4);
    for item in items {
        let mut keys: Vec<_> = item.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["check", "detail", "path", "status"]);
        assert_eq!(item["status"], "pass");
    }
}

#[test]
fn missing_file_exits_2_others_reported() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(&path)
        .arg(dir.path().join("missing.png"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 4);
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("missing.png"));
}

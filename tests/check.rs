//! End-to-end tests for `pixctl check`.

use assert_cmd::Command;
use image::{Rgba, RgbaImage};
use pixctl::contract::FRAME_COLOUR;
use serde_json::Value;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn pixctl() -> Command {
    let mut cmd = Command::cargo_bin("pixctl").unwrap();
    cmd.env_remove("NO_COLOR").env_remove("CLICOLOR_FORCE");
    cmd
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
    let p = path.display();
    assert_eq!(
        stdout,
        format!(
            "PASS  [width] {p}  300 px <= 800 px\n\
             PASS  [min-font-size] {p}  12.0 px >= 11 px\n\
             PASS  [opaque] {p}  no transparent pixel\n\
             PASS  [framed] {p}  frame and padding present\n\
             1 image: 1 passed, 0 failed, 0 review\n"
        )
    );
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
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "FAIL  [framed] {}  frame or padding missing\n",
            path.display()
        )),
        "{stdout}"
    );
}

#[test]
fn review_exits_0_strict_exits_1() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "blank.png", 300, 200, true);
    let out = pixctl().arg("check").arg(&path).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let review = format!(
        "REVIEW  [min-font-size] {}  font size estimate has low confidence\n",
        path.display()
    );
    assert!(String::from_utf8(out.stdout).unwrap().contains(&review));
    let out = pixctl()
        .args(["check", "--strict"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stdout).unwrap().contains(&review));
}

#[test]
fn json_is_an_envelope() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--json", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let raw = String::from_utf8(out.stdout).unwrap();
    let keys = ["\"schema\"", "\"results\"", "\"summary\"", "\"errors\""];
    let at: Vec<_> = keys.iter().map(|k| raw.find(k).unwrap()).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "key order: {raw}");
    let v: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["schema"], "pixctl/check/v1");
    assert_eq!(v["errors"].as_array().unwrap().len(), 0);
    assert_eq!(
        v["summary"],
        serde_json::json!({"images": 1, "pass": 1, "fail": 0, "review": 0})
    );
    let results = v["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    let r = &results[0];
    let mut keys: Vec<_> = r.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["checks", "path", "status"]);
    assert_eq!(r["path"], path.display().to_string());
    assert_eq!(r["status"], "pass");
    let checks = r["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 4);
    for c in checks {
        let mut keys: Vec<_> = c.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, ["check", "detail", "status"]);
        assert_eq!(c["status"], "pass");
    }
}

#[test]
fn json_image_status_is_the_worst_check() {
    let dir = TempDir::new().unwrap();
    let bare = png(dir.path(), "bare.png", 300, 200, false);
    let review = png(dir.path(), "blank.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--json"])
        .arg(&bare)
        .arg(&review)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let results = v["results"].as_array().unwrap();
    // fail > review: the bare image has a failed and a review check.
    assert_eq!(results[0]["status"], "fail");
    assert_eq!(results[1]["status"], "review");
    let status_of = |name: &str| {
        results[0]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["check"] == name)
            .unwrap()["status"]
            .clone()
    };
    assert_eq!(status_of("framed"), "fail");
    assert_eq!(status_of("min-font-size"), "review");
    assert_eq!(status_of("width"), "pass");
    assert_eq!(
        v["summary"],
        serde_json::json!({"images": 2, "pass": 0, "fail": 1, "review": 1})
    );
}

#[test]
fn strict_keeps_review_in_json_and_exits_1() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "blank.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--json", "--strict"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["results"][0]["status"], "review");
}

#[test]
fn mixed_run_has_results_and_errors() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--json", "--font-px", "12"])
        .arg(dir.path().join("missing.png"))
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["results"].as_array().unwrap().len(), 1);
    assert_eq!(v["errors"].as_array().unwrap().len(), 1);
    assert_eq!(v["summary"]["images"], 1);
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.starts_with("pixctl: "), "{stderr}");
    assert!(stderr.contains("missing.png"), "{stderr}");
}

#[test]
fn all_inputs_missing_gives_empty_results_and_exit_2() {
    let dir = TempDir::new().unwrap();
    let out = pixctl()
        .args(["check", "--json"])
        .arg(dir.path().join("a.png"))
        .arg(dir.path().join("b.png"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["results"].as_array().unwrap().len(), 0);
    assert_eq!(v["errors"].as_array().unwrap().len(), 2);
    assert_eq!(v["summary"]["images"], 0);
}

#[test]
fn text_ends_with_a_summary_line() {
    let dir = TempDir::new().unwrap();
    let bare = png(dir.path(), "bare.png", 300, 200, false);
    let review = png(dir.path(), "blank.png", 300, 200, true);
    let ok = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(&bare)
        .arg(&ok)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().last().unwrap(),
        "2 images: 1 passed, 1 failed, 0 review"
    );
    let out = pixctl().arg("check").arg(&review).output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().last().unwrap(),
        "1 image: 0 passed, 0 failed, 1 review"
    );
}

#[test]
fn text_without_loaded_images_has_no_summary_line() {
    let dir = TempDir::new().unwrap();
    let out = pixctl()
        .arg("check")
        .arg(dir.path().join("missing.png"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn clicolor_force_paints_only_the_tag() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .env("CLICOLOR_FORCE", "1")
        .args(["check", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "\x1b[32mPASS\x1b[0m  [width] {}  300 px <= 800 px\n",
            path.display()
        )),
        "{stdout:?}"
    );
}

#[test]
fn no_color_wins_over_clicolor_force() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .env("NO_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .args(["check", "--font-px", "12"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!String::from_utf8(out.stdout).unwrap().contains('\x1b'));
}

#[test]
fn piped_default_and_json_have_no_colour() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl().arg("check").arg(&path).output().unwrap();
    assert!(!String::from_utf8(out.stdout).unwrap().contains('\x1b'));
    let out = pixctl()
        .env("CLICOLOR_FORCE", "1")
        .args(["check", "--json"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!String::from_utf8(out.stdout).unwrap().contains('\x1b'));
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
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 5);
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("missing.png"));
}

#[test]
fn missing_file_first_does_not_stop_check() {
    let dir = TempDir::new().unwrap();
    let path = png(dir.path(), "ok.png", 300, 200, true);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(dir.path().join("missing.png"))
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 5);
}

#[test]
fn exit_2_wins_over_exit_1() {
    let dir = TempDir::new().unwrap();
    let bare = png(dir.path(), "bare.png", 300, 200, false);
    let out = pixctl()
        .args(["check", "--font-px", "12"])
        .arg(&bare)
        .arg(dir.path().join("missing.png"))
        .output()
        .unwrap();
    assert!(String::from_utf8(out.stdout).unwrap().contains("FAIL"));
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn a_failed_check_that_is_not_last_fails_the_image() {
    let dir = TempDir::new().unwrap();
    // Framed, with a font size that passes, so only `width` fails; `framed`
    // comes after it.
    let path = png(dir.path(), "wide.png", 1000, 200, true);
    let out = pixctl()
        .args(["check", "--json", "--font-px", "20"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let r = &v["results"][0];
    assert_eq!(r["status"], "fail");
    let statuses: Vec<_> = r["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["check"].as_str().unwrap(), c["status"].as_str().unwrap()))
        .collect();
    assert_eq!(
        statuses,
        [
            ("width", "fail"),
            ("min-font-size", "pass"),
            ("opaque", "pass"),
            ("framed", "pass")
        ]
    );
    assert_eq!(v["summary"]["fail"], 1);
}

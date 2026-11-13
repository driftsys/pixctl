//! End-to-end tests for `pixctl fit`.

use assert_cmd::Command;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use pixctl::checks::{run_all, Status};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn pixctl() -> Command {
    Command::cargo_bin("pixctl").unwrap()
}

fn png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let path = dir.join(name);
    RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]))
        .save(&path)
        .unwrap();
    path
}

#[test]
fn fit_writes_png_and_line() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let out = dir.path().join("out");
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(0));
    let target = out.join("a.png");
    assert!(target.exists());
    let stdout = String::from_utf8(res.stdout).unwrap();
    assert_eq!(
        stdout.trim_end(),
        format!(
            "FIT  [unchanged] {}  {} (334×234)",
            input.display(),
            target.display()
        )
    );
}

#[test]
fn fit_creates_out_dir() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let out = dir.path().join("x/y/z");
    pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .assert()
        .code(0);
    assert!(out.join("a.png").exists());
}

#[test]
fn fit_jpeg_input() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("p.jpg");
    RgbImage::from_pixel(300, 200, Rgb([255, 255, 255]))
        .save(&input)
        .unwrap();
    let out = dir.path().join("out");
    pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .assert()
        .code(0);
    let img = image::open(out.join("p.png")).unwrap();
    assert!(!img.color().has_alpha());
}

#[test]
fn fit_twice_is_byte_identical() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let (o1, o2) = (dir.path().join("o1"), dir.path().join("o2"));
    for o in [&o1, &o2] {
        pixctl()
            .args(["fit", "--out"])
            .arg(o)
            .arg(&input)
            .assert()
            .code(0);
    }
    assert_eq!(
        std::fs::read(o1.join("a.png")).unwrap(),
        std::fs::read(o2.join("a.png")).unwrap()
    );
}

#[test]
fn same_stem_collision_exits_2() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let b = dir.path().join("a.jpg");
    RgbImage::from_pixel(100, 100, Rgb([0, 0, 0]))
        .save(&b)
        .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    let a2 = src.join("a.png");
    std::fs::rename(&a, &a2).unwrap();
    let b2 = src.join("a.jpg");
    std::fs::rename(&b, &b2).unwrap();
    let out = dir.path().join("out");
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&a2)
        .arg(&b2)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    // The first input owns the output name.
    assert_eq!(image::open(out.join("a.png")).unwrap().width(), 334);
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains(&format!("pixctl: {}:", b2.display())));
}

#[test]
fn refuses_to_overwrite_input() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let before = std::fs::read(&input).unwrap();
    pixctl()
        .args(["fit", "--out"])
        .arg(dir.path())
        .arg(&input)
        .assert()
        .code(2);
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[test]
fn fit_output_passes_check() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    for (label, extra) in [("unchanged", vec![]), ("upscale", vec!["--font-px", "6"])] {
        let out = dir.path().join(label);
        pixctl()
            .args(["fit", "--out"])
            .arg(&out)
            .args(&extra)
            .arg(&input)
            .assert()
            .code(0);
        let rgba = image::open(out.join("a.png")).unwrap().to_rgba8();
        for o in run_all(&rgba, Some(12.0)) {
            if ["width", "opaque", "framed"].contains(&o.name) {
                assert_eq!(o.status, Status::Pass, "{label}: {} {}", o.name, o.detail);
            }
        }
    }
}

#[test]
fn case_only_collision_exits_2() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    let a = png(&src, "A.png", 300, 200);
    let b = src.join("a.jpg");
    RgbImage::from_pixel(100, 100, Rgb([0, 0, 0]))
        .save(&b)
        .unwrap();
    let out = dir.path().join("out");
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    // The first input keeps the output file.
    assert_eq!(image::open(out.join("A.png")).unwrap().width(), 334);
}

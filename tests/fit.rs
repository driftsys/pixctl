//! End-to-end tests for `pixctl fit`.

use assert_cmd::Command;
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use pixctl::checks::{is_framed, run_all, Status};
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

/// Runs `fit` on one input into `dir/label`; returns the exit code and stdout.
fn fit_with(dir: &Path, label: &str, input: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let out = dir.join(label);
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .args(extra)
        .arg(input)
        .output()
        .unwrap();
    (res.status.code(), String::from_utf8(res.stdout).unwrap())
}

#[test]
fn fit_output_passes_check() {
    let dir = TempDir::new().unwrap();
    let text = png(dir.path(), "a.png", 300, 200);
    let icon = png(dir.path(), "dot.png", 32, 32);
    let cases = [
        ("unchanged", &text, &[][..], "unchanged", (334, 234)),
        (
            "upscale",
            &text,
            &["--font-px", "6"][..],
            "upscale",
            (634, 434),
        ),
        ("icon", &icon, &[][..], "icon", (32, 32)),
    ];
    for (label, input, extra, rule, size) in cases {
        let (code, stdout) = fit_with(dir.path(), label, input, extra);
        assert_eq!(code, Some(0), "{label}");
        let name = input.file_name().unwrap().to_str().unwrap();
        assert!(stdout.starts_with(&format!("FIT  [{rule}] ")), "{stdout}");
        assert!(
            stdout
                .trim_end()
                .ends_with(&format!("({}×{})", size.0, size.1)),
            "{stdout}"
        );
        let rgba = image::open(dir.path().join(label).join(name))
            .unwrap()
            .to_rgba8();
        assert_eq!(rgba.dimensions(), size, "{label}");
        for o in run_all(&rgba, Some(12.0)) {
            assert_eq!(o.status, Status::Pass, "{label}: {} {}", o.name, o.detail);
        }
    }
}

#[test]
fn oversize_and_too_small_exit_0_with_a_border() {
    let dir = TempDir::new().unwrap();
    let wide = png(dir.path(), "wide.png", 800, 100);
    let (code, stdout) = fit_with(dir.path(), "o", &wide, &[]);
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with("FIT  [oversize] "), "{stdout}");
    let rgba = image::open(dir.path().join("o/wide.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(rgba.dimensions(), (834, 134));
    assert!(is_framed(&rgba));

    let small = png(dir.path(), "small.png", 300, 200);
    let (code, stdout) = fit_with(dir.path(), "s", &small, &["--font-px", "5"]);
    assert_eq!(code, Some(0));
    assert!(stdout.starts_with("FIT  [too-small] "), "{stdout}");
    let rgba = image::open(dir.path().join("s/small.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(rgba.dimensions(), (334, 234));
    assert!(is_framed(&rgba));
}

#[test]
fn colours_survive_fit() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("c.png");
    RgbaImage::from_pixel(300, 200, Rgba([200, 100, 50, 255]))
        .save(&input)
        .unwrap();
    fit_with(dir.path(), "o", &input, &[]);
    let out = image::open(dir.path().join("o/c.png")).unwrap().to_rgb8();
    assert_eq!(out.get_pixel(100, 100).0, [200, 100, 50]);
}

#[test]
fn failing_image_first_does_not_stop_fit() {
    let dir = TempDir::new().unwrap();
    let good = png(dir.path(), "good.png", 300, 200);
    let corrupt = dir.path().join("corrupt.png");
    std::fs::write(&corrupt, b"not an image").unwrap();
    let missing = dir.path().join("missing.png");
    let out = dir.path().join("out");
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&missing)
        .arg(&corrupt)
        .arg(&good)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(out.join("good.png").exists());
    let stderr = String::from_utf8(res.stderr).unwrap();
    for bad in [&missing, &corrupt] {
        let name = bad.display().to_string();
        assert_eq!(stderr.matches(&name).count(), 1, "{stderr}");
    }
    assert_eq!(stderr.lines().count(), 2, "{stderr}");
    assert!(
        stderr.lines().all(|l| l.starts_with("pixctl: ")),
        "{stderr}"
    );
}

#[test]
fn output_dir_failure_is_reported_once() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let b = png(dir.path(), "b.png", 300, 200);
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(blocker.join("sub"))
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(res.stdout.is_empty());
    let stderr = String::from_utf8(res.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("cannot create"), "{stderr}");
}

/// Input `src/a.png` and an existing output directory `out`.
fn linked_setup(dir: &Path) -> (PathBuf, PathBuf, Vec<u8>) {
    let src = dir.join("src");
    std::fs::create_dir(&src).unwrap();
    let input = png(&src, "a.png", 300, 200);
    let out = dir.join("out");
    std::fs::create_dir(&out).unwrap();
    let before = std::fs::read(&input).unwrap();
    (input, out, before)
}

#[test]
fn refuses_a_hard_link_to_an_input() {
    let dir = TempDir::new().unwrap();
    let (input, out, before) = linked_setup(dir.path());
    std::fs::hard_link(&input, out.join("a.png")).unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains("would overwrite an input"));
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn refuses_a_symlink_to_an_input() {
    let dir = TempDir::new().unwrap();
    let (input, out, before) = linked_setup(dir.path());
    std::os::unix::fs::symlink(&input, out.join("a.png")).unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains("would overwrite an input"));
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn refuses_a_dangling_symlink_at_the_target() {
    let dir = TempDir::new().unwrap();
    let (input, out, before) = linked_setup(dir.path());
    let missing = dir.path().join("elsewhere.png");
    std::os::unix::fs::symlink(&missing, out.join("a.png")).unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains("link to a missing file"));
    assert!(!missing.exists());
    assert_eq!(std::fs::read(&input).unwrap(), before);
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

#[test]
fn refuses_an_output_that_is_another_input() {
    let dir = TempDir::new().unwrap();
    let raw = dir.path().join("raw2");
    std::fs::create_dir(&raw).unwrap();
    let first = png(&raw, "a.png", 300, 200);
    let d = dir.path().join("d");
    std::fs::create_dir(&d).unwrap();
    let second = png(&d, "a.png", 320, 240);
    let before = std::fs::read(&second).unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&d)
        .arg(&first)
        .arg(&second)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains("would overwrite an input"));
    assert_eq!(std::fs::read(&second).unwrap(), before);
}

#[test]
fn missing_input_does_not_block_other_images_in_an_existing_out_dir() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let b = png(dir.path(), "b.png", 300, 200);
    let out = dir.path().join("out");
    pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&a)
        .arg(&b)
        .assert()
        .code(0);
    // Second run: both outputs exist, and the first input is gone.
    std::fs::remove_file(&a).unwrap();
    std::fs::remove_file(out.join("b.png")).unwrap();
    std::fs::write(out.join("b.png"), b"old").unwrap();
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .arg(&a)
        .arg(&b)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    let stderr = String::from_utf8(res.stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(!stderr.contains("overwrite"), "{stderr}");
    assert_eq!(image::open(out.join("b.png")).unwrap().width(), 334);
}

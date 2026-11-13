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
    assert!(res.stdout.is_empty(), "stdout must stay empty in text mode");
    let stderr = String::from_utf8(res.stderr).unwrap();
    assert_eq!(
        stderr.trim_end(),
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

/// Runs `fit` on one input into `dir/label`; returns the exit code and stderr,
/// which carries the `FIT` lines. Asserts that stdout is empty.
fn fit_with(dir: &Path, label: &str, input: &Path, extra: &[&str]) -> (Option<i32>, String) {
    let out = dir.join(label);
    let res = pixctl()
        .args(["fit", "--out"])
        .arg(&out)
        .args(extra)
        .arg(input)
        .output()
        .unwrap();
    assert!(res.stdout.is_empty(), "stdout must stay empty in text mode");
    (res.status.code(), String::from_utf8(res.stderr).unwrap())
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
        let (code, stderr) = fit_with(dir.path(), label, input, extra);
        assert_eq!(code, Some(0), "{label}");
        let name = input.file_name().unwrap().to_str().unwrap();
        assert!(stderr.starts_with(&format!("FIT  [{rule}] ")), "{stderr}");
        assert!(
            stderr
                .trim_end()
                .ends_with(&format!("({}×{})", size.0, size.1)),
            "{stderr}"
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
    let (code, stderr) = fit_with(dir.path(), "o", &wide, &[]);
    assert_eq!(code, Some(0));
    assert!(stderr.starts_with("FIT  [oversize] "), "{stderr}");
    let rgba = image::open(dir.path().join("o/wide.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(rgba.dimensions(), (834, 134));
    assert!(is_framed(&rgba));

    let small = png(dir.path(), "small.png", 300, 200);
    let (code, stderr) = fit_with(dir.path(), "s", &small, &["--font-px", "5"]);
    assert_eq!(code, Some(0));
    assert!(stderr.starts_with("FIT  [too-small] "), "{stderr}");
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
    // Two error lines and the `FIT` line of the good image.
    assert_eq!(stderr.lines().count(), 3, "{stderr}");
    assert_eq!(
        stderr.lines().filter(|l| l.starts_with("pixctl: ")).count(),
        2,
        "{stderr}"
    );
    assert_eq!(
        stderr.lines().filter(|l| l.starts_with("FIT  ")).count(),
        1,
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
        .args(["fit", "--json", "--out"])
        .arg(&out)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(2));
    assert!(String::from_utf8(res.stderr)
        .unwrap()
        .contains("link to a missing file"));
    let doc: serde_json::Value = serde_json::from_slice(&res.stdout).unwrap();
    assert_eq!(doc["errors"][0]["code"], "write_failed");
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
    let errors = stderr.lines().filter(|l| l.starts_with("pixctl: "));
    assert_eq!(errors.count(), 1, "{stderr}");
    assert!(!stderr.contains("overwrite"), "{stderr}");
    assert_eq!(image::open(out.join("b.png")).unwrap().width(), 334);
}

/// Runs `fit --json` and parses the envelope from stdout.
fn fit_json(args: &[&Path], out: &Path) -> (Option<i32>, serde_json::Value, String) {
    let res = pixctl()
        .args(["fit", "--json", "--out"])
        .arg(out)
        .args(args)
        .output()
        .unwrap();
    let doc = serde_json::from_slice(&res.stdout).expect("stdout is one JSON document");
    (
        res.status.code(),
        doc,
        String::from_utf8(res.stderr).unwrap(),
    )
}

#[test]
fn json_envelope_lists_written_images() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let out = dir.path().join("out");
    let (code, doc, stderr) = fit_json(&[&a], &out);
    assert_eq!(code, Some(0));
    assert_eq!(doc["schema"], "pixctl/fit/v1");
    assert_eq!(doc["summary"]["written"], 1);
    assert_eq!(doc["errors"].as_array().unwrap().len(), 0);
    let r = &doc["results"][0];
    assert_eq!(r["path"], a.to_string_lossy().as_ref());
    assert_eq!(r["out"], out.join("a.png").to_string_lossy().as_ref());
    assert_eq!(r["rule"], "unchanged");
    assert_eq!(r["factor"], 1.0);
    assert_eq!(r["width"], 334);
    assert_eq!(r["height"], 234);
    assert!(stderr.is_empty(), "no FIT line in JSON mode: {stderr}");
}

#[test]
fn json_keys_are_in_envelope_order() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let res = pixctl()
        .args(["fit", "--json", "--out"])
        .arg(dir.path().join("out"))
        .arg(&a)
        .output()
        .unwrap();
    let text = String::from_utf8(res.stdout).unwrap();
    let pos = |k: &str| text.find(&format!("\n  \"{k}\"")).unwrap();
    assert!(pos("schema") < pos("results"));
    assert!(pos("results") < pos("summary"));
    assert!(pos("summary") < pos("errors"));
}

#[test]
fn json_mixed_run_has_results_and_coded_errors() {
    let dir = TempDir::new().unwrap();
    let good = png(dir.path(), "good.png", 300, 200);
    let corrupt = dir.path().join("corrupt.png");
    std::fs::write(&corrupt, b"not an image").unwrap();
    let emf = dir.path().join("x.emf");
    std::fs::write(&emf, b"x").unwrap();
    let out = dir.path().join("out");
    let (code, doc, stderr) = fit_json(&[&corrupt, &emf, &good], &out);
    assert_eq!(code, Some(2));
    assert_eq!(doc["results"].as_array().unwrap().len(), 1);
    assert_eq!(doc["summary"]["written"], 1);
    let errors = doc["errors"].as_array().unwrap();
    assert_eq!(errors[0]["code"], "read_failed");
    assert_eq!(errors[0]["file"], corrupt.to_string_lossy().as_ref());
    assert_eq!(errors[1]["code"], "unsupported_type");
    assert_eq!(stderr.lines().count(), 2, "{stderr}");
    assert!(
        stderr.lines().all(|l| l.starts_with("pixctl: ")),
        "{stderr}"
    );
}

#[test]
fn json_all_inputs_failing_still_prints_an_envelope() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.png");
    let (code, doc, _) = fit_json(&[&missing], &dir.path().join("out"));
    assert_eq!(code, Some(2));
    assert_eq!(doc["results"].as_array().unwrap().len(), 0);
    assert_eq!(doc["summary"]["written"], 0);
    assert_eq!(doc["errors"][0]["code"], "read_failed");
}

#[test]
fn json_out_dir_failed() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    let out = blocker.join("sub");
    let (code, doc, stderr) = fit_json(&[&a], &out);
    assert_eq!(code, Some(2));
    assert_eq!(doc["results"].as_array().unwrap().len(), 0);
    let errors = doc["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0]["code"], "out_dir_failed");
    assert_eq!(errors[0]["file"], out.to_string_lossy().as_ref());
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
}

#[test]
fn json_same_file() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let (code, doc, _) = fit_json(&[&input], dir.path());
    assert_eq!(code, Some(2));
    assert_eq!(doc["results"].as_array().unwrap().len(), 0);
    assert_eq!(doc["errors"][0]["code"], "same_file");
    assert_eq!(doc["errors"][0]["file"], input.to_string_lossy().as_ref());
    assert_eq!(
        doc["errors"][0]["hint"],
        "choose a different --out directory"
    );
}

#[test]
fn json_upscale_reports_the_factor() {
    let dir = TempDir::new().unwrap();
    let a = png(dir.path(), "a.png", 300, 200);
    let res = pixctl()
        .args(["fit", "--json", "--font-px", "6", "--out"])
        .arg(dir.path().join("out"))
        .arg(&a)
        .output()
        .unwrap();
    assert_eq!(res.status.code(), Some(0));
    let doc: serde_json::Value = serde_json::from_slice(&res.stdout).unwrap();
    assert_eq!(doc["results"][0]["rule"], "upscale");
    assert_eq!(doc["results"][0]["factor"], 2.0);
}

#[test]
fn json_name_collision_with_three_inputs_names_the_first() {
    let dir = TempDir::new().unwrap();
    let mut inputs = Vec::new();
    for ext in ["png", "jpg", "jpeg"] {
        let sub = dir.path().join(format!("src-{ext}"));
        std::fs::create_dir(&sub).unwrap();
        let p = sub.join(format!("a.{ext}"));
        RgbImage::from_pixel(100, 100, Rgb([0, 0, 0]))
            .save(&p)
            .unwrap();
        inputs.push(p);
    }
    let refs: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
    let (code, doc, _) = fit_json(&refs, &dir.path().join("out"));
    assert_eq!(code, Some(2));
    let errors = doc["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    for e in errors {
        assert_eq!(e["code"], "name_collision");
        assert_eq!(e["with"], inputs[0].to_string_lossy().as_ref());
        assert_eq!(e["hint"], "rename one of the inputs");
    }
}

#[test]
fn json_name_collision_names_the_earlier_input() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir(&src).unwrap();
    let a = png(&src, "a.png", 300, 200);
    let b = src.join("a.jpg");
    RgbImage::from_pixel(100, 100, Rgb([0, 0, 0]))
        .save(&b)
        .unwrap();
    let (code, doc, _) = fit_json(&[&a, &b], &dir.path().join("out"));
    assert_eq!(code, Some(2));
    assert_eq!(doc["summary"]["written"], 1);
    let e = &doc["errors"][0];
    assert_eq!(e["code"], "name_collision");
    assert_eq!(e["file"], b.to_string_lossy().as_ref());
    assert_eq!(e["with"], a.to_string_lossy().as_ref());
}

#[test]
fn json_write_failed() {
    let dir = TempDir::new().unwrap();
    let input = png(dir.path(), "a.png", 300, 200);
    let out = dir.path().join("out");
    // A directory at the target path makes the write fail.
    std::fs::create_dir_all(out.join("a.png")).unwrap();
    let (code, doc, _) = fit_json(&[&input], &out);
    assert_eq!(code, Some(2));
    assert_eq!(doc["errors"][0]["code"], "write_failed");
    assert_eq!(doc["errors"][0]["file"], input.to_string_lossy().as_ref());
}

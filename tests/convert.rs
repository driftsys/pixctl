//! End-to-end tests for EMF/WMF conversion, with fake converter scripts.
#![cfg(unix)]

use assert_cmd::Command;
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn pixctl() -> Command {
    let mut c = Command::cargo_bin("pixctl").unwrap();
    c.env_remove("PIXCTL_CONVERTER")
        .env_remove("PIXCTL_CONVERT_TIMEOUT_SECS")
        .env("PATH", "");
    c
}

fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, format!("#!/bin/sh\nPATH=/bin:/usr/bin\n{body}\n")).unwrap();
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn fixture(dir: &Path) -> PathBuf {
    let p = dir.join("fixture.png");
    image::RgbaImage::from_pixel(40, 30, image::Rgba([10, 20, 30, 255]))
        .save(&p)
        .unwrap();
    p
}

/// An Inkscape fake that copies `png` to the `--export-filename=` value.
fn fake_inkscape(dir: &Path, png: &Path) -> PathBuf {
    script(
        dir,
        "fake-inkscape",
        &format!(
            "for a in \"$@\"; do case \"$a\" in --export-filename=*) out=\"${{a#--export-filename=}}\";; esac; done\ncp '{}' \"$out\"",
            png.display()
        ),
    )
}

/// A soffice fake that writes `<outdir>/<stem>.png`.
fn fake_soffice(dir: &Path, png: &Path) -> PathBuf {
    script(
        dir,
        "fake-soffice",
        &format!(
            "while [ $# -gt 0 ]; do case \"$1\" in --outdir) outdir=\"$2\"; shift;; -*) ;; *) in=\"$1\";; esac; shift; done\nb=$(basename \"$in\")\ncp '{}' \"$outdir/${{b%.*}}.png\"",
            png.display()
        ),
    )
}

fn emf(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"not a real emf").unwrap();
    p
}

fn json(out: &[u8]) -> Value {
    serde_json::from_slice(out).unwrap()
}

#[test]
fn inspect_uses_the_inkscape_converter() {
    let d = TempDir::new().unwrap();
    let conv = fake_inkscape(d.path(), &fixture(d.path()));
    let input = emf(d.path(), "a.emf");
    let out = pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let r = &json(&out)["results"][0];
    assert_eq!(r["path"], input.to_str().unwrap());
    assert_eq!(
        (r["width"].as_u64(), r["height"].as_u64()),
        (Some(40), Some(30))
    );
}

#[test]
fn inspect_uses_the_soffice_converter_on_a_wmf() {
    let d = TempDir::new().unwrap();
    let conv = fake_soffice(d.path(), &fixture(d.path()));
    let input = emf(d.path(), "b.wmf");
    let out = pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let r = &json(&out)["results"][0];
    assert_eq!(r["path"], input.to_str().unwrap());
    assert_eq!(r["width"].as_u64(), Some(40));
}

#[test]
fn converter_is_found_on_path() {
    let d = TempDir::new().unwrap();
    let bin = d.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    fake_inkscape(&bin, &fixture(d.path()));
    std::fs::rename(bin.join("fake-inkscape"), bin.join("inkscape")).unwrap();
    let input = emf(d.path(), "a.emf");
    pixctl()
        .env("PATH", &bin)
        .arg("inspect")
        .arg(&input)
        .assert()
        .success();
}

#[test]
fn fit_converts_an_upper_case_extension() {
    let d = TempDir::new().unwrap();
    let conv = fake_inkscape(d.path(), &fixture(d.path()));
    let input = emf(d.path(), "a.EMF");
    let out = d.path().join("out");
    pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .arg("fit")
        .arg(&input)
        .arg("--out")
        .arg(&out)
        .assert()
        .success();
    assert!(out.join("a.png").is_file());
}

#[test]
fn nonzero_exit_gives_convert_failed_with_the_status() {
    let d = TempDir::new().unwrap();
    let conv = script(d.path(), "fake-inkscape", "exit 3");
    let input = emf(d.path(), "a.emf");
    let out = pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let e = &json(&out)["errors"][0];
    assert_eq!(e["code"], "convert_failed");
    assert!(e["message"].as_str().unwrap().contains("exit status: 3"));
    assert_eq!(e["file"], input.to_str().unwrap());
}

#[test]
fn no_png_written_gives_convert_failed() {
    let d = TempDir::new().unwrap();
    let conv = script(d.path(), "fake-soffice", "exit 0");
    let input = emf(d.path(), "a.emf");
    let out = pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    assert_eq!(json(&out)["errors"][0]["code"], "convert_failed");
}

#[test]
fn unknown_converter_name_gives_convert_failed_with_a_hint() {
    let d = TempDir::new().unwrap();
    let input = emf(d.path(), "a.emf");
    let out = pixctl()
        .env("PIXCTL_CONVERTER", "gimp")
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let e = &json(&out)["errors"][0];
    assert_eq!(e["code"], "convert_failed");
    assert!(e["hint"].as_str().unwrap().contains("Inkscape"));
}

#[test]
fn a_hanging_converter_is_killed_and_the_next_file_is_processed() {
    let d = TempDir::new().unwrap();
    let png = fixture(d.path());
    let conv = script(d.path(), "fake-inkscape", "sleep 30");
    let input = emf(d.path(), "a.emf");
    let start = std::time::Instant::now();
    let out = pixctl()
        .env("PIXCTL_CONVERTER", &conv)
        .env("PIXCTL_CONVERT_TIMEOUT_SECS", "1")
        .args(["inspect", "--json"])
        .arg(&input)
        .arg(&png)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    assert!(start.elapsed().as_secs() < 20);
    let v = json(&out);
    assert_eq!(v["errors"][0]["code"], "convert_failed");
    assert!(v["errors"][0]["message"]
        .as_str()
        .unwrap()
        .contains("timed out"));
    assert_eq!(v["results"].as_array().unwrap().len(), 1);
}

#[test]
fn no_converter_gives_unsupported_type_with_the_hint() {
    let d = TempDir::new().unwrap();
    let input = emf(d.path(), "a.emf");
    let out = pixctl()
        .args(["inspect", "--json"])
        .arg(&input)
        .assert()
        .code(2)
        .get_output()
        .stdout
        .clone();
    let e = &json(&out)["errors"][0];
    assert_eq!(e["code"], "unsupported_type");
    assert_eq!(
        e["hint"],
        "install Inkscape or LibreOffice, or set PIXCTL_CONVERTER"
    );
}

#[test]
fn the_temporary_directory_is_removed_on_success_and_failure() {
    let d = TempDir::new().unwrap();
    let tmpdir = d.path().join("tmp");
    std::fs::create_dir(&tmpdir).unwrap();
    let png = fixture(d.path());
    let good = fake_inkscape(d.path(), &png);
    let bad = script(d.path(), "fake-soffice", "exit 1");
    let input = emf(d.path(), "a.emf");
    for conv in [good, bad] {
        pixctl()
            .env("TMPDIR", &tmpdir)
            .env("PIXCTL_CONVERTER", &conv)
            .arg("inspect")
            .arg(&input)
            .assert()
            .code(predicates::prelude::predicate::in_iter([0, 2]));
        let left: Vec<_> = std::fs::read_dir(&tmpdir).unwrap().collect();
        assert!(left.is_empty(), "{left:?}");
    }
}

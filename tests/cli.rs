use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;

fn pixctl() -> Command {
    Command::cargo_bin("pixctl").unwrap()
}

#[test]
fn version_prints_crate_version() {
    pixctl()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_all_subcommands() {
    pixctl()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("inspect"))
        .stdout(contains("fit"))
        .stdout(contains("check"));
}

#[test]
fn stubs_exit_2_with_tracking_link() {
    pixctl()
        .args(["fit", "a.png", "--out", "out"])
        .assert()
        .code(2)
        .stderr(contains("not yet implemented"));
}

#[test]
fn version_reports_contract() {
    pixctl()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("(display contract 1)"));
}

#[test]
fn fit_has_no_format_flag() {
    pixctl()
        .args(["fit", "a.png", "--out", "o", "--format", "png"])
        .assert()
        .code(2)
        .stderr(contains("unexpected argument"));
}

#[test]
fn check_accepts_strict_and_json() {
    pixctl()
        .args([
            "check",
            "missing.png",
            "--strict",
            "--json",
            "--font-px",
            "12",
        ])
        .assert()
        .code(2)
        .stderr(predicates::str::contains("unexpected argument").not());
}

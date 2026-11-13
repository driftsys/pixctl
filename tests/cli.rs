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

#[test]
fn subcommand_help_has_examples() {
    for command in ["inspect", "check", "fit"] {
        pixctl()
            .args([command, "--help"])
            .assert()
            .success()
            .stdout(contains("Examples:"))
            .stdout(contains(format!("pixctl {command} ")));
    }
}

#[test]
fn top_level_help_ends_with_the_documentation_link() {
    let out = pixctl().arg("--help").output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.trim_end().lines().last().unwrap().trim(),
        "Documentation: https://github.com/driftsys/pixctl#readme"
    );
}

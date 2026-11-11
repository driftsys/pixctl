use assert_cmd::Command;
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
    for args in [
        &["fit", "a.png", "--out", "out"][..],
        &["check", "a.png"][..],
    ] {
        pixctl()
            .args(args)
            .assert()
            .code(2)
            .stderr(contains("not yet implemented"));
    }
}

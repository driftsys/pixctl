use clap::Parser;
use pixctl::cli::{Cli, Command};
use pixctl::inspect;

const TRACKING: &str = "https://github.com/driftsys/pixctl/issues/1";

fn not_yet_implemented(name: &str) -> anyhow::Result<i32> {
    eprintln!("{name}: not yet implemented (tracking: {TRACKING})");
    Ok(2)
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Inspect {
            images,
            json,
            font_px,
        } => Ok(inspect::run(
            &images,
            json,
            font_px,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        )),
        Command::Fit { .. } => not_yet_implemented("fit"),
        Command::Check { .. } => not_yet_implemented("check"),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("pixctl: {err:#}");
            std::process::exit(2);
        }
    }
}

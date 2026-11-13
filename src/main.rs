use clap::Parser;
use pixctl::cli::{Cli, Command};
use pixctl::{check, fit_cmd, inspect};

fn main() {
    let cli = Cli::parse();
    let result: anyhow::Result<i32> = match cli.command {
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
        Command::Fit {
            images,
            out,
            font_px,
        } => Ok(fit_cmd::run(
            &images,
            &out,
            font_px,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        )),
        Command::Check {
            images,
            strict,
            font_px,
            json,
            ..
        } => Ok(check::run(
            &images,
            strict,
            font_px,
            json,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        )),
    };
    match result {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("pixctl: {err:#}");
            std::process::exit(2);
        }
    }
}

use clap::Parser;
use pixctl::cli::{Cli, Command};
use pixctl::{check, fit_cmd, inspect};

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Inspect {
            images,
            json,
            font_px,
        } => inspect::run(
            &images,
            json,
            font_px,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ),
        Command::Fit {
            images,
            out,
            font_px,
        } => fit_cmd::run(
            &images,
            &out,
            font_px,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ),
        Command::Check {
            images,
            strict,
            font_px,
            json,
            ..
        } => check::run(
            &images,
            strict,
            font_px,
            json,
            &mut std::io::stdout().lock(),
            &mut std::io::stderr().lock(),
        ),
    };
    std::process::exit(code);
}

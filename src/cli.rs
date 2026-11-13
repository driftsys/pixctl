use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pixctl",
    version = concat!(env!("CARGO_PKG_VERSION"), " (display contract 1)"),
    about = "Fits raster images for Markdown docs and gates their display size",
    after_help = "Documentation: https://github.com/driftsys/pixctl#readme"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Report size, alpha, estimated font size and rule
    #[command(after_help = "\
Examples:
  pixctl inspect docs/img/diagram.png
  pixctl inspect docs/img/*.png --json
  pixctl inspect screenshot.png --font-px 12")]
    Inspect {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Emit JSON
        #[arg(long)]
        json: bool,
        /// Use this font size in pixels instead of the estimate
        #[arg(long, value_name = "F", value_parser = parse_font_px)]
        font_px: Option<f32>,
    },
    /// Scale, flatten, pad and frame images as PNG
    #[command(after_help = "\
Examples:
  pixctl fit screenshot.png --out docs/img
  pixctl fit raw/*.png --out docs/img --font-px 12")]
    Fit {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Output directory
        #[arg(long)]
        out: PathBuf,
        /// Use this font size in pixels instead of the estimate
        #[arg(long, value_name = "F", value_parser = parse_font_px)]
        font_px: Option<f32>,
    },
    /// Run the display gate on images
    #[command(after_help = "\
Examples:
  pixctl check docs/img/*.png
  pixctl check docs/img/*.png --strict
  pixctl check docs/img/diagram.png --json")]
    Check {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Display target
        #[arg(long, value_enum, default_value_t = Target::Markdown)]
        target: Target,
        /// Treat REVIEW results as failures (exit 1)
        #[arg(long)]
        strict: bool,
        /// Use this font size in pixels instead of the estimate
        #[arg(long, value_name = "F", value_parser = parse_font_px)]
        font_px: Option<f32>,
        /// Emit JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Target {
    Markdown,
}

/// Accepts a finite number greater than zero.
fn parse_font_px(s: &str) -> Result<f32, String> {
    let value: f32 = s.parse().map_err(|_| format!("`{s}` is not a number"))?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!("`{s}` must be a finite number greater than 0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::CONTRACT_VERSION;
    use clap::CommandFactory;

    #[test]
    fn version_string_carries_contract_version() {
        let version = Cli::command().render_version();
        assert!(
            version.contains(&format!("(display contract {CONTRACT_VERSION})")),
            "{version}"
        );
    }

    #[test]
    fn parse_font_px_rejects_non_positive_and_non_finite() {
        for value in ["0", "0.0", "-0", "-3", "abc", "", "NaN", "inf", "-inf"] {
            assert!(parse_font_px(value).is_err(), "value {value}");
        }
    }

    #[test]
    fn parse_font_px_accepts_positive_decimals() {
        assert_eq!(parse_font_px("10.5"), Ok(10.5));
        assert_eq!(parse_font_px("8"), Ok(8.0));
    }
}

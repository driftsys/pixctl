use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pixctl",
    version,
    about = "Fits raster images for Markdown docs and gates their display size"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Report size, alpha, estimated font size and rule
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
    /// (not yet implemented) Convert, scale, flatten, pad and frame images
    Fit {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Output directory
        #[arg(long)]
        out: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Png)]
        format: Format,
    },
    /// (not yet implemented) Run the display gate on images
    Check {
        #[arg(required = true)]
        images: Vec<PathBuf>,
        /// Display target
        #[arg(long, value_enum, default_value_t = Target::Markdown)]
        target: Target,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Png,
    Webp,
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

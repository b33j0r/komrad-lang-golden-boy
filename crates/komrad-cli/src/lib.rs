use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use palette::LinSrgb;
use std::path::PathBuf;
use tracing::{debug, info, trace};

mod banner;

#[derive(Clone, Debug, Parser)]
#[command(name = "komrad", version, about = "Komrad CLI")]
struct Args {
    #[clap(subcommand)]
    subcommand: Option<Subcommands>,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[derive(Clone, Debug, Subcommand)]
enum Subcommands {
    Parse { file: Option<PathBuf> },

    Run { file: Option<PathBuf> },
}

pub async fn main() {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(args.verbose)
        .with_target(false)
        // .with_line_number(true)
        // .with_file(true)
        .without_time()
        .with_ansi(true)
        .with_level(true)
        .init();

    banner();

    info!("{}", "Komrad CLI starting".bright_cyan());
}

fn banner() {
    let text = "Komrad";

    let stops = vec![
        (0.0, LinSrgb::new(1.0, 0.0, 0.5)),
        (0.3, LinSrgb::new(0.85, 0.5, 0.9)),
        (0.7, LinSrgb::new(0.8, 0.7, 0.0)),
        (1.0, LinSrgb::new(0.3, 0.7, 0.9)),
    ];

    let banner = banner::gradient_banner(text, &stops);
    debug!("\n{}", banner);
}

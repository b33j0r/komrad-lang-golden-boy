use figlet_rs::FIGfont;
use owo_colors::OwoColorize;
use palette::{LinSrgb, Mix};
use tracing::{debug, info};

mod banner;

fn main() {
    let text = "KOMRAD 0.1";

    let stops = vec![
        (0.0, LinSrgb::new(1.0, 0.0, 0.5)),
        (0.35, LinSrgb::new(0.85, 0.5, 0.9)),
        (0.5, LinSrgb::new(0.8, 0.7, 0.0)),
        (1.0, LinSrgb::new(0.3, 0.65, 0.9)),
    ];

    let banner = banner::gradient_banner(text, &stops);

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
        .with_target(false)
        // .with_line_number(true)
        // .with_file(true)
        .without_time()
        .with_ansi(true)
        .with_level(true)
        .init();

    debug!("\n{}", banner);


}

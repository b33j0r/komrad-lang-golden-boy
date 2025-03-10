use clap::{Parser, Subcommand};
use komrad_vm::{ModuleCommand, Scope};
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

    /// Wait for ctrl+c to exit
    #[arg(long, global = true, default_value_t = false)]
    wait: bool,
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

    info!("{}", "Komrad CLI starting".bright_cyan());

    match args.subcommand {
        Some(Subcommands::Parse { file }) => {
            if let Some(file) = file {
                info!("Parsing file: {}", file.display());
                let source = std::fs::read_to_string(&file).expect("Failed to read file");
                match komrad_parser::parse_verbose(&source) {
                    Ok(module_builder) => {
                        info!("Parsed module: {:?}", module_builder);

                        info!("Built module: {:?}", module_builder);
                    }
                    Err(err) => {
                        info!("Failed to parse file: {}", err);
                    }
                }
            }
        }
        Some(Subcommands::Run { file }) => {
            if let Some(file) = file {
                info!("Running file: {}", file.display());
                let source = std::fs::read_to_string(&file).expect("Failed to read file");
                match komrad_parser::parse_verbose(&source) {
                    Ok(module_builder) => {
                        info!("Parsed module: {:?}", module_builder);

                        let system = komrad_vm::System::spawn();
                        let module = system.await.create_module("main").await;

                        let scope = module.get_scope().await;
                        info!("Module scope: {:?}", scope);

                        for statement in module_builder.statements() {
                            if statement.is_no_op() {
                                continue;
                            }
                            module
                                .send_command(ModuleCommand::ExecuteStatement(statement.clone()))
                                .await;
                        }

                        if args.wait {
                            info!("Waiting for ctrl+c to exit...");
                            tokio::signal::ctrl_c()
                                .await
                                .expect("Failed to wait for ctrl+c");
                        } else {
                            info!("Exiting immediately...");
                        }

                        info!("Module scope: {:?}", scope);
                    }
                    Err(err) => {
                        info!("Failed to parse file: {}", err);
                    }
                }
            }
        }
        None => {
            banner();
        }
    }
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

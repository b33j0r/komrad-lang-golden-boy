use crate::banner;
use crate::banner::banner;
use clap::{Parser, Subcommand};
use komrad_ast::prelude::{CallExpr, Expr, Message, Statement, Value};
use komrad_ast::sexpr::ToSexpr;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use tracing::{debug, info};

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

    /// Wait for 100 ms before exiting
    #[arg(long, global = true, default_value_t = false)]
    wait_100: bool,
}

#[derive(Clone, Debug, Subcommand)]
enum Subcommands {
    Parse {
        file: PathBuf,

        #[clap(long, value_enum)]
        fmt: Option<KomradOutputFormat>,
    },
    Run {
        file: PathBuf,
    },
}

#[derive(Clone, Debug, clap::ValueEnum, Default)]
enum KomradOutputFormat {
    Komrad,
    #[default]
    Sexpr,
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
    banner();

    match args.subcommand {
        Some(Subcommands::Parse { file, fmt }) => {
            info!("Parsing file: {}", file.display());
            let source = std::fs::read_to_string(&file).expect("Failed to read file");
            match komrad_parser::parse_verbose(&source) {
                Ok(module_builder) => {
                    debug!("Parsed module: {:?}", module_builder);
                    match fmt {
                        None | Some(KomradOutputFormat::Komrad) => {
                            let sexpr = module_builder.to_sexpr();
                            println!("{}", sexpr);
                        }
                        Some(KomradOutputFormat::Sexpr) => {
                            let sexpr = module_builder.to_sexpr();
                            println!("{}", sexpr);
                        }
                    }
                }
                Err(err) => {
                    info!("Failed to parse file: {}", err);
                }
            }
        }
        Some(Subcommands::Run { file }) => {
            info!("Running file: {}", file.display());
            let source = std::fs::read_to_string(&file).expect("Failed to read file");
            match komrad_parser::parse_verbose(&source) {
                Ok(module_builder) => {
                    let block = module_builder.build_block();
                    let system = komrad_vm::System::new();
                    let agent = system.create_agent("main", &block).await;

                    match agent
                        .send(Message::new(vec![Value::Word("main".into())], None))
                        .await
                    {
                        Ok(_) => {
                            info!("Main sent to agent");
                        }
                        Err(err) => {
                            info!("Failed to send main message: {}", err);
                        }
                    }

                    if args.wait_100 {
                        info!("Waiting for 100 ms...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                }
                Err(err) => {
                    info!("Failed to parse file: {}", err);
                }
            }
        }
        None => {
            println!("Use `komrad --help` for more information.");
        }
    }
}

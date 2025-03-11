use crate::banner;
use clap::{Parser, Subcommand};
use komrad_ast::prelude::{CallExpr, Expr, Statement, Value};
use komrad_ast::sexpr::ToSexpr;
use komrad_vm::ModuleCommand;
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
                    info!("Parsed module: {:?}", module_builder);

                    let system = komrad_vm::System::spawn();
                    let module = system.await.create_module("main").await;
                    let module_channel = module.get_channel();
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

                    module
                        .send_command(ModuleCommand::ExecuteStatement(Statement::Expr(
                            Expr::Call(
                                //
                                CallExpr::new(
                                    Expr::Value(Value::Channel(module_channel)).into(),
                                    vec![Expr::Value(Value::Word("main".to_string())).into()],
                                ),
                            ),
                        )))
                        .await;

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
        None => {
            banner::banner();
        }
    }
}

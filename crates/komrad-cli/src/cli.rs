use crate::banner::banner;
use clap::{Parser, Subcommand};
use komrad_ast::prelude::{Message, Value};
use komrad_ast::sexpr::ToSexpr;
use notify::Watcher;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
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

        #[clap(long, default_value_t = false)]
        watch: bool,
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

    tracing_subscriber::fmt()
        .with_max_level(args.verbose)
        .with_target(false)
        .without_time()
        .with_ansi(true)
        .with_level(true)
        .init();

    info!("{}", "Komrad CLI starting".bright_cyan());
    banner();

    match args.clone().subcommand {
        Some(Subcommands::Parse { file, fmt }) => handle_parse(file, fmt),
        Some(Subcommands::Run { file, watch }) => {
            if watch {
                handle_run_watch(file).await;
            } else {
                handle_run(file, &args).await;
            }
        }
        None => {
            println!("Use `komrad --help` for more information.");
        }
    }
}

fn handle_parse(file: PathBuf, fmt: Option<KomradOutputFormat>) {
    info!("Parsing file: {}", file.display());
    let source = std::fs::read_to_string(&file).expect("Failed to read file");
    match komrad_parser::parse_verbose(&source) {
        Ok(module_builder) => {
            debug!("Parsed module: {:?}", module_builder);
            let sexpr = module_builder.to_sexpr();
            match fmt {
                None | Some(KomradOutputFormat::Komrad) => println!("{}", sexpr),
                Some(KomradOutputFormat::Sexpr) => println!("{}", sexpr),
            }
        }
        Err(err) => {
            info!("Failed to parse file: {}", err);
        }
    }
}

/// Runs the file once by reading, parsing, building the block, creating the system/agent,
/// and sending the "main" message. Returns the system instance so that it can be shut down later.
async fn run_file_once(file: &PathBuf) -> Option<komrad_vm::System> {
    info!("Running file: {}", file.display());
    match tokio::fs::read_to_string(file).await {
        Ok(source) => {
            debug!("Read source: {}", source);
            match komrad_parser::parse_verbose(&source) {
                Ok(module_builder) => {
                    let block = module_builder.build_block();
                    let system = komrad_vm::System::new();
                    let agent = system.create_agent("main", &block).await;
                    match agent
                        .send(Message::new(vec![Value::Word("main".into())], None))
                        .await
                    {
                        Ok(_) => info!("Main sent to agent"),
                        Err(err) => info!("Failed to send main message: {}", err),
                    }
                    Some(system)
                }
                Err(err) => {
                    info!("Failed to parse file: {}", err);
                    None
                }
            }
        }
        Err(err) => {
            info!("Failed to read file: {}", err);
            None
        }
    }
}

/// Non‑watch mode: execute the file once then optionally wait.
async fn handle_run(file: PathBuf, args: &Args) {
    let system = run_file_once(&file).await;
    if args.wait_100 {
        info!("Waiting for 100 ms...");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    if args.wait {
        info!("Waiting for ctrl+c...");
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to wait for ctrl+c");
        if let Some(system) = system {
            system.shutdown().await;
        }
    }
}

/// Watch mode: set up a file watcher using `notify` v8 and hot-reload on file changes.
/// Before running the file again, the previous system instance is gracefully shut down.
async fn handle_run_watch(file: PathBuf) {
    use notify::{Config, RecommendedWatcher, RecursiveMode};
    use std::sync::{mpsc, Arc, Mutex};

    info!("Running file in watch mode: {}", file.display());
    // Initial run
    let mut active_system = run_file_once(&file).await;

    // Setup file watcher
    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).expect("Failed to initialize watcher");
    watcher
        .watch(&file, RecursiveMode::NonRecursive)
        .expect("Failed to watch file");

    info!("Watching for changes... Press ctrl+c to exit.");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received, exiting watch mode.");
                if let Some(system) = active_system.take() {
                    system.shutdown().await;
                }
                break;
            },
            event = tokio::task::spawn_blocking({
                let rx = rx.clone();
                move || rx.lock().unwrap().recv()
            }) => {
                match event {
                    Ok(Ok(ev)) => {
                        info!("File change detected: {:?}", ev);
                        // Shutdown the previously running system if any
                        if let Some(system) = active_system.take() {
                            info!("Shutting down previous system instance.");
                            system.shutdown().await;
                        }
                        // Re-run the file and store the new system instance
                        active_system = run_file_once(&file).await;
                    },
                    Ok(Err(e)) => {
                        info!("Watcher error: {}", e);
                    },
                    Err(e) => {
                        info!("Error in watcher task: {}", e);
                    }
                }
            }
        }
    }
}

mod commands;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "loom", version, about = "Loom workflow compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new workflow directory
    Init {
        /// Workflow name
        name: String,
    },
    /// Validate a workflow directory
    Validate {
        /// Workflow directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Compile workflow to target language
    Build {
        /// Workflow directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Target language
        #[arg(long, default_value = "rust")]
        lang: String,

        /// Emit format (overrides --lang)
        #[arg(long)]
        emit: Option<String>,
    },
    /// Print the state graph
    Graph {
        /// Workflow directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Profile to render
        #[arg(long)]
        profile: Option<String>,

        /// Output format: mermaid, dot
        #[arg(long, default_value = "mermaid")]
        format: String,
    },
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            commands::init::run(&name)
        }
        Commands::Validate { dir } => {
            commands::validate::run(&dir)
        }
        Commands::Build { dir, lang, emit } => {
            let format = if let Some(emit) = &emit {
                match emit.as_str() {
                    "toml" => commands::build::EmitFormat::Toml,
                    _ => return Err(miette::miette!("unsupported emit format: {}", emit)),
                }
            } else {
                match lang.as_str() {
                    "rust" => commands::build::EmitFormat::Rust,
                    _ => return Err(miette::miette!("unsupported language: {} (supported: rust)", lang)),
                }
            };
            commands::build::run(&dir, format)
        }
        Commands::Graph { dir, profile, format } => {
            let fmt = match format.as_str() {
                "mermaid" => loom_core::graph::render::RenderFormat::Mermaid,
                "dot" => loom_core::graph::render::RenderFormat::Dot,
                _ => return Err(miette::miette!("unsupported format: {} (supported: mermaid, dot)", format)),
            };
            commands::graph::run(&dir, profile.as_deref(), fmt)
        }
    }
}

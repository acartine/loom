mod commands;
mod templates;

use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

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
        /// Built-in template identifier
        #[arg(long)]
        template: Option<String>,

        /// Workflow name
        name: String,
    },
    /// Inspect bundled workflow templates
    #[command(alias = "templates")]
    Template {
        #[command(subcommand)]
        command: TemplateCommands,
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

        /// Output format: mermaid, dot, ascii
        #[arg(long, default_value = "mermaid")]
        format: String,
    },
    /// Interactively simulate walking a workflow
    Sim {
        /// Workflow directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,

        /// Profile to simulate
        #[arg(long)]
        profile: Option<String>,
    },
    /// Diff two workflow versions
    Diff {
        /// Old workflow directory
        old_dir: PathBuf,

        /// New workflow directory
        new_dir: PathBuf,
    },
    /// Check backward compatibility between workflow versions
    CheckCompat {
        /// Old workflow directory
        old_dir: PathBuf,

        /// New workflow directory
        new_dir: PathBuf,

        /// Emit TOML state migration map
        #[arg(long)]
        emit_map: bool,
    },
    /// Update the installed loom binary to the latest release
    Update {
        /// Report whether an update is available without changing anything
        #[arg(long)]
        check: bool,

        /// Reinstall the resolved version even when it matches the current version
        #[arg(long)]
        force: bool,
    },
    /// Remove the installed loom binary from the system
    Uninstall {
        /// Skip the confirmation prompt
        #[arg(long)]
        force: bool,

        /// Also remove channel directories (~/.local/bin/acartine_loom/)
        #[arg(long)]
        purge: bool,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// List bundled workflow templates
    List,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { template, name } => commands::init::run(&name, template.as_deref()),
        Commands::Template { command } => match command {
            TemplateCommands::List => commands::template::list(),
        },
        Commands::Validate { dir } => commands::validate::run(&dir),
        Commands::Build { dir, lang, emit } => {
            let format = if let Some(emit) = &emit {
                match emit.as_str() {
                    "toml" => commands::build::EmitFormat::Toml,
                    "knots-bundle" => commands::build::EmitFormat::KnotsBundle,
                    _ => {
                        return Err(miette::miette!(
                            "unsupported emit format: {} (supported: toml, knots-bundle)",
                            emit
                        ))
                    }
                }
            } else {
                match lang.as_str() {
                    "rust" => commands::build::EmitFormat::Rust,
                    "go" => commands::build::EmitFormat::Go,
                    "python" => commands::build::EmitFormat::Python,
                    _ => {
                        return Err(miette::miette!(
                            "unsupported language: {} (supported: rust, go, python)",
                            lang
                        ))
                    }
                }
            };
            commands::build::run(&dir, format)
        }
        Commands::Graph {
            dir,
            profile,
            format,
        } => {
            let fmt = match format.as_str() {
                "mermaid" => loom_core::graph::render::RenderFormat::Mermaid,
                "dot" => loom_core::graph::render::RenderFormat::Dot,
                "ascii" => loom_core::graph::render::RenderFormat::Ascii,
                _ => {
                    return Err(miette::miette!(
                        "unsupported format: {} (supported: mermaid, dot, ascii)",
                        format
                    ))
                }
            };
            commands::graph::run(&dir, profile.as_deref(), fmt)
        }
        Commands::Sim { dir, profile } => commands::sim::run(&dir, profile.as_deref()),
        Commands::Diff { old_dir, new_dir } => commands::diff::run(&old_dir, &new_dir),
        Commands::CheckCompat {
            old_dir,
            new_dir,
            emit_map,
        } => commands::compat::run(&old_dir, &new_dir, emit_map),
        Commands::Update { check, force } => commands::update::run(check, force),
        Commands::Uninstall { force, purge } => commands::uninstall::run(force, purge),
        Commands::Completions { shell } => {
            commands::completions::run(shell);
            Ok(())
        }
    }
}

use super::install_paths::{
    channel_root, current_executable_path, validate_install_location, BIN_NAME,
};
use miette::{Context, IntoDiagnostic};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// Find symlinks in well-known locations that point to the given target.
fn find_active_symlinks(target: &Path) -> Vec<PathBuf> {
    let canonical_target = fs::canonicalize(target).ok();
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("{home}/.local/bin/{BIN_NAME}"),
        format!("/usr/local/bin/{BIN_NAME}"),
        format!("/opt/homebrew/bin/{BIN_NAME}"),
    ];

    candidates
        .iter()
        .filter_map(|candidate| {
            let path = PathBuf::from(candidate);
            let meta = fs::symlink_metadata(&path).ok()?;
            if !meta.file_type().is_symlink() {
                return None;
            }
            let resolved = fs::canonicalize(&path).ok()?;
            if let Some(ref ct) = canonical_target {
                if &resolved == ct {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

pub fn run(force: bool, purge: bool) -> miette::Result<()> {
    let executable = current_executable_path()?;
    validate_install_location(&executable, "uninstall")?;

    let channel_root = channel_root();
    let channel_exists = channel_root.is_dir();
    let in_channel = executable.starts_with(&channel_root);

    // Find symlinks pointing to this binary (e.g. ~/.local/bin/loom -> channel binary)
    let symlinks = if in_channel {
        find_active_symlinks(&executable)
    } else {
        Vec::new()
    };

    let mut targets: Vec<String> = Vec::new();

    for link in &symlinks {
        targets.push(format!("symlink: {}", link.display()));
    }
    targets.push(format!("binary: {}", executable.display()));

    if purge && channel_exists {
        targets.push(format!("channel directory: {}", channel_root.display()));
    }

    println!("The following will be removed:");
    for t in &targets {
        println!("  {t}");
    }

    if channel_exists && !purge {
        println!(
            "\nNote: channel directory {} still exists. Use --purge to remove it.",
            channel_root.display()
        );
    }

    if !force {
        print!("\nContinue? [y/N] ");
        io::stdout().flush().into_diagnostic()?;
        let response = io::stdin()
            .lock()
            .lines()
            .next()
            .unwrap_or(Ok(String::new()))
            .into_diagnostic()?;
        if !matches!(response.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Remove symlinks first
    for link in &symlinks {
        fs::remove_file(link)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to remove {}", link.display()))?;
        println!("Removed {}", link.display());
    }

    // Remove the binary
    fs::remove_file(&executable)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to remove {}", executable.display()))?;
    println!("Removed {}", executable.display());

    // Purge channel directory
    if purge && channel_exists {
        fs::remove_dir_all(&channel_root)
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to remove {}", channel_root.display()))?;
        println!("Removed {}", channel_root.display());
    }

    println!("\nloom has been uninstalled.");
    Ok(())
}

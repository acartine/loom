use miette::IntoDiagnostic;
use std::env;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;

pub const BIN_NAME: &str = "loom";
pub const CHANNEL_DIR_NAME: &str = "acartine_loom";

pub fn current_executable_path() -> miette::Result<PathBuf> {
    env::current_exe().into_diagnostic()
}

pub fn validate_install_location(path: &Path, action: &str) -> miette::Result<()> {
    if path.file_name() != Some(OsStr::new(BIN_NAME)) {
        return Err(miette::miette!(
            "refusing to {action} from {}: the executable does not look like an installed `{BIN_NAME}` binary",
            path.display()
        ));
    }

    if looks_like_cargo_install(path) {
        return Err(miette::miette!(
            "refusing to {action} from {}: this looks like a cargo-installed binary; use `cargo uninstall loom` instead",
            path.display()
        ));
    }

    if !looks_like_installed_binary(path) {
        return Err(miette::miette!(
            "refusing to {action} from {}: install Loom into ~/.local/bin, /usr/local/bin, /usr/bin, or /opt/homebrew/bin first",
            path.display()
        ));
    }

    Ok(())
}

pub fn looks_like_installed_binary(path: &Path) -> bool {
    // Direct install: ~/.local/bin/loom
    if path
        .components()
        .any(|component| matches!(component, Component::Normal(part) if part == ".local"))
        && path.parent().and_then(Path::file_name) == Some(OsStr::new("bin"))
    {
        return true;
    }

    // Channel install: ~/.local/bin/acartine_loom/{release,local}/loom
    if path.starts_with(channel_root()) {
        return true;
    }

    const PREFIXES: &[&str] = &["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"];
    PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

fn looks_like_cargo_install(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Normal(part) if part == ".cargo"))
        && path.parent().and_then(Path::file_name) == Some(OsStr::new("bin"))
}

pub fn ensure_parent_writable(path: &Path) -> miette::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        miette::miette!(
            "cannot operate on {} because it has no parent directory",
            path.display()
        )
    })?;
    let probe = NamedTempFile::new_in(parent)
        .into_diagnostic()
        .map_err(|_| miette::miette!("cannot write to {}", parent.display()))?;
    drop(probe);
    Ok(())
}

pub fn channel_root() -> PathBuf {
    let base = env::var("LOOM_CHANNEL_ROOT").unwrap_or_else(|_| {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned());
        format!("{home}/.local/bin/{CHANNEL_DIR_NAME}")
    });
    PathBuf::from(base)
}

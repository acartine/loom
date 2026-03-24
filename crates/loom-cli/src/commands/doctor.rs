use super::update::{
    build_client, detect_release_target, normalize_version, release_base_url, release_urls,
    resolve_latest_tag, VERSION,
};
use clap::CommandFactory;
use clap_complete::Shell;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const ZSH_COMPLETION_MARKER: &str = "# >>> loom completions >>>";
const ZSH_COMPLETION_BLOCK: &str = r#"# >>> loom completions >>>
if [[ -d "$HOME/.zfunc" ]]; then
  fpath=("$HOME/.zfunc" $fpath)
fi
if ! whence -w compdef >/dev/null 2>&1; then
  autoload -Uz compinit
  compinit
fi
# <<< loom completions <<<
"#;

const BASH_COMPLETION_MARKER: &str = "# >>> loom completions >>>";
const BASH_COMPLETION_BLOCK: &str = r#"# >>> loom completions >>>
if [ -f "$HOME/.bash_completion.d/loom" ]; then
  . "$HOME/.bash_completion.d/loom"
fi
# <<< loom completions <<<
"#;

enum CheckStatus {
    Ok,
    Warn,
    Error,
}

struct CheckResult {
    name: String,
    status: CheckStatus,
    message: String,
    fix_message: Option<String>,
}

pub fn run(fix: bool) -> miette::Result<()> {
    let results = vec![check_version(), check_completions()];

    let issue_count = results
        .iter()
        .filter(|r| !matches!(r.status, CheckStatus::Ok))
        .count();

    print_results(&results);

    if fix && issue_count > 0 {
        println!();
        let mut fixed = 0;
        for result in &results {
            if matches!(result.status, CheckStatus::Ok) {
                continue;
            }
            match result.name.as_str() {
                "version" => {
                    println!("  Fixing version...");
                    match super::update::run(false, false) {
                        Ok(()) => {
                            println!("  \u{2713} version: fixed");
                            fixed += 1;
                        }
                        Err(e) => println!("  \u{2717} version: could not fix: {e}"),
                    }
                }
                "shell completions" => match fix_completions() {
                    Ok(msg) => {
                        println!("  {msg}");
                        println!("  \u{2713} shell completions: fixed");
                        fixed += 1;
                    }
                    Err(e) => println!("  \u{2717} shell completions: could not fix: {e}"),
                },
                _ => {}
            }
        }
        println!();
        if fixed == issue_count {
            println!("All issues fixed.");
        } else {
            println!("{} of {} issue(s) fixed.", fixed, issue_count);
        }
    } else if issue_count > 0 {
        println!();
        println!("{issue_count} issue(s) found. Run `loom doctor --fix` to fix.");
    } else {
        println!();
        println!("All checks passed.");
    }

    Ok(())
}

fn print_results(results: &[CheckResult]) {
    for result in results {
        let icon = match result.status {
            CheckStatus::Ok => "\u{2713}",
            CheckStatus::Warn => "!",
            CheckStatus::Error => "\u{2717}",
        };
        println!("  {icon} {}: {}", result.name, result.message);
        if let Some(fix_msg) = &result.fix_message {
            println!("    {fix_msg}");
        }
    }
}

fn check_version() -> CheckResult {
    let name = "version".to_string();

    let target = match detect_release_target() {
        Ok(t) => t,
        Err(e) => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("could not detect platform: {e}"),
                fix_message: None,
            }
        }
    };

    let client = match build_client() {
        Ok(c) => c,
        Err(e) => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("could not create HTTP client: {e}"),
                fix_message: None,
            }
        }
    };

    let urls = release_urls(&release_base_url(), &target, None);
    let latest_tag = match resolve_latest_tag(&client, &urls.archive_url) {
        Ok(t) => t,
        Err(e) => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("could not check for updates (offline?): {e}"),
                fix_message: None,
            }
        }
    };

    let latest_version = match normalize_version(&latest_tag) {
        Ok(v) => v,
        Err(e) => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("could not parse latest version '{latest_tag}': {e}"),
                fix_message: None,
            }
        }
    };

    let current_version = match normalize_version(VERSION) {
        Ok(v) => v,
        Err(e) => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("could not parse current version '{VERSION}': {e}"),
                fix_message: None,
            }
        }
    };

    if latest_version > current_version {
        CheckResult {
            name,
            status: CheckStatus::Error,
            message: format!("loom {VERSION} is out of date (latest: {latest_tag})"),
            fix_message: Some("Run `loom update` or `loom doctor --fix` to update.".to_string()),
        }
    } else {
        CheckResult {
            name,
            status: CheckStatus::Ok,
            message: format!("loom {VERSION} (up to date)"),
            fix_message: None,
        }
    }
}

fn detect_shell() -> Option<String> {
    env::var("SHELL").ok().and_then(|s| {
        Path::new(&s)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
    })
}

fn check_completions() -> CheckResult {
    let name = "shell completions".to_string();

    let shell_name = match detect_shell() {
        Some(s) => s,
        None => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: "could not detect shell ($SHELL not set)".to_string(),
                fix_message: None,
            }
        }
    };

    let installed = match shell_name.as_str() {
        "zsh" => check_zsh_completions(),
        "bash" => check_bash_completions(),
        "fish" => check_fish_completions(),
        _ => {
            return CheckResult {
                name,
                status: CheckStatus::Warn,
                message: format!("unsupported shell for completion check: {shell_name}"),
                fix_message: None,
            }
        }
    };

    if installed {
        CheckResult {
            name,
            status: CheckStatus::Ok,
            message: format!("installed ({shell_name})"),
            fix_message: None,
        }
    } else {
        CheckResult {
            name,
            status: CheckStatus::Error,
            message: format!("not installed ({shell_name})"),
            fix_message: Some(format!(
                "Run `loom doctor --fix` to install {shell_name} completions."
            )),
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var("HOME").ok().map(PathBuf::from)
}

fn read_file_if_exists(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(_) => None,
    }
}

fn zsh_rc_loads_loom(path: &Path) -> bool {
    read_file_if_exists(path)
        .map(|content| {
            content.contains(ZSH_COMPLETION_MARKER)
                || (content.contains(".zfunc")
                    && (content.contains("compinit") || content.contains("compdef")))
        })
        .unwrap_or(false)
}

fn bash_rc_loads_loom(path: &Path) -> bool {
    read_file_if_exists(path)
        .map(|content| {
            content.contains(BASH_COMPLETION_MARKER)
                || content.contains(".bash_completion.d/loom")
                || (content.contains(".bash_completion.d")
                    && (content.contains("source") || content.contains(". \"$HOME/")))
        })
        .unwrap_or(false)
}

fn check_zsh_completions() -> bool {
    // Check $fpath directories for _loom
    if let Ok(fpath) = env::var("FPATH") {
        for dir in fpath.split(':') {
            if Path::new(dir).join("_loom").exists() {
                return true;
            }
        }
    }

    // Check common locations
    if let Some(home) = home_dir() {
        let zshrc = home.join(".zshrc");
        let dot_zfunc = home.join(".zfunc/_loom");
        if dot_zfunc.exists() && zsh_rc_loads_loom(&zshrc) {
            return true;
        }

        let other_common_paths = [
            home.join(".zsh/completions/_loom"),
            home.join(".local/share/zsh/site-functions/_loom"),
        ];
        for path in &other_common_paths {
            if path.exists() {
                return true;
            }
        }
    }

    // Check system-wide
    let system_paths = [
        "/usr/local/share/zsh/site-functions/_loom",
        "/usr/share/zsh/site-functions/_loom",
        "/opt/homebrew/share/zsh/site-functions/_loom",
    ];
    for path in &system_paths {
        if Path::new(path).exists() {
            return true;
        }
    }

    false
}

fn check_bash_completions() -> bool {
    if let Some(home) = home_dir() {
        let bashrc = home.join(".bashrc");
        let bash_profile = home.join(".bash_profile");
        let bash_completion_d = home.join(".bash_completion.d/loom");
        if bash_completion_d.exists()
            && (bash_rc_loads_loom(&bashrc) || bash_rc_loads_loom(&bash_profile))
        {
            return true;
        }

        let other_common_paths = [home.join(".local/share/bash-completion/completions/loom")];
        for path in &other_common_paths {
            if path.exists() {
                return true;
            }
        }
    }

    let system_paths = [
        "/usr/share/bash-completion/completions/loom",
        "/usr/local/share/bash-completion/completions/loom",
        "/etc/bash_completion.d/loom",
        "/opt/homebrew/share/bash-completion/completions/loom",
    ];
    for path in &system_paths {
        if Path::new(path).exists() {
            return true;
        }
    }

    false
}

fn check_fish_completions() -> bool {
    if let Some(home) = home_dir() {
        if home.join(".config/fish/completions/loom.fish").exists() {
            return true;
        }
    }

    let system_paths = [
        "/usr/share/fish/vendor_completions.d/loom.fish",
        "/usr/local/share/fish/vendor_completions.d/loom.fish",
        "/opt/homebrew/share/fish/vendor_completions.d/loom.fish",
    ];
    for path in &system_paths {
        if Path::new(path).exists() {
            return true;
        }
    }

    false
}

fn fix_completions() -> miette::Result<String> {
    let shell_name =
        detect_shell().ok_or_else(|| miette::miette!("could not detect shell ($SHELL not set)"))?;

    let (shell, dest, rc_path, rc_block) = match shell_name.as_str() {
        "zsh" => {
            let home = home_dir().ok_or_else(|| miette::miette!("$HOME not set"))?;
            let dir = home.join(".zfunc");
            fs::create_dir_all(&dir)
                .map_err(|e| miette::miette!("failed to create {}: {e}", dir.display()))?;
            let dest = dir.join("_loom");
            (Shell::Zsh, dest, home.join(".zshrc"), ZSH_COMPLETION_BLOCK)
        }
        "bash" => {
            let home = home_dir().ok_or_else(|| miette::miette!("$HOME not set"))?;
            let dir = home.join(".bash_completion.d");
            fs::create_dir_all(&dir)
                .map_err(|e| miette::miette!("failed to create {}: {e}", dir.display()))?;
            let dest = dir.join("loom");
            (
                Shell::Bash,
                dest,
                home.join(".bashrc"),
                BASH_COMPLETION_BLOCK,
            )
        }
        "fish" => {
            let home = home_dir().ok_or_else(|| miette::miette!("$HOME not set"))?;
            let dir = home.join(".config/fish/completions");
            fs::create_dir_all(&dir)
                .map_err(|e| miette::miette!("failed to create {}: {e}", dir.display()))?;
            let dest = dir.join("loom.fish");
            (Shell::Fish, dest, home.join(".config/fish/config.fish"), "")
        }
        _ => {
            return Err(miette::miette!(
                "unsupported shell for completion install: {shell_name}"
            ))
        }
    };

    // Generate completions to the destination file
    let mut cmd = crate::Cli::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "loom", &mut buf);
    fs::write(&dest, &buf)
        .map_err(|e| miette::miette!("failed to write {}: {e}", dest.display()))?;

    if !rc_block.is_empty() {
        append_block_if_missing(&rc_path, rc_block, ZSH_COMPLETION_MARKER)?;
    }

    Ok(format!(
        "Installed {shell_name} completions to {}",
        dest.display(),
    ))
}

fn append_block_if_missing(path: &Path, block: &str, marker: &str) -> miette::Result<()> {
    let mut content = read_file_if_exists(path).unwrap_or_default();
    if content.contains(marker) {
        return Ok(());
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(block);

    fs::write(path, content).map_err(|e| miette::miette!("failed to write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn env_lock() -> MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn detect_shell_from_env() {
        let _guard = env_lock();
        // Temporarily set SHELL for the test
        let original = env::var("SHELL").ok();
        env::set_var("SHELL", "/bin/zsh");
        assert_eq!(detect_shell(), Some("zsh".to_string()));

        env::set_var("SHELL", "/usr/local/bin/fish");
        assert_eq!(detect_shell(), Some("fish".to_string()));

        env::set_var("SHELL", "/bin/bash");
        assert_eq!(detect_shell(), Some("bash".to_string()));

        match original {
            Some(v) => env::set_var("SHELL", v),
            None => env::remove_var("SHELL"),
        }
    }

    #[test]
    fn check_fish_completions_in_temp_home() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let fish_dir = tmpdir.path().join(".config/fish/completions");
        fs::create_dir_all(&fish_dir).unwrap();
        fs::write(fish_dir.join("loom.fish"), "# completions").unwrap();

        let original = env::var("HOME").ok();
        env::set_var("HOME", tmpdir.path().to_str().unwrap());

        assert!(check_fish_completions());

        match original {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
    }

    #[test]
    fn check_zsh_completions_in_temp_fpath() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let fpath_dir = tmpdir.path().join("zfuncs");
        fs::create_dir_all(&fpath_dir).unwrap();
        fs::write(fpath_dir.join("_loom"), "# completions").unwrap();

        let original = env::var("FPATH").ok();
        env::set_var("FPATH", fpath_dir.to_str().unwrap());

        assert!(check_zsh_completions());

        match original {
            Some(v) => env::set_var("FPATH", v),
            None => env::remove_var("FPATH"),
        }
    }

    #[test]
    fn check_bash_completions_in_temp_home() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let comp_dir = tmpdir.path().join(".bash_completion.d");
        fs::create_dir_all(&comp_dir).unwrap();
        fs::write(comp_dir.join("loom"), "# completions").unwrap();
        fs::write(
            tmpdir.path().join(".bashrc"),
            r#"if [ -f "$HOME/.bash_completion.d/loom" ]; then
  . "$HOME/.bash_completion.d/loom"
fi
"#,
        )
        .unwrap();

        let original = env::var("HOME").ok();
        env::set_var("HOME", tmpdir.path().to_str().unwrap());

        assert!(check_bash_completions());

        match original {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
    }

    #[test]
    fn check_zsh_completions_requires_shell_wiring_for_dot_zfunc() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let zfunc_dir = tmpdir.path().join(".zfunc");
        fs::create_dir_all(&zfunc_dir).unwrap();
        fs::write(zfunc_dir.join("_loom"), "# completions").unwrap();

        let original_home = env::var("HOME").ok();
        let original_fpath = env::var("FPATH").ok();
        env::set_var("HOME", tmpdir.path().to_str().unwrap());
        env::remove_var("FPATH");

        assert!(!check_zsh_completions());

        fs::write(tmpdir.path().join(".zshrc"), ZSH_COMPLETION_BLOCK).unwrap();
        assert!(check_zsh_completions());

        match original_home {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
        match original_fpath {
            Some(v) => env::set_var("FPATH", v),
            None => env::remove_var("FPATH"),
        }
    }

    #[test]
    fn fix_completions_updates_zsh_startup_file() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let original_home = env::var("HOME").ok();
        let original_shell = env::var("SHELL").ok();
        let original_fpath = env::var("FPATH").ok();

        env::set_var("HOME", tmpdir.path().to_str().unwrap());
        env::set_var("SHELL", "/bin/zsh");
        env::remove_var("FPATH");

        fix_completions().unwrap();

        assert!(tmpdir.path().join(".zfunc/_loom").exists());
        let zshrc = fs::read_to_string(tmpdir.path().join(".zshrc")).unwrap();
        assert!(zshrc.contains(ZSH_COMPLETION_MARKER));
        assert!(check_zsh_completions());

        match original_home {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
        match original_shell {
            Some(v) => env::set_var("SHELL", v),
            None => env::remove_var("SHELL"),
        }
        match original_fpath {
            Some(v) => env::set_var("FPATH", v),
            None => env::remove_var("FPATH"),
        }
    }

    #[test]
    fn fix_completions_updates_bash_startup_file() {
        let _guard = env_lock();
        let tmpdir = tempfile::tempdir().unwrap();
        let original_home = env::var("HOME").ok();
        let original_shell = env::var("SHELL").ok();

        env::set_var("HOME", tmpdir.path().to_str().unwrap());
        env::set_var("SHELL", "/bin/bash");

        fix_completions().unwrap();

        assert!(tmpdir.path().join(".bash_completion.d/loom").exists());
        let bashrc = fs::read_to_string(tmpdir.path().join(".bashrc")).unwrap();
        assert!(bashrc.contains(BASH_COMPLETION_MARKER));
        assert!(check_bash_completions());

        match original_home {
            Some(v) => env::set_var("HOME", v),
            None => env::remove_var("HOME"),
        }
        match original_shell {
            Some(v) => env::set_var("SHELL", v),
            None => env::remove_var("SHELL"),
        }
    }
}

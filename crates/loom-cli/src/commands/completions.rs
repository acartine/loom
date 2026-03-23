use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::Cli;

pub fn run(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "loom", &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_bash_completions() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Bash, &mut cmd, "loom", &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("loom"));
    }

    #[test]
    fn generate_zsh_completions() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Zsh, &mut cmd, "loom", &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("loom"));
    }

    #[test]
    fn generate_fish_completions() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Fish, &mut cmd, "loom", &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("loom"));
    }

    #[test]
    fn generate_powershell_completions() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::PowerShell, &mut cmd, "loom", &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("loom"));
    }

    #[test]
    fn generate_elvish_completions() {
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        generate(Shell::Elvish, &mut cmd, "loom", &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("loom"));
    }
}

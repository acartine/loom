mod common;

use common::TestInstall;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::process::{Command, Stdio};

#[test]
fn uninstall_force_removes_binary() {
    let install = TestInstall::new();
    assert!(install.executable.exists());

    let output = Command::new(&install.executable)
        .args(["uninstall", "--force"])
        .output()
        .expect("run loom uninstall --force");

    assert!(
        output.status.success(),
        "uninstall --force failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!install.executable.exists(), "binary should be removed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("loom has been uninstalled"),
        "stdout: {stdout}"
    );
}

#[test]
fn uninstall_force_removes_symlink_and_channel_binary() {
    let install = TestInstall::new();
    let tempdir_root = install._tempdir.path();
    let channel_root = tempdir_root.join(".local/bin/acartine_loom");
    let release_dir = channel_root.join("release");
    fs::create_dir_all(&release_dir).expect("create channel dir");

    let channel_binary = release_dir.join("loom");
    fs::copy(common::loom_bin(), &channel_binary).expect("copy channel binary");

    // Replace the direct binary with a symlink to the channel binary
    fs::remove_file(&install.executable).expect("remove direct binary");
    unix_fs::symlink(&channel_binary, &install.executable).expect("create symlink");

    // Run from the channel binary directly (since current_exe() resolves symlinks).
    // Set HOME so find_active_symlinks finds the symlink at ~/.local/bin/loom.
    let output = Command::new(&channel_binary)
        .args(["uninstall", "--force"])
        .env("LOOM_CHANNEL_ROOT", channel_root.to_str().unwrap())
        .env("HOME", tempdir_root.to_str().unwrap())
        .output()
        .expect("run loom uninstall --force");

    assert!(
        output.status.success(),
        "uninstall failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!install.executable.exists(), "symlink should be removed");
    assert!(!channel_binary.exists(), "channel binary should be removed");
}

#[test]
fn uninstall_purge_removes_channel_dir() {
    let install = TestInstall::new();
    let channel_root = install._tempdir.path().join(".local/bin/acartine_loom");
    let release_dir = channel_root.join("release");
    let local_dir = channel_root.join("local");
    fs::create_dir_all(&release_dir).expect("create release dir");
    fs::create_dir_all(&local_dir).expect("create local dir");
    fs::write(release_dir.join("loom"), b"release-binary").expect("write release binary");
    fs::write(local_dir.join("loom"), b"local-binary").expect("write local binary");

    let output = Command::new(&install.executable)
        .args(["uninstall", "--force", "--purge"])
        .env("LOOM_CHANNEL_ROOT", channel_root.to_str().unwrap())
        .output()
        .expect("run loom uninstall --force --purge");

    assert!(
        output.status.success(),
        "uninstall --purge failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!install.executable.exists(), "binary should be removed");
    assert!(!channel_root.exists(), "channel root should be removed");
}

#[test]
fn uninstall_rejects_non_installed_binary() {
    let output = Command::new(common::loom_bin())
        .args(["uninstall", "--force"])
        .output()
        .expect("run loom uninstall from target/debug");

    assert!(
        !output.status.success(),
        "command should fail for non-installed binary"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("refusing to uninstall"), "stderr: {stderr}");
}

#[test]
fn uninstall_without_force_aborts_on_empty_stdin() {
    let install = TestInstall::new();

    let output = Command::new(&install.executable)
        .arg("uninstall")
        .stdin(Stdio::null())
        .output()
        .expect("run loom uninstall without --force");

    assert!(
        output.status.success(),
        "command should succeed (abort is not an error)"
    );
    assert!(
        install.executable.exists(),
        "binary should still exist after abort"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Aborted"), "stdout: {stdout}");
}

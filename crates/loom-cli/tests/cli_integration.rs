use std::path::PathBuf;
use std::process::Command;

/// Returns the path to the `loom` binary built by cargo.
fn loom_bin() -> PathBuf {
    // env!("CARGO_BIN_EXE_loom") is only available inside unit tests of the
    // same crate. For integration tests we locate the binary via the build
    // directory that `cargo test` already populates.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Walk up to workspace root, then into target/debug
    path.pop(); // crates/
    path.pop(); // workspace root
    path.push("target");
    path.push("debug");
    path.push("loom");
    path
}

/// Returns the workspace root directory.
fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

#[test]
fn test_validate() {
    let output = Command::new(loom_bin())
        .arg("validate")
        .arg("tests/fixtures/knots_sdlc")
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "validate should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_build_rust() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--lang", "rust"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "build --lang rust should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pub enum State"),
        "expected 'pub enum State' in stdout, got: {}",
        stdout
    );
}

#[test]
fn test_build_toml() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--emit", "toml"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "build --emit toml should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[workflow]"),
        "expected '[workflow]' in stdout, got: {}",
        stdout
    );
}

#[test]
fn test_graph_mermaid() {
    let output = Command::new(loom_bin())
        .args([
            "graph",
            "tests/fixtures/knots_sdlc",
            "--format",
            "mermaid",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "graph --format mermaid should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("stateDiagram") || stdout.contains("graph"),
        "expected mermaid diagram in stdout, got: {}",
        stdout
    );
}

#[test]
fn test_graph_dot() {
    let output = Command::new(loom_bin())
        .args(["graph", "tests/fixtures/knots_sdlc", "--format", "dot"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "graph --format dot should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph"),
        "expected 'digraph' in stdout, got: {}",
        stdout
    );
}

#[test]
fn test_graph_with_profile() {
    let output = Command::new(loom_bin())
        .args([
            "graph",
            "tests/fixtures/knots_sdlc",
            "--profile",
            "autopilot",
            "--format",
            "mermaid",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "graph with --profile autopilot should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_init_and_validate() {
    let parent = std::env::temp_dir().join("loom_test_init_parent");
    let workflow_name = "test_workflow";
    let workflow_dir = parent.join(workflow_name);

    // Clean up from any prior run
    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(&parent).expect("failed to create temp parent dir");

    let init_output = Command::new(loom_bin())
        .args(["init", workflow_name])
        .current_dir(&parent)
        .output()
        .expect("failed to execute loom init");

    assert!(
        init_output.status.success(),
        "init should succeed, stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let validate_output = Command::new(loom_bin())
        .args(["validate", workflow_dir.to_str().unwrap()])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom validate");

    // Clean up before asserting so we don't leave temp files on failure
    let _ = std::fs::remove_dir_all(&parent);

    assert!(
        validate_output.status.success(),
        "validate on init'd dir should succeed, stderr: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn test_build_unsupported_lang() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--lang", "python"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        !output.status.success(),
        "build --lang python should fail"
    );
}

#[test]
fn test_validate_nonexistent() {
    let output = Command::new(loom_bin())
        .args(["validate", "/tmp/nonexistent_loom_dir"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        !output.status.success(),
        "validate on nonexistent dir should fail"
    );
}

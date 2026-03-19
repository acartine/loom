use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    path.push(format!("{}_{}_{}", prefix, std::process::id(), nanos));
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
fn test_build_knots_bundle() {
    let output = Command::new(loom_bin())
        .args([
            "build",
            "tests/fixtures/knots_sdlc",
            "--emit",
            "knots-bundle",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "build --emit knots-bundle should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).expect("knots bundle output should be json");
    assert_eq!(json["format"], "knots-bundle");
    assert_eq!(json["workflow"]["name"], "knots_sdlc");
    assert_eq!(json["workflow"]["default_profile"], "autopilot");
}

#[test]
fn test_graph_mermaid() {
    let output = Command::new(loom_bin())
        .args(["graph", "tests/fixtures/knots_sdlc", "--format", "mermaid"])
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
fn test_template_list() {
    let output = Command::new(loom_bin())
        .args(["template", "list"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom template list");

    assert!(
        output.status.success(),
        "template list should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("minimal"),
        "template list should include minimal"
    );
    assert!(
        stdout.contains("knots_sdlc"),
        "template list should include knots_sdlc"
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
    assert!(
        !String::from_utf8_lossy(&validate_output.stderr).contains("warning:"),
        "validate on init'd dir should be warning-free, stderr: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn test_init_knots_sdlc_template_and_validate() {
    let parent = unique_temp_dir("loom_test_init_knots_sdlc");
    let workflow_name = "payments-flow";
    let workflow_dir = parent.join(workflow_name);

    std::fs::create_dir_all(&parent).expect("failed to create temp parent dir");

    let init_output = Command::new(loom_bin())
        .args(["init", "--template", "knots_sdlc", workflow_name])
        .current_dir(&parent)
        .output()
        .expect("failed to execute loom init");

    assert!(
        init_output.status.success(),
        "init --template knots_sdlc should succeed, stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let config = std::fs::read_to_string(workflow_dir.join("loom.toml"))
        .expect("knots_sdlc template should write loom.toml");
    let workflow = std::fs::read_to_string(workflow_dir.join("workflow.loom"))
        .expect("knots_sdlc template should write workflow.loom");

    assert!(
        workflow_dir.join("profiles/autopilot.loom").exists(),
        "knots_sdlc template should write bundled profiles"
    );
    assert!(config.contains("name = \"payments_flow\""));
    assert!(workflow.contains("workflow payments_flow v1"));

    let validate_output = Command::new(loom_bin())
        .args(["validate", workflow_dir.to_str().unwrap()])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom validate");

    let _ = std::fs::remove_dir_all(&parent);

    assert!(
        validate_output.status.success(),
        "validate on init'd knots_sdlc dir should succeed, stderr: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn test_init_with_path_and_validate() {
    let parent = std::env::temp_dir().join("loom_test_init_path_parent");
    let workflow_dir = parent.join("nested").join("test-workflow");

    let _ = std::fs::remove_dir_all(&parent);
    std::fs::create_dir_all(parent.join("nested")).expect("failed to create temp parent dir");

    let init_output = Command::new(loom_bin())
        .args(["init", workflow_dir.to_str().unwrap()])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom init");

    assert!(
        init_output.status.success(),
        "init with path should succeed, stderr: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let validate_output = Command::new(loom_bin())
        .args(["validate", workflow_dir.to_str().unwrap()])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom validate");

    let _ = std::fs::remove_dir_all(&parent);

    assert!(
        validate_output.status.success(),
        "validate on path init'd dir should succeed, stderr: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&validate_output.stderr).contains("warning:"),
        "validate on path init'd dir should be warning-free, stderr: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn test_build_go() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--lang", "go"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "build --lang go should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("type State int"),
        "expected 'type State int' in Go output"
    );
}

#[test]
fn test_build_python() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--lang", "python"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "build --lang python should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("class State(Enum)"),
        "expected 'class State(Enum)' in Python output"
    );
}

#[test]
fn test_build_unsupported_lang() {
    let output = Command::new(loom_bin())
        .args(["build", "tests/fixtures/knots_sdlc", "--lang", "java"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(!output.status.success(), "build --lang java should fail");
}

#[test]
fn test_graph_ascii() {
    let output = Command::new(loom_bin())
        .args(["graph", "tests/fixtures/knots_sdlc", "--format", "ascii"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "graph --format ascii should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("States:") && stdout.contains("Transitions:"),
        "expected ASCII graph output"
    );
}

#[test]
fn test_diff() {
    let output = Command::new(loom_bin())
        .args([
            "diff",
            "tests/fixtures/knots_sdlc",
            "tests/fixtures/knots_sdlc_v2",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    assert!(
        output.status.success(),
        "diff should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("+") || stdout.contains("-") || stdout.contains("~"),
        "expected diff markers in output"
    );
}

#[test]
fn test_check_compat() {
    let output = Command::new(loom_bin())
        .args([
            "check-compat",
            "tests/fixtures/knots_sdlc",
            "tests/fixtures/knots_sdlc_v2",
        ])
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute loom");

    // Should exit non-zero because there are breaking changes
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("breaking") || stdout.contains("Breaking"),
        "expected breaking changes in output"
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

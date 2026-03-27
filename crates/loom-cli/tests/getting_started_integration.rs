use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // workspace root
    path
}

/// Run smoke-install.sh to build and install loom locally into `install_dir`.
fn smoke_install(install_dir: &std::path::Path) -> PathBuf {
    let root = workspace_root();
    let script = root.join("scripts/release/smoke-install.sh");

    let output = Command::new("bash")
        .arg(&script)
        .env("LOOM_SMOKE_INSTALL_DIR", install_dir)
        .current_dir(&root)
        .output()
        .expect("failed to run smoke-install.sh");

    assert!(
        output.status.success(),
        "smoke-install.sh failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let bin = install_dir.join("loom");
    assert!(bin.exists(), "installed binary should exist at {:?}", bin);
    bin
}

fn run_loom(loom: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(loom)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute loom {:?}: {}", args, e))
}

fn run_loom_in(
    loom: &std::path::Path,
    args: &[&str],
    dir: &std::path::Path,
) -> std::process::Output {
    Command::new(loom)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute loom {:?}: {}", args, e))
}

fn assert_success(output: &std::process::Output, context: &str) {
    assert!(
        output.status.success(),
        "{} failed (exit {:?}):\nstdout: {}\nstderr: {}",
        context,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore] // slow: runs cargo build --release via smoke-install.sh
fn getting_started_walkthrough() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let install_dir = tmp.path().join("install");
    std::fs::create_dir_all(&install_dir).expect("create install dir");

    // ── Step 1: Local install ──────────────────────────────────
    println!("Step 1: Local install via smoke-install.sh");
    let loom = smoke_install(&install_dir);

    let version_out = run_loom(&loom, &["--version"]);
    assert_success(&version_out, "loom --version");
    let version_str = stdout_of(&version_out);
    assert!(
        version_str.starts_with("loom "),
        "unexpected version output: {}",
        version_str
    );
    println!("  Installed: {}", version_str.trim());

    // ── Step 2: loom templates list ────────────────────────────
    println!("Step 2: loom templates list");
    let tpl_out = run_loom(&loom, &["templates", "list"]);
    assert_success(&tpl_out, "loom templates list");
    let tpl_stdout = stdout_of(&tpl_out);
    assert!(
        tpl_stdout.contains("minimal"),
        "templates list should include minimal, got: {}",
        tpl_stdout
    );
    assert!(
        tpl_stdout.contains("knots_sdlc"),
        "templates list should include knots_sdlc, got: {}",
        tpl_stdout
    );
    println!("  Found minimal and knots_sdlc templates");

    // ── Step 3: loom init knots_sdlc ───────────────────────────
    println!("Step 3: loom init knots_sdlc");
    let projects_dir = tmp.path().join("projects");
    std::fs::create_dir_all(&projects_dir).expect("create projects dir");

    let init_out = run_loom_in(&loom, &["init", "knots_sdlc"], &projects_dir);
    assert_success(&init_out, "loom init knots_sdlc");

    let knots_dir = projects_dir.join("knots_sdlc");
    println!("  Scaffolded at {:?}", knots_dir);

    // ── Step 4: Read the scaffold ──────────────────────────────
    println!("Step 4: Verify scaffold structure");
    assert!(
        knots_dir.join("workflow.loom").exists(),
        "missing workflow.loom"
    );
    assert!(knots_dir.join("loom.toml").exists(), "missing loom.toml");
    assert!(knots_dir.join("prompts").is_dir(), "missing prompts/");
    assert!(knots_dir.join("profiles").is_dir(), "missing profiles/");
    assert!(
        knots_dir.join("prompts/planning.md").exists(),
        "missing prompts/planning.md"
    );
    assert!(
        knots_dir.join("profiles/autopilot.loom").exists(),
        "missing profiles/autopilot.loom"
    );
    println!("  All expected files present");

    // ── Step 5: loom validate ──────────────────────────────────
    println!("Step 5: loom validate");
    let val_out = run_loom(&loom, &["validate", knots_dir.to_str().unwrap()]);
    assert_success(&val_out, "loom validate knots_sdlc");
    assert!(
        !String::from_utf8_lossy(&val_out.stderr).contains("warning:"),
        "validate should be warning-free, stderr: {}",
        String::from_utf8_lossy(&val_out.stderr)
    );
    println!("  Validation passed");

    // ── Step 6: loom build + compile + test per language ────────
    println!("Step 6: loom build (compile & test generated code)");
    let knots_path = knots_dir.to_str().unwrap();
    let codegen_dir = tmp.path().join("codegen");
    std::fs::create_dir_all(&codegen_dir).expect("create codegen dir");

    // 6a: Rust — generate, compile, and run assertions
    {
        println!("  6a: Rust");
        let build_out = run_loom(&loom, &["build", knots_path, "--lang", "rust"]);
        assert_success(&build_out, "loom build --lang rust");
        let generated = stdout_of(&build_out);

        let rust_dir = codegen_dir.join("rust_test");
        std::fs::create_dir_all(rust_dir.join("src")).expect("create rust src dir");

        std::fs::write(rust_dir.join("src/lib.rs"), &generated).expect("write lib.rs");
        std::fs::write(
            rust_dir.join("Cargo.toml"),
            r#"[package]
name = "loom-codegen-test"
version = "0.0.0"
edition = "2021"

[[bin]]
name = "test_codegen"
path = "src/main.rs"
"#,
        )
        .expect("write Cargo.toml");
        std::fs::write(
            rust_dir.join("src/main.rs"),
            r#"#[allow(dead_code, special_module_name)]
mod lib;
use lib::*;

fn main() {
    // ── State enum completeness ──
    let all_states = [
        State::Planning, State::PlanReview,
        State::Implementation, State::ImplementationReview,
        State::Shipment, State::ShipmentReview,
        State::Shipped, State::Abandoned, State::Blocked, State::Deferred,
        State::ReadyForPlanning, State::ReadyForPlanReview,
        State::ReadyForImplementation, State::ReadyForImplementationReview,
        State::ReadyForShipment, State::ReadyForShipmentReview,
    ];
    assert_eq!(all_states.len(), 16, "expected 16 states");
    for s in &all_states {
        assert!(!s.display_name().is_empty(), "{:?} display_name empty", s);
    }

    // ── Terminal / non-terminal ──
    assert!(State::Shipped.is_terminal());
    assert!(State::Abandoned.is_terminal());
    assert!(!State::Deferred.is_terminal(), "escape state is not terminal");
    assert!(!State::Planning.is_terminal());

    // ── Outcome target() and is_success() ──
    assert!(PlanningOutcome::PlanComplete.is_success());
    assert!(!PlanningOutcome::InsufficientContext.is_success());
    assert_eq!(PlanningOutcome::PlanComplete.target(), State::ReadyForPlanReview);
    assert_eq!(PlanningOutcome::BlockedByDependency.target(), State::Blocked);

    assert!(ShipmentReviewOutcome::Approved.is_success());
    assert!(!ShipmentReviewOutcome::NeedsRevision.is_success());
    assert_eq!(ShipmentReviewOutcome::Approved.target(), State::Shipped);

    // ── apply(): walk the full happy path through all 3 phases ──
    // Planning phase
    let s = apply(State::Planning, Outcome::Planning(PlanningOutcome::PlanComplete)).unwrap();
    assert_eq!(s, State::ReadyForPlanReview);
    let s = apply(State::PlanReview, Outcome::PlanReview(PlanReviewOutcome::Approved)).unwrap();
    assert_eq!(s, State::ReadyForImplementation);

    // Implementation phase
    let s = apply(State::Implementation, Outcome::Implementation(ImplementationOutcome::ImplementationComplete)).unwrap();
    assert_eq!(s, State::ReadyForImplementationReview);
    let s = apply(State::ImplementationReview, Outcome::ImplementationReview(ImplementationReviewOutcome::Approved)).unwrap();
    assert_eq!(s, State::ReadyForShipment);

    // Shipment phase
    let s = apply(State::Shipment, Outcome::Shipment(ShipmentOutcome::ShipmentComplete)).unwrap();
    assert_eq!(s, State::ReadyForShipmentReview);
    let s = apply(State::ShipmentReview, Outcome::ShipmentReview(ShipmentReviewOutcome::Approved)).unwrap();
    assert_eq!(s, State::Shipped);

    // ── apply(): failure / retry paths ──
    let s = apply(State::Planning, Outcome::Planning(PlanningOutcome::InsufficientContext)).unwrap();
    assert_eq!(s, State::ReadyForPlanning, "insufficient context loops back");

    let s = apply(State::ImplementationReview, Outcome::ImplementationReview(ImplementationReviewOutcome::ChangesRequested)).unwrap();
    assert_eq!(s, State::ReadyForImplementation, "changes requested loops back");

    // ── apply(): escape to blocked ──
    let s = apply(State::Planning, Outcome::Planning(PlanningOutcome::BlockedByDependency)).unwrap();
    assert_eq!(s, State::Blocked);

    // ── apply(): mismatched outcome type is error ──
    assert!(apply(State::Planning, Outcome::Shipment(ShipmentOutcome::ShipmentComplete)).is_err());
    assert!(apply(State::Shipped, Outcome::Planning(PlanningOutcome::PlanComplete)).is_err());

    // ── Profiles ──
    assert_eq!(AUTOPILOT.id, "autopilot");
    assert!(!AUTOPILOT.description.is_empty());
    assert_eq!(AUTOPILOT.phases.len(), 3);
    assert_eq!(AUTOPILOT_WITH_PR.id, "autopilot_with_pr");
    assert_eq!(SEMIAUTO.id, "semiauto");

    // ── Prompt metadata ──
    assert_eq!(PROMPT_PLANNING.name, "planning");
    assert!(!PROMPT_PLANNING.accept.is_empty());
    assert!(!PROMPT_PLANNING.outcomes.is_empty());
    assert!(!PROMPT_PLANNING.body.is_empty());
    // Check a success outcome targets the right state
    let success = PROMPT_PLANNING.outcomes.iter().find(|o| o.is_success).unwrap();
    assert_eq!(success.target, State::ReadyForPlanReview);

    println!("rust codegen assertions passed");
}
"#,
        )
        .expect("write main.rs");

        let cargo_out = Command::new("cargo")
            .args(["run", "--quiet"])
            .current_dir(&rust_dir)
            .output()
            .expect("cargo run codegen test");
        assert_success(&cargo_out, "rust codegen compile+run");
        println!("    compile + test OK");
    }

    // 6b: Go — generate, compile, and run test
    {
        println!("  6b: Go");
        let build_out = run_loom(&loom, &["build", knots_path, "--lang", "go"]);
        assert_success(&build_out, "loom build --lang go");
        let generated = stdout_of(&build_out);

        let go_dir = codegen_dir.join("go_test");
        std::fs::create_dir_all(&go_dir).expect("create go dir");

        std::fs::write(go_dir.join("workflow.go"), &generated).expect("write workflow.go");

        let go_mod_out = Command::new("go")
            .args(["mod", "init", "loom_codegen_test"])
            .current_dir(&go_dir)
            .output()
            .expect("go mod init");
        assert_success(&go_mod_out, "go mod init");

        {
            std::fs::write(
                go_dir.join("workflow_test.go"),
                r#"package workflow

import "testing"

func TestAllStatesHaveDisplayNames(t *testing.T) {
	states := []State{
		Planning, PlanReview, Implementation, ImplementationReview,
		Shipment, ShipmentReview, Shipped, Abandoned, Blocked, Deferred,
		ReadyForPlanning, ReadyForPlanReview, ReadyForImplementation,
		ReadyForImplementationReview, ReadyForShipment, ReadyForShipmentReview,
	}
	if len(states) != 16 {
		t.Fatalf("expected 16 states, got %d", len(states))
	}
	for _, s := range states {
		if s.DisplayName() == "" {
			t.Fatalf("state %d has empty DisplayName", s)
		}
	}
}

func TestTerminalStates(t *testing.T) {
	if !Shipped.IsTerminal() {
		t.Fatal("Shipped should be terminal")
	}
	if !Abandoned.IsTerminal() {
		t.Fatal("Abandoned should be terminal")
	}
	if Deferred.IsTerminal() {
		t.Fatal("Deferred (escape) should not be terminal")
	}
	if Planning.IsTerminal() {
		t.Fatal("Planning should not be terminal")
	}
}

func TestFullHappyPath(t *testing.T) {
	// Planning phase
	s, err := Apply(Planning, NewPlanningOutcome(PlanningPlanComplete))
	if err != nil { t.Fatal(err) }
	if s != ReadyForPlanReview { t.Fatalf("got %v", s) }

	s, err = Apply(PlanReview, NewPlanReviewOutcome(PlanReviewApproved))
	if err != nil { t.Fatal(err) }
	if s != ReadyForImplementation { t.Fatalf("got %v", s) }

	// Implementation phase
	s, err = Apply(Implementation, NewImplementationOutcome(ImplementationImplementationComplete))
	if err != nil { t.Fatal(err) }
	if s != ReadyForImplementationReview { t.Fatalf("got %v", s) }

	s, err = Apply(ImplementationReview, NewImplementationReviewOutcome(ImplementationReviewApproved))
	if err != nil { t.Fatal(err) }
	if s != ReadyForShipment { t.Fatalf("got %v", s) }

	// Shipment phase
	s, err = Apply(Shipment, NewShipmentOutcome(ShipmentShipmentComplete))
	if err != nil { t.Fatal(err) }
	if s != ReadyForShipmentReview { t.Fatalf("got %v", s) }

	s, err = Apply(ShipmentReview, NewShipmentReviewOutcome(ShipmentReviewApproved))
	if err != nil { t.Fatal(err) }
	if s != Shipped { t.Fatalf("got %v", s) }
}

func TestApplyMismatchedOutcome(t *testing.T) {
	_, err := Apply(Planning, NewShipmentOutcome(ShipmentShipmentComplete))
	if err == nil {
		t.Fatal("mismatched outcome type should return error")
	}
}

func TestApplyTerminalState(t *testing.T) {
	_, err := Apply(Shipped, NewPlanningOutcome(PlanningPlanComplete))
	if err == nil {
		t.Fatal("Apply on terminal state should return error")
	}
}

func TestProfiles(t *testing.T) {
	if ProfileAutopilot.ID != "autopilot" {
		t.Fatalf("expected autopilot, got %s", ProfileAutopilot.ID)
	}
	if ProfileAutopilotWithPr.ID != "autopilot_with_pr" {
		t.Fatalf("got %s", ProfileAutopilotWithPr.ID)
	}
	if len(ProfileAutopilot.Phases) != 3 {
		t.Fatalf("expected 3 phases, got %d", len(ProfileAutopilot.Phases))
	}
}
"#,
            )
            .expect("write workflow_test.go");

            let go_test_out = Command::new("go")
                .args(["test", "-v", "./..."])
                .current_dir(&go_dir)
                .output()
                .expect("go test");
            assert_success(&go_test_out, "go codegen test");
            println!("    compile + test OK");
        }
    }

    // 6c: Python — generate, mypy type-check, and run test
    {
        println!("  6c: Python");
        let build_out = run_loom(&loom, &["build", knots_path, "--lang", "python"]);
        assert_success(&build_out, "loom build --lang python");
        let generated = stdout_of(&build_out);

        let py_dir = codegen_dir.join("py_test");
        std::fs::create_dir_all(&py_dir).expect("create py dir");

        std::fs::write(py_dir.join("workflow.py"), &generated).expect("write workflow.py");

        // mypy type check (non-strict) via uvx (ephemeral, no global install)
        if which_exists("uvx") {
            let mypy_out = Command::new("uvx")
                .args(["mypy", "workflow.py"])
                .current_dir(&py_dir)
                .output()
                .expect("uvx mypy workflow.py");
            assert_success(&mypy_out, "mypy type check");
            println!("    mypy (via uvx) OK");
        } else {
            println!("    uvx not found, skipping mypy type check");
        }

        // Runtime test
        std::fs::write(
            py_dir.join("test_workflow.py"),
            r#"from workflow import (
    State, Executor, apply,
    PlanningOutcome, PlanReviewOutcome,
    ImplementationOutcome, ImplementationReviewOutcome,
    ShipmentOutcome, ShipmentReviewOutcome,
    AUTOPILOT, AUTOPILOT_WITH_PR, SEMIAUTO,
    PROMPT_PLANNING,
)

# ── State enum completeness ──
all_states = list(State)
assert len(all_states) == 16, f"expected 16 states, got {len(all_states)}"
for s in all_states:
    assert s.display_name, f"{s} has empty display_name"

# ── Terminal / non-terminal ──
assert State.SHIPPED.is_terminal is True
assert State.ABANDONED.is_terminal is True
assert State.DEFERRED.is_terminal is False, "escape state is not terminal"
assert State.PLANNING.is_terminal is False

# ── Executor enum ──
assert Executor.AGENT.value == 0
assert Executor.HUMAN.value == 1

# ── Outcome target() ──
assert PlanningOutcome.PLAN_COMPLETE.target() == State.READY_FOR_PLAN_REVIEW
assert PlanningOutcome.BLOCKED_BY_DEPENDENCY.target() == State.BLOCKED
assert ShipmentReviewOutcome.APPROVED.target() == State.SHIPPED

# ── Outcome is_success() ──
assert PlanningOutcome.PLAN_COMPLETE.is_success() is True
assert PlanningOutcome.INSUFFICIENT_CONTEXT.is_success() is False
assert ShipmentReviewOutcome.APPROVED.is_success() is True
assert ShipmentReviewOutcome.APPROVED_ALREADY_MERGED.is_success() is True
assert ShipmentReviewOutcome.NEEDS_REVISION.is_success() is False

# ── apply(): full happy path through all 3 phases ──
s = apply(State.PLANNING, PlanningOutcome.PLAN_COMPLETE)
assert s == State.READY_FOR_PLAN_REVIEW

s = apply(State.PLAN_REVIEW, PlanReviewOutcome.APPROVED)
assert s == State.READY_FOR_IMPLEMENTATION

s = apply(State.IMPLEMENTATION, ImplementationOutcome.IMPLEMENTATION_COMPLETE)
assert s == State.READY_FOR_IMPLEMENTATION_REVIEW

s = apply(State.IMPLEMENTATION_REVIEW, ImplementationReviewOutcome.APPROVED)
assert s == State.READY_FOR_SHIPMENT

s = apply(State.SHIPMENT, ShipmentOutcome.SHIPMENT_COMPLETE)
assert s == State.READY_FOR_SHIPMENT_REVIEW

s = apply(State.SHIPMENT_REVIEW, ShipmentReviewOutcome.APPROVED)
assert s == State.SHIPPED

# ── apply(): failure / retry paths ──
s = apply(State.PLANNING, PlanningOutcome.INSUFFICIENT_CONTEXT)
assert s == State.READY_FOR_PLANNING, "insufficient context loops back"

s = apply(State.IMPLEMENTATION_REVIEW, ImplementationReviewOutcome.CHANGES_REQUESTED)
assert s == State.READY_FOR_IMPLEMENTATION, "changes requested loops back"

# ── apply(): escape to blocked ──
s = apply(State.PLANNING, PlanningOutcome.BLOCKED_BY_DEPENDENCY)
assert s == State.BLOCKED

# ── apply(): mismatched outcome raises ValueError ──
try:
    apply(State.PLANNING, ShipmentOutcome.SHIPMENT_COMPLETE)
    assert False, "mismatched outcome should raise ValueError"
except ValueError:
    pass

try:
    apply(State.SHIPPED, PlanningOutcome.PLAN_COMPLETE)
    assert False, "apply on terminal state should raise ValueError"
except ValueError:
    pass

# ── Profiles ──
assert AUTOPILOT.id == "autopilot"
assert AUTOPILOT.description != ""
assert len(AUTOPILOT.phases) == 3
assert AUTOPILOT_WITH_PR.id == "autopilot_with_pr"
assert SEMIAUTO.id == "semiauto"

# ── Prompt metadata ──
assert PROMPT_PLANNING.name == "planning"
assert len(PROMPT_PLANNING.accept) > 0
assert len(PROMPT_PLANNING.outcomes) > 0
assert PROMPT_PLANNING.body != ""
success = [o for o in PROMPT_PLANNING.outcomes if o.is_success]
assert len(success) > 0, "planning prompt should have success outcomes"
assert success[0].target == State.READY_FOR_PLAN_REVIEW

print("python codegen assertions passed")
"#,
        )
        .expect("write test_workflow.py");

        let py_out = Command::new("python3")
            .arg("test_workflow.py")
            .current_dir(&py_dir)
            .output()
            .expect("python3 test_workflow.py");
        assert_success(&py_out, "python codegen test");
        println!("    runtime test OK");
    }

    // 6d: TOML — generate and parse in-process
    {
        println!("  6d: TOML");
        let build_out = run_loom(&loom, &["build", knots_path, "--emit", "toml"]);
        assert_success(&build_out, "loom build --emit toml");
        let generated = stdout_of(&build_out);

        let parsed: toml::Value = generated.parse().expect("generated TOML should parse");
        let table = parsed.as_table().expect("TOML root should be a table");

        // All top-level sections present
        for section in [
            "workflow", "states", "steps", "phases", "profiles", "prompts",
        ] {
            assert!(
                table.contains_key(section),
                "missing top-level section: {}",
                section
            );
        }

        // Workflow metadata
        let workflow = table["workflow"].as_table().expect("workflow section");
        assert_eq!(workflow["name"].as_str().unwrap(), "knots_sdlc");
        assert_eq!(workflow["version"].as_integer().unwrap(), 1);
        assert_eq!(workflow["default_profile"].as_str().unwrap(), "autopilot");

        // States: check count, kinds, and key action states
        let states = table["states"].as_table().expect("states section");
        assert_eq!(states.len(), 16, "expected 16 states, got {}", states.len());

        let planning = states["planning"].as_table().expect("states.planning");
        assert_eq!(planning["kind"].as_str().unwrap(), "action");
        assert_eq!(planning["action_type"].as_str().unwrap(), "produce");
        assert_eq!(planning["executor"].as_str().unwrap(), "agent");

        let plan_review = states["plan_review"]
            .as_table()
            .expect("states.plan_review");
        assert_eq!(plan_review["action_type"].as_str().unwrap(), "gate");
        assert_eq!(plan_review["gate_kind"].as_str().unwrap(), "review");
        let constraints = plan_review["constraints"]
            .as_array()
            .expect("constraints array");
        assert!(constraints.iter().any(|c| c.as_str() == Some("read_only")));

        assert_eq!(
            states["shipped"].as_table().unwrap()["kind"]
                .as_str()
                .unwrap(),
            "terminal"
        );
        assert_eq!(
            states["deferred"].as_table().unwrap()["kind"]
                .as_str()
                .unwrap(),
            "escape"
        );
        assert_eq!(
            states["ready_for_planning"].as_table().unwrap()["kind"]
                .as_str()
                .unwrap(),
            "queue"
        );

        // Steps
        let steps = table["steps"].as_table().expect("steps section");
        assert_eq!(steps.len(), 6, "expected 6 steps");
        let plan_step = steps["planning"].as_table().expect("steps.planning");
        assert_eq!(plan_step["action"].as_str().unwrap(), "planning");

        // Phases
        let phases = table["phases"].as_table().expect("phases section");
        assert_eq!(phases.len(), 3, "expected 3 phases");
        for phase_name in ["planning_phase", "implementation_phase", "shipment_phase"] {
            let phase = phases[phase_name]
                .as_table()
                .unwrap_or_else(|| panic!("missing phase {}", phase_name));
            assert!(
                phase.contains_key("produce"),
                "{} missing produce",
                phase_name
            );
            assert!(phase.contains_key("gate"), "{} missing gate", phase_name);
        }

        // Profiles
        let profiles = table["profiles"].as_table().expect("profiles section");
        for profile_name in ["autopilot", "autopilot_with_pr", "semiauto"] {
            assert!(
                profiles.contains_key(profile_name),
                "missing profile: {}",
                profile_name
            );
        }
        let autopilot = profiles["autopilot"]
            .as_table()
            .expect("profiles.autopilot");
        assert!(!autopilot["description"].as_str().unwrap().is_empty());
        let ap_phases = autopilot["phases"].as_array().expect("autopilot phases");
        assert_eq!(ap_phases.len(), 3);

        // Prompts
        let prompts = table["prompts"].as_table().expect("prompts section");
        for prompt_name in [
            "planning",
            "plan_review",
            "implementation",
            "implementation_review",
            "shipment",
            "shipment_review",
        ] {
            let prompt = prompts[prompt_name]
                .as_table()
                .unwrap_or_else(|| panic!("missing prompt {}", prompt_name));
            assert!(
                prompt.contains_key("accept"),
                "{} missing accept",
                prompt_name
            );
            assert!(
                prompt.contains_key("success"),
                "{} missing success outcomes",
                prompt_name
            );
            assert!(
                prompt.contains_key("failure"),
                "{} missing failure outcomes",
                prompt_name
            );
        }
        println!("    parse + validate OK");
    }

    // ── Step 7: loom graph (multiple formats) ──────────────────
    println!("Step 7: loom graph");
    for (format, expected) in [
        ("mermaid", vec!["stateDiagram", "graph"]),
        ("ascii", vec!["States:"]),
        ("dot", vec!["digraph"]),
    ] {
        let graph_out = run_loom(&loom, &["graph", knots_path, "--format", format]);
        assert_success(&graph_out, &format!("loom graph --format {}", format));
        let stdout = stdout_of(&graph_out);
        assert!(
            expected.iter().any(|e| stdout.contains(e)),
            "graph --format {}: expected one of {:?} in output, got: {}",
            format,
            expected,
            &stdout[..stdout.len().min(200)]
        );
        println!("  graph --format {} OK", format);
    }

    // ── Step 8: loom sim (interactive, piped stdin) ────────────
    println!("Step 8: loom sim");
    let mut sim_child = Command::new(&loom)
        .args(["sim", knots_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn loom sim");

    {
        let stdin = sim_child.stdin.as_mut().expect("open sim stdin");
        stdin.write_all(b"1\nq\n").expect("write to sim stdin");
    }

    let sim_out = sim_child.wait_with_output().expect("wait for loom sim");
    assert_success(&sim_out, "loom sim");
    let sim_stdout = stdout_of(&sim_out);
    assert!(
        sim_stdout.contains("Current state:"),
        "sim should print current state, got: {}",
        sim_stdout
    );
    assert!(
        sim_stdout.contains("Quit."),
        "sim should acknowledge quit, got: {}",
        sim_stdout
    );
    println!("  Simulation OK");

    // ── Step 9: loom init my_workflow (minimal template) ───────
    println!("Step 9: loom init my_workflow (minimal)");
    let init2_out = run_loom_in(&loom, &["init", "my_workflow"], &projects_dir);
    assert_success(&init2_out, "loom init my_workflow");

    let my_dir = projects_dir.join("my_workflow");
    assert!(
        my_dir.join("workflow.loom").exists(),
        "missing workflow.loom"
    );
    assert!(my_dir.join("loom.toml").exists(), "missing loom.toml");

    let val2_out = run_loom(&loom, &["validate", my_dir.to_str().unwrap()]);
    assert_success(&val2_out, "loom validate my_workflow");
    assert!(
        !String::from_utf8_lossy(&val2_out.stderr).contains("warning:"),
        "validate my_workflow should be warning-free"
    );
    println!("  Minimal scaffold OK");

    println!("\nAll getting started steps passed!");
}

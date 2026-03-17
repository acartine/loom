use std::io::{self, BufRead, Write};
use std::path::Path;

use loom_core::graph::profile;
use loom_core::sim;

pub fn run(dir: &Path, profile_name: Option<&str>) -> miette::Result<()> {
    let (ir, _diag) = loom_core::load_workflow(dir)
        .map_err(|errors| {
            let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            miette::miette!("failed to load workflow:\n{}", msgs.join("\n"))
        })?;

    let working_ir = if let Some(pname) = profile_name {
        profile::extract_profile_subgraph(&ir, pname)
            .ok_or_else(|| miette::miette!("profile '{}' not found", pname))?
    } else {
        ir
    };

    let mut state = sim::new(&working_ir, profile_name)
        .map_err(|e| miette::miette!("{}", e))?;

    let label = profile_name.unwrap_or(&working_ir.name);
    println!("[{}] Starting at: {}\n", label, state.current);

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    loop {
        print_current_state(&state, &working_ir);

        if sim::is_terminal(&state, &working_ir) {
            println!("Reached terminal state. Simulation complete.");
            break;
        }

        let transitions = sim::available_transitions(&state, &working_ir);
        if transitions.is_empty() {
            println!("No transitions available. Simulation complete.");
            break;
        }

        print_transitions(&transitions);

        print!("> ");
        io::stdout().flush().ok();

        let mut input = String::new();
        if reader.read_line(&mut input).is_err() || input.is_empty() {
            break;
        }

        let input = input.trim();
        if input == "q" || input == "quit" {
            println!("Quit.");
            break;
        }

        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= transitions.len() => {
                let transition = &transitions[n - 1];
                sim::apply(&mut state, transition);
                println!();
            }
            _ => {
                println!(
                    "Invalid input. Enter a number 1-{} or 'q' to quit.\n",
                    transitions.len()
                );
            }
        }
    }

    Ok(())
}

fn print_current_state(state: &sim::SimState, ir: &loom_core::ir::WorkflowIR) {
    let display = ir.states.get(&state.current)
        .map(|s| s.display_name())
        .unwrap_or("???");
    println!("Current state: {} ({})", state.current, display);
}

fn print_transitions(transitions: &[sim::SimTransition]) {
    for (i, t) in transitions.iter().enumerate() {
        println!("  {}. {}", i + 1, t.label);
    }
}

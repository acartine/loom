pub mod ast;

use pest::Parser;
use pest_derive::Parser;

use ast::*;
use crate::error::{LoomError, LoomResult};

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct LoomParser;

/// Parse a workflow .loom file into an AST
pub fn parse_workflow(input: &str) -> LoomResult<Workflow> {
    let pairs = LoomParser::parse(Rule::file, input)
        .map_err(|e| LoomError::Parse { message: e.to_string() })?;

    let file_pair = pairs.into_iter().next().unwrap();
    let workflow_pair = file_pair.into_inner().next().unwrap();
    build_workflow(workflow_pair)
}

/// Parse a profile .loom file into a ProfileDecl
pub fn parse_profile_file(input: &str) -> LoomResult<ProfileDecl> {
    let pairs = LoomParser::parse(Rule::profile_file, input)
        .map_err(|e| LoomError::Parse { message: e.to_string() })?;

    let file_pair = pairs.into_iter().next().unwrap();
    let profile_pair = file_pair.into_inner().next().unwrap();
    build_profile(profile_pair)
}

fn build_workflow(pair: pest::iterators::Pair<Rule>) -> LoomResult<Workflow> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let version_pair = inner.next().unwrap();
    let version: u32 = version_pair
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .parse()
        .unwrap();

    let mut declarations = Vec::new();
    for pair in inner {
        if pair.as_rule() == Rule::declaration {
            let decl = build_declaration(pair)?;
            declarations.push(decl);
        }
    }

    Ok(Workflow {
        name,
        version,
        declarations,
    })
}

fn build_declaration(pair: pest::iterators::Pair<Rule>) -> LoomResult<Declaration> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::queue_decl => Ok(Declaration::Queue(build_queue(inner)?)),
        Rule::action_decl => Ok(Declaration::Action(build_action(inner)?)),
        Rule::terminal_decl => Ok(Declaration::Terminal(build_terminal(inner)?)),
        Rule::escape_decl => Ok(Declaration::Escape(build_escape(inner)?)),
        Rule::step_decl => Ok(Declaration::Step(build_step(inner)?)),
        Rule::phase_decl => Ok(Declaration::Phase(build_phase(inner)?)),
        Rule::profile_decl => Ok(Declaration::Profile(build_profile(inner)?)),
        Rule::include_decl => Ok(Declaration::Include(build_include(inner)?)),
        Rule::wildcard_transition => Ok(Declaration::WildcardTransition(build_wildcard(inner)?)),
        _ => unreachable!("unexpected rule: {:?}", inner.as_rule()),
    }
}

fn build_queue(pair: pest::iterators::Pair<Rule>) -> LoomResult<QueueDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let display_name = strip_quotes(inner.next().unwrap().as_str());
    Ok(QueueDecl { name, display_name })
}

fn build_action(pair: pest::iterators::Pair<Rule>) -> LoomResult<ActionDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let display_name = strip_quotes(inner.next().unwrap().as_str());
    let body_pair = inner.next().unwrap();
    let mut body_inner = body_pair.into_inner();

    let action_type_pair = body_inner.next().unwrap();
    let action_type = build_action_type(action_type_pair)?;

    let prompt_pair = body_inner.next().unwrap();
    let prompt = prompt_pair.into_inner().next().unwrap().as_str().to_string();

    let mut constraints = Vec::new();
    for constraint_pair in body_inner {
        if constraint_pair.as_rule() == Rule::constraint {
            let kind_pair = constraint_pair.into_inner().next().unwrap();
            constraints.push(match kind_pair.as_str() {
                "read_only" => Constraint::ReadOnly,
                "no_git_write" => Constraint::NoGitWrite,
                "metadata_only" => Constraint::MetadataOnly,
                _ => unreachable!(),
            });
        }
    }

    Ok(ActionDecl {
        name,
        display_name,
        action_type,
        prompt,
        constraints,
    })
}

fn build_action_type(pair: pest::iterators::Pair<Rule>) -> LoomResult<ActionType> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::produce_type => {
            let executor = build_executor(inner.into_inner().next().unwrap());
            Ok(ActionType::Produce(executor))
        }
        Rule::gate_type => {
            let mut gi = inner.into_inner();
            let gate_kind = match gi.next().unwrap().as_str() {
                "approve" => GateKind::Approve,
                "auth" => GateKind::Auth,
                "review" => GateKind::Review,
                _ => unreachable!(),
            };
            let executor = build_executor(gi.next().unwrap());
            Ok(ActionType::Gate(gate_kind, executor))
        }
        _ => unreachable!(),
    }
}

fn build_executor(pair: pest::iterators::Pair<Rule>) -> Executor {
    match pair.as_str() {
        "agent" => Executor::Agent,
        "human" => Executor::Human,
        _ => unreachable!(),
    }
}

fn build_terminal(pair: pest::iterators::Pair<Rule>) -> LoomResult<TerminalDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let display_name = inner.next().map(|p| strip_quotes(p.as_str()));
    Ok(TerminalDecl { name, display_name })
}

fn build_escape(pair: pest::iterators::Pair<Rule>) -> LoomResult<EscapeDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let display_name = inner.next().map(|p| strip_quotes(p.as_str()));
    Ok(EscapeDecl { name, display_name })
}

fn build_step(pair: pest::iterators::Pair<Rule>) -> LoomResult<StepDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let queue = inner.next().unwrap().as_str().to_string();
    let action = inner.next().unwrap().as_str().to_string();
    Ok(StepDecl { name, queue, action })
}

fn build_phase(pair: pest::iterators::Pair<Rule>) -> LoomResult<PhaseDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let body_pair = inner.next().unwrap();
    let mut body_inner = body_pair.into_inner();
    let produce_step = body_inner.next().unwrap().as_str().to_string();
    let gate_step = body_inner.next().unwrap().as_str().to_string();
    Ok(PhaseDecl {
        name,
        produce_step,
        gate_step,
    })
}

fn build_profile(pair: pest::iterators::Pair<Rule>) -> LoomResult<ProfileDecl> {
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    let mut display_name = None;
    let mut fields = Vec::new();

    for p in inner {
        match p.as_rule() {
            Rule::string => {
                display_name = Some(strip_quotes(p.as_str()));
            }
            Rule::profile_body => {
                for field_pair in p.into_inner() {
                    let field_inner = field_pair.into_inner().next().unwrap();
                    match field_inner.as_rule() {
                        Rule::profile_phases => {
                            let list_pair = field_inner.into_inner().next().unwrap();
                            let phases: Vec<String> = list_pair
                                .into_inner()
                                .map(|p| p.as_str().to_string())
                                .collect();
                            fields.push(ProfileField::Phases(phases));
                        }
                        Rule::profile_output => {
                            let kind = field_inner.into_inner().next().unwrap();
                            let output = match kind.as_str() {
                                "local" => OutputKind::Local,
                                "remote" => OutputKind::Remote,
                                "remote_main" => OutputKind::RemoteMain,
                                "pr" => OutputKind::Pr,
                                _ => unreachable!(),
                            };
                            fields.push(ProfileField::Output(output));
                        }
                        Rule::profile_override => {
                            let mut oi = field_inner.into_inner();
                            let action = oi.next().unwrap().as_str().to_string();
                            let body = oi.next().unwrap(); // override_body
                            let field = body.into_inner().next().unwrap(); // override_field
                            let exec_override = field.into_inner().next().unwrap(); // executor_override
                            let exec_pair = exec_override.into_inner().next().unwrap(); // executor
                            let executor = build_executor(exec_pair);
                            fields.push(ProfileField::Override(OverrideDecl {
                                action,
                                executor,
                            }));
                        }
                        Rule::profile_description => {
                            let desc = strip_quotes(field_inner.into_inner().next().unwrap().as_str());
                            fields.push(ProfileField::Description(desc));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ProfileDecl {
        name,
        display_name,
        fields,
    })
}

fn build_include(pair: pest::iterators::Pair<Rule>) -> LoomResult<IncludeDecl> {
    let path = strip_quotes(pair.into_inner().next().unwrap().as_str());
    Ok(IncludeDecl { path })
}

fn build_wildcard(pair: pest::iterators::Pair<Rule>) -> LoomResult<WildcardTransitionDecl> {
    let target = pair.into_inner().next().unwrap().as_str().to_string();
    Ok(WildcardTransitionDecl { target })
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches('"').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_workflow() {
        let input = r#"
            workflow test v1 {
                queue q1 "Queue One"
                terminal done "Done"
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        assert_eq!(wf.name, "test");
        assert_eq!(wf.version, 1);
        assert_eq!(wf.declarations.len(), 2);
    }

    #[test]
    fn test_parse_action() {
        let input = r#"
            workflow test v1 {
                action planning "Planning" {
                    produce agent
                    prompt planning
                    constraint read_only
                }
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        assert_eq!(wf.declarations.len(), 1);
        if let Declaration::Action(a) = &wf.declarations[0] {
            assert_eq!(a.name, "planning");
            assert_eq!(a.action_type, ActionType::Produce(Executor::Agent));
            assert_eq!(a.prompt, "planning");
            assert_eq!(a.constraints, vec![Constraint::ReadOnly]);
        } else {
            panic!("expected action");
        }
    }

    #[test]
    fn test_parse_gate_action() {
        let input = r#"
            workflow test v1 {
                action review "Review" {
                    gate review human
                    prompt review
                }
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        if let Declaration::Action(a) = &wf.declarations[0] {
            assert_eq!(
                a.action_type,
                ActionType::Gate(GateKind::Review, Executor::Human)
            );
        } else {
            panic!("expected action");
        }
    }

    #[test]
    fn test_parse_step() {
        let input = r#"
            workflow test v1 {
                step plan {
                    q1 -> a1
                }
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        if let Declaration::Step(s) = &wf.declarations[0] {
            assert_eq!(s.name, "plan");
            assert_eq!(s.queue, "q1");
            assert_eq!(s.action, "a1");
        } else {
            panic!("expected step");
        }
    }

    #[test]
    fn test_parse_phase() {
        let input = r#"
            workflow test v1 {
                phase p1 {
                    produce s1
                    gate s2
                }
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        if let Declaration::Phase(p) = &wf.declarations[0] {
            assert_eq!(p.name, "p1");
            assert_eq!(p.produce_step, "s1");
            assert_eq!(p.gate_step, "s2");
        } else {
            panic!("expected phase");
        }
    }

    #[test]
    fn test_parse_profile() {
        let input = r#"
            workflow test v1 {
                profile auto "Autopilot" {
                    description "Full auto"
                    phases [p1, p2]
                    output remote_main
                    override review {
                        executor human
                    }
                }
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        if let Declaration::Profile(p) = &wf.declarations[0] {
            assert_eq!(p.name, "auto");
            assert_eq!(p.display_name, Some("Autopilot".to_string()));
            assert_eq!(p.fields.len(), 4);
        } else {
            panic!("expected profile");
        }
    }

    #[test]
    fn test_parse_wildcard() {
        let input = r#"
            workflow test v1 {
                * -> abandoned
            }
        "#;
        let wf = parse_workflow(input).unwrap();
        if let Declaration::WildcardTransition(w) = &wf.declarations[0] {
            assert_eq!(w.target, "abandoned");
        } else {
            panic!("expected wildcard");
        }
    }

    #[test]
    fn test_parse_knots_sdlc() {
        let input = std::fs::read_to_string("../../tests/fixtures/knots_sdlc/workflow.loom").unwrap();
        let wf = parse_workflow(&input).unwrap();
        assert_eq!(wf.name, "knots_sdlc");
        assert_eq!(wf.version, 1);
        // 6 queues + 6 actions + 2 terminals + 1 escape + 2 wildcards + 6 steps + 3 phases + 6 includes = 32
        assert_eq!(wf.declarations.len(), 32);
    }

    #[test]
    fn test_parse_profile_file() {
        let input = std::fs::read_to_string("../../tests/fixtures/knots_sdlc/profiles/semiauto.loom").unwrap();
        let profile = parse_profile_file(&input).unwrap();
        assert_eq!(profile.name, "semiauto");
        assert_eq!(profile.display_name, Some("Semi-automatic".to_string()));
    }
}

use serde_yaml::Value;
use std::fs;

fn load_ci_workflow() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.github/workflows/ci.yml");
    let contents = fs::read_to_string(path).expect("failed to read ci.yml");
    serde_yaml::from_str(&contents).expect("ci.yml is not valid YAML")
}

#[test]
fn ci_workflow_is_valid_yaml() {
    let workflow = load_ci_workflow();
    assert!(workflow.is_mapping(), "workflow should be a YAML mapping");
}

#[test]
fn ci_workflow_has_required_top_level_keys() {
    let workflow = load_ci_workflow();
    let map = workflow.as_mapping().unwrap();

    assert!(
        map.contains_key(&Value::String("name".into())),
        "workflow must have a 'name' key"
    );
    assert!(
        map.contains_key(&Value::String("on".into())),
        "workflow must have an 'on' key"
    );
    assert!(
        map.contains_key(&Value::String("jobs".into())),
        "workflow must have a 'jobs' key"
    );
}

#[test]
fn ci_workflow_triggers_on_push_and_pull_request() {
    let workflow = load_ci_workflow();
    let on = &workflow["on"];

    assert!(
        !on.is_null(),
        "workflow must have an 'on' trigger configuration"
    );

    let on_map = on.as_mapping().expect("'on' should be a mapping");
    assert!(
        on_map.contains_key(&Value::String("push".into())),
        "'on' must include 'push' trigger"
    );
    assert!(
        on_map.contains_key(&Value::String("pull_request".into())),
        "'on' must include 'pull_request' trigger"
    );

    // push should target main branch
    let push = &on["push"];
    let push_map = push.as_mapping().expect("push should be a mapping");
    assert!(
        push_map.contains_key(&Value::String("branches".into())),
        "push trigger should specify branches"
    );
}

#[test]
fn ci_workflow_has_at_least_one_job() {
    let workflow = load_ci_workflow();
    let jobs = workflow["jobs"]
        .as_mapping()
        .expect("'jobs' should be a mapping");
    assert!(!jobs.is_empty(), "jobs must contain at least one job");
}

#[test]
fn ci_workflow_job_steps_include_cargo_commands() {
    let workflow = load_ci_workflow();
    let jobs = workflow["jobs"].as_mapping().unwrap();

    // Get the first job
    let (_, job) = jobs.iter().next().expect("should have at least one job");
    let steps = job["steps"].as_sequence().expect("job should have steps");

    let step_texts: Vec<String> = steps
        .iter()
        .filter_map(|step| {
            // Collect both 'run' and 'name' fields for matching
            let run = step.get("run").and_then(Value::as_str).unwrap_or("");
            let name = step.get("name").and_then(Value::as_str).unwrap_or("");
            let uses = step.get("uses").and_then(Value::as_str).unwrap_or("");
            Some(format!("{} {} {}", run, name, uses))
        })
        .collect();

    let all_text = step_texts.join("\n");

    assert!(
        all_text.contains("cargo build"),
        "steps must include 'cargo build'"
    );
    assert!(
        all_text.contains("cargo test"),
        "steps must include 'cargo test'"
    );
    assert!(
        all_text.contains("cargo clippy"),
        "steps must include 'cargo clippy'"
    );
    assert!(
        all_text.contains("cargo fmt"),
        "steps must include 'cargo fmt'"
    );
}

#[test]
fn ci_workflow_has_caching() {
    let workflow = load_ci_workflow();
    let jobs = workflow["jobs"].as_mapping().unwrap();
    let (_, job) = jobs.iter().next().unwrap();
    let steps = job["steps"].as_sequence().unwrap();

    let has_cache = steps.iter().any(|step| {
        let uses = step.get("uses").and_then(Value::as_str).unwrap_or("");
        let name = step.get("name").and_then(Value::as_str).unwrap_or("");
        uses.contains("cache") || name.to_lowercase().contains("cache")
    });

    assert!(has_cache, "workflow must include a caching step");
}

#[test]
fn ci_workflow_uses_rust_stable_toolchain() {
    let workflow = load_ci_workflow();
    let jobs = workflow["jobs"].as_mapping().unwrap();
    let (_, job) = jobs.iter().next().unwrap();
    let steps = job["steps"].as_sequence().unwrap();

    let has_rust_toolchain = steps.iter().any(|step| {
        let uses = step.get("uses").and_then(Value::as_str).unwrap_or("");
        uses.contains("rust-toolchain")
    });

    assert!(
        has_rust_toolchain,
        "workflow must use dtolnay/rust-toolchain or equivalent"
    );
}

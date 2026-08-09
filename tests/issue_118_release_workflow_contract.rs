//! Regression contract for the release workflow's Clippy package boundary.
//!
//! The release workflow must lint the Rustkyll package without linting
//! unchanged vendored workspace dependencies. Keep it aligned with main CI.

fn step_run_command<'a>(workflow: &'a str, step_name: &str) -> &'a str {
    let marker = format!("- name: {step_name}");
    let step = workflow
        .split_once(&marker)
        .unwrap_or_else(|| panic!("workflow must contain step {step_name:?}"))
        .1;

    step.lines()
        .find_map(|line| line.trim().strip_prefix("run: "))
        .unwrap_or_else(|| panic!("workflow step {step_name:?} must contain a run command"))
}

#[test]
fn release_clippy_uses_the_main_ci_project_boundary() {
    let main_ci = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");
    let release = std::fs::read_to_string(".github/workflows/release.yml")
        .expect(".github/workflows/release.yml must exist");

    let expected = "cargo clippy -p rustkyll --no-deps -- -D warnings";
    let main_command = step_run_command(&main_ci, "cargo clippy");
    let release_command = step_run_command(&release, "Run clippy");

    assert_eq!(
        main_command, expected,
        "main CI's Rustkyll-only Clippy boundary changed unexpectedly"
    );
    assert_eq!(
        release_command, main_command,
        "release Clippy must match main CI and must not lint vendored workspace dependencies"
    );
}

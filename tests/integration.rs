use std::process::Command;

#[test]
fn binary_help_exits_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("--help")
        .output()
        .expect("failed to run rustkyll binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rustkyll"));
}

#[test]
fn binary_runs_without_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .output()
        .expect("failed to run rustkyll binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from rustkyll"));
}

#[test]
fn binary_build_subcommand_without_source_fails_gracefully() {
    // Build without a valid source directory should fail with a config error
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg("/nonexistent/path")
        .output()
        .expect("failed to run rustkyll binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Build failed"),
        "Expected build failure message, got: {}",
        stderr
    );
}

#[test]
fn library_exposes_public_items() {
    // Verify the library crate is usable
    assert_eq!(rustkyll::project_name(), "rustkyll");
    assert!(!rustkyll::version().is_empty());
}

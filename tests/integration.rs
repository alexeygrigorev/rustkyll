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
fn binary_build_subcommand_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .output()
        .expect("failed to run rustkyll binary");

    assert!(output.status.success());
}

#[test]
fn library_exposes_public_items() {
    // Verify the library crate is usable
    assert_eq!(rustkyll::project_name(), "rustkyll");
    assert!(!rustkyll::version().is_empty());
}

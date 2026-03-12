use std::process::Command;

#[test]
fn binary_help_exits_successfully() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyl"))
        .arg("--help")
        .output()
        .expect("failed to run rustkyl binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rustkyl"));
}

#[test]
fn binary_runs_without_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyl"))
        .output()
        .expect("failed to run rustkyl binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello from rustkyl"));
}

#[test]
fn binary_build_subcommand_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustkyl"))
        .arg("build")
        .output()
        .expect("failed to run rustkyl binary");

    assert!(output.status.success());
}

#[test]
fn library_exposes_public_items() {
    // Verify the library crate is usable
    assert_eq!(rustkyl::project_name(), "rustkyl");
    assert!(!rustkyl::version().is_empty());
}

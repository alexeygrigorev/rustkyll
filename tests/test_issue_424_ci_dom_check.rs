/// Tests for issue #424: CI DTC DOM regression check
///
/// Validates that the ci.yml workflow contains the dom-check job
/// with the correct structure, steps, and configuration.

#[test]
fn ci_yml_has_dom_check_job() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    // Basic YAML validity: must contain the expected job keys
    assert!(
        ci_yml.contains("  check:"),
        "ci.yml must contain the existing 'check' job"
    );
    assert!(
        ci_yml.contains("  dom-check:"),
        "ci.yml must contain the new 'dom-check' job"
    );
}

#[test]
fn dom_check_job_has_timeout() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    // Find the dom-check section and verify timeout
    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    assert!(
        dom_check_section.contains("timeout-minutes: 30"),
        "dom-check job must have timeout-minutes: 30"
    );
}

#[test]
fn dom_check_job_has_no_needs_dependency() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    // Extract dom-check job block (from "  dom-check:" until the next top-level job or EOF)
    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Find the next job at indent level 2 (two spaces + word + colon) after dom-check
    let next_job = dom_check_section[12..] // skip "  dom-check:"
        .find("\n  ")
        .and_then(|pos| {
            // Check if this is a new job definition (not indented further)
            let after = &dom_check_section[12 + pos + 3..];
            if after.starts_with(char::is_alphabetic) && after.contains(':') {
                Some(12 + pos)
            } else {
                None
            }
        });

    let dom_check_block = match next_job {
        Some(end) => &dom_check_section[..end],
        None => dom_check_section,
    };

    // The dom-check block must not contain "needs:"
    assert!(
        !dom_check_block.contains("needs:"),
        "dom-check job must NOT have a 'needs:' dependency (runs in parallel with 'check')"
    );
}

#[test]
fn dom_check_has_required_steps() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Verify all required steps are present
    let required_patterns = [
        ("checkout", "actions/checkout@v4"),
        ("rust toolchain", "dtolnay/rust-toolchain@stable"),
        ("rust cache", "Swatinem/rust-cache@v2"),
        ("ruby setup", "ruby/setup-ruby@v1"),
        ("ruby version 3.3", "ruby-version: '3.3'"),
        ("gem cache", "actions/cache@v4"),
        ("clone DTC", "datatalksclub.github.io"),
        ("bundle install", "bundle install"),
        ("jekyll build", "bundle exec jekyll build"),
        ("cargo build release", "cargo build --release"),
        ("rustkyll build", "rustkyll build"),
        ("install uv", "astral-sh/setup-uv"),
        ("dom_compare.py", "dom_compare.py"),
        ("assert matched count", "790"),
    ];

    for (description, pattern) in required_patterns {
        assert!(
            dom_check_section.contains(pattern),
            "dom-check job must contain '{}' for: {}",
            pattern,
            description
        );
    }
}

#[test]
fn dom_check_clones_dtc_with_depth_1() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    assert!(
        dom_check_section.contains("--depth 1"),
        "DTC clone must use --depth 1 (shallow clone)"
    );
}

#[test]
fn check_job_is_unchanged() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    // Extract the check job block
    let check_start = ci_yml.find("  check:").expect("check job must exist");
    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");

    // The check job should end before dom-check starts
    // There should be a blank line separating them
    let check_block = ci_yml[check_start..dom_check_start].trim_end();

    let expected_check = r#"check:
    name: Build & Test
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry and target
        uses: Swatinem/rust-cache@v2

      - name: cargo test
        run: cargo test -p rustkyll --verbose

      - name: cargo clippy
        run: cargo clippy -p rustkyll --no-deps -- -D warnings

      - name: cargo fmt --check
        run: cargo fmt --check"#;

    assert!(
        check_block.contains(expected_check),
        "The 'check' job must be unchanged from the original. Got:\n{}",
        check_block
    );
}

#[test]
fn dom_check_assertion_parses_summary_format() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Verify the assertion step checks both matched count and total diffs
    assert!(
        dom_check_section.contains("-lt 790"),
        "Assertion must check matched count >= 790"
    );
    assert!(
        dom_check_section.contains("-ne 0"),
        "Assertion must check total diffs == 0"
    );
}

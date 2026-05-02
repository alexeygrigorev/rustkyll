/// Tests for issue #457: CI -- add DTC docs to per-push DOM check
///
/// Validates that:
/// 1. dom-baselines.json has the correct baseline for DataTalksClub/docs (56)
/// 2. ci.yml dom-check job includes DTC docs clone, build, and assertion steps

#[test]
fn dom_baselines_dtc_docs_is_56() {
    let baselines_str = std::fs::read_to_string("docs/dom-baselines.json")
        .expect("docs/dom-baselines.json must exist");
    let baselines: serde_json::Value =
        serde_json::from_str(&baselines_str).expect("dom-baselines.json must be valid JSON");

    let dtc_docs_baseline = baselines
        .get("DataTalksClub/docs")
        .expect("DataTalksClub/docs must exist in dom-baselines.json")
        .as_i64()
        .expect("DataTalksClub/docs value must be a number");

    assert_eq!(
        dtc_docs_baseline, 56,
        "DataTalksClub/docs baseline must be 56 (matched count), got {}",
        dtc_docs_baseline
    );
}

#[test]
fn ci_yml_dom_check_clones_dtc_docs() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    assert!(
        dom_check_section.contains("DataTalksClub/docs"),
        "dom-check job must clone the DataTalksClub/docs repository"
    );
}

#[test]
fn ci_yml_dom_check_installs_dtc_docs_gems() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must have a bundle install step that references the docs directory
    assert!(
        dom_check_section.contains("cd dtc-docs")
            || dom_check_section.contains("cd DataTalksClub-docs"),
        "dom-check job must cd into the DTC docs directory for bundle install"
    );
    assert!(
        dom_check_section.contains("just-the-docs") || dom_check_section.contains("bundle install"),
        "dom-check job must install gems for DTC docs (needs just-the-docs theme)"
    );
}

#[test]
fn ci_yml_dom_check_builds_dtc_docs_with_jekyll() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must build DTC docs with Jekyll (separate from DTC main)
    // Count occurrences of "bundle exec jekyll build" - should be 2 (main + docs)
    let jekyll_build_count = dom_check_section
        .matches("bundle exec jekyll build")
        .count();
    assert!(
        jekyll_build_count >= 2,
        "dom-check must have at least 2 Jekyll builds (DTC main + DTC docs), found {}",
        jekyll_build_count
    );
}

#[test]
fn ci_yml_dom_check_builds_dtc_docs_with_rustkyll() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must build DTC docs with rustkyll (separate from DTC main)
    let rustkyll_build_count = dom_check_section.matches("rustkyll build").count();
    assert!(
        rustkyll_build_count >= 2,
        "dom-check must have at least 2 rustkyll builds (DTC main + DTC docs), found {}",
        rustkyll_build_count
    );
}

#[test]
fn ci_yml_dom_check_runs_dom_compare_for_dtc_docs() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must run dom_compare.py twice (DTC main + DTC docs)
    let dom_compare_count = dom_check_section.matches("dom_compare.py").count();
    assert!(
        dom_compare_count >= 2,
        "dom-check must run dom_compare.py at least twice (DTC main + DTC docs), found {}",
        dom_compare_count
    );
}

#[test]
fn ci_yml_dom_check_asserts_dtc_docs_baseline() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must assert DTC docs matched count >= 56
    assert!(
        dom_check_section.contains("-lt 56"),
        "dom-check must assert DTC docs matched count >= 56 (check for '-lt 56')"
    );
    // Must assert DTC docs total diffs <= 26
    assert!(
        dom_check_section.contains("-gt 26"),
        "dom-check must assert DTC docs total diffs <= 26 (check for '-gt 26')"
    );
}

#[test]
fn ci_yml_dom_check_uploads_dtc_docs_report_on_failure() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // Must upload DTC docs DOM report on failure
    assert!(
        dom_check_section.contains("dtc-docs-dom-report")
            || dom_check_section.contains("dom-report-docs"),
        "dom-check must upload DTC docs DOM report as artifact on failure"
    );
}

#[test]
fn ci_yml_dtc_main_assertions_unchanged() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    // DTC main assertions must still be present
    assert!(
        dom_check_section.contains("-lt 795"),
        "DTC main assertion (matched >= 795) must still be present"
    );
    assert!(
        dom_check_section.contains("-ne 0"),
        "DTC main assertion (0 diffs) must still be present"
    );
}

/// Tests for issue #457: CI -- add DTC docs to per-push DOM check
///
/// Validates that:
/// 1. dom-baselines.json has the correct baseline for DataTalksClub/docs (56)
/// 2. ci.yml dom-check job includes DTC docs clone, build, and validation steps

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
fn ci_yml_dom_check_builds_dtc_docs_with_rustkyll() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    let rustkyll_build_count = dom_check_section.matches("rustkyll build").count();
    assert!(
        rustkyll_build_count >= 2,
        "dom-check must have at least 2 rustkyll builds (DTC main + DTC docs), found {}",
        rustkyll_build_count
    );
}

#[test]
fn ci_yml_dom_check_validates_dtc_docs_output() {
    let ci_yml = std::fs::read_to_string(".github/workflows/ci.yml")
        .expect(".github/workflows/ci.yml must exist");

    let dom_check_start = ci_yml
        .find("  dom-check:")
        .expect("dom-check job must exist");
    let dom_check_section = &ci_yml[dom_check_start..];

    assert!(
        dom_check_section.contains("-lt 56"),
        "dom-check must assert DTC docs HTML file count >= 56 (check for '-lt 56')"
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

    assert!(
        dom_check_section.contains("-lt 795"),
        "DTC main assertion (HTML count >= 795) must still be present"
    );
}

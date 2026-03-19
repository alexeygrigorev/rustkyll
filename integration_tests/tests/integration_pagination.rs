//! Integration tests for pagination with real sites (Issue 98).
//!
//! These tests require website checkouts in `websites/`.
//!
//! Run with: cargo test -p integration-tests --test integration_pagination

use rustkyll::collection;
use rustkyll::config::SiteConfig;
use rustkyll::pagination::PaginationConfig;

/// Test with the hyde website which uses paginator.
#[test]
fn test_hyde_pagination_builds() {
    use std::path::PathBuf;

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let site_dir = project_root.join("websites/hyde");
    if !site_dir.exists() {
        eprintln!("Skipping: hyde site not present");
        return;
    }

    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config);
    assert!(
        pagination_config.is_some(),
        "Hyde should have pagination config"
    );
    let pagination_config = pagination_config.unwrap();
    assert_eq!(pagination_config.per_page, 5);

    // Load posts
    let (posts, _) = collection::load_collection("posts", &site_dir, &config).unwrap();
    assert!(!posts.is_empty(), "Hyde should have posts");

    // Load pages
    let (pages, _) = collection::load_pages(&site_dir, &config).unwrap();
    let index_page = rustkyll::pagination::find_index_page(&pages);
    assert!(index_page.is_some(), "Hyde should have an index page");
}

/// Test with beautiful-jekyll which uses paginator.
#[test]
fn test_beautiful_jekyll_pagination_builds() {
    use std::path::PathBuf;

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let site_dir = project_root.join("websites/beautiful-jekyll");
    if !site_dir.exists() {
        eprintln!("Skipping: beautiful-jekyll site not present");
        return;
    }

    let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
    let pagination_config = PaginationConfig::from_config(&config);
    assert!(
        pagination_config.is_some(),
        "beautiful-jekyll should have pagination config"
    );
    let pagination_config = pagination_config.unwrap();
    assert_eq!(pagination_config.per_page, 5);
}

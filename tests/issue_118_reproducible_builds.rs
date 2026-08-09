use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

use rustkyll::collection::{CollectionItem, Page};
use rustkyll::frontmatter::FrontMatter;
use rustkyll::incremental::{self, BuildManifest};
use rustkyll::sitemap;

const FROZEN_EPOCH: u64 = 946_771_200; // 2000-01-02T00:00:00Z

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn normalize_file_mtimes(root: &Path) {
    let modified = SystemTime::UNIX_EPOCH + Duration::from_secs(FROZEN_EPOCH);
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                let file = fs::OpenOptions::new()
                    .write(true)
                    .open(entry.path())
                    .unwrap();
                file.set_modified(modified).unwrap();
            }
        }
    }
}

fn make_site(root: &Path, timezone: Option<&str>) {
    let timezone_line = timezone
        .map(|value| format!("timezone: {value}\n"))
        .unwrap_or_default();
    write(
        &root.join("_config.yml"),
        &format!(
            "url: https://example.com\ntitle: Reproducible\n{timezone_line}collections:\n  notes:\n    output: true\n"
        ),
    );
    write(
        &root.join("index.md"),
        "---\n---\nsite={{ site.time }}; note={{ site.notes | map: \"date\" | first }}\n",
    );
    write(&root.join("a.md"), "---\n---\na\n");
    write(&root.join("z.md"), "---\n---\nz\n");
    write(
        &root.join("_notes/undated.md"),
        "---\ntitle: Undated\n---\nnote\n",
    );
    normalize_file_mtimes(root);
}

fn run_build(source: &Path, destination: &Path, epoch: &str, timezone: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rustkyll"))
        .arg("build")
        .arg("--source")
        .arg(source)
        .arg("--destination")
        .arg(destination)
        .env_clear()
        .env("SOURCE_DATE_EPOCH", epoch)
        .env("TZ", timezone)
        .output()
        .unwrap()
}

fn output_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                files.push((
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(entry.path()).unwrap(),
                ));
            }
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn collection_item(slug: &str, url: &str) -> CollectionItem {
    CollectionItem {
        slug: slug.to_string(),
        front_matter: FrontMatter::new(),
        content: String::new(),
        html_content: String::new(),
        excerpt: None,
        excerpt_html: None,
        url: url.to_string(),
        date: None,
        collection_name: "notes".to_string(),
        source_path: format!("_notes/{slug}.md"),
        id: format!("/notes/{slug}"),
    }
}

fn page(slug: &str, url: &str) -> Page {
    Page {
        slug: slug.to_string(),
        front_matter: FrontMatter::new(),
        content: String::new(),
        html_content: String::new(),
        url: url.to_string(),
        source_path: format!("{slug}.md"),
    }
}

#[test]
fn source_date_epoch_drives_site_collection_and_feed_times() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("output");
    make_site(&source, Some("Europe/Berlin"));

    let output = run_build(&source, &destination, "946771200", "Pacific/Honolulu");
    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let index = fs::read_to_string(destination.join("index.html")).unwrap();
    assert!(index.contains("site=2000-01-02 01:00:00"), "{index}");
    assert!(index.contains("note=2000-01-02 01:00:00 +0100"), "{index}");
    let feed = fs::read_to_string(destination.join("feed.xml")).unwrap();
    assert!(feed.contains("<updated>2000-01-02T00:00:00+00:00</updated>"));
}

#[test]
fn source_date_epoch_controls_future_post_cutoff() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("output");
    make_site(&source, None);
    write(
        &source.join("_posts/2000-01-01-past.md"),
        "---\ntitle: Past\n---\npast\n",
    );
    write(
        &source.join("_posts/2000-01-03-future.md"),
        "---\ntitle: Future\n---\nfuture\n",
    );
    normalize_file_mtimes(&source);

    let output = run_build(&source, &destination, "946771200", "Europe/Berlin");
    assert!(output.status.success());
    let tree = output_tree(&destination);
    assert!(tree
        .iter()
        .any(|(path, _)| path.to_string_lossy().contains("past")));
    assert!(!tree
        .iter()
        .any(|(path, _)| path.to_string_lossy().contains("future")));
}

#[test]
fn invalid_source_date_epoch_fails_closed_without_echoing_the_value() {
    let invalid = [
        "",
        "-1",
        "+1",
        "1.5",
        " 1",
        "18446744073709551616",
        "not-an-epoch",
    ];
    for (index, value) in invalid.into_iter().enumerate() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join(format!("output-{index}"));
        make_site(&source, None);
        let output = run_build(&source, &destination, value, "UTC");
        assert!(
            !output.status.success(),
            "accepted invalid epoch at index {index}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("invalid SOURCE_DATE_EPOCH"), "{stderr}");
        if !value.is_empty() {
            assert!(
                !stderr.contains(value),
                "diagnostic exposed input: {stderr}"
            );
        }
    }
}

#[test]
fn sitemap_is_root_first_then_sorted_by_final_location() {
    let first = sitemap::collect_entries(
        "https://example.com",
        &[
            ("z".to_string(), vec![collection_item("z", "/z.html")]),
            ("a".to_string(), vec![collection_item("a", "/a.html")]),
        ],
        &[page("middle", "/middle.html")],
    );
    let second = sitemap::collect_entries(
        "https://example.com",
        &[
            ("a".to_string(), vec![collection_item("a", "/a.html")]),
            ("z".to_string(), vec![collection_item("z", "/z.html")]),
        ],
        &[page("middle", "/middle.html")],
    );
    let locations: Vec<_> = first.iter().map(|entry| entry.loc.as_str()).collect();
    assert_eq!(first, second);
    assert_eq!(
        locations,
        [
            "https://example.com/",
            "https://example.com/a.html",
            "https://example.com/middle.html",
            "https://example.com/z.html",
        ]
    );
}

#[test]
fn manifest_serialization_is_canonical_across_map_construction_order() {
    fn manifest(reverse: bool) -> BuildManifest {
        let keys: Vec<_> = if reverse {
            (0..20).rev().collect()
        } else {
            (0..20).collect()
        };
        let mut source_files = HashMap::new();
        let mut output_map = HashMap::new();
        let mut global_files = HashMap::new();
        for key in keys {
            source_files.insert(format!("source-{key:02}"), key);
            output_map.insert(format!("source-{key:02}"), format!("output-{key:02}"));
            global_files.insert(format!("global-{key:02}"), key);
        }
        BuildManifest {
            source_files,
            output_map,
            global_files,
        }
    }

    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    incremental::save_manifest(first.path(), &manifest(false)).unwrap();
    incremental::save_manifest(second.path(), &manifest(true)).unwrap();
    let first_bytes = fs::read(first.path().join(".rustkyll-manifest.json")).unwrap();
    let second_bytes = fs::read(second.path().join(".rustkyll-manifest.json")).unwrap();
    assert_eq!(first_bytes, second_bytes);

    let text = String::from_utf8(first_bytes).unwrap();
    for prefix in ["source", "output", "global"] {
        for key in 0..19 {
            let left = text.find(&format!("\"{prefix}-{key:02}\"")).unwrap();
            let right = text.find(&format!("\"{prefix}-{:02}\"", key + 1)).unwrap();
            assert!(left < right, "manifest keys are not canonical: {text}");
        }
    }
}

#[test]
fn complete_trees_match_across_separate_workspaces_and_processes() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let first_source = first.path().join("source");
    let second_source = second.path().join("source");
    let first_output = first.path().join("output");
    let second_output = second.path().join("output");
    make_site(&first_source, None);
    make_site(&second_source, None);

    let first_result = run_build(&first_source, &first_output, "946771200", "UTC");
    let second_result = run_build(
        &second_source,
        &second_output,
        "946771200",
        "Pacific/Honolulu",
    );
    assert!(first_result.status.success());
    assert!(second_result.status.success());
    assert_eq!(output_tree(&first_output), output_tree(&second_output));
}

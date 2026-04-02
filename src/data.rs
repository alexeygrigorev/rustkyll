use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Errors that can occur when loading data files.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("data directory does not exist: {0}")]
    DirectoryNotFound(String),

    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to read file {path}: {source}")]
    ReadFile {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to parse YAML in {path}: {source}")]
    YamlParse {
        path: String,
        source: crate::yaml::YamlParseError,
    },

    #[error("failed to parse JSON in {path}: {source}")]
    JsonParse {
        path: String,
        source: serde_json::Error,
    },
}

/// A data tree mapping keys (derived from filenames) to parsed YAML values.
///
/// Uses `BTreeMap` to ensure deterministic alphabetical iteration order,
/// matching Jekyll's behavior of iterating data files in sorted order.
pub type DataTree = BTreeMap<String, serde_yaml::Value>;

/// Load all YAML data files from the given directory into a data tree.
///
/// Top-level `.yaml` and `.yml` files become top-level keys (using the file stem).
/// Subdirectories become nested mappings where the directory name is the key and
/// each file inside becomes a sub-key.
///
/// # Errors
///
/// Returns `DataError::DirectoryNotFound` if `data_dir` does not exist.
/// Returns `DataError::YamlParse` (with the filename) if any file contains invalid YAML.
pub fn load_data(data_dir: &Path) -> Result<DataTree, DataError> {
    load_data_internal(data_dir, None, None)
}

/// Load data and apply config-aware postprocessing that depends on the site root.
pub fn load_data_with_config(
    data_dir: &Path,
    site_source: &Path,
    config: &crate::config::SiteConfig,
) -> Result<DataTree, DataError> {
    load_data_internal(data_dir, Some(site_source), Some(config))
}

fn load_data_internal(
    data_dir: &Path,
    site_source: Option<&Path>,
    config: Option<&crate::config::SiteConfig>,
) -> Result<DataTree, DataError> {
    if !data_dir.exists() {
        return Err(DataError::DirectoryNotFound(data_dir.display().to_string()));
    }

    let mut tree = DataTree::new();

    // Collect and sort directory entries to ensure deterministic order.
    // Jekyll processes data files in alphabetical order; fs::read_dir does not
    // guarantee any particular ordering.
    let mut paths: Vec<_> = fs::read_dir(data_dir)
        .map_err(|e| DataError::ReadDir {
            path: data_dir.display().to_string(),
            source: e,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::ReadDir {
            path: data_dir.display().to_string(),
            source: e,
        })?;
    paths.sort_by_key(|e| e.file_name());

    for entry in paths {
        let path = entry.path();

        if path.is_dir() {
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };
            let sub_tree = load_directory_files(&path)?;
            // Convert the sub-tree BTreeMap into a serde_yaml::Value::Mapping
            // (serde_yaml::Mapping preserves insertion order)
            let mut mapping = serde_yaml::Mapping::new();
            for (key, value) in sub_tree {
                mapping.insert(serde_yaml::Value::String(key), value);
            }
            tree.insert(dir_name, serde_yaml::Value::Mapping(mapping));
        } else if is_data_file(&path) {
            let key = file_stem(&path);
            if let Some(key) = key {
                let value = load_data_file(&path)?;
                tree.insert(key, value);
            }
        }
    }

    if let (Some(site_source), Some(config)) = (site_source, config) {
        postprocess_data(&mut tree, site_source, config);
    }

    Ok(tree)
}

fn postprocess_data(
    tree: &mut DataTree,
    site_source: &Path,
    config: &crate::config::SiteConfig,
) {
    let Some(theme_name) = config.extras.get("theme").and_then(|v| v.as_str()) else {
        return;
    };

    let Some(versions) = tree.get_mut("versions").and_then(|v| v.as_mapping_mut()) else {
        return;
    };

    let current_key = serde_yaml::Value::String("current".to_string());
    let Some(current) = versions.get_mut(&current_key) else {
        return;
    };

    let Some(current_str) = current.as_str() else {
        return;
    };

    if !current_str.eq_ignore_ascii_case("auto") {
        return;
    }

    let Some(version) = extract_theme_version_from_gemfile_lock(site_source, theme_name) else {
        return;
    };

    *current = serde_yaml::Value::String(format!("v{}", version));
}

fn extract_theme_version_from_gemfile_lock(site_source: &Path, theme_name: &str) -> Option<String> {
    let gemfile_lock = site_source.join("Gemfile.lock");
    let content = fs::read_to_string(gemfile_lock).ok()?;
    extract_theme_version_from_lock_content(&content, theme_name)
}

fn extract_theme_version_from_lock_content(content: &str, theme_name: &str) -> Option<String> {
    let mut in_specs = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "specs:" {
            in_specs = true;
            continue;
        }

        if in_specs && !line.starts_with(' ') {
            break;
        }

        if !in_specs || !line.starts_with("    ") {
            continue;
        }

        let prefix = format!("{theme_name} (");
        let candidate = line.trim_start();
        if !candidate.starts_with(&prefix) {
            continue;
        }

        let remainder = &candidate[prefix.len()..];
        let end = remainder.find(')')?;
        return Some(remainder[..end].to_string());
    }

    None
}

/// Load all YAML files in a single directory (non-recursive) into a BTreeMap.
///
/// Uses `BTreeMap` so that keys are in sorted order, matching Jekyll's
/// alphabetical iteration over data subdirectory files.
fn load_directory_files(dir: &Path) -> Result<BTreeMap<String, serde_yaml::Value>, DataError> {
    let mut map = BTreeMap::new();

    // Collect and sort entries for deterministic ordering
    let mut paths: Vec<_> = fs::read_dir(dir)
        .map_err(|e| DataError::ReadDir {
            path: dir.display().to_string(),
            source: e,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::ReadDir {
            path: dir.display().to_string(),
            source: e,
        })?;
    paths.sort_by_key(|e| e.file_name());

    for entry in paths {
        let path = entry.path();

        if path.is_file() && is_data_file(&path) {
            if let Some(key) = file_stem(&path) {
                let value = load_data_file(&path)?;
                map.insert(key, value);
            }
        }
    }

    Ok(map)
}

/// Check whether a path has a `.yaml` or `.yml` extension.
fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml") | Some("yml")
    )
}

/// Check whether a path has a `.json` extension.
fn is_json_file(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("json"))
}

/// Check whether a path is a supported data file (YAML or JSON).
fn is_data_file(path: &Path) -> bool {
    is_yaml_file(path) || is_json_file(path)
}

/// Extract the file stem (filename without extension) as a String.
fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Read and parse a single data file (YAML or JSON).
fn load_data_file(path: &Path) -> Result<serde_yaml::Value, DataError> {
    if is_json_file(path) {
        load_json_file(path)
    } else {
        load_yaml_file(path)
    }
}

/// Read and parse a single JSON file, converting the result to serde_yaml::Value
/// for uniform handling with YAML data files.
fn load_json_file(path: &Path) -> Result<serde_yaml::Value, DataError> {
    let content = fs::read_to_string(path).map_err(|e| DataError::ReadFile {
        path: path.display().to_string(),
        source: e,
    })?;

    let json_value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| DataError::JsonParse {
            path: path.display().to_string(),
            source: e,
        })?;

    Ok(json_to_yaml(json_value))
}

/// Convert a serde_json::Value to a serde_yaml::Value.
///
/// This allows JSON data files to be used interchangeably with YAML data files
/// in the site data tree, matching Jekyll's behavior.
fn json_to_yaml(json: serde_json::Value) -> serde_yaml::Value {
    match json {
        serde_json::Value::Null => serde_yaml::Value::Null,
        serde_json::Value::Bool(b) => serde_yaml::Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(i))
            } else if let Some(f) = n.as_f64() {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => serde_yaml::Value::String(s),
        serde_json::Value::Array(arr) => {
            serde_yaml::Value::Sequence(arr.into_iter().map(json_to_yaml).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (key, value) in obj {
                mapping.insert(serde_yaml::Value::String(key), json_to_yaml(value));
            }
            serde_yaml::Value::Mapping(mapping)
        }
    }
}

/// Read and parse a single YAML file, tolerating duplicate keys.
fn load_yaml_file(path: &Path) -> Result<serde_yaml::Value, DataError> {
    let content = fs::read_to_string(path).map_err(|e| DataError::ReadFile {
        path: path.display().to_string(),
        source: e,
    })?;

    let value = crate::yaml::parse_yaml_lenient(&content).map_err(|e| DataError::YamlParse {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ========================================================================
    // Unit: YAML parsing basics
    // ========================================================================

    #[test]
    fn test_load_yaml_sequence() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("items.yaml");
        fs::write(&file, "- one\n- two\n- three\n").unwrap();

        let value = load_yaml_file(&file).unwrap();
        assert!(value.is_sequence(), "Expected a sequence, got {:?}", value);
        assert_eq!(value.as_sequence().unwrap().len(), 3);
    }

    #[test]
    fn test_load_yaml_mapping() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("config.yaml");
        fs::write(&file, "name: test\nversion: 1\n").unwrap();

        let value = load_yaml_file(&file).unwrap();
        assert!(value.is_mapping(), "Expected a mapping, got {:?}", value);
        let mapping = value.as_mapping().unwrap();
        assert_eq!(
            mapping
                .get(serde_yaml::Value::String("name".into()))
                .and_then(|v| v.as_str()),
            Some("test")
        );
    }

    #[test]
    fn test_invalid_yaml_includes_filename() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.yaml");
        fs::write(&file, ":\n  - :\n    invalid: [unclosed").unwrap();

        let err = load_yaml_file(&file).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad.yaml"),
            "Error should include filename, got: {}",
            msg
        );
    }

    // ========================================================================
    // Unit: Directory traversal and key derivation
    // ========================================================================

    #[test]
    fn test_yaml_extension_key() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("foo.yaml"), "value: 1\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("foo"), "Expected key 'foo' in {:?}", tree);
    }

    #[test]
    fn test_yml_extension_key() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("bar.yml"), "value: 2\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("bar"), "Expected key 'bar' in {:?}", tree);
    }

    #[test]
    fn test_subdirectory_nested_key() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("baz.yml"), "- question: why\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("sub"), "Expected key 'sub' in {:?}", tree);

        let sub_mapping = tree["sub"].as_mapping().unwrap();
        assert!(
            sub_mapping.contains_key(serde_yaml::Value::String("baz".into())),
            "Expected nested key 'baz' in {:?}",
            sub_mapping
        );
    }

    // ========================================================================
    // Unit: Empty and missing directories
    // ========================================================================

    #[test]
    fn test_empty_directory() {
        let dir = TempDir::new().unwrap();
        let tree = load_data(dir.path()).unwrap();
        assert!(tree.is_empty());
    }

    #[test]
    fn test_nonexistent_directory() {
        let err = load_data(Path::new("/nonexistent/path/_data")).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("does not exist"),
            "Expected 'does not exist' in error, got: {}",
            msg
        );
    }

    // ========================================================================
    // Unit: Non-YAML files are ignored
    // ========================================================================

    #[test]
    fn test_non_yaml_files_ignored() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("readme.txt"), "ignore me").unwrap();
        fs::write(dir.path().join("data.yaml"), "key: val\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert_eq!(tree.len(), 1);
        assert!(tree.contains_key("data"));
    }

    // ========================================================================
    // Integration: Load actual DataTalks.Club data
    // ========================================================================

    fn data_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join("_data")
    }

    #[test]
    fn test_load_real_data_succeeds() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_real_events() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let events = &tree["events"];
        let seq = events.as_sequence().expect("events should be a sequence");
        assert!(!seq.is_empty(), "Expected events, got {}", seq.len());

        // First event should have expected keys
        let first = seq[0].as_mapping().expect("event should be a mapping");
        for key in &["time", "title", "speakers", "type", "link"] {
            assert!(
                first.contains_key(serde_yaml::Value::String((*key).to_string())),
                "First event missing key '{}'",
                key
            );
        }
    }

    #[test]
    fn test_real_events_extra() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let extra = &tree["events_extra"];
        assert!(
            extra.is_sequence(),
            "events_extra should be a sequence, got {:?}",
            extra
        );
    }

    #[test]
    fn test_real_header() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let header = &tree["header"];
        let mapping = header.as_mapping().expect("header should be a mapping");
        assert!(
            mapping.contains_key(serde_yaml::Value::String("announcement".into())),
            "header should contain 'announcement' key"
        );
    }

    #[test]
    fn test_real_navigation() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let nav = &tree["navigation"];
        let mapping = nav.as_mapping().expect("navigation should be a mapping");
        assert!(mapping.contains_key(serde_yaml::Value::String("top".into())));
        assert!(mapping.contains_key(serde_yaml::Value::String("bottom".into())));

        let top = mapping
            .get(serde_yaml::Value::String("top".into()))
            .unwrap();
        let top_seq = top
            .as_sequence()
            .expect("navigation.top should be a sequence");
        assert!(!top_seq.is_empty(), "navigation.top should not be empty");
    }

    #[test]
    fn test_real_sponsors() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let sponsors = &tree["sponsors"];
        let seq = sponsors
            .as_sequence()
            .expect("sponsors should be a sequence");
        assert!(!seq.is_empty(), "sponsors should not be empty");

        // Check a sponsor has expected keys
        let sponsor = seq[0].as_mapping().expect("sponsor should be a mapping");
        for key in &["name", "link", "image", "from", "to"] {
            assert!(
                sponsor.contains_key(serde_yaml::Value::String((*key).to_string())),
                "Sponsor missing key '{}'",
                key
            );
        }
    }

    #[test]
    fn test_real_faqs() {
        let dir = data_dir();
        let tree = load_data(&dir).unwrap();

        let faqs = &tree["faqs"];
        let mapping = faqs.as_mapping().expect("faqs should be a mapping");
        assert_eq!(
            mapping.len(),
            9,
            "Expected 9 FAQ files, got {}",
            mapping.len()
        );

        // Check a specific FAQ file
        let de_zoomcamp = mapping
            .get(serde_yaml::Value::String(
                "data-engineering-zoomcamp".into(),
            ))
            .expect("faqs should contain 'data-engineering-zoomcamp'");
        let faq_seq = de_zoomcamp
            .as_sequence()
            .expect("FAQ file should be a sequence");
        assert!(!faq_seq.is_empty());

        // Each FAQ entry should have question and answer
        let first_faq = faq_seq[0]
            .as_mapping()
            .expect("FAQ entry should be a mapping");
        assert!(first_faq.contains_key(serde_yaml::Value::String("question".into())));
        assert!(first_faq.contains_key(serde_yaml::Value::String("answer".into())));
    }

    // ========================================================================
    // Issue 43: Duplicate keys in data files
    // ========================================================================

    #[test]
    fn test_data_file_duplicate_keys_last_wins() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.yaml");
        fs::write(&file, "name: first\nvalue: 42\nname: second\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        let value = &tree["test"];
        let mapping = value.as_mapping().unwrap();
        assert_eq!(
            mapping
                .get(serde_yaml::Value::String("name".into()))
                .and_then(|v| v.as_str()),
            Some("second")
        );
    }

    // ========================================================================
    // Issue 122: Data listing order is deterministic and sorted
    // ========================================================================

    #[test]
    fn test_data_tree_keys_sorted_alphabetically() {
        let dir = TempDir::new().unwrap();
        // Create files in non-alphabetical order
        fs::write(dir.path().join("zebra.yaml"), "value: 3\n").unwrap();
        fs::write(dir.path().join("alpha.yml"), "value: 1\n").unwrap();
        fs::write(dir.path().join("middle.yaml"), "value: 2\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        let keys: Vec<&String> = tree.keys().collect();
        assert_eq!(
            keys,
            vec!["alpha", "middle", "zebra"],
            "DataTree keys should be in sorted order"
        );
    }

    #[test]
    fn test_subdirectory_keys_sorted_alphabetically() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("faqs");
        fs::create_dir(&sub).unwrap();
        // Create files in non-alphabetical order
        fs::write(sub.join("zebra-faq.yml"), "- q: z\n").unwrap();
        fs::write(sub.join("alpha-faq.yml"), "- q: a\n").unwrap();
        fs::write(sub.join("middle-faq.yml"), "- q: m\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        let faqs = tree["faqs"].as_mapping().unwrap();
        let keys: Vec<&str> = faqs.keys().filter_map(|k| k.as_str()).collect();
        assert_eq!(
            keys,
            vec!["alpha-faq", "middle-faq", "zebra-faq"],
            "Subdirectory keys should be in sorted order"
        );
    }

    #[test]
    fn test_data_order_deterministic_across_loads() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("c.yaml"), "value: 3\n").unwrap();
        fs::write(dir.path().join("a.yaml"), "value: 1\n").unwrap();
        fs::write(dir.path().join("b.yaml"), "value: 2\n").unwrap();

        // Load multiple times and verify order is always the same
        for _ in 0..5 {
            let tree = load_data(dir.path()).unwrap();
            let keys: Vec<&String> = tree.keys().collect();
            assert_eq!(
                keys,
                vec!["a", "b", "c"],
                "DataTree keys must be deterministically sorted"
            );
        }
    }

    #[test]
    fn test_data_file_duplicate_nested_keys_last_wins() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nested.yml");
        fs::write(&file, "config:\n  mode: debug\n  mode: release\n").unwrap();

        let tree = load_data(dir.path()).unwrap();
        let value = &tree["nested"];
        let config = value
            .as_mapping()
            .unwrap()
            .get(serde_yaml::Value::String("config".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            config
                .get(serde_yaml::Value::String("mode".into()))
                .and_then(|v| v.as_str()),
            Some("release")
        );
    }

    // ========================================================================
    // Issue 196: JSON data file support
    // ========================================================================

    #[test]
    fn test_load_json_array_data_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("items.json");
        fs::write(
            &file,
            r#"[{"id": 1, "name": "first"}, {"id": 2, "name": "second"}]"#,
        )
        .unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(
            tree.contains_key("items"),
            "Expected key 'items' in {:?}",
            tree
        );

        let items = tree["items"]
            .as_sequence()
            .expect("JSON array should become a YAML sequence");
        assert_eq!(items.len(), 2);

        let first = items[0].as_mapping().unwrap();
        assert_eq!(
            first
                .get(serde_yaml::Value::String("name".into()))
                .and_then(|v| v.as_str()),
            Some("first")
        );
    }

    #[test]
    fn test_load_json_object_data_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("config.json");
        fs::write(&file, r#"{"title": "Test Site", "version": 42}"#).unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("config"));

        let config = tree["config"].as_mapping().unwrap();
        assert_eq!(
            config
                .get(serde_yaml::Value::String("title".into()))
                .and_then(|v| v.as_str()),
            Some("Test Site")
        );
    }

    #[test]
    fn test_load_json_with_unicode_content() {
        // JSON data files with non-ASCII content (Arabic, Cyrillic, CJK)
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("translations.json");
        fs::write(
            &file,
            r#"[
                {"lang": "ar", "text": "\u0645\u0631\u062d\u0628\u0627"},
                {"lang": "ru", "text": "\u041f\u0440\u0438\u0432\u0435\u0442"},
                {"lang": "zh", "text": "\u4f60\u597d"}
            ]"#,
        )
        .unwrap();

        let tree = load_data(dir.path()).unwrap();
        let items = tree["translations"].as_sequence().unwrap();
        assert_eq!(items.len(), 3);

        let arabic = items[0].as_mapping().unwrap();
        let text = arabic
            .get(serde_yaml::Value::String("text".into()))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(text, "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}");

        let russian = items[1].as_mapping().unwrap();
        let text = russian
            .get(serde_yaml::Value::String("text".into()))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(text, "\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}");
    }

    #[test]
    fn test_json_and_yaml_coexist() {
        // Both JSON and YAML files in the same data directory
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("config.yaml"), "name: test\n").unwrap();
        fs::write(dir.path().join("items.json"), r#"[{"id": 1}]"#).unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("config"), "YAML file should be loaded");
        assert!(tree.contains_key("items"), "JSON file should be loaded");
    }

    #[test]
    fn test_json_in_subdirectory() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("api");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("endpoints.json"), r#"["/users", "/posts"]"#).unwrap();

        let tree = load_data(dir.path()).unwrap();
        assert!(tree.contains_key("api"));
        let api = tree["api"].as_mapping().unwrap();
        assert!(api.contains_key(serde_yaml::Value::String("endpoints".into())));
    }

    #[test]
    fn test_json_with_nested_objects_and_arrays() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("photos.json");
        fs::write(
            &file,
            r#"[{"id": "abc", "meta": {"ratio": 1.5, "alt": "a photo"}, "variants": ["url1", "url2"]}]"#,
        )
        .unwrap();

        let tree = load_data(dir.path()).unwrap();
        let photos = tree["photos"].as_sequence().unwrap();
        assert_eq!(photos.len(), 1);

        let photo = photos[0].as_mapping().unwrap();
        let meta = photo
            .get(serde_yaml::Value::String("meta".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        let ratio = meta.get(serde_yaml::Value::String("ratio".into())).unwrap();
        // 1.5 should be a float
        assert!(
            ratio.as_f64().is_some(),
            "ratio should be a number: {:?}",
            ratio
        );

        let variants = photo
            .get(serde_yaml::Value::String("variants".into()))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn test_invalid_json_includes_filename() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.json");
        fs::write(&file, "{ invalid json }").unwrap();

        let err = load_data(dir.path()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("bad.json"),
            "Error should include filename, got: {}",
            msg
        );
    }

    #[test]
    fn test_issue543_resolves_versions_current_auto_from_gemfile_lock() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("_data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("versions.yml"), "current: auto\n").unwrap();
        fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  specs:\n    jekyll-vitepress-theme (1.1.1)\n\nDEPENDENCIES\n  jekyll-vitepress-theme\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_yaml_str("theme: jekyll-vitepress-theme\n")
            .unwrap();
        let tree = load_data_with_config(&data_dir, dir.path(), &config).unwrap();

        let versions = tree["versions"].as_mapping().unwrap();
        assert_eq!(
            versions
                .get(serde_yaml::Value::String("current".into()))
                .and_then(|v| v.as_str()),
            Some("v1.1.1")
        );
    }

    #[test]
    fn test_issue543_auto_is_case_insensitive() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("_data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("versions.yml"), "current: AUTO\n").unwrap();
        fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  specs:\n    jekyll-vitepress-theme (2.0.0)\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_yaml_str("theme: jekyll-vitepress-theme\n")
            .unwrap();
        let tree = load_data_with_config(&data_dir, dir.path(), &config).unwrap();

        let versions = tree["versions"].as_mapping().unwrap();
        assert_eq!(
            versions
                .get(serde_yaml::Value::String("current".into()))
                .and_then(|v| v.as_str()),
            Some("v2.0.0")
        );
    }

    #[test]
    fn test_issue543_missing_gemfile_lock_leaves_auto_unchanged() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("_data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("versions.yml"), "current: auto\n").unwrap();

        let config = crate::config::SiteConfig::from_yaml_str("theme: jekyll-vitepress-theme\n")
            .unwrap();
        let tree = load_data_with_config(&data_dir, dir.path(), &config).unwrap();

        let versions = tree["versions"].as_mapping().unwrap();
        assert_eq!(
            versions
                .get(serde_yaml::Value::String("current".into()))
                .and_then(|v| v.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn test_issue543_missing_theme_gem_leaves_auto_unchanged() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("_data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("versions.yml"), "current: auto\n").unwrap();
        fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  specs:\n    some-other-theme (0.5.2)\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_yaml_str("theme: jekyll-vitepress-theme\n")
            .unwrap();
        let tree = load_data_with_config(&data_dir, dir.path(), &config).unwrap();

        let versions = tree["versions"].as_mapping().unwrap();
        assert_eq!(
            versions
                .get(serde_yaml::Value::String("current".into()))
                .and_then(|v| v.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn test_issue543_non_auto_current_is_unchanged() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("_data");
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("versions.yml"), "current: v9.9.9\n").unwrap();
        fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  specs:\n    jekyll-vitepress-theme (1.1.1)\n",
        )
        .unwrap();

        let config = crate::config::SiteConfig::from_yaml_str("theme: jekyll-vitepress-theme\n")
            .unwrap();
        let tree = load_data_with_config(&data_dir, dir.path(), &config).unwrap();

        let versions = tree["versions"].as_mapping().unwrap();
        assert_eq!(
            versions
                .get(serde_yaml::Value::String("current".into()))
                .and_then(|v| v.as_str()),
            Some("v9.9.9")
        );
    }

    #[test]
    fn test_issue543_extracts_theme_version_from_specs_section() {
        let lock = "GEM\n  specs:\n    jekyll-vitepress-theme (1.1.1)\n    other-gem (0.1.0)\n\nDEPENDENCIES\n  jekyll-vitepress-theme\n";
        assert_eq!(
            extract_theme_version_from_lock_content(lock, "jekyll-vitepress-theme"),
            Some("1.1.1".to_string())
        );
    }

    #[test]
    fn test_issue543_extract_theme_version_returns_none_for_empty_lock() {
        assert_eq!(
            extract_theme_version_from_lock_content("", "jekyll-vitepress-theme"),
            None
        );
    }
}

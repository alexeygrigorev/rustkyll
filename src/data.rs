use std::collections::HashMap;
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
}

/// A data tree mapping keys (derived from filenames) to parsed YAML values.
pub type DataTree = HashMap<String, serde_yaml::Value>;

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
    if !data_dir.exists() {
        return Err(DataError::DirectoryNotFound(data_dir.display().to_string()));
    }

    let mut tree = DataTree::new();

    let entries = fs::read_dir(data_dir).map_err(|e| DataError::ReadDir {
        path: data_dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| DataError::ReadDir {
            path: data_dir.display().to_string(),
            source: e,
        })?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };
            let sub_tree = load_directory_files(&path)?;
            // Convert the sub-tree HashMap into a serde_yaml::Value::Mapping
            let mut mapping = serde_yaml::Mapping::new();
            for (key, value) in sub_tree {
                mapping.insert(serde_yaml::Value::String(key), value);
            }
            tree.insert(dir_name, serde_yaml::Value::Mapping(mapping));
        } else if is_yaml_file(&path) {
            let key = file_stem(&path);
            if let Some(key) = key {
                let value = load_yaml_file(&path)?;
                tree.insert(key, value);
            }
        }
    }

    Ok(tree)
}

/// Load all YAML files in a single directory (non-recursive) into a HashMap.
fn load_directory_files(dir: &Path) -> Result<HashMap<String, serde_yaml::Value>, DataError> {
    let mut map = HashMap::new();

    let entries = fs::read_dir(dir).map_err(|e| DataError::ReadDir {
        path: dir.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| DataError::ReadDir {
            path: dir.display().to_string(),
            source: e,
        })?;
        let path = entry.path();

        if path.is_file() && is_yaml_file(&path) {
            if let Some(key) = file_stem(&path) {
                let value = load_yaml_file(&path)?;
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

/// Extract the file stem (filename without extension) as a String.
fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
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
    use std::fs;
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
            .join("datatalksclub.github.io")
            .join("_data")
    }

    #[test]
    fn test_load_real_data_succeeds() {
        let dir = data_dir();
        if !dir.exists() {
            eprintln!("Skipping integration test: {:?} not found", dir);
            return;
        }
        let tree = load_data(&dir).unwrap();
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_real_events() {
        let dir = data_dir();
        if !dir.exists() {
            return;
        }
        let tree = load_data(&dir).unwrap();

        let events = &tree["events"];
        let seq = events.as_sequence().expect("events should be a sequence");
        assert!(seq.len() > 100, "Expected > 100 events, got {}", seq.len());

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
        if !dir.exists() {
            return;
        }
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
        if !dir.exists() {
            return;
        }
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
        if !dir.exists() {
            return;
        }
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
        if !dir.exists() {
            return;
        }
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
        if !dir.exists() {
            return;
        }
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
}

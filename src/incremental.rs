//! Incremental build support.
//!
//! Tracks file modification times across builds using a manifest file.
//! On subsequent builds, only files whose source has changed since the
//! last build are regenerated. If global files (config, layouts, includes)
//! have changed, a full rebuild is triggered.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Name of the manifest file stored in the destination directory.
const MANIFEST_FILENAME: &str = ".rustkyll-manifest.json";

/// The build manifest tracks file modification times from the previous build.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BuildManifest {
    /// Source file path (relative to source dir) -> modification timestamp (seconds since epoch).
    pub source_files: HashMap<String, u64>,

    /// Source file path (relative) -> output file path (relative to destination dir).
    pub output_map: HashMap<String, String>,

    /// Global file path (relative) -> modification timestamp.
    /// These are config, layout, and include files. If any change, full rebuild is needed.
    pub global_files: HashMap<String, u64>,
}

/// Result of comparing current file state against the manifest.
#[derive(Debug, PartialEq)]
pub enum IncrementalAction {
    /// No changes detected; skip all rebuilds.
    SkipAll,
    /// Global files changed; must do a full rebuild.
    FullRebuild,
    /// Only some source files changed; rebuild only these (relative paths).
    RebuildPartial(Vec<String>),
}

/// Load the build manifest from the destination directory.
/// Returns `None` if the manifest does not exist or cannot be parsed.
pub fn load_manifest(destination: &Path) -> Option<BuildManifest> {
    let manifest_path = destination.join(MANIFEST_FILENAME);
    let content = fs::read_to_string(&manifest_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save the build manifest to the destination directory.
pub fn save_manifest(destination: &Path, manifest: &BuildManifest) -> Result<(), std::io::Error> {
    let manifest_path = destination.join(MANIFEST_FILENAME);
    let content = serde_json::to_string_pretty(manifest).map_err(std::io::Error::other)?;
    fs::write(&manifest_path, content)
}

/// Get the modification time of a file as seconds since the Unix epoch.
/// Returns `None` if the file does not exist or metadata is unavailable.
pub fn file_mtime(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Collect modification times for global files (config, layouts, includes).
///
/// Returns a map of relative path -> mtime for all files found under:
/// - `_config.yml`
/// - `_layouts/` directory
/// - `_includes/` directory
pub fn collect_global_files(source: &Path) -> HashMap<String, u64> {
    collect_global_files_with_includes_dir(source, "_includes")
}

/// Like `collect_global_files` but also tracks a custom includes directory.
pub fn collect_global_files_with_includes_dir(
    source: &Path,
    includes_dir: &str,
) -> HashMap<String, u64> {
    let mut globals = HashMap::new();

    // Config file
    let config_path = source.join("_config.yml");
    if let Some(mtime) = file_mtime(&config_path) {
        globals.insert("_config.yml".to_string(), mtime);
    }

    // Layout files
    collect_dir_mtimes(source, &source.join("_layouts"), &mut globals);

    // Include files (default _includes)
    collect_dir_mtimes(source, &source.join("_includes"), &mut globals);

    // Custom includes dir (if different from default)
    if includes_dir != "_includes" {
        collect_dir_mtimes(source, &source.join(includes_dir), &mut globals);
    }

    globals
}

/// Recursively collect modification times for all files under `dir`.
fn collect_dir_mtimes(source: &Path, dir: &Path, out: &mut HashMap<String, u64>) {
    if !dir.exists() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_dir_mtimes(source, &path, out);
        } else if let Some(mtime) = file_mtime(&path) {
            if let Ok(rel) = path.strip_prefix(source) {
                globals_insert(out, rel, mtime);
            }
        }
    }
}

fn globals_insert(out: &mut HashMap<String, u64>, rel: &Path, mtime: u64) {
    // Normalize to forward slashes for cross-platform consistency
    let key = rel.to_string_lossy().replace('\\', "/");
    out.insert(key, mtime);
}

/// Collect modification times for source content files (collections, pages).
///
/// `source_paths` should be relative paths from the source directory.
pub fn collect_source_files(source: &Path, source_paths: &[PathBuf]) -> HashMap<String, u64> {
    let mut files = HashMap::new();
    for rel_path in source_paths {
        let abs_path = source.join(rel_path);
        if let Some(mtime) = file_mtime(&abs_path) {
            let key = rel_path.to_string_lossy().replace('\\', "/");
            files.insert(key, mtime);
        }
    }
    files
}

/// Determine what needs to be rebuilt by comparing current state against the manifest.
pub fn determine_action(
    prev_manifest: &BuildManifest,
    current_globals: &HashMap<String, u64>,
    current_sources: &HashMap<String, u64>,
) -> IncrementalAction {
    // Check if any global files changed
    if globals_changed(&prev_manifest.global_files, current_globals) {
        return IncrementalAction::FullRebuild;
    }

    // Check which source files changed
    let changed: Vec<String> = current_sources
        .iter()
        .filter(|(path, mtime)| {
            match prev_manifest.source_files.get(path.as_str()) {
                Some(prev_mtime) => **mtime != *prev_mtime,
                None => true, // new file
            }
        })
        .map(|(path, _)| path.clone())
        .collect();

    // Also check for deleted files (in manifest but not in current sources)
    let deleted: Vec<String> = prev_manifest
        .source_files
        .keys()
        .filter(|path| !current_sources.contains_key(path.as_str()))
        .cloned()
        .collect();

    if changed.is_empty() && deleted.is_empty() {
        IncrementalAction::SkipAll
    } else {
        let mut all_changed = changed;
        all_changed.extend(deleted);
        IncrementalAction::RebuildPartial(all_changed)
    }
}

/// Check if global files have changed between the previous and current state.
fn globals_changed(prev: &HashMap<String, u64>, current: &HashMap<String, u64>) -> bool {
    if prev.len() != current.len() {
        return true;
    }
    for (path, mtime) in current {
        match prev.get(path) {
            Some(prev_mtime) if prev_mtime == mtime => {}
            _ => return true,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_round_trip() {
        let dir = TempDir::new().unwrap();
        let mut manifest = BuildManifest::default();
        manifest
            .source_files
            .insert("_posts/hello.md".to_string(), 1000);
        manifest.output_map.insert(
            "_posts/hello.md".to_string(),
            "posts/hello.html".to_string(),
        );
        manifest.global_files.insert("_config.yml".to_string(), 500);

        save_manifest(dir.path(), &manifest).unwrap();
        let loaded = load_manifest(dir.path()).unwrap();
        assert_eq!(manifest, loaded);
    }

    #[test]
    fn test_load_manifest_missing_file() {
        let dir = TempDir::new().unwrap();
        assert!(load_manifest(dir.path()).is_none());
    }

    #[test]
    fn test_load_manifest_invalid_json() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(MANIFEST_FILENAME), "not json").unwrap();
        assert!(load_manifest(dir.path()).is_none());
    }

    #[test]
    fn test_file_mtime_existing_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();
        let mtime = file_mtime(&file_path);
        assert!(mtime.is_some());
        assert!(mtime.unwrap() > 0);
    }

    #[test]
    fn test_file_mtime_missing_file() {
        assert!(file_mtime(Path::new("/nonexistent/file.txt")).is_none());
    }

    #[test]
    fn test_collect_global_files() {
        let dir = TempDir::new().unwrap();

        // Create config
        fs::write(dir.path().join("_config.yml"), "title: test").unwrap();

        // Create layouts dir with a file
        let layouts = dir.path().join("_layouts");
        fs::create_dir(&layouts).unwrap();
        fs::write(layouts.join("default.html"), "<html></html>").unwrap();

        // Create includes dir with a file
        let includes = dir.path().join("_includes");
        fs::create_dir(&includes).unwrap();
        fs::write(includes.join("header.html"), "<header></header>").unwrap();

        let globals = collect_global_files(dir.path());
        assert!(globals.contains_key("_config.yml"));
        assert!(globals.contains_key("_layouts/default.html"));
        assert!(globals.contains_key("_includes/header.html"));
        assert_eq!(globals.len(), 3);
    }

    #[test]
    fn test_collect_source_files() {
        let dir = TempDir::new().unwrap();

        let posts_dir = dir.path().join("_posts");
        fs::create_dir(&posts_dir).unwrap();
        fs::write(posts_dir.join("hello.md"), "# Hello").unwrap();

        let sources = collect_source_files(dir.path(), &[PathBuf::from("_posts/hello.md")]);
        assert!(sources.contains_key("_posts/hello.md"));
        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn test_determine_action_skip_all() {
        let globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);

        let manifest = BuildManifest {
            source_files: sources.clone(),
            output_map: HashMap::new(),
            global_files: globals.clone(),
        };

        let action = determine_action(&manifest, &globals, &sources);
        assert_eq!(action, IncrementalAction::SkipAll);
    }

    #[test]
    fn test_determine_action_full_rebuild_config_changed() {
        let prev_globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let curr_globals = HashMap::from([("_config.yml".to_string(), 200u64)]);
        let sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);

        let manifest = BuildManifest {
            source_files: sources.clone(),
            output_map: HashMap::new(),
            global_files: prev_globals,
        };

        let action = determine_action(&manifest, &curr_globals, &sources);
        assert_eq!(action, IncrementalAction::FullRebuild);
    }

    #[test]
    fn test_determine_action_full_rebuild_layout_changed() {
        let prev_globals = HashMap::from([
            ("_config.yml".to_string(), 100u64),
            ("_layouts/default.html".to_string(), 100u64),
        ]);
        let curr_globals = HashMap::from([
            ("_config.yml".to_string(), 100u64),
            ("_layouts/default.html".to_string(), 300u64),
        ]);
        let sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);

        let manifest = BuildManifest {
            source_files: sources.clone(),
            output_map: HashMap::new(),
            global_files: prev_globals,
        };

        let action = determine_action(&manifest, &curr_globals, &sources);
        assert_eq!(action, IncrementalAction::FullRebuild);
    }

    #[test]
    fn test_determine_action_full_rebuild_new_global_file() {
        let prev_globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let curr_globals = HashMap::from([
            ("_config.yml".to_string(), 100u64),
            ("_layouts/new.html".to_string(), 300u64),
        ]);
        let sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);

        let manifest = BuildManifest {
            source_files: sources.clone(),
            output_map: HashMap::new(),
            global_files: prev_globals,
        };

        let action = determine_action(&manifest, &curr_globals, &sources);
        assert_eq!(action, IncrementalAction::FullRebuild);
    }

    #[test]
    fn test_determine_action_partial_rebuild_source_changed() {
        let globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let prev_sources = HashMap::from([
            ("_posts/a.md".to_string(), 200u64),
            ("_posts/b.md".to_string(), 200u64),
        ]);
        let curr_sources = HashMap::from([
            ("_posts/a.md".to_string(), 300u64), // changed
            ("_posts/b.md".to_string(), 200u64), // unchanged
        ]);

        let manifest = BuildManifest {
            source_files: prev_sources,
            output_map: HashMap::new(),
            global_files: globals.clone(),
        };

        let action = determine_action(&manifest, &globals, &curr_sources);
        match action {
            IncrementalAction::RebuildPartial(changed) => {
                assert_eq!(changed.len(), 1);
                assert!(changed.contains(&"_posts/a.md".to_string()));
            }
            other => panic!("Expected RebuildPartial, got {:?}", other),
        }
    }

    #[test]
    fn test_determine_action_partial_rebuild_new_source() {
        let globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let prev_sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);
        let curr_sources = HashMap::from([
            ("_posts/a.md".to_string(), 200u64),
            ("_posts/new.md".to_string(), 300u64), // new file
        ]);

        let manifest = BuildManifest {
            source_files: prev_sources,
            output_map: HashMap::new(),
            global_files: globals.clone(),
        };

        let action = determine_action(&manifest, &globals, &curr_sources);
        match action {
            IncrementalAction::RebuildPartial(changed) => {
                assert_eq!(changed.len(), 1);
                assert!(changed.contains(&"_posts/new.md".to_string()));
            }
            other => panic!("Expected RebuildPartial, got {:?}", other),
        }
    }

    #[test]
    fn test_determine_action_partial_rebuild_deleted_source() {
        let globals = HashMap::from([("_config.yml".to_string(), 100u64)]);
        let prev_sources = HashMap::from([
            ("_posts/a.md".to_string(), 200u64),
            ("_posts/deleted.md".to_string(), 200u64),
        ]);
        let curr_sources = HashMap::from([("_posts/a.md".to_string(), 200u64)]);

        let manifest = BuildManifest {
            source_files: prev_sources,
            output_map: HashMap::new(),
            global_files: globals.clone(),
        };

        let action = determine_action(&manifest, &globals, &curr_sources);
        match action {
            IncrementalAction::RebuildPartial(changed) => {
                assert_eq!(changed.len(), 1);
                assert!(changed.contains(&"_posts/deleted.md".to_string()));
            }
            other => panic!("Expected RebuildPartial, got {:?}", other),
        }
    }

    #[test]
    fn test_first_build_creates_manifest() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("_site");
        fs::create_dir(&dest).unwrap();

        // No manifest exists initially
        assert!(load_manifest(&dest).is_none());

        // Save a manifest (simulating first build)
        let manifest = BuildManifest {
            source_files: HashMap::from([("_posts/hello.md".to_string(), 100)]),
            output_map: HashMap::from([(
                "_posts/hello.md".to_string(),
                "posts/hello.html".to_string(),
            )]),
            global_files: HashMap::from([("_config.yml".to_string(), 50)]),
        };
        save_manifest(&dest, &manifest).unwrap();

        // Manifest now exists
        let loaded = load_manifest(&dest).unwrap();
        assert_eq!(loaded.source_files.len(), 1);
        assert_eq!(loaded.global_files.len(), 1);
    }
}

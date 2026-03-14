use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rayon::prelude::*;

use crate::config::SiteConfig;

/// Errors that can occur when copying static files.
#[derive(Debug, thiserror::Error)]
pub enum StaticFileError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to copy file from {src} to {dst}: {source}")]
    CopyFile {
        src: String,
        dst: String,
        source: std::io::Error,
    },
}

/// Determine if a given relative path should be copied as a static file.
///
/// A file is static if it is NOT:
/// - In a `_` prefixed directory or named with a `_` prefix at root level
/// - In a `.` prefixed directory or a dotfile
/// - On the exclude list from `_config.yml`
/// - A `.md` file (those are pages, processed separately)
/// - `_config.yml` itself
pub fn is_static_file(path: &Path, config: &SiteConfig) -> bool {
    let path_str = path.to_string_lossy();

    // Reject _config.yml
    if path_str == "_config.yml" {
        return false;
    }

    // Reject .md files
    if let Some(ext) = path.extension() {
        if ext.eq_ignore_ascii_case("md") {
            return false;
        }
    }

    // Check each component of the path
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            // Reject paths with underscore-prefixed components
            if name_str.starts_with('_') {
                return false;
            }
            // Reject paths with dot-prefixed components
            if name_str.starts_with('.') {
                return false;
            }
        }
    }

    // Check exclude list
    for excluded in &config.exclude {
        let excluded_trimmed = excluded.trim_end_matches('/');
        // Check if the path itself matches the excluded entry
        if path_str == excluded_trimmed {
            return false;
        }
        // Check if the path starts with the excluded directory
        if path_str.starts_with(&format!("{}/", excluded_trimmed)) {
            return false;
        }
        // Also check the raw exclude entry with trailing slash
        if excluded.ends_with('/') && path_str.starts_with(excluded.as_str()) {
            return false;
        }
    }

    true
}

/// Check if a file starts with YAML front matter delimiters (`---`).
///
/// Only checks text-based file extensions that Jekyll would process
/// (`.xml`, `.html`, `.htm`, `.json`, `.txt`). Binary files and other
/// extensions are never checked and always return `false`.
///
/// Returns `false` if the file cannot be read or is not a text-based type.
fn file_has_front_matter(path: &Path) -> bool {
    // Only check extensions that might contain front matter
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "xml" | "html" | "htm" | "json" | "txt" | "scss" | "css" => {}
        _ => return false,
    }

    // Read just the first few bytes to check for `---\n`
    let mut buf = [0u8; 8];
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    use std::io::Read;
    let mut reader = std::io::BufReader::new(file);
    let n = match reader.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return false,
    };
    let start = std::str::from_utf8(&buf[..n]).unwrap_or("");
    // Check for BOM + --- or just ---
    let trimmed = start.trim_start_matches('\u{feff}');
    trimmed.starts_with("---")
}

/// Walk the source directory recursively and return all static file paths
/// (relative to the source directory).
///
/// # Errors
///
/// Returns `StaticFileError::ReadDir` if a directory cannot be read.
pub fn collect_static_files(
    source_dir: &Path,
    config: &SiteConfig,
) -> Result<Vec<PathBuf>, StaticFileError> {
    let mut files = Vec::new();
    collect_recursive(source_dir, source_dir, config, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_recursive(
    base: &Path,
    current: &Path,
    config: &SiteConfig,
    files: &mut Vec<PathBuf>,
) -> Result<(), StaticFileError> {
    let entries = fs::read_dir(current).map_err(|e| StaticFileError::ReadDir {
        path: current.display().to_string(),
        source: e,
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| StaticFileError::ReadDir {
            path: current.display().to_string(),
            source: e,
        })?;

        let full_path = entry.path();
        let relative = full_path
            .strip_prefix(base)
            .expect("path should be under base directory");

        if full_path.is_dir() {
            // Skip directories that start with _ or .
            let dir_name = entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();
            if dir_name_str.starts_with('_') || dir_name_str.starts_with('.') {
                continue;
            }
            // Skip excluded directories
            let rel_str = relative.to_string_lossy();
            let mut skip = false;
            for excluded in &config.exclude {
                let excluded_trimmed = excluded.trim_end_matches('/');
                if rel_str == excluded_trimmed {
                    skip = true;
                    break;
                }
            }
            if skip {
                continue;
            }
            collect_recursive(base, &full_path, config, files)?;
        } else if is_static_file(relative, config) {
            // Skip text-based files that have YAML front matter (they are
            // processed as pages by the template engine, not copied as static).
            // This matches Jekyll's behavior: any file starting with `---`
            // is treated as a processable file, not a static asset.
            if file_has_front_matter(&full_path) {
                continue;
            }
            files.push(relative.to_path_buf());
        }
    }

    Ok(())
}

/// Copy all static files from the source directory to the output directory,
/// preserving directory structure.
///
/// Returns the number of files copied.
///
/// # Errors
///
/// Returns `StaticFileError` if a directory cannot be read, created, or a file
/// cannot be copied.
pub fn copy_static_files(
    source_dir: &Path,
    output_dir: &Path,
    config: &SiteConfig,
) -> Result<usize, StaticFileError> {
    let files = collect_static_files(source_dir, config)?;
    let count = files.len();

    // Pre-create all necessary parent directories before parallel copy.
    // collect all unique parent dirs and create them sequentially (safe, fast).
    let mut parent_dirs: HashSet<PathBuf> = HashSet::new();
    for relative_path in &files {
        let dst = output_dir.join(relative_path);
        if let Some(parent) = dst.parent() {
            parent_dirs.insert(parent.to_path_buf());
        }
    }
    for dir in &parent_dirs {
        fs::create_dir_all(dir).map_err(|e| StaticFileError::CreateDir {
            path: dir.display().to_string(),
            source: e,
        })?;
    }

    // Copy files in parallel using rayon
    let errors: Mutex<Vec<StaticFileError>> = Mutex::new(Vec::new());
    files.par_iter().for_each(|relative_path| {
        let src = source_dir.join(relative_path);
        let dst = output_dir.join(relative_path);
        if let Err(e) = fs::copy(&src, &dst) {
            errors.lock().unwrap().push(StaticFileError::CopyFile {
                src: src.display().to_string(),
                dst: dst.display().to_string(),
                source: e,
            });
        }
    });

    let errors = errors.into_inner().unwrap();
    if let Some(err) = errors.into_iter().next() {
        return Err(err);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_config() -> SiteConfig {
        SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            ..Default::default()
        }
    }

    fn config_with_excludes(excludes: Vec<&str>) -> SiteConfig {
        let mut config = empty_config();
        config.exclude = excludes.into_iter().map(String::from).collect();
        config
    }

    // ========================================================================
    // Unit: is_static_file
    // ========================================================================

    #[test]
    fn test_static_file_css() {
        assert!(is_static_file(
            Path::new("assets/styles.css"),
            &empty_config()
        ));
    }

    #[test]
    fn test_static_file_image() {
        assert!(is_static_file(
            Path::new("images/cover.jpg"),
            &empty_config()
        ));
    }

    #[test]
    fn test_static_file_cname() {
        assert!(is_static_file(Path::new("CNAME"), &empty_config()));
    }

    #[test]
    fn test_static_file_robots_txt() {
        assert!(is_static_file(Path::new("robots.txt"), &empty_config()));
    }

    #[test]
    fn test_static_file_favicon() {
        assert!(is_static_file(Path::new("favicon.ico"), &empty_config()));
    }

    #[test]
    fn test_static_file_browserconfig_xml() {
        assert!(is_static_file(
            Path::new("browserconfig.xml"),
            &empty_config()
        ));
    }

    #[test]
    fn test_static_file_site_webmanifest() {
        assert!(is_static_file(
            Path::new("site.webmanifest"),
            &empty_config()
        ));
    }

    #[test]
    fn test_static_file_podcast_timestamps() {
        assert!(is_static_file(
            Path::new("podcast-timestamps/s01e03.txt"),
            &empty_config()
        ));
    }

    #[test]
    fn test_not_static_underscore_dir() {
        assert!(!is_static_file(
            Path::new("_layouts/default.html"),
            &empty_config()
        ));
    }

    #[test]
    fn test_not_static_config_yml() {
        assert!(!is_static_file(Path::new("_config.yml"), &empty_config()));
    }

    #[test]
    fn test_not_static_dotfile() {
        assert!(!is_static_file(Path::new(".gitignore"), &empty_config()));
    }

    #[test]
    fn test_not_static_dot_dir() {
        assert!(!is_static_file(
            Path::new(".github/workflows/ci.yml"),
            &empty_config()
        ));
    }

    #[test]
    fn test_not_static_markdown() {
        assert!(!is_static_file(Path::new("index.md"), &empty_config()));
    }

    #[test]
    fn test_not_static_excluded_file() {
        let config = config_with_excludes(vec!["README.md"]);
        assert!(!is_static_file(Path::new("README.md"), &config));
    }

    #[test]
    fn test_not_static_excluded_dir_with_slash() {
        let config = config_with_excludes(vec!["scripts/"]);
        assert!(!is_static_file(Path::new("scripts/deploy.sh"), &config));
    }

    #[test]
    fn test_not_static_excluded_dir_node_modules() {
        let config = config_with_excludes(vec!["node_modules/"]);
        assert!(!is_static_file(
            Path::new("node_modules/foo/bar.js"),
            &config
        ));
    }

    #[test]
    fn test_not_static_excluded_gemfile() {
        let config = config_with_excludes(vec!["Gemfile"]);
        assert!(!is_static_file(Path::new("Gemfile"), &config));
    }

    // ========================================================================
    // Unit: collect_static_files
    // ========================================================================

    #[test]
    fn test_collect_mixed_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Static files
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets/styles.css"), "body{}").unwrap();
        fs::create_dir_all(root.join("images/books")).unwrap();
        fs::write(root.join("images/books/foo.jpg"), [0xFF, 0xD8]).unwrap();
        fs::write(root.join("CNAME"), "example.com").unwrap();
        fs::write(root.join("robots.txt"), "User-agent: *").unwrap();

        // Non-static files
        fs::create_dir_all(root.join("_layouts")).unwrap();
        fs::write(root.join("_layouts/default.html"), "<html>").unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/config"), "[core]").unwrap();
        fs::write(root.join("index.md"), "# Hello").unwrap();
        fs::write(root.join("_config.yml"), "url: test").unwrap();

        let config = empty_config();
        let files = collect_static_files(root, &config).unwrap();

        assert_eq!(files.len(), 4);
        assert!(files.contains(&PathBuf::from("assets/styles.css")));
        assert!(files.contains(&PathBuf::from("images/books/foo.jpg")));
        assert!(files.contains(&PathBuf::from("CNAME")));
        assert!(files.contains(&PathBuf::from("robots.txt")));
    }

    #[test]
    fn test_collect_subdirectories_traversed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::create_dir_all(root.join("images/books")).unwrap();
        fs::write(root.join("images/books/foo.jpg"), "img").unwrap();
        fs::create_dir_all(root.join("images/posts")).unwrap();
        fs::write(root.join("images/posts/bar.png"), "img").unwrap();

        let config = empty_config();
        let files = collect_static_files(root, &config).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("images/books/foo.jpg")));
        assert!(files.contains(&PathBuf::from("images/posts/bar.png")));
    }

    #[test]
    fn test_collect_respects_exclude_list() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        fs::write(root.join("CNAME"), "example.com").unwrap();
        fs::write(root.join("Gemfile"), "source ...").unwrap();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/deploy.sh"), "#!/bin/bash").unwrap();

        let config = config_with_excludes(vec!["Gemfile", "scripts/"]);
        let files = collect_static_files(root, &config).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files.contains(&PathBuf::from("CNAME")));
    }

    // ========================================================================
    // Integration: copy_static_files
    // ========================================================================

    #[test]
    fn test_copy_static_files_integration() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();
        let src = src_tmp.path();
        let dst = dst_tmp.path();

        // Static files
        fs::create_dir_all(src.join("assets")).unwrap();
        fs::write(src.join("assets/styles.css"), "body { margin: 0; }").unwrap();
        fs::create_dir_all(src.join("images")).unwrap();
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        fs::write(src.join("images/logo.png"), &png_bytes).unwrap();
        fs::write(src.join("CNAME"), "datatalks.club").unwrap();

        // Non-static files
        fs::create_dir_all(src.join("_layouts")).unwrap();
        fs::write(src.join("_layouts/default.html"), "<html>").unwrap();
        fs::write(src.join("index.md"), "# Home").unwrap();
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(src.join(".git/config"), "[core]").unwrap();

        let config = config_with_excludes(vec!["README.md"]);
        fs::write(src.join("README.md"), "# Readme").unwrap();

        let count = copy_static_files(src, dst, &config).unwrap();

        assert_eq!(count, 3);

        // Verify copied files
        assert_eq!(
            fs::read_to_string(dst.join("assets/styles.css")).unwrap(),
            "body { margin: 0; }"
        );
        assert_eq!(fs::read(dst.join("images/logo.png")).unwrap(), png_bytes);
        assert_eq!(
            fs::read_to_string(dst.join("CNAME")).unwrap(),
            "datatalks.club"
        );

        // Verify non-static files were NOT copied
        assert!(!dst.join("_layouts").exists());
        assert!(!dst.join("README.md").exists());
        assert!(!dst.join("index.md").exists());
        assert!(!dst.join(".git").exists());
    }

    // ========================================================================
    // Integration: real site static files
    // ========================================================================

    #[test]
    fn test_real_site_static_files() {
        let site_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io");
        if !site_dir.exists() {
            // Skip if the real site directory is not available
            return;
        }

        let config = SiteConfig::from_file(&site_dir.join("_config.yml")).unwrap();
        let files = collect_static_files(&site_dir, &config).unwrap();

        // Convert to string set for easier checking
        let file_strs: Vec<String> = files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        // Verify expected static files are present
        assert!(
            file_strs.contains(&"assets/styles.css".to_string()),
            "Should contain assets/styles.css"
        );
        assert!(
            file_strs.iter().any(|f| f == "images/cover.jpg"),
            "Should contain images/cover.jpg"
        );
        assert!(
            file_strs.contains(&"CNAME".to_string()),
            "Should contain CNAME"
        );
        assert!(
            file_strs.contains(&"robots.txt".to_string()),
            "Should contain robots.txt"
        );
        assert!(
            file_strs.contains(&"favicon.ico".to_string()),
            "Should contain favicon.ico"
        );
        assert!(
            file_strs.contains(&"site.webmanifest".to_string()),
            "Should contain site.webmanifest"
        );

        // Verify no underscore-prefixed paths
        for f in &file_strs {
            for component in Path::new(f).components() {
                if let std::path::Component::Normal(name) = component {
                    assert!(
                        !name.to_string_lossy().starts_with('_'),
                        "Should not contain underscore-prefixed path: {}",
                        f
                    );
                }
            }
        }

        // Verify no .md files
        for f in &file_strs {
            assert!(!f.ends_with(".md"), "Should not contain .md files: {}", f);
        }

        // Verify no excluded files
        assert!(
            !file_strs.contains(&"Gemfile".to_string()),
            "Should not contain Gemfile"
        );
        assert!(
            !file_strs.contains(&"Makefile".to_string()),
            "Should not contain Makefile"
        );
        assert!(
            !file_strs.iter().any(|f| f.starts_with("scripts/")),
            "Should not contain scripts/"
        );
        assert!(
            !file_strs.iter().any(|f| f.starts_with("node_modules/")),
            "Should not contain node_modules/"
        );
    }

    // ========================================================================
    // Parallel static file copying tests
    // ========================================================================

    #[test]
    fn test_parallel_copy_many_files() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();
        let src = src_tmp.path();
        let dst = dst_tmp.path();

        // Create 150+ files across nested directories
        for i in 0..160 {
            let subdir = format!("dir{}", i % 10);
            let dir_path = src.join(&subdir);
            fs::create_dir_all(&dir_path).unwrap();
            fs::write(
                dir_path.join(format!("file{}.txt", i)),
                format!("content-{}", i),
            )
            .unwrap();
        }

        let config = empty_config();
        let count = copy_static_files(src, dst, &config).unwrap();

        assert_eq!(count, 160);

        // Verify all files were copied with correct content
        for i in 0..160 {
            let subdir = format!("dir{}", i % 10);
            let content = fs::read_to_string(dst.join(&subdir).join(format!("file{}.txt", i)))
                .unwrap_or_else(|_| panic!("file {} should be copied", i));
            assert_eq!(content, format!("content-{}", i));
        }
    }

    #[test]
    fn test_parallel_copy_preserves_nested_dirs() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();
        let src = src_tmp.path();
        let dst = dst_tmp.path();

        // Create deeply nested structure
        fs::create_dir_all(src.join("a/b/c/d")).unwrap();
        fs::write(src.join("a/b/c/d/deep.txt"), "deep content").unwrap();
        fs::create_dir_all(src.join("x/y")).unwrap();
        fs::write(src.join("x/y/shallow.txt"), "shallow content").unwrap();
        fs::write(src.join("root.txt"), "root content").unwrap();

        let config = empty_config();
        let count = copy_static_files(src, dst, &config).unwrap();

        assert_eq!(count, 3);
        assert_eq!(
            fs::read_to_string(dst.join("a/b/c/d/deep.txt")).unwrap(),
            "deep content"
        );
        assert_eq!(
            fs::read_to_string(dst.join("x/y/shallow.txt")).unwrap(),
            "shallow content"
        );
        assert_eq!(
            fs::read_to_string(dst.join("root.txt")).unwrap(),
            "root content"
        );
    }

    #[test]
    fn test_parallel_copy_returns_correct_count() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();
        let src = src_tmp.path();
        let dst = dst_tmp.path();

        for i in 0..50 {
            fs::write(src.join(format!("file{}.css", i)), "body{}").unwrap();
        }
        // Add some non-static files that should not be counted
        fs::write(src.join("readme.md"), "# Hello").unwrap();
        fs::create_dir_all(src.join("_layouts")).unwrap();
        fs::write(src.join("_layouts/default.html"), "<html>").unwrap();

        let config = empty_config();
        let count = copy_static_files(src, dst, &config).unwrap();

        assert_eq!(count, 50);
    }
}

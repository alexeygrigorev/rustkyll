use std::fs;
use std::path::{Path, PathBuf};

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

    for relative_path in &files {
        let src = source_dir.join(relative_path);
        let dst = output_dir.join(relative_path);

        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| StaticFileError::CreateDir {
                    path: parent.display().to_string(),
                    source: e,
                })?;
            }
        }

        fs::copy(&src, &dst).map_err(|e| StaticFileError::CopyFile {
            src: src.display().to_string(),
            dst: dst.display().to_string(),
            source: e,
        })?;
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
}

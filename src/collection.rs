use std::fs;
use std::path::Path;

use crate::config::SiteConfig;
use crate::frontmatter::{self, FrontMatter};

/// Errors that can occur when loading collections.
#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
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

    #[error("failed to parse file {path}: {source}")]
    Parse {
        path: String,
        source: frontmatter::ParseError,
    },
}

/// A single item from a Jekyll collection.
#[derive(Debug, Clone)]
pub struct CollectionItem {
    /// Filename stem (e.g. `alexeygrigorev` from `alexeygrigorev.md`,
    /// `segmentation` from `2020-11-29-segmentation.md` for posts).
    pub slug: String,

    /// Parsed YAML front matter key-value pairs.
    pub front_matter: FrontMatter,

    /// Raw markdown body.
    pub content: String,

    /// Markdown body converted to HTML.
    pub html_content: String,

    /// Content before `<!--more-->` separator, if present.
    pub excerpt: Option<String>,

    /// Generated URL path (e.g. `/people/alexeygrigorev.html`).
    pub url: String,

    /// Extracted date for posts (from `YYYY-MM-DD-title.md` filename or front matter).
    pub date: Option<String>,

    /// Which collection this item belongs to (e.g. `people`, `posts`).
    pub collection_name: String,

    /// Relative path to the source file (e.g. `_posts/2020-11-29-segmentation.md`).
    pub source_path: String,
}

/// Regex-free post filename parsing. Extracts date and slug from `YYYY-MM-DD-title.md`.
///
/// Returns `(Option<date_string>, slug)`.
pub fn parse_post_filename(filename: &str) -> (Option<String>, String) {
    let stem = filename.strip_suffix(".md").unwrap_or(filename);

    // Check if the stem starts with a date pattern: exactly YYYY-MM-DD-
    if stem.len() >= 11 {
        let maybe_date = &stem[..10];
        let parts: Vec<&str> = maybe_date.split('-').collect();
        if parts.len() == 3
            && parts[0].len() == 4
            && parts[1].len() == 2
            && parts[2].len() == 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].chars().all(|c| c.is_ascii_digit())
        {
            // Must have a '-' after the date, followed by the slug
            if stem.len() > 11 && stem.as_bytes()[10] == b'-' {
                let date = maybe_date.to_string();
                let slug = stem[11..].to_string();
                return (Some(date), slug);
            }
        }
    }

    (None, stem.to_string())
}

/// Generate a URL from a permalink pattern by substituting `:collection` and `:title`.
pub fn generate_url(pattern: &str, collection: &str, title: &str) -> String {
    pattern
        .replace(":collection", collection)
        .replace(":title", title)
}

/// Returns true if the filename should be skipped (starts with `_` or is not `.md`).
fn should_skip(filename: &str) -> bool {
    filename.starts_with('_') || !filename.ends_with(".md")
}

/// Load all items from a collection directory.
///
/// For posts (`collection_name == "posts"`), the global permalink pattern from
/// `config.permalink` is used. For other collections, the collection's own
/// permalink pattern is used.
///
/// Files whose name starts with `_` are skipped. Non-`.md` files are skipped.
/// If a single file fails to parse, the error is collected but loading continues.
///
/// Returns an empty Vec if the directory does not exist.
///
/// # Errors
///
/// Returns `CollectionError::ReadDir` if the directory exists but cannot be read.
pub fn load_collection(
    collection_name: &str,
    site_dir: &Path,
    config: &SiteConfig,
) -> Result<(Vec<CollectionItem>, Vec<CollectionError>), CollectionError> {
    let dir = site_dir.join(format!("_{}", collection_name));

    if !dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let entries = fs::read_dir(&dir).map_err(|e| CollectionError::ReadDir {
        path: dir.display().to_string(),
        source: e,
    })?;

    let permalink_pattern = if collection_name == "posts" {
        config.permalink.clone()
    } else {
        config
            .collection(collection_name)
            .map(|c| c.permalink.clone())
            .unwrap_or_else(|| "/:collection/:title.html".to_string())
    };

    let mut items = Vec::new();
    let mut errors = Vec::new();

    let mut file_paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    file_paths.sort();

    for path in file_paths {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if should_skip(&filename) {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                errors.push(CollectionError::ReadFile {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        let doc = match frontmatter::parse_document(&raw) {
            Ok(doc) => doc,
            Err(e) => {
                errors.push(CollectionError::Parse {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        let is_posts = collection_name == "posts";
        let stem = filename.strip_suffix(".md").unwrap_or(&filename);

        let (date, slug) = if is_posts {
            parse_post_filename(&filename)
        } else {
            (None, stem.to_string())
        };

        let url = generate_url(&permalink_pattern, collection_name, &slug);
        let html_content = frontmatter::markdown_to_html(&doc.content);

        let source_path = path
            .strip_prefix(site_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

        items.push(CollectionItem {
            slug,
            front_matter: doc.front_matter,
            content: doc.content,
            html_content,
            excerpt: doc.excerpt,
            url,
            date,
            collection_name: collection_name.to_string(),
            source_path,
        });
    }

    Ok((items, errors))
}

/// A standalone page (root-level `.md` file, not part of any collection).
#[derive(Debug, Clone)]
pub struct Page {
    /// Filename stem (e.g. `index` from `index.md`).
    pub slug: String,

    /// Parsed YAML front matter.
    pub front_matter: FrontMatter,

    /// Raw markdown body.
    pub content: String,

    /// Markdown body converted to HTML.
    pub html_content: String,

    /// Generated URL path.
    pub url: String,

    /// Relative path to the source file (e.g. `index.md`).
    pub source_path: String,
}

/// Load standalone `.md` pages from the root directory.
///
/// Skips `README.md` and files whose name starts with `_`.
/// If a file fails to parse, the error is collected but loading continues.
///
/// Returns an empty Vec if the directory does not exist.
///
/// # Errors
///
/// Returns `CollectionError::ReadDir` if the directory cannot be read.
pub fn load_pages(site_dir: &Path) -> Result<(Vec<Page>, Vec<CollectionError>), CollectionError> {
    if !site_dir.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let entries = fs::read_dir(site_dir).map_err(|e| CollectionError::ReadDir {
        path: site_dir.display().to_string(),
        source: e,
    })?;

    let mut pages = Vec::new();
    let mut errors = Vec::new();

    let mut file_paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    file_paths.sort();

    for path in file_paths {
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if !filename.ends_with(".md") {
            continue;
        }

        if filename == "README.md" || filename.starts_with('_') {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) => {
                errors.push(CollectionError::ReadFile {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        let doc = match frontmatter::parse_document(&raw) {
            Ok(doc) => doc,
            Err(e) => {
                errors.push(CollectionError::Parse {
                    path: path.display().to_string(),
                    source: e,
                });
                continue;
            }
        };

        let stem = filename.strip_suffix(".md").unwrap_or(&filename);

        // Use front matter `permalink` if present, otherwise `/:title.html`
        let url = doc
            .front_matter
            .get("permalink")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("/{}.html", stem));

        let html_content = frontmatter::markdown_to_html(&doc.content);

        let source_path_str = path
            .strip_prefix(site_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));

        pages.push(Page {
            slug: stem.to_string(),
            front_matter: doc.front_matter,
            content: doc.content,
            html_content,
            url,
            source_path: source_path_str,
        });
    }

    Ok((pages, errors))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn site_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("datatalksclub.github.io")
    }

    fn test_config() -> SiteConfig {
        SiteConfig::from_file(&site_dir().join("_config.yml")).unwrap()
    }

    // ========================================================================
    // Unit: Post filename parsing
    // ========================================================================

    #[test]
    fn test_parse_post_filename_standard() {
        let (date, slug) = parse_post_filename("2020-11-29-segmentation.md");
        assert_eq!(date, Some("2020-11-29".to_string()));
        assert_eq!(slug, "segmentation");
    }

    #[test]
    fn test_parse_post_filename_with_hyphens_in_slug() {
        let (date, slug) = parse_post_filename("2021-01-01-ml-deployment-lambda.md");
        assert_eq!(date, Some("2021-01-01".to_string()));
        assert_eq!(slug, "ml-deployment-lambda");
    }

    #[test]
    fn test_parse_post_filename_no_date() {
        let (date, slug) = parse_post_filename("non-date-filename.md");
        assert_eq!(date, None);
        assert_eq!(slug, "non-date-filename");
    }

    // ========================================================================
    // Unit: URL generation
    // ========================================================================

    #[test]
    fn test_generate_url_collection_pattern() {
        let url = generate_url("/:collection/:title.html", "people", "alexeygrigorev");
        assert_eq!(url, "/people/alexeygrigorev.html");
    }

    #[test]
    fn test_generate_url_books_pattern() {
        let url = generate_url("/:collection/:title.html", "books", "20201214-ml-bookcamp");
        assert_eq!(url, "/books/20201214-ml-bookcamp.html");
    }

    #[test]
    fn test_generate_url_blog_pattern() {
        let url = generate_url("/blog/:title.html", "posts", "segmentation");
        assert_eq!(url, "/blog/segmentation.html");
    }

    // ========================================================================
    // Unit: Skip underscore-prefixed files
    // ========================================================================

    #[test]
    fn test_should_skip_underscore() {
        assert!(should_skip("_template.md"));
    }

    #[test]
    fn test_should_skip_non_md() {
        assert!(should_skip("file.txt"));
    }

    #[test]
    fn test_should_not_skip_regular_md() {
        assert!(!should_skip("alexeygrigorev.md"));
    }

    // ========================================================================
    // Integration: Load real _people/ collection
    // ========================================================================

    #[test]
    fn test_load_people_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("people", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 424,
            "Expected 424+ people items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_people_known_item() {
        let config = test_config();
        let (items, _) = load_collection("people", &site_dir(), &config).unwrap();
        let alexey = items.iter().find(|i| i.slug == "alexeygrigorev");
        assert!(alexey.is_some(), "Expected to find alexeygrigorev");
        let alexey = alexey.unwrap();
        assert_eq!(
            alexey.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Alexey Grigorev")
        );
        assert!(alexey.front_matter.contains_key("short"));
        assert!(alexey.front_matter.contains_key("picture"));
        assert_eq!(alexey.url, "/people/alexeygrigorev.html");
        assert_eq!(alexey.collection_name, "people");
    }

    // ========================================================================
    // Integration: Load real _books/ collection
    // ========================================================================

    #[test]
    fn test_load_books_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("books", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 98,
            "Expected 98+ books items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_books_known_item() {
        let config = test_config();
        let (items, _) = load_collection("books", &site_dir(), &config).unwrap();
        let book = items.iter().find(|i| i.slug == "20201214-ml-bookcamp");
        assert!(book.is_some(), "Expected to find 20201214-ml-bookcamp");
        let book = book.unwrap();
        assert_eq!(
            book.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Machine Learning Bookcamp")
        );
        assert_eq!(book.url, "/books/20201214-ml-bookcamp.html");
    }

    // ========================================================================
    // Integration: Load real _podcast/ collection
    // ========================================================================

    #[test]
    fn test_load_podcast_collection_count() {
        let config = test_config();
        let (items, errors) = load_collection("podcast", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert!(
            items.len() >= 193,
            "Expected 193+ podcast items, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_podcast_skips_underscore_files() {
        let config = test_config();
        let (items, _) = load_collection("podcast", &site_dir(), &config).unwrap();
        // None of the loaded items should have a slug starting with '_'
        for item in &items {
            assert!(
                !item.slug.starts_with('_'),
                "Found underscore-prefixed item: {}",
                item.slug
            );
        }
    }

    // ========================================================================
    // Integration: Load real _posts/ directory
    // ========================================================================

    #[test]
    fn test_load_posts_count() {
        let config = test_config();
        let (items, errors) = load_collection("posts", &site_dir(), &config).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(items.len(), 55, "Expected 55 posts, got {}", items.len());
    }

    #[test]
    fn test_load_posts_known_item() {
        let config = test_config();
        let (items, _) = load_collection("posts", &site_dir(), &config).unwrap();
        let post = items.iter().find(|i| i.slug == "segmentation");
        assert!(post.is_some(), "Expected to find segmentation post");
        let post = post.unwrap();
        assert_eq!(post.date, Some("2020-11-29".to_string()));
        assert_eq!(post.url, "/blog/segmentation.html");
        assert_eq!(post.collection_name, "posts");
    }

    // ========================================================================
    // Integration: Load real _courses/, _conferences/, _tools/
    // ========================================================================

    #[test]
    fn test_load_courses_count() {
        let config = test_config();
        let (items, _) = load_collection("courses", &site_dir(), &config).unwrap();
        assert!(items.len() >= 1, "Expected 1+ courses, got {}", items.len());
    }

    #[test]
    fn test_load_conferences_count() {
        let config = test_config();
        let (items, _) = load_collection("conferences", &site_dir(), &config).unwrap();
        assert!(
            items.len() >= 2,
            "Expected 2+ conferences, got {}",
            items.len()
        );
    }

    #[test]
    fn test_load_tools_count() {
        let config = test_config();
        let (items, _) = load_collection("tools", &site_dir(), &config).unwrap();
        assert!(items.len() >= 2, "Expected 2+ tools, got {}", items.len());
    }

    // ========================================================================
    // Integration: Load standalone pages
    // ========================================================================

    #[test]
    fn test_load_pages_count() {
        let (pages, errors) = load_pages(&site_dir()).unwrap();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
        assert_eq!(
            pages.len(),
            10,
            "Expected 10 standalone pages, got {}",
            pages.len()
        );
    }

    #[test]
    fn test_load_pages_index() {
        let (pages, _) = load_pages(&site_dir()).unwrap();
        let index = pages.iter().find(|p| p.slug == "index");
        assert!(index.is_some(), "Expected to find index page");
        let index = index.unwrap();
        assert_eq!(
            index.front_matter.get("title").and_then(|v| v.as_str()),
            Some("Welcome to DataTalks.Club")
        );
    }

    #[test]
    fn test_load_pages_excludes_readme() {
        let (pages, _) = load_pages(&site_dir()).unwrap();
        let readme = pages.iter().find(|p| p.slug == "README");
        assert!(readme.is_none(), "README.md should be excluded");
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[test]
    fn test_nonexistent_collection_returns_empty() {
        let config = test_config();
        let (items, errors) = load_collection("nonexistent", &site_dir(), &config).unwrap();
        assert!(items.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nonexistent_pages_dir_returns_empty() {
        let (pages, errors) = load_pages(Path::new("/nonexistent/dir")).unwrap();
        assert!(pages.is_empty());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_collection_items_have_nonempty_fields() {
        let config = test_config();
        let (items, _) = load_collection("people", &site_dir(), &config).unwrap();
        for item in &items {
            assert!(!item.slug.is_empty(), "Slug should not be empty");
            assert!(!item.url.is_empty(), "URL should not be empty");
            assert_eq!(item.collection_name, "people");
        }
    }

    #[test]
    fn test_file_with_no_front_matter() {
        // Create a temp dir with a .md file that has no front matter
        let dir = tempfile::TempDir::new().unwrap();
        let collection_dir = dir.path().join("_test");
        fs::create_dir(&collection_dir).unwrap();
        fs::write(collection_dir.join("nofront.md"), "Just plain content.").unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: None,
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![],
        };

        let (items, errors) = load_collection("test", dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, "nofront");
        assert!(items[0].front_matter.is_empty());
        assert_eq!(items[0].content, "Just plain content.");
    }

    #[test]
    fn test_file_with_front_matter_only() {
        let dir = tempfile::TempDir::new().unwrap();
        let collection_dir = dir.path().join("_test");
        fs::create_dir(&collection_dir).unwrap();
        fs::write(
            collection_dir.join("frontonly.md"),
            "---\ntitle: Just Front Matter\n---\n",
        )
        .unwrap();

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: None,
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec![],
            collections: HashMap::new(),
            defaults: vec![],
        };

        let (items, errors) = load_collection("test", dir.path(), &config).unwrap();
        assert!(errors.is_empty());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, "frontonly");
        assert_eq!(
            items[0].front_matter.get("title").and_then(|v| v.as_str()),
            Some("Just Front Matter")
        );
    }
}

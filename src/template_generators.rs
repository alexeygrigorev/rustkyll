//! Template translation page generator.
//!
//! Emulates Jekyll's `TranslatePageGenerator` plugin pattern, which generates
//! translated pages by combining `_templates/*.html` files with `_translations/*.yml` data.
//!
//! Detection is based on directory/file presence (not site name):
//! - `_translations/` directory exists
//! - `_templates/` directory exists
//! - `_plugins/templates.rb` exists (or contains `TranslatePageGenerator`)

use std::fs;
use std::path::Path;

use crate::collection::Page;
use crate::frontmatter::FrontMatter;
use crate::template::translate_tag;

/// Check whether the template translation generator should be activated.
///
/// Returns true if the site has both `_translations/` and `_templates/`
/// directories, and `_plugins/` contains a templates.rb with the
/// `TranslatePageGenerator` class.
pub fn should_activate_template_generator(source_dir: &Path) -> bool {
    let translations_dir = source_dir.join("_translations");
    let templates_dir = source_dir.join("_templates");

    if !translations_dir.is_dir() || !templates_dir.is_dir() {
        return false;
    }

    // Check for the generator plugin
    let plugins_dir = source_dir.join("_plugins");
    if !plugins_dir.is_dir() {
        return false;
    }

    // Look for templates.rb containing TranslatePageGenerator
    let templates_rb = plugins_dir.join("templates.rb");
    if templates_rb.is_file() {
        if let Ok(content) = fs::read_to_string(&templates_rb) {
            if content.contains("TranslatePageGenerator") {
                return true;
            }
        }
    }

    // Also check other .rb files in plugins dir
    if let Ok(entries) = fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rb") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains("TranslatePageGenerator") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Generate template translation pages.
///
/// For each language in `_translations/` and each template in `_templates/`,
/// generates a Page at the translated URL path.
///
/// Returns a vector of virtual Page structs ready for the rendering pipeline.
pub fn generate_template_pages(source_dir: &Path) -> Result<Vec<Page>, String> {
    let translations_dir = source_dir.join("_translations");
    let templates_dir = source_dir.join("_templates");

    // Load translations
    let store = translate_tag::load_translations(&translations_dir)?;

    // Set the global translation store for the {% translate %} tag
    translate_tag::set_translations(store.clone());

    // Load templates
    let templates = load_templates(&templates_dir)?;

    let mut pages = Vec::new();

    // For each language
    for (lang, url_map) in &store.urls {
        // For each template
        for template in &templates {
            let template_id = &template.id;

            // Look up translated URL
            let translated_url = match url_map.get(template_id.as_str()) {
                Some(url) if !url.is_empty() => url.clone(),
                _ => continue, // Skip templates without URL mapping
            };

            // Handle URLs ending in "/" by appending "index"
            let dst = if translated_url.ends_with('/') {
                format!("{}index", translated_url)
            } else {
                translated_url.clone()
            };

            let url = format!("/{}/{}.html", lang, dst);

            // Build front matter
            let mut fm = template.front_matter.clone();
            fm.insert("lang".to_string(), serde_yaml::Value::String(lang.clone()));
            // Set page.id to template ID (matching Ruby behavior where page.id
            // comes from read_yaml which sets it from the template's front matter)
            if !fm.contains_key("id") {
                fm.insert(
                    "id".to_string(),
                    serde_yaml::Value::String(template_id.clone()),
                );
            }

            pages.push(Page {
                slug: dst.clone(),
                front_matter: fm,
                content: template.content.clone(),
                html_content: String::new(),
                url,
                source_path: format!("_templates/{}", template.filename),
            });
        }

        // Always generate /{lang}/index.html from _templates/index.html
        // (even if "index" isn't in the URL mappings)
        let has_index = url_map.contains_key("index");
        if !has_index {
            if let Some(index_template) = templates.iter().find(|t| t.id == "index") {
                let url = format!("/{}/index.html", lang);
                let mut fm = index_template.front_matter.clone();
                fm.insert("lang".to_string(), serde_yaml::Value::String(lang.clone()));
                if !fm.contains_key("id") {
                    fm.insert(
                        "id".to_string(),
                        serde_yaml::Value::String("index".to_string()),
                    );
                }

                pages.push(Page {
                    slug: "index".to_string(),
                    front_matter: fm,
                    content: index_template.content.clone(),
                    html_content: String::new(),
                    url,
                    source_path: "_templates/index.html".to_string(),
                });
            }
        }
    }

    Ok(pages)
}

/// A parsed template file.
#[derive(Debug, Clone)]
struct TemplateFile {
    /// Template ID (filename without extension).
    id: String,
    /// Original filename (e.g., "about-us.html").
    filename: String,
    /// Parsed front matter.
    front_matter: FrontMatter,
    /// Template content (after front matter).
    content: String,
}

/// Load all template files from the `_templates/` directory.
fn load_templates(templates_dir: &Path) -> Result<Vec<TemplateFile>, String> {
    let mut templates = Vec::new();

    let entries =
        fs::read_dir(templates_dir).map_err(|e| format!("Failed to read templates dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Only process .html files
        if !filename.ends_with(".html") {
            continue;
        }

        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read template {}: {}", filename, e))?;

        // Parse front matter
        let (fm, body) = parse_front_matter(&content);

        templates.push(TemplateFile {
            id,
            filename,
            front_matter: fm,
            content: body,
        });
    }

    templates.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(templates)
}

/// Parse YAML front matter from a template file.
/// Returns (front_matter, body_content).
fn parse_front_matter(content: &str) -> (FrontMatter, String) {
    if !content.starts_with("---") {
        return (FrontMatter::new(), content.to_string());
    }

    // Find the closing ---
    let rest = &content[3..];
    if let Some(end) = rest.find("\n---") {
        let yaml_str = &rest[..end];
        let body = &rest[end + 4..]; // Skip past \n---

        // Skip the first newline after ---
        let body = body.strip_prefix('\n').unwrap_or(body);

        match serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
            Ok(serde_yaml::Value::Mapping(map)) => {
                let mut fm = FrontMatter::new();
                for (k, v) in map {
                    if let Some(key) = k.as_str() {
                        fm.insert(key.to_string(), v);
                    }
                }
                (fm, body.to_string())
            }
            _ => (FrontMatter::new(), content.to_string()),
        }
    } else {
        (FrontMatter::new(), content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_activate_template_generator_with_all_dirs() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_activate");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();
        std::fs::create_dir_all(dir.join("_plugins")).unwrap();
        std::fs::write(
            dir.join("_plugins/templates.rb"),
            "class TranslatePageGenerator < Generator\nend",
        )
        .unwrap();

        assert!(should_activate_template_generator(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_translations() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_no_trans");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_templates")).unwrap();
        std::fs::create_dir_all(dir.join("_plugins")).unwrap();
        std::fs::write(
            dir.join("_plugins/templates.rb"),
            "class TranslatePageGenerator",
        )
        .unwrap();

        assert!(!should_activate_template_generator(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_templates() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_no_tmpl");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_plugins")).unwrap();
        std::fs::write(
            dir.join("_plugins/templates.rb"),
            "class TranslatePageGenerator",
        )
        .unwrap();

        assert!(!should_activate_template_generator(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_should_not_activate_without_plugin() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_no_plugin");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();
        std::fs::create_dir_all(dir.join("_plugins")).unwrap();
        // No templates.rb file

        assert!(!should_activate_template_generator(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_template_pages_basic() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_pages");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();

        // Write a translation file
        std::fs::write(
            dir.join("_translations/en.yml"),
            r#"en:
  url:
    about-us: about-us
    buy: buy
  about-us:
    title: "About"
  buy:
    title: "Buy"
"#,
        )
        .unwrap();

        // Write template files
        std::fs::write(
            dir.join("_templates/about-us.html"),
            "---\nlayout: base\nid: about-us\n---\nAbout page content",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/buy.html"),
            "---\nlayout: base\nid: buy\n---\nBuy page content",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/index.html"),
            "---\nlayout: base\nid: index\n---\nHome page",
        )
        .unwrap();

        let pages = generate_template_pages(&dir).unwrap();

        // Should have 3 pages: about-us, buy, index (index is always generated)
        assert!(
            pages.len() >= 3,
            "Expected at least 3 pages, got {}",
            pages.len()
        );

        // Check about-us page
        let about = pages.iter().find(|p| p.url == "/en/about-us.html");
        assert!(about.is_some(), "Missing /en/about-us.html page");
        let about = about.unwrap();
        assert_eq!(
            about.front_matter.get("lang").unwrap().as_str().unwrap(),
            "en"
        );
        assert_eq!(
            about.front_matter.get("id").unwrap().as_str().unwrap(),
            "about-us"
        );
        assert_eq!(about.content, "About page content");

        // Check buy page
        let buy = pages.iter().find(|p| p.url == "/en/buy.html");
        assert!(buy.is_some(), "Missing /en/buy.html page");

        // Check index page
        let index = pages.iter().find(|p| p.url == "/en/index.html");
        assert!(index.is_some(), "Missing /en/index.html page");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_template_pages_multi_language() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_multilang");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();

        std::fs::write(
            dir.join("_translations/en.yml"),
            "en:\n  url:\n    about-us: about-us\n  about-us:\n    title: \"About\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("_translations/fr.yml"),
            "fr:\n  url:\n    about-us: a-propos-de-nous\n  about-us:\n    title: \"\u{00C0} propos\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("_templates/about-us.html"),
            "---\nlayout: base\n---\nContent",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/index.html"),
            "---\nlayout: base\n---\nHome",
        )
        .unwrap();

        let pages = generate_template_pages(&dir).unwrap();

        // Should have pages for both languages
        let en_about = pages.iter().find(|p| p.url == "/en/about-us.html");
        assert!(en_about.is_some(), "Missing /en/about-us.html");

        let fr_about = pages.iter().find(|p| p.url == "/fr/a-propos-de-nous.html");
        assert!(fr_about.is_some(), "Missing /fr/a-propos-de-nous.html");
        assert_eq!(
            fr_about
                .unwrap()
                .front_matter
                .get("lang")
                .unwrap()
                .as_str()
                .unwrap(),
            "fr"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_template_pages_skips_missing_url() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_skip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();

        // Only "about-us" has a URL mapping, "buy" does not
        std::fs::write(
            dir.join("_translations/en.yml"),
            "en:\n  url:\n    about-us: about-us\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("_templates/about-us.html"),
            "---\nlayout: base\n---\nAbout",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/buy.html"),
            "---\nlayout: base\n---\nBuy",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/index.html"),
            "---\nlayout: base\n---\nHome",
        )
        .unwrap();

        let pages = generate_template_pages(&dir).unwrap();

        // about-us and index should exist, buy should not
        assert!(pages.iter().any(|p| p.url == "/en/about-us.html"));
        assert!(pages.iter().any(|p| p.url == "/en/index.html"));
        assert!(
            !pages.iter().any(|p| p.url.contains("buy")),
            "buy should be skipped (no URL mapping)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_template_pages_url_with_trailing_slash() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_slash");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();

        std::fs::write(
            dir.join("_translations/en.yml"),
            "en:\n  url:\n    download: download/\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("_templates/download.html"),
            "---\nlayout: base\n---\nDownload",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/index.html"),
            "---\nlayout: base\n---\nHome",
        )
        .unwrap();

        let pages = generate_template_pages(&dir).unwrap();

        // URL ending in / should become /en/download/index.html
        let download = pages.iter().find(|p| p.url == "/en/download/index.html");
        assert!(
            download.is_some(),
            "Missing /en/download/index.html, pages: {:?}",
            pages.iter().map(|p| &p.url).collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_generate_template_preserves_layout() {
        let dir = std::env::temp_dir().join("rustkyll_test_btc_gen_layout");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("_translations")).unwrap();
        std::fs::create_dir_all(dir.join("_templates")).unwrap();

        std::fs::write(
            dir.join("_translations/en.yml"),
            "en:\n  url:\n    about-us: about-us\n",
        )
        .unwrap();

        std::fs::write(
            dir.join("_templates/about-us.html"),
            "---\nlayout: about-sidebar\nid: about-us\n---\nContent",
        )
        .unwrap();
        std::fs::write(
            dir.join("_templates/index.html"),
            "---\nlayout: base\n---\nHome",
        )
        .unwrap();

        let pages = generate_template_pages(&dir).unwrap();
        let about = pages.iter().find(|p| p.url == "/en/about-us.html").unwrap();
        assert_eq!(
            about.front_matter.get("layout").unwrap().as_str().unwrap(),
            "about-sidebar"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_front_matter() {
        let content = "---\nlayout: base\nid: about-us\n---\nBody content here";
        let (fm, body) = parse_front_matter(content);
        assert_eq!(fm.get("layout").unwrap().as_str().unwrap(), "base");
        assert_eq!(fm.get("id").unwrap().as_str().unwrap(), "about-us");
        assert_eq!(body, "Body content here");
    }

    #[test]
    fn test_parse_front_matter_no_front_matter() {
        let content = "Just plain content";
        let (fm, body) = parse_front_matter(content);
        assert!(fm.is_empty());
        assert_eq!(body, "Just plain content");
    }

    #[test]
    fn test_detection_with_real_bitcoin_org() {
        let bitcoin_dir = Path::new("/home/alexey/git/rustkyl/websites/bitcoin-org");
        assert!(
            bitcoin_dir.exists(),
            "bitcoin-org website directory must exist"
        );
        assert!(
            should_activate_template_generator(bitcoin_dir),
            "Template generator should be activated for bitcoin-org"
        );
    }
}

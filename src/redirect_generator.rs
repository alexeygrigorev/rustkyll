//! Redirect page generator emulating bitcoin-org's `redirects.rb` plugin.
//!
//! Reads the `redirects:` hash from `_config.yml` and generates a Page for each
//! entry using the `redirect` layout.
//!
//! Detection is based on:
//! - `_plugins/redirects.rb` exists and contains `RedirectPageGenerator`
//! - `_config.yml` has a non-empty `redirects:` hash
//!
//! This is separate from `redirect_from`/`redirect_to` (jekyll-redirect-from),
//! which uses bare meta-refresh HTML. This generator uses the site's `redirect`
//! layout (full site chrome).

use std::fs;
use std::path::Path;

use crate::collection::Page;
use crate::frontmatter::FrontMatter;

/// Check whether the redirect page generator should be activated.
///
/// Returns true if:
/// - `_plugins/redirects.rb` exists and contains `RedirectPageGenerator`
/// - The config extras contain a non-empty `redirects` mapping
pub fn should_activate(
    source_dir: &Path,
    config_extras: &std::collections::HashMap<String, serde_yaml::Value>,
) -> bool {
    // Check for the plugin file
    if !has_redirect_plugin(source_dir) {
        return false;
    }

    // Check for non-empty redirects hash in config
    has_redirects_config(config_extras)
}

/// Check if `_plugins/redirects.rb` exists and contains `RedirectPageGenerator`.
fn has_redirect_plugin(source_dir: &Path) -> bool {
    let redirects_rb = source_dir.join("_plugins").join("redirects.rb");
    if !redirects_rb.is_file() {
        return false;
    }
    match fs::read_to_string(&redirects_rb) {
        Ok(content) => content.contains("RedirectPageGenerator"),
        Err(_) => false,
    }
}

/// Check if the config extras contain a non-empty `redirects:` mapping.
fn has_redirects_config(
    config_extras: &std::collections::HashMap<String, serde_yaml::Value>,
) -> bool {
    match config_extras.get("redirects") {
        Some(serde_yaml::Value::Mapping(redirects)) => !redirects.is_empty(),
        _ => false,
    }
}

/// Split a source path into (directory, filename) following redirects.rb logic.
///
/// The path is split on `/`, the last segment gets `.html` appended to become
/// the filename, and the remaining segments are joined as the directory.
///
/// Examples:
/// - `/en/bitcoin-for-developers` -> (`/en`, `bitcoin-for-developers.html`)
/// - `/releases/2011/04/27/v0.3.21` -> (`/releases/2011/04/27`, `v0.3.21.html`)
/// - `/bitcoin_es_latam.pdf` -> (`/`, `bitcoin_es_latam.pdf.html`)
/// - `/en/developer-guide#blockchain` -> (`/en`, `developer-guide#blockchain.html`)
pub fn split_redirect_path(src: &str) -> (String, String) {
    let parts: Vec<&str> = src.split('/').collect();
    // The last non-empty segment becomes the filename
    let filename = if let Some(last) = parts.last() {
        format!("{}.html", last)
    } else {
        "index.html".to_string()
    };

    let dir_parts: Vec<&str> = if parts.len() > 1 {
        parts[..parts.len() - 1].to_vec()
    } else {
        vec![""]
    };

    let dir = dir_parts.join("/");
    let dir = if dir.is_empty() { "/".to_string() } else { dir };

    (dir, filename)
}

/// Generate redirect pages from the `redirects:` config hash.
///
/// Each entry in the hash maps a source path to a destination URL.
/// A Page is created for each entry with layout `redirect` and
/// `page.redirect` set to the destination URL.
pub fn generate_redirect_pages(
    config_extras: &std::collections::HashMap<String, serde_yaml::Value>,
) -> Vec<Page> {
    let redirects_map = match config_extras.get("redirects") {
        Some(serde_yaml::Value::Mapping(map)) => map,
        _ => return Vec::new(),
    };

    let mut pages = Vec::new();

    for (src_val, dst_val) in redirects_map {
        let src = match src_val.as_str() {
            Some(s) => s,
            None => continue,
        };
        let dst = match dst_val.as_str() {
            Some(s) => s,
            None => continue,
        };

        let (dir, filename) = split_redirect_path(src);

        // Build URL: dir + "/" + filename (without .html extension for URL)
        let url = if dir == "/" {
            format!("/{}", filename)
        } else {
            format!("{}/{}", dir, filename)
        };

        // Build front matter
        let mut fm = FrontMatter::new();
        fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String("redirect".to_string()),
        );
        fm.insert(
            "redirect".to_string(),
            serde_yaml::Value::String(dst.to_string()),
        );
        fm.insert(
            "lang".to_string(),
            serde_yaml::Value::String("en".to_string()),
        );

        pages.push(Page {
            slug: filename.trim_end_matches(".html").to_string(),
            front_matter: fm,
            content: String::new(),
            html_content: String::new(),
            url,
            source_path: format!("_redirects{}", src),
        });
    }

    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a config extras HashMap with a redirects mapping.
    fn make_extras_with_redirects(
        yaml_str: &str,
    ) -> std::collections::HashMap<String, serde_yaml::Value> {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(yaml_str).unwrap();
        extras.insert("redirects".to_string(), redirects);
        extras
    }

    // --- Detection tests ---

    #[test]
    fn test_should_activate_with_plugin_and_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/redirects.rb"),
            "class RedirectPageGenerator < Generator\nend\n",
        )
        .unwrap();

        let extras = make_extras_with_redirects(
            "/en/old-page: /en/new-page\n/en/another: https://example.com/",
        );

        assert!(
            should_activate(root, &extras),
            "Should activate with plugin and redirects config"
        );
    }

    #[test]
    fn test_should_not_activate_without_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        // No _plugins directory at all

        let extras = make_extras_with_redirects("/en/old-page: /en/new-page");

        assert!(
            !should_activate(root, &extras),
            "Should NOT activate without plugin file"
        );
    }

    #[test]
    fn test_should_not_activate_without_redirects_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/redirects.rb"),
            "class RedirectPageGenerator < Generator\nend\n",
        )
        .unwrap();

        // Config without redirects key
        let extras = std::collections::HashMap::new();

        assert!(
            !should_activate(root, &extras),
            "Should NOT activate without redirects in config"
        );
    }

    #[test]
    fn test_should_not_activate_with_empty_redirects() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/redirects.rb"),
            "class RedirectPageGenerator < Generator\nend\n",
        )
        .unwrap();

        // Config with empty redirects hash
        let mut extras = std::collections::HashMap::new();
        extras.insert(
            "redirects".to_string(),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );

        assert!(
            !should_activate(root, &extras),
            "Should NOT activate with empty redirects hash"
        );
    }

    #[test]
    fn test_should_not_activate_with_wrong_plugin_content() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/redirects.rb"),
            "class SomeOtherGenerator < Generator\nend\n",
        )
        .unwrap();

        let extras = make_extras_with_redirects("/en/old-page: /en/new-page");

        assert!(
            !should_activate(root, &extras),
            "Should NOT activate without RedirectPageGenerator in plugin"
        );
    }

    // --- Path splitting tests ---

    #[test]
    fn test_split_simple_path() {
        let (dir, file) = split_redirect_path("/en/bitcoin-for-developers");
        assert_eq!(dir, "/en");
        assert_eq!(file, "bitcoin-for-developers.html");
    }

    #[test]
    fn test_split_deep_path() {
        let (dir, file) = split_redirect_path("/releases/2011/04/27/v0.3.21");
        assert_eq!(dir, "/releases/2011/04/27");
        assert_eq!(file, "v0.3.21.html");
    }

    #[test]
    fn test_split_root_level_path() {
        let (dir, file) = split_redirect_path("/bitcoin_es_latam.pdf");
        assert_eq!(dir, "/");
        assert_eq!(file, "bitcoin_es_latam.pdf.html");
    }

    #[test]
    fn test_split_path_with_fragment() {
        let (dir, file) = split_redirect_path("/en/developer-guide#blockchain");
        assert_eq!(dir, "/en");
        assert_eq!(file, "developer-guide#blockchain.html");
    }

    #[test]
    fn test_split_path_unicode() {
        let (dir, file) = split_redirect_path("/es/artículos");
        assert_eq!(dir, "/es");
        assert_eq!(file, "art\u{00ed}culos.html");
    }

    // --- Page generation tests ---

    #[test]
    fn test_generate_redirect_pages_basic() {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(
            r#"
/en/old-page: /en/new-page
/en/another: https://example.com/target
/bitcoin.pdf: /files/bitcoin.pdf
"#,
        )
        .unwrap();
        extras.insert("redirects".to_string(), redirects);

        let pages = generate_redirect_pages(&extras);

        assert_eq!(pages.len(), 3, "Should generate 3 redirect pages");

        // Check all pages have redirect layout
        for page in &pages {
            assert_eq!(
                page.front_matter.get("layout").unwrap().as_str().unwrap(),
                "redirect",
                "All redirect pages should use 'redirect' layout"
            );
        }

        // Check all pages have lang = "en"
        for page in &pages {
            assert_eq!(
                page.front_matter.get("lang").unwrap().as_str().unwrap(),
                "en",
                "All redirect pages should have lang='en'"
            );
        }
    }

    #[test]
    fn test_generate_redirect_pages_redirect_url() {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(
            r#"
/en/old-page: /en/new-page
/en/external: https://developer.bitcoin.org/
"#,
        )
        .unwrap();
        extras.insert("redirects".to_string(), redirects);

        let pages = generate_redirect_pages(&extras);

        // Find the internal redirect page
        let internal = pages.iter().find(|p| p.url.contains("old-page"));
        assert!(internal.is_some(), "Should have internal redirect page");
        assert_eq!(
            internal
                .unwrap()
                .front_matter
                .get("redirect")
                .unwrap()
                .as_str()
                .unwrap(),
            "/en/new-page",
            "Internal redirect URL should be preserved"
        );

        // Find the external redirect page
        let external = pages.iter().find(|p| p.url.contains("external"));
        assert!(external.is_some(), "Should have external redirect page");
        assert_eq!(
            external
                .unwrap()
                .front_matter
                .get("redirect")
                .unwrap()
                .as_str()
                .unwrap(),
            "https://developer.bitcoin.org/",
            "External redirect URL should be preserved"
        );
    }

    #[test]
    fn test_generate_redirect_pages_url_structure() {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(
            r#"
/en/bitcoin-for-developers: https://developer.bitcoin.org/
/releases/2011/04/27/v0.3.21: /en/release/v0.3.21
/bitcoin_es_latam.pdf: /files/bitcoin-paper/bitcoin_es_latam.pdf
"#,
        )
        .unwrap();
        extras.insert("redirects".to_string(), redirects);

        let pages = generate_redirect_pages(&extras);

        let devs = pages
            .iter()
            .find(|p| p.url.contains("bitcoin-for-developers"));
        assert!(devs.is_some(), "Missing bitcoin-for-developers redirect");
        assert_eq!(devs.unwrap().url, "/en/bitcoin-for-developers.html");

        let release = pages.iter().find(|p| p.url.contains("v0.3.21"));
        assert!(release.is_some(), "Missing release redirect");
        assert_eq!(release.unwrap().url, "/releases/2011/04/27/v0.3.21.html");

        let pdf = pages.iter().find(|p| p.url.contains("bitcoin_es_latam"));
        assert!(pdf.is_some(), "Missing PDF redirect");
        assert_eq!(pdf.unwrap().url, "/bitcoin_es_latam.pdf.html");
    }

    #[test]
    fn test_generate_redirect_pages_no_redirects_key() {
        let extras = std::collections::HashMap::new();
        let pages = generate_redirect_pages(&extras);
        assert!(pages.is_empty(), "No redirects key should produce no pages");
    }

    #[test]
    fn test_generate_redirect_pages_fragment_in_path() {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(
            r#"
/en/developer-guide#blockchain: https://developer.bitcoin.org/devguide/block_chain#block-chain
"#,
        )
        .unwrap();
        extras.insert("redirects".to_string(), redirects);

        let pages = generate_redirect_pages(&extras);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].url, "/en/developer-guide#blockchain.html");
        assert_eq!(
            pages[0]
                .front_matter
                .get("redirect")
                .unwrap()
                .as_str()
                .unwrap(),
            "https://developer.bitcoin.org/devguide/block_chain#block-chain"
        );
    }

    #[test]
    fn test_generate_redirect_pages_unicode_content() {
        let mut extras = std::collections::HashMap::new();
        let redirects: serde_yaml::Value = serde_yaml::from_str(
            r#"
/es/artículos: /es/documentos
/nl/bitcoin-paper: /nl/bitcoin-document
"#,
        )
        .unwrap();
        extras.insert("redirects".to_string(), redirects);

        let pages = generate_redirect_pages(&extras);
        assert_eq!(pages.len(), 2);

        let es = pages.iter().find(|p| p.url.contains("art"));
        assert!(es.is_some(), "Should have Spanish redirect page");
        assert_eq!(
            es.unwrap()
                .front_matter
                .get("redirect")
                .unwrap()
                .as_str()
                .unwrap(),
            "/es/documentos"
        );
    }
}

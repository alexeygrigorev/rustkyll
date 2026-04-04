//! Alert shorturl page generator emulating bitcoin-org's `alerts.rb` shorturl behavior.
//!
//! When an alert collection item has a `shorturl` field in its front matter,
//! two alias pages are generated at the site root:
//! - `/{shorturl}.html`
//! - `/{shorturl}/index.html`
//!
//! These pages use the `alert` layout and contain a canonical link + JS redirect
//! to the main alert URL at `/en/alert/{filename_stem}`.
//!
//! Detection is based on:
//! - `_plugins/alerts.rb` exists and contains `AlertPageGenerator`
//! - The `alerts` collection exists with `output: true`
//! - Alert documents have `shorturl` in front matter

use std::fs;
use std::path::Path;

use crate::collection::{CollectionItem, Page};
use crate::config::SiteConfig;
use crate::frontmatter::FrontMatter;

/// Check whether the alert shorturl generator should be activated.
///
/// Returns true if:
/// - `_plugins/alerts.rb` exists and contains `AlertPageGenerator`
/// - The `alerts` collection is configured with `output: true`
pub fn should_activate(source_dir: &Path, config: &SiteConfig) -> bool {
    if !has_alert_plugin(source_dir) {
        return false;
    }

    // Check that alerts collection exists and has output: true
    match config.collection("alerts") {
        Some(coll) => coll.output,
        None => false,
    }
}

/// Check if `_plugins/alerts.rb` exists and contains `AlertPageGenerator`.
fn has_alert_plugin(source_dir: &Path) -> bool {
    let alerts_rb = source_dir.join("_plugins").join("alerts.rb");
    if !alerts_rb.is_file() {
        return false;
    }
    match fs::read_to_string(&alerts_rb) {
        Ok(content) => content.contains("AlertPageGenerator"),
        Err(_) => false,
    }
}

/// Generate shorturl alias pages from alert collection items.
///
/// For each alert with a `shorturl` front matter field, generates two pages:
/// - `/{shorturl}.html` at the site root
/// - `/{shorturl}/index.html` at the site root
///
/// Both pages use the `alert` layout with `canonical` pointing to the main
/// alert URL, `lang` set to `"en"`, and `date` from the alert filename.
/// They do NOT have `category` set to `alert`.
pub fn generate_shorturl_pages(alerts: &[CollectionItem]) -> Vec<Page> {
    let mut pages = Vec::new();

    for alert in alerts {
        let shorturl = match alert.front_matter.get("shorturl") {
            Some(val) => match val.as_str() {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            },
            None => continue,
        };

        // Extract the filename stem from the source path for the canonical URL.
        // source_path is like "_alerts/2013-08-11-android.html" or "_alerts/2015-07-04-spv-mining.md"
        let filename_stem = Path::new(&alert.source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&alert.slug);

        let canonical = format!("/en/alert/{}", filename_stem);

        // Extract date from alert (already parsed during collection loading)
        let date_str = alert
            .date
            .clone()
            .or_else(|| {
                alert
                    .front_matter
                    .get("date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        // Read the alert's content (body) so the shorturl pages can render it
        // through the alert layout (which checks page.canonical for redirect behavior)
        let content = alert.content.clone();
        let html_content = alert.html_content.clone();

        // Page 1: /{shorturl}.html
        {
            let mut fm = FrontMatter::new();
            fm.insert(
                "layout".to_string(),
                serde_yaml::Value::String("alert".to_string()),
            );
            fm.insert(
                "lang".to_string(),
                serde_yaml::Value::String("en".to_string()),
            );
            fm.insert(
                "canonical".to_string(),
                serde_yaml::Value::String(canonical.clone()),
            );
            if !date_str.is_empty() {
                fm.insert(
                    "date".to_string(),
                    serde_yaml::Value::String(date_str.clone()),
                );
            }
            // Copy title from the alert
            if let Some(title) = alert.front_matter.get("title") {
                fm.insert("title".to_string(), title.clone());
            }

            pages.push(Page {
                slug: shorturl.clone(),
                front_matter: fm,
                content: content.clone(),
                html_content: html_content.clone(),
                url: format!("/{}.html", shorturl),
                source_path: format!("_shorturl/{}.html", shorturl),
            });
        }

        // Page 2: /{shorturl}/index.html
        {
            let mut fm = FrontMatter::new();
            fm.insert(
                "layout".to_string(),
                serde_yaml::Value::String("alert".to_string()),
            );
            fm.insert(
                "lang".to_string(),
                serde_yaml::Value::String("en".to_string()),
            );
            fm.insert(
                "canonical".to_string(),
                serde_yaml::Value::String(canonical.clone()),
            );
            if !date_str.is_empty() {
                fm.insert(
                    "date".to_string(),
                    serde_yaml::Value::String(date_str.clone()),
                );
            }
            if let Some(title) = alert.front_matter.get("title") {
                fm.insert("title".to_string(), title.clone());
            }

            pages.push(Page {
                slug: "index".to_string(),
                front_matter: fm,
                content: content.clone(),
                html_content: html_content.clone(),
                url: format!("/{}/index.html", shorturl),
                source_path: format!("_shorturl/{}/index.html", shorturl),
            });
        }
    }

    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_alert(filename: &str, shorturl: Option<&str>, date: Option<&str>) -> CollectionItem {
        let mut fm = FrontMatter::new();
        fm.insert(
            "title".to_string(),
            serde_yaml::Value::String("Test Alert".to_string()),
        );
        fm.insert(
            "layout".to_string(),
            serde_yaml::Value::String("alert".to_string()),
        );
        if let Some(su) = shorturl {
            fm.insert(
                "shorturl".to_string(),
                serde_yaml::Value::String(su.to_string()),
            );
        }
        if let Some(d) = date {
            fm.insert("date".to_string(), serde_yaml::Value::String(d.to_string()));
        }

        let stem = Path::new(filename).file_stem().unwrap().to_str().unwrap();

        CollectionItem {
            slug: stem.to_string(),
            front_matter: fm,
            content: "Alert body".to_string(),
            html_content: "<p>Alert body</p>".to_string(),
            excerpt: None,
            excerpt_html: None,
            url: format!("/en/alert/{}", stem),
            date: date.map(|d| d.to_string()),
            collection_name: "alerts".to_string(),
            source_path: format!("_alerts/{}", filename),
            id: format!("/en/alert/{}", stem),
        }
    }

    // --- Detection tests ---

    #[test]
    fn test_should_activate_with_plugin_and_collection() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/alerts.rb"),
            "class AlertPageGenerator < Generator\nend\n",
        )
        .unwrap();

        let config_yaml = r#"
collections:
  alerts:
    output: true
"#;
        let config: SiteConfig = serde_yaml::from_str(config_yaml).unwrap();

        assert!(
            should_activate(root, &config),
            "Should activate with plugin and alerts collection with output:true"
        );
    }

    #[test]
    fn test_should_not_activate_without_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        let config_yaml = r#"
collections:
  alerts:
    output: true
"#;
        let config: SiteConfig = serde_yaml::from_str(config_yaml).unwrap();

        assert!(
            !should_activate(root, &config),
            "Should NOT activate without plugin file"
        );
    }

    #[test]
    fn test_should_not_activate_without_output_true() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/alerts.rb"),
            "class AlertPageGenerator < Generator\nend\n",
        )
        .unwrap();

        let config_yaml = r#"
collections:
  alerts:
    output: false
"#;
        let config: SiteConfig = serde_yaml::from_str(config_yaml).unwrap();

        assert!(
            !should_activate(root, &config),
            "Should NOT activate when alerts collection has output: false"
        );
    }

    #[test]
    fn test_should_not_activate_without_alerts_collection() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("_plugins")).unwrap();
        fs::write(
            root.join("_plugins/alerts.rb"),
            "class AlertPageGenerator < Generator\nend\n",
        )
        .unwrap();

        let config_yaml = r#"
collections:
  posts:
    output: true
"#;
        let config: SiteConfig = serde_yaml::from_str(config_yaml).unwrap();

        assert!(
            !should_activate(root, &config),
            "Should NOT activate without alerts collection"
        );
    }

    // --- Generation tests ---

    #[test]
    fn test_alert_with_shorturl_generates_two_pages() {
        let alert = make_alert(
            "2013-08-11-android.html",
            Some("android"),
            Some("2013-08-11"),
        );
        let pages = generate_shorturl_pages(&[alert]);

        assert_eq!(pages.len(), 2, "Should generate exactly 2 shorturl pages");

        let html_page = pages.iter().find(|p| p.url == "/android.html");
        assert!(html_page.is_some(), "Should generate /android.html");

        let index_page = pages.iter().find(|p| p.url == "/android/index.html");
        assert!(index_page.is_some(), "Should generate /android/index.html");
    }

    #[test]
    fn test_alert_without_shorturl_generates_no_pages() {
        let alert = make_alert("2014-02-11-malleability.html", None, Some("2014-02-11"));
        let pages = generate_shorturl_pages(&[alert]);

        assert!(
            pages.is_empty(),
            "Alert without shorturl should generate no pages"
        );
    }

    #[test]
    fn test_alert_with_empty_shorturl_generates_no_pages() {
        let alert = make_alert("2014-02-11-malleability.html", Some(""), Some("2014-02-11"));
        let pages = generate_shorturl_pages(&[alert]);

        assert!(
            pages.is_empty(),
            "Alert with empty shorturl should generate no pages"
        );
    }

    #[test]
    fn test_shorturl_page_properties() {
        let alert = make_alert(
            "2013-08-11-android.html",
            Some("android"),
            Some("2013-08-11"),
        );
        let pages = generate_shorturl_pages(&[alert]);

        for page in &pages {
            // Layout should be "alert"
            assert_eq!(
                page.front_matter.get("layout").unwrap().as_str().unwrap(),
                "alert",
                "Shorturl page should use 'alert' layout"
            );

            // Lang should be "en"
            assert_eq!(
                page.front_matter.get("lang").unwrap().as_str().unwrap(),
                "en",
                "Shorturl page should have lang='en'"
            );

            // Canonical should point to /en/alert/{filename_stem}
            assert_eq!(
                page.front_matter
                    .get("canonical")
                    .unwrap()
                    .as_str()
                    .unwrap(),
                "/en/alert/2013-08-11-android",
                "Canonical should point to main alert URL"
            );

            // Should NOT have category set to "alert"
            assert!(
                page.front_matter.get("category").is_none(),
                "Shorturl page should NOT have category"
            );

            // Should have date
            assert_eq!(
                page.front_matter.get("date").unwrap().as_str().unwrap(),
                "2013-08-11",
                "Shorturl page should have the alert's date"
            );
        }
    }

    #[test]
    fn test_canonical_url_from_html_source() {
        let alert = make_alert(
            "2013-08-11-android.html",
            Some("android"),
            Some("2013-08-11"),
        );
        let pages = generate_shorturl_pages(&[alert]);

        let page = &pages[0];
        assert_eq!(
            page.front_matter
                .get("canonical")
                .unwrap()
                .as_str()
                .unwrap(),
            "/en/alert/2013-08-11-android",
            "Canonical from .html source should strip extension"
        );
    }

    #[test]
    fn test_canonical_url_from_md_source() {
        let alert = make_alert(
            "2015-07-04-spv-mining.md",
            Some("spv-mining"),
            Some("2015-07-04"),
        );
        let pages = generate_shorturl_pages(&[alert]);

        let page = &pages[0];
        assert_eq!(
            page.front_matter
                .get("canonical")
                .unwrap()
                .as_str()
                .unwrap(),
            "/en/alert/2015-07-04-spv-mining",
            "Canonical from .md source should strip extension"
        );
    }

    #[test]
    fn test_multiple_alerts_generate_correct_count() {
        let alerts = vec![
            make_alert(
                "2013-08-11-android.html",
                Some("android"),
                Some("2013-08-11"),
            ),
            make_alert(
                "2015-07-04-spv-mining.md",
                Some("spv-mining"),
                Some("2015-07-04"),
            ),
            make_alert("2014-02-11-malleability.html", None, Some("2014-02-11")),
        ];
        let pages = generate_shorturl_pages(&alerts);

        assert_eq!(
            pages.len(),
            4,
            "2 alerts with shorturls x 2 pages each = 4 pages"
        );
    }

    #[test]
    fn test_shorturl_page_unicode_title() {
        let mut alert = make_alert(
            "2013-08-11-android.html",
            Some("android"),
            Some("2013-08-11"),
        );
        alert.front_matter.insert(
            "title".to_string(),
            serde_yaml::Value::String(
                "Vulnerabilidad de seguridad en Android \u{2014} Acci\u{00f3}n requerida"
                    .to_string(),
            ),
        );
        let pages = generate_shorturl_pages(&[alert]);

        assert_eq!(pages.len(), 2);
        for page in &pages {
            let title = page.front_matter.get("title").unwrap().as_str().unwrap();
            assert!(
                title.contains('\u{2014}'),
                "Unicode em-dash should be preserved in title"
            );
            assert!(
                title.contains('\u{00f3}'),
                "Unicode accented character should be preserved"
            );
        }
    }

    #[test]
    fn test_all_14_bitcoin_org_alerts_produce_28_pages() {
        // Simulate all 14 alerts from bitcoin-org that have shorturls
        let alerts = vec![
            make_alert(
                "2012-02-18-protocol-change.html",
                Some("feb20"),
                Some("2012-02-18"),
            ),
            make_alert(
                "2012-03-16-critical-vulnerability.html",
                Some("critfix"),
                Some("2012-03-16"),
            ),
            make_alert("2012-05-14-dos.html", Some("dos"), Some("2012-05-14")),
            make_alert(
                "2013-03-11-chain-fork.html",
                Some("chainfork"),
                Some("2013-03-11"),
            ),
            make_alert(
                "2013-03-15-upgrade-deadline.html",
                Some("may15"),
                Some("2013-03-15"),
            ),
            make_alert(
                "2013-08-11-android.html",
                Some("android"),
                Some("2013-08-11"),
            ),
            make_alert(
                "2014-04-11-heartbleed.html",
                Some("heartbleed"),
                Some("2014-04-11"),
            ),
            make_alert(
                "2015-07-04-spv-mining.md",
                Some("spv-mining"),
                Some("2015-07-04"),
            ),
            make_alert(
                "2015-10-12-upnp-vulnerability.md",
                Some("upnp-vulnerability"),
                Some("2015-10-12"),
            ),
            make_alert(
                "2016-08-17-binary-safety.md",
                Some("binary-safety"),
                Some("2016-08-17"),
            ),
            make_alert(
                "2016-11-01-alert-retirement.md",
                Some("alert-retirement"),
                Some("2016-11-01"),
            ),
            make_alert(
                "2017-07-12-potential-split.md",
                Some("potential-split"),
                Some("2017-07-12"),
            ),
            make_alert(
                "2017-10-09-segwit2x-safety.md",
                Some("segwit2x-safety"),
                Some("2017-10-09"),
            ),
            make_alert(
                "2018-09-21-required-upgrade.md",
                Some("required-upgrade"),
                Some("2018-09-21"),
            ),
        ];
        let pages = generate_shorturl_pages(&alerts);

        assert_eq!(
            pages.len(),
            28,
            "14 alerts with shorturls should generate 28 pages"
        );

        // Verify specific shorturl pages exist
        assert!(pages.iter().any(|p| p.url == "/android.html"));
        assert!(pages.iter().any(|p| p.url == "/android/index.html"));
        assert!(pages.iter().any(|p| p.url == "/feb20.html"));
        assert!(pages.iter().any(|p| p.url == "/feb20/index.html"));
        assert!(pages.iter().any(|p| p.url == "/spv-mining.html"));
        assert!(pages.iter().any(|p| p.url == "/spv-mining/index.html"));
    }
}

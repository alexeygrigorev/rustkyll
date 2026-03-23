use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Errors that can occur when loading or parsing the site configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse config YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to parse config YAML (lenient): {0}")]
    YamlLenient(#[from] crate::yaml::YamlParseError),
}

/// Configuration for a single Jekyll collection.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CollectionConfig {
    /// Whether pages should be generated for items in this collection.
    #[serde(default)]
    pub output: bool,

    /// Permalink pattern for items in this collection.
    #[serde(default)]
    pub permalink: String,

    /// Front matter field to sort collection items by.
    /// Matches Jekyll's `sort_by` collection option.
    #[serde(default)]
    pub sort_by: Option<String>,
}

/// Scope section of a default configuration entry.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DefaultScope {
    /// Path prefix this default applies to.
    #[serde(default)]
    pub path: String,

    /// Collection type this default applies to.
    #[serde(rename = "type", default)]
    pub type_name: String,
}

/// Values section of a default configuration entry.
///
/// Captures all key-value pairs from the `values:` section of a defaults entry,
/// not just `layout`. This allows any front matter key to be set as a default.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DefaultValues {
    /// All default values as a generic key-value map.
    #[serde(flatten)]
    pub values: HashMap<String, serde_yaml::Value>,
}

impl DefaultValues {
    /// Get the `layout` value if present.
    pub fn layout(&self) -> Option<&str> {
        self.values.get("layout").and_then(|v| v.as_str())
    }
}

/// A single defaults entry mapping a scope to values.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DefaultConfig {
    /// The scope that determines which files this default applies to.
    pub scope: DefaultScope,

    /// The default values to apply.
    pub values: DefaultValues,
}

/// Deserialize a string field that may be null in YAML.
///
/// Jekyll configs commonly have keys with no value (e.g., `url:` or `baseurl:`),
/// which YAML parses as null. This deserializer treats null as an empty string
/// instead of failing with "invalid type: unit value, expected a string".
fn deserialize_string_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.unwrap_or_default())
}

/// Top-level site configuration parsed from `_config.yml`.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Site URL (e.g., "https://example.com").
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub url: String,

    /// Site base URL path for sites deployed to subpaths (e.g., "/blog").
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub baseurl: String,

    /// Site name.
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub name: String,

    /// Site title.
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub title: String,

    /// Twitter handle or configuration (optional).
    /// Accepts both a plain string (`twitter: "@handle"`) and a map
    /// (`twitter: { username: "handle" }`).
    #[serde(default)]
    pub twitter: Option<serde_yaml::Value>,

    /// GitHub repository in `owner/repo` format (optional).
    #[serde(default)]
    pub repository: Option<String>,

    /// Global default permalink pattern for posts.
    #[serde(
        default = "default_permalink",
        deserialize_with = "deserialize_string_or_null_with_default"
    )]
    pub permalink: String,

    /// List of files/directories to exclude from site generation.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// List of files/directories to force-include even if they would normally
    /// be skipped (e.g., directories starting with `_` or `.`).
    /// Matches Jekyll's `include:` configuration key.
    #[serde(default)]
    pub include: Vec<String>,

    /// Map of collection name to collection configuration.
    #[serde(default)]
    pub collections: HashMap<String, CollectionConfig>,

    /// List of default scope/values mappings.
    #[serde(default)]
    pub defaults: Vec<DefaultConfig>,

    /// Catch-all for unknown config keys.
    /// Any YAML key not matched by a named field above is captured here.
    /// These are exposed in templates as `site.<key>`.
    #[serde(flatten)]
    pub extras: HashMap<String, serde_yaml::Value>,
}

/// Like `deserialize_string_or_null` but falls back to the default permalink
/// when the value is null.
fn deserialize_string_or_null_with_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.unwrap_or_else(default_permalink))
}

fn default_permalink() -> String {
    "date".to_string()
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            baseurl: String::new(),
            name: String::new(),
            title: String::new(),
            twitter: None,
            repository: None,
            permalink: default_permalink(),
            exclude: vec![],
            include: vec![],
            collections: HashMap::new(),
            defaults: vec![],
            extras: HashMap::new(),
        }
    }
}

/// Compute a specificity score for a default scope, matching Jekyll's behavior.
///
/// Jekyll applies defaults from least specific to most specific, so that more
/// specific scopes override less specific ones regardless of declaration order.
/// Specificity is determined by:
///   1. Path length (longer path prefix = more specific)
///   2. Type presence (having a type adds specificity beyond any path)
fn scope_specificity(scope: &DefaultScope) -> usize {
    let mut score = scope.path.len();
    if !scope.type_name.is_empty() {
        // Having a type makes a scope more specific than any path-only scope.
        // Adding a large constant ensures type-scoped defaults always beat
        // path-only defaults.
        score += 1000;
    }
    score
}

impl SiteConfig {
    /// Load and parse a site configuration from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Io` if the file cannot be read, or
    /// `ConfigError::Yaml` if the YAML is invalid or missing required fields.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    /// Parse a site configuration from a YAML string.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::Yaml` if the YAML is invalid or missing required fields.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ConfigError> {
        // Handle empty or comment-only YAML (which serde_yaml parses as Null)
        let trimmed = yaml.trim();
        if trimmed.is_empty()
            || trimmed
                .lines()
                .all(|l| l.trim().is_empty() || l.trim().starts_with('#'))
        {
            return Ok(Self::default());
        }
        let config: SiteConfig = crate::yaml::from_str_lenient(yaml)?;
        Ok(config)
    }

    /// Look up the default layout for a given collection type.
    ///
    /// Returns `None` if no default layout is defined for the given type.
    pub fn default_layout_for(&self, collection_type: &str) -> Option<&str> {
        self.defaults
            .iter()
            .find(|d| d.scope.type_name == collection_type)
            .and_then(|d| d.values.layout())
    }

    /// Get all matching default values for a given collection type and path, merged.
    ///
    /// Defaults are applied by specificity: more specific scopes override less
    /// specific ones, matching Jekyll's behavior. Among entries with equal
    /// specificity, later entries in the config override earlier ones.
    /// Scope matching uses `type_name` exact match and `path` prefix match.
    /// An empty `path` in the scope matches all items.
    pub fn defaults_for(
        &self,
        type_name: &str,
        item_path: &str,
    ) -> HashMap<String, serde_yaml::Value> {
        // Collect matching defaults with their specificity and declaration order
        let mut matches: Vec<(usize, usize, &DefaultConfig)> = Vec::new();

        for (decl_order, default) in self.defaults.iter().enumerate() {
            // Match if scope type matches exactly, or scope type is empty
            // (empty type matches all items, matching Jekyll behavior)
            if !default.scope.type_name.is_empty() && default.scope.type_name != type_name {
                continue;
            }

            // Empty path matches everything; non-empty path is a prefix match
            if !default.scope.path.is_empty() && !item_path.starts_with(&default.scope.path) {
                continue;
            }

            let specificity = scope_specificity(&default.scope);
            matches.push((specificity, decl_order, default));
        }

        // Sort by specificity (ascending), then declaration order (ascending).
        // Less specific defaults are applied first, more specific override them.
        matches.sort_by_key(|&(spec, decl, _)| (spec, decl));

        let mut result = HashMap::new();
        for (_, _, default) in matches {
            for (key, value) in &default.values.values {
                result.insert(key.clone(), value.clone());
            }
        }

        result
    }

    /// Get all matching default values for a standalone page.
    ///
    /// Standalone pages match defaults with `type: "pages"` or with an empty type
    /// (path-only scoping). This matches Jekyll's behavior where standalone pages
    /// are of type "pages" and also match unscoped defaults.
    ///
    /// Defaults are applied by specificity: more specific scopes override less
    /// specific ones, matching Jekyll's behavior. Among entries with equal
    /// specificity, later entries in the config override earlier ones.
    pub fn defaults_for_page(&self, item_path: &str) -> HashMap<String, serde_yaml::Value> {
        // Collect matching defaults with their specificity and declaration order
        let mut matches: Vec<(usize, usize, &DefaultConfig)> = Vec::new();

        for (decl_order, default) in self.defaults.iter().enumerate() {
            // Match if type is "pages" or empty (empty type matches all items)
            let type_matches =
                default.scope.type_name.is_empty() || default.scope.type_name == "pages";

            if !type_matches {
                continue;
            }

            // Empty path matches everything; non-empty path is a prefix match
            if !default.scope.path.is_empty() && !item_path.starts_with(&default.scope.path) {
                continue;
            }

            let specificity = scope_specificity(&default.scope);
            matches.push((specificity, decl_order, default));
        }

        // Sort by specificity (ascending), then declaration order (ascending).
        // Less specific defaults are applied first, more specific override them.
        matches.sort_by_key(|&(spec, decl, _)| (spec, decl));

        let mut result = HashMap::new();
        for (_, _, default) in matches {
            for (key, value) in &default.values.values {
                result.insert(key.clone(), value.clone());
            }
        }

        result
    }

    /// Look up a collection by name.
    ///
    /// Returns `None` if no collection with the given name exists.
    pub fn collection(&self, name: &str) -> Option<&CollectionConfig> {
        self.collections.get(name)
    }

    /// Check if the CommonMark HARDBREAKS option is enabled in the site config.
    ///
    /// Returns `true` when the config has:
    /// ```yaml
    /// commonmark:
    ///   options: ["HARDBREAKS"]
    /// ```
    /// AND the markdown processor is NOT kramdown (since HARDBREAKS is a
    /// CommonMark-specific option).
    pub fn has_commonmark_hardbreaks(&self) -> bool {
        // Only applies to non-kramdown processors
        let is_kramdown = self
            .extras
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(|m| m.eq_ignore_ascii_case("kramdown"))
            .unwrap_or(true);
        if is_kramdown {
            return false;
        }

        self.extras
            .get("commonmark")
            .and_then(|v| v.as_mapping())
            .and_then(|m| m.get(serde_yaml::Value::String("options".to_string())))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter().any(|item| {
                    item.as_str()
                        .map(|s| s.eq_ignore_ascii_case("HARDBREAKS"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    /// Check if the CommonMark autolink extension is enabled in the site config.
    ///
    /// Returns `true` when the config has:
    /// ```yaml
    /// commonmark:
    ///   extensions: ["autolink"]
    /// ```
    /// AND the markdown processor is NOT kramdown (since autolink is a
    /// CommonMark/GFM-specific extension).
    pub fn has_commonmark_autolink(&self) -> bool {
        // Only applies to non-kramdown processors
        let is_kramdown = self
            .extras
            .get("markdown")
            .and_then(|v| v.as_str())
            .map(|m| m.eq_ignore_ascii_case("kramdown"))
            .unwrap_or(true);
        if is_kramdown {
            return false;
        }

        self.extras
            .get("commonmark")
            .and_then(|v| v.as_mapping())
            .and_then(|m| m.get(serde_yaml::Value::String("extensions".to_string())))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter().any(|item| {
                    item.as_str()
                        .map(|s| s.eq_ignore_ascii_case("autolink"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn real_config_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join("_config.yml")
    }

    // ========================================================================
    // Parse the real config file
    // ========================================================================

    #[test]
    fn test_parse_real_config_basic_fields() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.url, "https://datatalks.club");
        assert_eq!(config.name, "DataTalks.Club");
        assert_eq!(config.title, "DataTalks.Club");
        assert_eq!(
            config.twitter,
            Some(serde_yaml::Value::String("@DataTalksClub".to_string()))
        );
        assert_eq!(config.permalink, "/blog/:title.html");
    }

    #[test]
    fn test_parse_real_config_exclude_list() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.exclude.len(), 13);
        assert!(config.exclude.contains(&"Gemfile".to_string()));
        assert!(config.exclude.contains(&"node_modules/".to_string()));
        assert!(config.exclude.contains(&"scripts/".to_string()));
    }

    // ========================================================================
    // Collections parsing
    // ========================================================================

    #[test]
    fn test_parse_real_config_collections_count() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.collections.len(), 6);
    }

    #[test]
    fn test_parse_real_config_collection_names() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        let expected = [
            "books",
            "people",
            "conferences",
            "podcast",
            "courses",
            "tools",
        ];
        for name in &expected {
            assert!(
                config.collections.contains_key(*name),
                "Missing collection: {}",
                name
            );
        }
    }

    #[test]
    fn test_parse_real_config_all_collections_output_true() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        for (name, coll) in &config.collections {
            assert!(coll.output, "Collection {} should have output: true", name);
        }
    }

    #[test]
    fn test_parse_real_config_all_collections_permalink() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        for (name, coll) in &config.collections {
            assert_eq!(
                coll.permalink, "/:collection/:title.html",
                "Collection {} has wrong permalink",
                name
            );
        }
    }

    // ========================================================================
    // Defaults parsing
    // ========================================================================

    #[test]
    fn test_parse_real_config_defaults_count() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.defaults.len(), 3);
    }

    #[test]
    fn test_parse_real_config_defaults_mappings() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();

        let find_layout = |type_name: &str| -> Option<String> {
            config
                .defaults
                .iter()
                .find(|d| d.scope.type_name == type_name)
                .and_then(|d| d.values.layout().map(|s| s.to_string()))
        };

        assert_eq!(find_layout("people"), Some("author".to_string()));
        assert_eq!(find_layout("books"), Some("book".to_string()));
        assert_eq!(find_layout("podcast"), Some("podcast".to_string()));
    }

    // ========================================================================
    // Convenience methods
    // ========================================================================

    #[test]
    fn test_default_layout_for_people() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("people"), Some("author"));
    }

    #[test]
    fn test_default_layout_for_books() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("books"), Some("book"));
    }

    #[test]
    fn test_default_layout_for_podcast() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("podcast"), Some("podcast"));
    }

    #[test]
    fn test_default_layout_for_courses_returns_none() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("courses"), None);
    }

    #[test]
    fn test_default_layout_for_nonexistent_returns_none() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("nonexistent"), None);
    }

    #[test]
    fn test_collection_lookup_books() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        let books = config.collection("books");
        assert!(books.is_some());
        let books = books.unwrap();
        assert!(books.output);
        assert_eq!(books.permalink, "/:collection/:title.html");
    }

    #[test]
    fn test_collection_lookup_nonexistent() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert!(config.collection("nonexistent").is_none());
    }

    // ========================================================================
    // Missing optional fields
    // ========================================================================

    #[test]
    fn test_parse_minimal_config() {
        let yaml = r#"
url: "https://example.com"
name: "Test Site"
title: "Test"
permalink: "/:title.html"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "https://example.com");
        assert_eq!(config.name, "Test Site");
        assert_eq!(config.title, "Test");
        assert!(config.twitter.is_none());
        assert!(config.exclude.is_empty());
        assert!(config.collections.is_empty());
        assert!(config.defaults.is_empty());
        assert!(config.extras.is_empty());
    }

    // ========================================================================
    // Error handling
    // ========================================================================

    #[test]
    fn test_parse_empty_string_succeeds_with_defaults() {
        let config = SiteConfig::from_yaml_str("").unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "");
        assert_eq!(config.title, "");
    }

    #[test]
    fn test_parse_invalid_yaml_returns_error() {
        let result = SiteConfig::from_yaml_str(": : :");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_url_uses_default() {
        let yaml = r#"
name: "Test"
title: "Test"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "Test");
        assert_eq!(config.title, "Test");
    }

    // ========================================================================
    // Round-trip sanity
    // ========================================================================

    #[test]
    fn test_programmatic_config_convenience_methods() {
        let mut collections = HashMap::new();
        collections.insert(
            "posts".to_string(),
            CollectionConfig {
                output: true,
                permalink: "/:title.html".to_string(),
                sort_by: None,
            },
        );

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: Some(serde_yaml::Value::String("@test".to_string())),
            repository: None,
            permalink: "/:title.html".to_string(),
            exclude: vec!["node_modules/".to_string()],
            collections,
            defaults: vec![DefaultConfig {
                scope: DefaultScope {
                    path: String::new(),
                    type_name: "posts".to_string(),
                },
                values: DefaultValues {
                    values: {
                        let mut m = HashMap::new();
                        m.insert(
                            "layout".to_string(),
                            serde_yaml::Value::String("post".to_string()),
                        );
                        m
                    },
                },
            }],
            ..Default::default()
        };

        assert_eq!(config.default_layout_for("posts"), Some("post"));
        assert_eq!(config.default_layout_for("pages"), None);
        assert!(config.collection("posts").is_some());
        assert!(config.collection("missing").is_none());
    }

    // ========================================================================
    // Issue 23: Optional fields with defaults
    // ========================================================================

    #[test]
    fn test_no_url_defaults_to_empty() {
        let yaml = "name: Test\ntitle: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
    }

    #[test]
    fn test_no_name_defaults_to_empty() {
        let yaml = "url: https://example.com\ntitle: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.name, "");
    }

    #[test]
    fn test_no_title_defaults_to_empty() {
        let yaml = "url: https://example.com\nname: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.title, "");
    }

    #[test]
    fn test_no_url_name_title_all_default() {
        let yaml = "permalink: /:title.html\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "");
        assert_eq!(config.title, "");
    }

    #[test]
    fn test_only_url_set() {
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "https://example.com");
        assert_eq!(config.name, "");
        assert_eq!(config.title, "");
    }

    #[test]
    fn test_comment_only_yaml_parses() {
        let yaml = "# This is a comment\n# Another comment\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "");
    }

    // ========================================================================
    // Issue 23: Twitter as flexible type
    // ========================================================================

    #[test]
    fn test_twitter_as_string() {
        let yaml = "twitter: \"@DataTalksClub\"\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(matches!(config.twitter, Some(serde_yaml::Value::String(_))));
        assert_eq!(config.twitter.unwrap().as_str().unwrap(), "@DataTalksClub");
    }

    #[test]
    fn test_twitter_as_map() {
        let yaml = "twitter:\n  username: handle\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(matches!(
            config.twitter,
            Some(serde_yaml::Value::Mapping(_))
        ));
        let map = config.twitter.unwrap();
        assert_eq!(
            map.as_mapping()
                .unwrap()
                .get("username")
                .unwrap()
                .as_str()
                .unwrap(),
            "handle"
        );
    }

    #[test]
    fn test_twitter_missing() {
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(config.twitter.is_none());
    }

    #[test]
    fn test_twitter_null() {
        let yaml = "twitter: null\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        // serde_yaml deserializes null into None for Option
        assert!(config.twitter.is_none());
    }

    // ========================================================================
    // Issue 23: Unknown config keys (catch-all)
    // ========================================================================

    #[test]
    fn test_unknown_key_locale() {
        let yaml = "locale: en\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(
            config.extras.get("locale").and_then(|v| v.as_str()),
            Some("en")
        );
    }

    #[test]
    fn test_unknown_key_nested_sass() {
        let yaml = "sass:\n  style: compressed\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let sass = config.extras.get("sass").unwrap();
        assert!(sass.is_mapping());
        assert_eq!(
            sass.as_mapping()
                .unwrap()
                .get("style")
                .unwrap()
                .as_str()
                .unwrap(),
            "compressed"
        );
    }

    #[test]
    fn test_unknown_key_kramdown() {
        let yaml = "kramdown:\n  input: GFM\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(config.extras.contains_key("kramdown"));
    }

    #[test]
    fn test_many_unknown_keys() {
        let yaml = r#"
locale: en
sass: { style: compressed }
kramdown: { input: GFM }
paginate: 10
timezone: UTC
markdown: kramdown
highlighter: rouge
encoding: utf-8
future: true
show_excerpts: false
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.extras.len(), 10);
        assert_eq!(
            config.extras.get("paginate").and_then(|v| v.as_u64()),
            Some(10)
        );
        assert_eq!(
            config.extras.get("timezone").and_then(|v| v.as_str()),
            Some("UTC")
        );
    }

    #[test]
    fn test_known_fields_not_in_extras() {
        let yaml = r#"
url: https://example.com
name: Test
title: Test
permalink: /:title.html
locale: en
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        // Known fields should NOT appear in extras
        assert!(!config.extras.contains_key("url"));
        assert!(!config.extras.contains_key("name"));
        assert!(!config.extras.contains_key("title"));
        assert!(!config.extras.contains_key("permalink"));
        assert!(!config.extras.contains_key("collections"));
        assert!(!config.extras.contains_key("defaults"));
        assert!(!config.extras.contains_key("exclude"));
        // Unknown field should be in extras
        assert!(config.extras.contains_key("locale"));
    }

    // ========================================================================
    // Issue 23: DTC config still parses with extras
    // ========================================================================

    #[test]
    fn test_real_config_has_theme_in_extras() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        // The DTC config has `theme: jekyll-theme-cayman` which is an unknown key
        assert_eq!(
            config.extras.get("theme").and_then(|v| v.as_str()),
            Some("jekyll-theme-cayman")
        );
    }

    // ========================================================================
    // Issue 24: baseurl parsing
    // ========================================================================

    #[test]
    fn test_baseurl_present() {
        let yaml = "baseurl: /blog\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.baseurl, "/blog");
    }

    #[test]
    fn test_baseurl_empty_string() {
        let yaml = "baseurl: \"\"\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.baseurl, "");
    }

    #[test]
    fn test_baseurl_missing_defaults_empty() {
        let yaml = "url: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.baseurl, "");
    }

    #[test]
    fn test_baseurl_unquoted() {
        let yaml = "baseurl: /repo-name\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.baseurl, "/repo-name");
    }

    // ========================================================================
    // Issue 28: Generalized front matter defaults
    // ========================================================================

    #[test]
    fn test_generalized_defaults_parse_multiple_values() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
      comments: true
      read_time: true
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.defaults.len(), 1);
        let values = &config.defaults[0].values;
        assert_eq!(values.layout(), Some("post"));
        assert_eq!(
            values.values.get("comments").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            values.values.get("read_time").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_generalized_defaults_layout_only() {
        let yaml = r#"
defaults:
  - scope:
      type: "people"
    values:
      layout: "author"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.default_layout_for("people"), Some("author"));
    }

    #[test]
    fn test_generalized_defaults_non_layout_keys_only() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values:
      author: "Default Author"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let values = &config.defaults[0].values;
        assert_eq!(values.layout(), None);
        assert_eq!(
            values.values.get("author").and_then(|v| v.as_str()),
            Some("Default Author")
        );
    }

    #[test]
    fn test_generalized_defaults_empty_values() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values: {}
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.defaults.len(), 1);
        assert!(config.defaults[0].values.values.is_empty());
    }

    #[test]
    fn test_defaults_for_basic() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
      comments: true
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for("posts", "");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("post")
        );
        assert_eq!(
            defaults.get("comments").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn test_defaults_for_type_filtering() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
  - scope:
      type: "people"
    values:
      layout: "author"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let post_defaults = config.defaults_for("posts", "");
        assert_eq!(
            post_defaults.get("layout").and_then(|v| v.as_str()),
            Some("post")
        );
        assert!(!post_defaults.contains_key("author"));
    }

    #[test]
    fn test_defaults_for_path_override() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
      path: ""
    values:
      layout: "post"
  - scope:
      type: "posts"
      path: "special/"
    values:
      layout: "special-post"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for("posts", "special/my-post");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("special-post")
        );
    }

    #[test]
    fn test_defaults_for_no_defaults() {
        let config = SiteConfig::default();
        let defaults = config.defaults_for("posts", "");
        assert!(defaults.is_empty());
    }

    #[test]
    fn test_defaults_for_path_no_match() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
      path: "2021/"
    values:
      layout: "new-layout"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for("posts", "2020/my-post");
        assert!(defaults.is_empty());
    }

    #[test]
    fn test_dtc_defaults_backward_compat() {
        let config = SiteConfig::from_file(&real_config_path()).unwrap();
        assert_eq!(config.default_layout_for("people"), Some("author"));
        assert_eq!(config.default_layout_for("books"), Some("book"));
        assert_eq!(config.default_layout_for("podcast"), Some("podcast"));
        assert_eq!(config.default_layout_for("courses"), None);
    }

    // ========================================================================
    // Issue 43: Duplicate YAML keys in config
    // ========================================================================

    #[test]
    fn test_duplicate_top_level_key_last_wins() {
        let yaml = "url: https://first.com\nname: Test\nurl: https://second.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "https://second.com");
    }

    #[test]
    fn test_duplicate_nested_key_last_wins() {
        let yaml = r#"
sass:
  style: expanded
  style: compressed
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let sass = config.extras.get("sass").unwrap();
        assert_eq!(
            sass.as_mapping()
                .unwrap()
                .get("style")
                .unwrap()
                .as_str()
                .unwrap(),
            "compressed"
        );
    }

    #[test]
    fn test_duplicate_key_different_types_last_wins() {
        let yaml = "url: 123\nname: Test\nurl: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "https://example.com");
    }

    #[test]
    fn test_triple_duplicate_key_third_wins() {
        let yaml = "url: first\nurl: second\nurl: third\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "third");
    }

    // ========================================================================
    // Issue 51: Null YAML values for string fields
    // ========================================================================

    #[test]
    fn test_null_url_defaults_to_empty() {
        let yaml = "url:\nname: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "Test");
    }

    #[test]
    fn test_null_baseurl_defaults_to_empty() {
        let yaml = "baseurl:\nurl: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.baseurl, "");
        assert_eq!(config.url, "https://example.com");
    }

    #[test]
    fn test_null_name_defaults_to_empty() {
        let yaml = "name:\ntitle: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.name, "");
        assert_eq!(config.title, "Test");
    }

    #[test]
    fn test_null_title_defaults_to_empty() {
        let yaml = "title:\nname: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.title, "");
    }

    #[test]
    fn test_null_permalink_defaults_to_default() {
        let yaml = "permalink:\nurl: https://example.com\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.permalink, "date");
    }

    #[test]
    fn test_many_null_values_parse_successfully() {
        // Mimics configs like minimal-mistakes where many keys have null values
        let yaml = r#"
url:
baseurl:
name:
title:
subtitle:
description:
permalink:
repository:
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.baseurl, "");
        assert_eq!(config.name, "");
        assert_eq!(config.title, "");
        assert_eq!(config.permalink, "date");
    }

    #[test]
    fn test_default_permalink_is_date() {
        // Jekyll's default permalink is "date" which expands to
        // /:categories/:year/:month/:day/:title.html
        let config = SiteConfig::default();
        assert_eq!(config.permalink, "date");
    }

    #[test]
    fn test_explicit_permalink_overrides_default() {
        let yaml = "permalink: pretty\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.permalink, "pretty");
    }

    #[test]
    fn test_null_with_comment_parses() {
        // YAML like `url: # the base hostname` is also null
        let yaml = "url: # the base hostname\nname: Test\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.url, "");
        assert_eq!(config.name, "Test");
    }

    // ========================================================================
    // Issue 74: defaults_for_page
    // ========================================================================

    #[test]
    fn test_defaults_for_page_type_pages() {
        let yaml = r#"
defaults:
  - scope:
      path: ""
      type: "pages"
    values:
      layout: "page"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("index.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("page")
        );
    }

    #[test]
    fn test_defaults_for_page_empty_type_matches() {
        let yaml = r#"
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("docs/getting-started/intro.md");
        assert_eq!(defaults.get("layout").and_then(|v| v.as_str()), Some("doc"));
    }

    #[test]
    fn test_defaults_for_page_path_no_match() {
        let yaml = r#"
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("blog/post.md");
        assert!(defaults.is_empty());
    }

    #[test]
    fn test_defaults_for_page_posts_type_no_match() {
        let yaml = r#"
defaults:
  - scope:
      type: "posts"
    values:
      layout: "post"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("index.md");
        assert!(defaults.is_empty());
    }

    // ========================================================================
    // Issue 74: defaults_for with empty type_name scope
    // ========================================================================

    #[test]
    fn test_defaults_for_empty_type_matches_all() {
        let yaml = r#"
defaults:
  - scope:
      path: ""
    values:
      layout: "default"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for("posts", "");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("default")
        );
    }

    // ========================================================================
    // Issue 174: Path-scoped defaults specificity
    // ========================================================================

    #[test]
    fn test_defaults_for_page_specific_path_wins_over_empty_path() {
        // Reproduces the large-docs-site bug: path "docs" is more specific
        // than path "" and should win, regardless of declaration order.
        let yaml = r#"
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
  - scope:
      path: ""
    values:
      layout: "default"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("docs/api-reference/page-1.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("doc"),
            "More specific path 'docs' should override less specific path ''"
        );
    }

    #[test]
    fn test_defaults_for_page_specific_path_wins_reversed_order() {
        // Same test but with declaration order reversed -- specificity should
        // still determine the winner.
        let yaml = r#"
defaults:
  - scope:
      path: ""
    values:
      layout: "default"
  - scope:
      path: "docs"
    values:
      layout: "doc"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("docs/getting-started/intro.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("doc"),
            "More specific path 'docs' should override less specific path '' regardless of order"
        );
    }

    #[test]
    fn test_defaults_for_page_root_page_gets_less_specific_default() {
        // A page at the root should NOT match path "docs" but should match path ""
        let yaml = r#"
defaults:
  - scope:
      path: "docs"
    values:
      layout: "doc"
  - scope:
      path: ""
    values:
      layout: "default"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("index.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("default"),
            "Root page should get 'default' layout, not 'doc'"
        );
    }

    #[test]
    fn test_defaults_for_page_type_scoped_wins_over_path_only() {
        // A default with type "pages" should be more specific than one without
        let yaml = r#"
defaults:
  - scope:
      path: ""
    values:
      layout: "base"
  - scope:
      path: ""
      type: "pages"
    values:
      layout: "page"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("about.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("page"),
            "Type-scoped default should override unscoped default"
        );
    }

    #[test]
    fn test_defaults_for_collection_specific_path_wins() {
        // Same specificity fix should work for collection items too
        let yaml = r#"
defaults:
  - scope:
      path: "special/"
      type: "posts"
    values:
      layout: "special-post"
  - scope:
      path: ""
      type: "posts"
    values:
      layout: "post"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for("posts", "special/my-post.md");
        assert_eq!(
            defaults.get("layout").and_then(|v| v.as_str()),
            Some("special-post"),
            "More specific path 'special/' should override less specific path ''"
        );
    }

    #[test]
    fn test_defaults_for_page_three_level_specificity() {
        // Three levels of path specificity
        let yaml = r#"
defaults:
  - scope:
      path: ""
    values:
      layout: "base"
  - scope:
      path: "docs"
    values:
      layout: "doc"
  - scope:
      path: "docs/api"
    values:
      layout: "api-doc"
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();

        let base = config.defaults_for_page("index.md");
        assert_eq!(base.get("layout").and_then(|v| v.as_str()), Some("base"));

        let doc = config.defaults_for_page("docs/guide/intro.md");
        assert_eq!(doc.get("layout").and_then(|v| v.as_str()), Some("doc"));

        let api = config.defaults_for_page("docs/api/endpoints.md");
        assert_eq!(api.get("layout").and_then(|v| v.as_str()), Some("api-doc"));
    }

    #[test]
    fn test_defaults_for_page_non_layout_values_merge_across_specificity() {
        // Non-overlapping keys from different specificity levels should all be present
        let yaml = r#"
defaults:
  - scope:
      path: ""
    values:
      layout: "default"
      sidebar: true
  - scope:
      path: "docs"
    values:
      layout: "doc"
      toc: true
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        let defaults = config.defaults_for_page("docs/page.md");
        // layout should come from more specific "docs" scope
        assert_eq!(defaults.get("layout").and_then(|v| v.as_str()), Some("doc"));
        // toc should come from more specific "docs" scope
        assert_eq!(defaults.get("toc").and_then(|v| v.as_bool()), Some(true));
        // sidebar should be inherited from less specific "" scope
        assert_eq!(
            defaults.get("sidebar").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    // ========================================================================
    // Issue 223: has_commonmark_hardbreaks config parsing
    // ========================================================================

    #[test]
    fn test_issue223_parse_hardbreaks_from_commonmark_options() {
        let yaml = r#"
markdown: CommonMarkGhPages
commonmark:
  options: ["UNSAFE", "HARDBREAKS"]
  extensions: ["strikethrough", "autolink", "table"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            config.has_commonmark_hardbreaks(),
            "Should detect HARDBREAKS in commonmark.options"
        );
    }

    #[test]
    fn test_issue223_no_hardbreaks_when_options_missing() {
        let yaml = r#"
markdown: CommonMarkGhPages
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_hardbreaks(),
            "Should return false when commonmark key is absent"
        );
    }

    #[test]
    fn test_issue223_no_hardbreaks_for_kramdown() {
        let yaml = r#"
markdown: kramdown
commonmark:
  options: ["HARDBREAKS"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_hardbreaks(),
            "Should return false for kramdown even if commonmark.options has HARDBREAKS"
        );
    }

    #[test]
    fn test_issue223_no_hardbreaks_default_markdown() {
        // When markdown key is absent, default is kramdown
        let yaml = r#"
title: My Site
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_hardbreaks(),
            "Should return false when markdown key is absent (defaults to kramdown)"
        );
    }

    #[test]
    fn test_issue223_hardbreaks_without_unsafe() {
        let yaml = r#"
markdown: CommonMarkGhPages
commonmark:
  options: ["HARDBREAKS"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            config.has_commonmark_hardbreaks(),
            "Should detect HARDBREAKS even without UNSAFE"
        );
    }

    #[test]
    fn test_issue223_no_hardbreaks_with_only_unsafe() {
        let yaml = r#"
markdown: CommonMarkGhPages
commonmark:
  options: ["UNSAFE"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_hardbreaks(),
            "Should return false when only UNSAFE is in options (no HARDBREAKS)"
        );
    }

    #[test]
    fn test_issue223_real_muan_blog_config() {
        let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/muan-blog/_config.yml");
        assert!(
            config_path.exists(),
            "Required test fixture not found: {}",
            config_path.display()
        );
        let config = SiteConfig::from_file(&config_path).unwrap();
        assert!(
            config.has_commonmark_hardbreaks(),
            "muan-blog config should have HARDBREAKS enabled. extras: {:?}",
            config.extras.get("commonmark")
        );
    }

    // ========================================================================
    // Issue 294: has_commonmark_autolink config parsing
    // ========================================================================

    #[test]
    fn test_issue294_autolink_extension_detected() {
        let yaml = r#"
markdown: CommonMarkGhPages
commonmark:
  options: ["HARDBREAKS"]
  extensions: ["strikethrough", "autolink", "table"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            config.has_commonmark_autolink(),
            "Should detect autolink in commonmark.extensions"
        );
    }

    #[test]
    fn test_issue294_autolink_not_present() {
        let yaml = r#"
markdown: CommonMarkGhPages
commonmark:
  extensions: ["strikethrough", "table"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_autolink(),
            "Should return false when autolink not in extensions"
        );
    }

    #[test]
    fn test_issue294_autolink_kramdown_ignored() {
        let yaml = r#"
markdown: kramdown
commonmark:
  extensions: ["autolink"]
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_autolink(),
            "Should return false for kramdown even with autolink extension"
        );
    }

    #[test]
    fn test_issue294_autolink_no_commonmark_key() {
        let yaml = r#"
markdown: CommonMarkGhPages
"#;
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_autolink(),
            "Should return false when commonmark key is absent"
        );
    }

    #[test]
    fn test_issue294_autolink_default_markdown() {
        let yaml = "title: Test Site\n";
        let config = SiteConfig::from_yaml_str(yaml).unwrap();
        assert!(
            !config.has_commonmark_autolink(),
            "Should return false when markdown defaults to kramdown"
        );
    }

    #[test]
    fn test_issue294_real_muan_blog_autolink() {
        let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/muan-blog/_config.yml");
        assert!(
            config_path.exists(),
            "Required test fixture not found: {}",
            config_path.display()
        );
        let config = SiteConfig::from_file(&config_path).unwrap();
        assert!(
            config.has_commonmark_autolink(),
            "muan-blog config should have autolink extension. extras: {:?}",
            config.extras.get("commonmark")
        );
    }
}

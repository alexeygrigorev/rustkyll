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
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DefaultValues {
    /// Default layout template name.
    #[serde(default)]
    pub layout: String,
}

/// A single defaults entry mapping a scope to values.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DefaultConfig {
    /// The scope that determines which files this default applies to.
    pub scope: DefaultScope,

    /// The default values to apply.
    pub values: DefaultValues,
}

/// Top-level site configuration parsed from `_config.yml`.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    /// Site URL (e.g., "https://datatalks.club").
    pub url: String,

    /// Site name.
    pub name: String,

    /// Site title.
    pub title: String,

    /// Twitter handle (optional).
    #[serde(default)]
    pub twitter: Option<String>,

    /// Global default permalink pattern for posts.
    #[serde(default = "default_permalink")]
    pub permalink: String,

    /// List of files/directories to exclude from site generation.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Map of collection name to collection configuration.
    #[serde(default)]
    pub collections: HashMap<String, CollectionConfig>,

    /// List of default scope/values mappings.
    #[serde(default)]
    pub defaults: Vec<DefaultConfig>,
}

fn default_permalink() -> String {
    "/:title.html".to_string()
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
        let config: SiteConfig = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Look up the default layout for a given collection type.
    ///
    /// Returns `None` if no default layout is defined for the given type.
    pub fn default_layout_for(&self, collection_type: &str) -> Option<&str> {
        self.defaults
            .iter()
            .find(|d| d.scope.type_name == collection_type)
            .map(|d| d.values.layout.as_str())
    }

    /// Look up a collection by name.
    ///
    /// Returns `None` if no collection with the given name exists.
    pub fn collection(&self, name: &str) -> Option<&CollectionConfig> {
        self.collections.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn real_config_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("datatalksclub.github.io")
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
        assert_eq!(config.twitter, Some("@DataTalksClub".to_string()));
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
                .map(|d| d.values.layout.clone())
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
    }

    // ========================================================================
    // Error handling
    // ========================================================================

    #[test]
    fn test_parse_empty_string_returns_error() {
        let result = SiteConfig::from_yaml_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_yaml_returns_error() {
        let result = SiteConfig::from_yaml_str(": : :");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_field_returns_error() {
        let yaml = r#"
name: "Test"
title: "Test"
"#;
        let result = SiteConfig::from_yaml_str(yaml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("url") || err_msg.contains("missing"),
            "Error should mention the missing field, got: {}",
            err_msg
        );
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
            },
        );

        let config = SiteConfig {
            url: "https://example.com".to_string(),
            name: "Test".to_string(),
            title: "Test".to_string(),
            twitter: Some("@test".to_string()),
            permalink: "/:title.html".to_string(),
            exclude: vec!["node_modules/".to_string()],
            collections,
            defaults: vec![DefaultConfig {
                scope: DefaultScope {
                    path: String::new(),
                    type_name: "posts".to_string(),
                },
                values: DefaultValues {
                    layout: "post".to_string(),
                },
            }],
        };

        assert_eq!(config.default_layout_for("posts"), Some("post"));
        assert_eq!(config.default_layout_for("pages"), None);
        assert!(config.collection("posts").is_some());
        assert!(config.collection("missing").is_none());
    }
}

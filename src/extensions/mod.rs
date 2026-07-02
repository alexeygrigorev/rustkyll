//! Minimal compiled-in extension framework.
//!
//! An *extension* is a post-render HTML transform. Extensions are **opt-in**:
//! they are only active when listed under an `extensions:` block in
//! `_config.yml`. A site without that block gets an empty [`Registry`] whose
//! transform list is a no-op, so output is byte-identical to a build without
//! this subsystem.
//!
//! ```yaml
//! extensions:
//!   - wikilinks:
//!       scope: [wiki, people]   # which collections it applies to (empty = all)
//!       on_broken: warn         # warn | ignore
//! ```
//!
//! The design is deliberately small but object-safe: [`HtmlTransform`] is a
//! `dyn`-compatible trait so that a future `WasmExtension` adapter (loading
//! `.wasm` from `_extensions/`) can implement it without changing the trait or
//! the registry. Only the compiled-in path is built here.
//!
//! The one genuinely new piece of plumbing is [`SiteIndex`]: a map from page /
//! collection-item slugs and slugified front-matter titles to their URLs. It is
//! built once from every page and collection item and passed into each
//! transform so links can be resolved.

pub mod jemoji;
pub mod mentions;
#[cfg(feature = "wasm")]
pub mod wasm;
pub mod wikilinks;

use crate::archives::slugify;
use crate::config::SiteConfig;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// A read-only index used by transforms to resolve link targets to URLs.
///
/// Two lookups are maintained: item slugs and slugified front-matter titles.
/// Resolution prefers slugs over titles. All keys are slugified (lowercased,
/// spaces to dashes), so lookups are inherently case-insensitive.
#[derive(Debug, Clone, Default)]
pub struct SiteIndex {
    slugs: HashMap<String, String>,
    titles: HashMap<String, String>,
}

impl SiteIndex {
    /// Create an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one page / collection item.
    ///
    /// `slug` is the item's slug, `title` its optional front-matter title, and
    /// `url` the URL both keys should resolve to. The first entry for a given
    /// key wins (so earlier items take precedence on collisions).
    pub fn insert(&mut self, slug: &str, title: Option<&str>, url: &str) {
        let slug_key = slugify(slug);
        if !slug_key.is_empty() {
            self.slugs
                .entry(slug_key)
                .or_insert_with(|| url.to_string());
        }
        if let Some(title) = title {
            let title_key = slugify(title);
            if !title_key.is_empty() {
                self.titles
                    .entry(title_key)
                    .or_insert_with(|| url.to_string());
            }
        }
    }

    /// Resolve a link target to a URL.
    ///
    /// The target is slugified and matched against slugs first, then slugified
    /// titles. Returns `None` when neither matches.
    pub fn resolve(&self, target: &str) -> Option<&str> {
        let key = slugify(target);
        self.slugs
            .get(&key)
            .or_else(|| self.titles.get(&key))
            .map(String::as_str)
    }

    /// Number of distinct slug + title keys in the index.
    pub fn len(&self) -> usize {
        self.slugs.len() + self.titles.len()
    }

    /// Whether the index has no entries.
    pub fn is_empty(&self) -> bool {
        self.slugs.is_empty() && self.titles.is_empty()
    }
}

/// Contextual information passed to a transform for a single rendered document.
pub struct TransformContext<'a> {
    /// Site `baseurl` (may be empty). Applied to emitted, resolved links.
    pub baseurl: &'a str,
    /// The collection this document belongs to, or `None` for standalone pages.
    pub collection: Option<&'a str>,
}

/// The result of applying a transform: the new HTML plus any build warnings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TransformResult {
    /// Transformed HTML.
    pub html: String,
    /// Build warnings emitted while transforming (e.g. broken links).
    pub warnings: Vec<String>,
}

/// A single post-render HTML transform.
///
/// Object-safe on purpose: the registry stores `Box<dyn HtmlTransform>` so a
/// future WASM-backed extension can implement this trait unchanged.
pub trait HtmlTransform: Send + Sync {
    /// The extension's name (matches its `_config.yml` key).
    fn name(&self) -> &str;

    /// Transform `html`, using `index` to resolve links, returning the new HTML
    /// and any warnings.
    fn transform(&self, html: &str, index: &SiteIndex, ctx: &TransformContext) -> TransformResult;
}

/// Error produced while building a [`Registry`] from site config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    /// The `extensions:` value was not a list.
    NotAList,
    /// An entry did not name a known extension.
    UnknownExtension(String),
    /// An extension's config block was invalid.
    InvalidConfig {
        /// The extension whose config failed to parse.
        extension: String,
        /// A human-readable reason.
        message: String,
    },
    /// A `- wasm:` entry was declared but the binary was built without the
    /// `wasm` feature.
    WasmUnsupported,
    /// A `.wasm` plugin could not be loaded or instantiated (missing file,
    /// corrupt bytes, or a module missing the required `transform` export).
    WasmLoad {
        /// The `.wasm` path that failed to load.
        path: String,
        /// A human-readable reason.
        message: String,
    },
}

impl fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtensionError::NotAList => {
                write!(f, "`extensions:` must be a list of extension entries")
            }
            ExtensionError::UnknownExtension(name) => {
                write!(f, "unknown extension `{name}`")
            }
            ExtensionError::InvalidConfig { extension, message } => {
                write!(f, "invalid config for extension `{extension}`: {message}")
            }
            ExtensionError::WasmUnsupported => write!(
                f,
                "`- wasm:` extension entries require the `wasm` feature, but this \
                 binary was built without wasm support"
            ),
            ExtensionError::WasmLoad { path, message } => {
                write!(f, "failed to load wasm extension `{path}`: {message}")
            }
        }
    }
}

impl std::error::Error for ExtensionError {}

/// The set of enabled extensions, in declared order.
#[derive(Default)]
pub struct Registry {
    transforms: Vec<Box<dyn HtmlTransform>>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.transforms.iter().map(|t| t.name()).collect();
        f.debug_struct("Registry")
            .field("extensions", &names)
            .finish()
    }
}

impl Registry {
    /// Build a registry from the site config's `extensions:` block.
    ///
    /// Returns an empty (no-op) registry when the block is absent. Fails loudly
    /// (`Err`) on an unknown extension name or an invalid per-extension config.
    ///
    /// Relative `- wasm:` paths are resolved against the current directory; use
    /// [`Registry::from_config_with_source`] to resolve them against the site
    /// source root instead.
    pub fn from_config(config: &SiteConfig) -> Result<Registry, ExtensionError> {
        Self::from_config_with_source(config, Path::new("."))
    }

    /// Build a registry, resolving relative `- wasm:` plugin paths against
    /// `source_root` (the site source directory).
    pub fn from_config_with_source(
        config: &SiteConfig,
        source_root: &Path,
    ) -> Result<Registry, ExtensionError> {
        // Silence the unused parameter when built without the `wasm` feature
        // (only wasm entries consult the source root).
        #[cfg(not(feature = "wasm"))]
        let _ = source_root;

        let mut transforms: Vec<Box<dyn HtmlTransform>> = Vec::new();

        // Auto-activate plugin-based transforms from `plugins:`/`gems:` (the same
        // Jekyll gospel used before the extension framework existed), independent
        // of any `extensions:` block. They are PREPENDED in a FIXED order —
        // `mentions` then `jemoji` — BEFORE any `extensions:` entries, so that
        // `Registry::apply` threads HTML through them as
        // `mentions -> jemoji -> <extensions...>`. This reproduces, byte for byte,
        // the historic inline post-render order (mentions, then jemoji, then the
        // wikilinks registry). The order is independent of the order the plugins
        // appear in `plugins:`/`gems:`.
        if crate::mentions::has_mentions_plugin(config) {
            transforms.push(Box::new(mentions::Mentions::from_config(config)));
        }
        if crate::jemoji::has_jemoji_plugin(config) {
            transforms.push(Box::new(jemoji::Jemoji::new()));
        }

        // Opt-in `extensions:` block (may be absent). When present it is parsed
        // after the auto-activated plugin transforms so declared extensions run
        // last, preserving the mentions -> jemoji -> extensions order.
        if let Some(value) = config.extras.get("extensions") {
            let seq = value.as_sequence().ok_or(ExtensionError::NotAList)?;

            for entry in seq {
                let (name, cfg) = parse_entry(entry)?;
                match name.as_str() {
                    wikilinks::WikiLinks::NAME => {
                        let ext = wikilinks::WikiLinks::configure(&cfg)?;
                        transforms.push(Box::new(ext));
                    }
                    "wasm" => {
                        let (path, config_json) = parse_wasm_entry(&cfg)?;
                        #[cfg(feature = "wasm")]
                        {
                            let ext = wasm::WasmExtension::load(&path, source_root, config_json)?;
                            transforms.push(Box::new(ext));
                        }
                        #[cfg(not(feature = "wasm"))]
                        {
                            let _ = (path, config_json);
                            return Err(ExtensionError::WasmUnsupported);
                        }
                    }
                    other => return Err(ExtensionError::UnknownExtension(other.to_string())),
                }
            }
        }

        Ok(Registry { transforms })
    }

    /// Whether no extensions are enabled.
    pub fn is_empty(&self) -> bool {
        self.transforms.is_empty()
    }

    /// Number of enabled extensions.
    pub fn len(&self) -> usize {
        self.transforms.len()
    }

    /// The enabled transforms, in declared order.
    pub fn html_transforms(&self) -> &[Box<dyn HtmlTransform>] {
        &self.transforms
    }

    /// Apply every enabled transform in order, threading the HTML through each
    /// and accumulating warnings.
    pub fn apply(&self, html: &str, index: &SiteIndex, ctx: &TransformContext) -> TransformResult {
        let mut current = html.to_string();
        let mut warnings = Vec::new();
        for t in &self.transforms {
            let res = t.transform(&current, index, ctx);
            current = res.html;
            warnings.extend(res.warnings);
        }
        TransformResult {
            html: current,
            warnings,
        }
    }
}

/// Parse one `extensions:` list entry into `(name, config-value)`.
///
/// Supports a bare string (`- wikilinks`) or a single-key mapping
/// (`- wikilinks: { ... }`). The config value is `Null` for the bare form.
fn parse_entry(entry: &serde_yaml::Value) -> Result<(String, serde_yaml::Value), ExtensionError> {
    if let Some(name) = entry.as_str() {
        return Ok((name.to_string(), serde_yaml::Value::Null));
    }
    if let Some(map) = entry.as_mapping() {
        if map.len() != 1 {
            return Err(ExtensionError::InvalidConfig {
                extension: "extensions".to_string(),
                message: "each entry must have exactly one extension name".to_string(),
            });
        }
        let (key, val) = map.iter().next().expect("len == 1 checked above");
        let name = key.as_str().ok_or_else(|| ExtensionError::InvalidConfig {
            extension: "extensions".to_string(),
            message: "extension name must be a string".to_string(),
        })?;
        return Ok((name.to_string(), val.clone()));
    }
    Err(ExtensionError::NotAList)
}

/// Parse a `- wasm:` entry's config value into `(path, optional config JSON)`.
///
/// Two forms are supported:
/// - a bare string: `- wasm: _extensions/foo.wasm` (no plugin config).
/// - a mapping: `- wasm: { path: _extensions/foo.wasm, config: { ... } }`; the
///   optional `config:` mapping is serialized to a JSON string for the plugin.
fn parse_wasm_entry(value: &serde_yaml::Value) -> Result<(String, Option<String>), ExtensionError> {
    let invalid = |message: &str| ExtensionError::InvalidConfig {
        extension: "wasm".to_string(),
        message: message.to_string(),
    };

    if let Some(path) = value.as_str() {
        return Ok((path.to_string(), None));
    }
    if let Some(map) = value.as_mapping() {
        let path = map
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid("a `- wasm:` mapping entry must have a string `path` field"))?
            .to_string();
        let config_json = match map.get("config") {
            Some(cfg) if !cfg.is_null() => Some(serde_json::to_string(cfg).map_err(|e| {
                invalid(&format!("`config` block is not serializable to JSON: {e}"))
            })?),
            _ => None,
        };
        return Ok((path, config_json));
    }
    Err(invalid(
        "a `- wasm:` entry must be a path string or a mapping with a `path` field",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_extensions_yaml(yaml: &str) -> SiteConfig {
        let full = format!("title: t\n{yaml}");
        crate::config::SiteConfig::from_yaml_str(&full).expect("parse config")
    }

    // --- SiteIndex construction & resolution ---

    #[test]
    fn test_site_index_resolves_known_slug() {
        let mut idx = SiteIndex::new();
        idx.insert(
            "event-tracking",
            Some("Event Tracking"),
            "/wiki/event-tracking/",
        );
        assert_eq!(idx.resolve("event-tracking"), Some("/wiki/event-tracking/"));
    }

    #[test]
    fn test_site_index_resolves_by_title_when_slug_differs() {
        let mut idx = SiteIndex::new();
        idx.insert("evt-track", Some("Event Tracking"), "/wiki/evt-track/");
        // slug "event-tracking" does not exist, but title "Event Tracking" slugifies to it.
        assert_eq!(idx.resolve("event-tracking"), Some("/wiki/evt-track/"));
    }

    #[test]
    fn test_site_index_case_insensitive() {
        let mut idx = SiteIndex::new();
        idx.insert("event-tracking", None, "/wiki/event-tracking/");
        assert_eq!(
            idx.resolve("Event-Tracking"),
            Some("/wiki/event-tracking/"),
            "resolution must be case-insensitive"
        );
    }

    #[test]
    fn test_site_index_miss_returns_none() {
        let mut idx = SiteIndex::new();
        idx.insert(
            "event-tracking",
            Some("Event Tracking"),
            "/wiki/event-tracking/",
        );
        assert_eq!(idx.resolve("does-not-exist"), None);
    }

    #[test]
    fn test_site_index_slug_takes_precedence_over_title() {
        let mut idx = SiteIndex::new();
        // First item's slug is "alpha".
        idx.insert("alpha", None, "/wiki/alpha/");
        // Second item's title slugifies to "alpha" too; slug map must win.
        idx.insert("beta", Some("Alpha"), "/wiki/beta/");
        assert_eq!(idx.resolve("alpha"), Some("/wiki/alpha/"));
    }

    // --- Config parsing ---

    #[test]
    fn test_registry_empty_when_no_extensions_block() {
        let config = SiteConfig::default();
        let registry = Registry::from_config(&config).expect("empty registry");
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_parses_wikilinks() {
        let config = config_with_extensions_yaml(
            "extensions:\n  - wikilinks:\n      scope: [wiki, people]\n      on_broken: warn\n",
        );
        let registry = Registry::from_config(&config).expect("registry");
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.html_transforms()[0].name(), "wikilinks");
    }

    #[test]
    fn test_registry_declared_order_preserved() {
        // wikilinks appears twice with different configs; both parse and keep order.
        let config = config_with_extensions_yaml(
            "extensions:\n  - wikilinks:\n      scope: [wiki]\n  - wikilinks:\n      scope: [people]\n",
        );
        let registry = Registry::from_config(&config).expect("registry");
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.html_transforms()[0].name(), "wikilinks");
        assert_eq!(registry.html_transforms()[1].name(), "wikilinks");
    }

    #[test]
    fn test_registry_bare_string_entry() {
        let config = config_with_extensions_yaml("extensions:\n  - wikilinks\n");
        let registry = Registry::from_config(&config).expect("registry");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_unknown_extension_errors() {
        let config = config_with_extensions_yaml("extensions:\n  - nonsense\n");
        let err = Registry::from_config(&config).expect_err("unknown extension must error");
        assert_eq!(
            err,
            ExtensionError::UnknownExtension("nonsense".to_string())
        );
    }

    #[test]
    fn test_registry_invalid_on_broken_errors() {
        let config =
            config_with_extensions_yaml("extensions:\n  - wikilinks:\n      on_broken: explode\n");
        let err = Registry::from_config(&config).expect_err("invalid on_broken must error");
        match err {
            ExtensionError::InvalidConfig { extension, .. } => assert_eq!(extension, "wikilinks"),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn test_registry_extensions_not_a_list_errors() {
        let config = config_with_extensions_yaml("extensions: wikilinks\n");
        let err = Registry::from_config(&config).expect_err("scalar extensions must error");
        assert_eq!(err, ExtensionError::NotAList);
    }

    // --- opt-in off no-op ---

    // --- Issue 603: jemoji / mentions auto-activation from plugins:/gems: ---

    #[test]
    fn test_registry_auto_activates_jemoji_from_plugins() {
        // plugins: [jemoji] with NO extensions: block -> non-empty registry
        // containing a jemoji transform.
        let config = config_with_extensions_yaml("plugins:\n  - jemoji\n");
        let registry = Registry::from_config(&config).expect("registry");
        assert!(
            !registry.is_empty(),
            "jemoji plugin must auto-activate a transform without an extensions: block"
        );
        let names: Vec<&str> = registry
            .html_transforms()
            .iter()
            .map(|t| t.name())
            .collect();
        assert_eq!(names, vec!["jemoji"]);
    }

    #[test]
    fn test_registry_auto_activates_mentions_from_gems() {
        // gems: [jekyll-mentions] with NO extensions: block -> mentions transform.
        let config = config_with_extensions_yaml("gems:\n  - jekyll-mentions\n");
        let registry = Registry::from_config(&config).expect("registry");
        let names: Vec<&str> = registry
            .html_transforms()
            .iter()
            .map(|t| t.name())
            .collect();
        assert_eq!(names, vec!["mentions"]);
    }

    #[test]
    fn test_registry_empty_when_neither_plugin_nor_extensions() {
        // No plugins, no extensions block -> empty (byte-identical no-op) registry.
        let config = config_with_extensions_yaml("plugins:\n  - jekyll-feed\n");
        let registry = Registry::from_config(&config).expect("registry");
        assert!(
            registry.is_empty(),
            "registry must stay empty when no jemoji/mentions plugin and no extensions block"
        );
    }

    #[test]
    fn test_registry_order_is_mentions_jemoji_wikilinks() {
        // plugins: [jemoji, jekyll-mentions] + extensions: [wikilinks]
        // -> order must be exactly [mentions, jemoji, wikilinks].
        let config = config_with_extensions_yaml(
            "plugins:\n  - jemoji\n  - jekyll-mentions\nextensions:\n  - wikilinks\n",
        );
        let registry = Registry::from_config(&config).expect("registry");
        let names: Vec<&str> = registry
            .html_transforms()
            .iter()
            .map(|t| t.name())
            .collect();
        assert_eq!(names, vec!["mentions", "jemoji", "wikilinks"]);
    }

    #[test]
    fn test_registry_order_independent_of_plugin_list_order() {
        // plugins: [jekyll-mentions, jemoji] (reversed) -> still [mentions, jemoji].
        let config = config_with_extensions_yaml("plugins:\n  - jekyll-mentions\n  - jemoji\n");
        let registry = Registry::from_config(&config).expect("registry");
        let names: Vec<&str> = registry
            .html_transforms()
            .iter()
            .map(|t| t.name())
            .collect();
        assert_eq!(names, vec!["mentions", "jemoji"]);
    }

    #[test]
    fn test_mentions_transform_honors_custom_base_url_not_ctx_baseurl() {
        // Custom jekyll-mentions.base_url must win; TransformContext.baseurl is a
        // different (site) value and must NOT be used for mentions.
        let config = config_with_extensions_yaml(
            "plugins:\n  - jekyll-mentions\njekyll-mentions:\n  base_url: https://gitlab.com\n",
        );
        let registry = Registry::from_config(&config).expect("registry");
        let idx = SiteIndex::new();
        let ctx = TransformContext {
            baseurl: "/blog",
            collection: None,
        };
        let res = registry.apply("<p>@user</p>", &idx, &ctx);
        assert_eq!(
            res.html, "<p><a href=\"https://gitlab.com/user\" class=\"user-mention\">@user</a></p>",
            "mention must use configured base_url, not ctx.baseurl"
        );
    }

    #[test]
    fn test_registry_byte_identical_to_inline_pipeline() {
        // The registry (mentions + jemoji auto-activated) must produce output
        // byte-identical to the historic inline pipeline:
        // process_jemoji(process_mentions(html, base_url)).
        let config = config_with_extensions_yaml("plugins:\n  - jemoji\n  - jekyll-mentions\n");
        let registry = Registry::from_config(&config).expect("registry");
        let idx = SiteIndex::new();
        let ctx = TransformContext {
            baseurl: "/site",
            collection: Some("posts"),
        };
        let fixture = "<p>:heart: hi @alice <code>:heart: @bob</code> mail user@example.com team @jekyll/core done</p>";
        let via_registry = registry.apply(fixture, &idx, &ctx).html;

        let base_url = crate::mentions::mentions_base_url(&config);
        let via_inline =
            crate::jemoji::process_jemoji(&crate::mentions::process_mentions(fixture, base_url));

        assert_eq!(
            via_registry, via_inline,
            "registry output must be byte-identical to inline mentions->jemoji pipeline"
        );
    }

    #[test]
    fn test_empty_registry_apply_is_noop() {
        let config = SiteConfig::default();
        let registry = Registry::from_config(&config).unwrap();
        let idx = SiteIndex::new();
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let html = "<p>[[event-tracking]] and @user and :heart:</p>";
        let res = registry.apply(html, &idx, &ctx);
        assert_eq!(
            res.html, html,
            "empty registry must return input byte-identical"
        );
        assert!(res.warnings.is_empty());
    }

    // --- Issue 602: WASM external-extension adapter ---

    #[cfg(feature = "wasm")]
    fn wasm_fixture_dir() -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm");
        assert!(
            dir.join("sample.wasm").exists(),
            "sample.wasm fixture missing at {} — build via tests/fixtures/wasm/sample-plugin/README.md",
            dir.display()
        );
        dir
    }

    #[cfg(feature = "wasm")]
    #[test]
    fn test_registry_loads_wasm_after_wikilinks_in_order() {
        let config =
            config_with_extensions_yaml("extensions:\n  - wikilinks\n  - wasm: sample.wasm\n");
        let registry =
            Registry::from_config_with_source(&config, &wasm_fixture_dir()).expect("registry");
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.html_transforms()[0].name(), "wikilinks");
        assert_eq!(
            registry.html_transforms()[1].name(),
            "wasm:sample.wasm",
            "wasm entry must load and preserve declared order after wikilinks"
        );
    }

    #[cfg(feature = "wasm")]
    #[test]
    fn test_registry_wasm_missing_file_errors() {
        let config =
            config_with_extensions_yaml("extensions:\n  - wasm: _extensions/does-not-exist.wasm\n");
        let err = Registry::from_config_with_source(&config, Path::new("/tmp"))
            .expect_err("missing wasm file must error");
        assert!(
            matches!(err, ExtensionError::WasmLoad { .. }),
            "expected WasmLoad, got {err:?}"
        );
    }

    #[cfg(feature = "wasm")]
    #[test]
    fn test_registry_wasm_config_block_reaches_plugin() {
        // `config: { marker: ref }` must flow to the plugin and change behavior.
        let config = config_with_extensions_yaml(
            "extensions:\n  - wasm:\n      path: sample.wasm\n      config:\n        marker: ref\n",
        );
        let registry =
            Registry::from_config_with_source(&config, &wasm_fixture_dir()).expect("registry");
        assert_eq!(registry.len(), 1);
        let mut idx = SiteIndex::new();
        idx.insert("foo", None, "/wiki/foo/");
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let res = registry.apply("{{ref:foo}}", &idx, &ctx);
        assert_eq!(
            res.html, "<a href=\"/wiki/foo/\">foo</a>",
            "config.marker=ref must switch the recognized token to {{{{ref:...}}}}"
        );
    }

    #[cfg(feature = "wasm")]
    #[test]
    fn test_registry_apply_surfaces_wasm_warnings() {
        // Warnings returned by the plugin must flow through Registry::apply.
        let config = config_with_extensions_yaml("extensions:\n  - wasm: sample.wasm\n");
        let registry =
            Registry::from_config_with_source(&config, &wasm_fixture_dir()).expect("registry");
        let idx = SiteIndex::new();
        let ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let res = registry.apply("{{link:missing}}", &idx, &ctx);
        assert!(
            res.warnings.iter().any(|w| w.contains("missing")),
            "plugin warning must surface in Registry::apply, got {:?}",
            res.warnings
        );
        assert!(res.html.contains("class=\"broken\""));
    }

    #[cfg(not(feature = "wasm"))]
    #[test]
    fn test_registry_wasm_entry_errors_without_feature() {
        let config = config_with_extensions_yaml("extensions:\n  - wasm: _extensions/foo.wasm\n");
        let err = Registry::from_config(&config)
            .expect_err("a wasm entry must fail loudly when built without the wasm feature");
        assert_eq!(err, ExtensionError::WasmUnsupported);
    }
}

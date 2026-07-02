//! `wikilinks` extension: Obsidian/Wikipedia-style `[[target]]` cross-links.
//!
//! Recognizes `[[target]]` and `[[target|label]]` in rendered HTML and rewrites
//! them into anchor tags. Targets are resolved against the site's [`SiteIndex`]
//! (slug first, then slugified front-matter title, case-insensitive). Emitted
//! hrefs have the site `baseurl` applied so links work under a subpath.
//!
//! `[[...]]` occurrences inside `<code>` / `<pre>` are left untouched, matching
//! the skip-tag approach used by `mentions` / `jemoji`.

use super::{ExtensionError, HtmlTransform, SiteIndex, TransformContext, TransformResult};

/// What to do when a `[[target]]` fails to resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnBroken {
    /// Emit a `broken-link` span and record a build warning.
    Warn,
    /// Emit a `broken-link` span silently (no warning).
    Ignore,
}

/// The compiled-in wikilinks extension.
#[derive(Debug, Clone)]
pub struct WikiLinks {
    /// Collections this extension applies to. Empty = all collections + pages.
    scope: Vec<String>,
    /// Broken-link behavior.
    on_broken: OnBroken,
}

impl WikiLinks {
    /// The extension's config key / name.
    pub const NAME: &'static str = "wikilinks";

    /// Parse the extension's config block into a typed [`WikiLinks`].
    ///
    /// Accepts `Null` (no config -> defaults), or a mapping with optional
    /// `scope` (list of collection names) and `on_broken` (`warn` | `ignore`).
    /// Fails loudly on an invalid `on_broken` value or malformed fields.
    pub fn configure(value: &serde_yaml::Value) -> Result<WikiLinks, ExtensionError> {
        let mut scope: Vec<String> = Vec::new();
        let mut on_broken = OnBroken::Warn;

        if !value.is_null() {
            let map = value
                .as_mapping()
                .ok_or_else(|| ExtensionError::InvalidConfig {
                    extension: Self::NAME.to_string(),
                    message: "config must be a mapping".to_string(),
                })?;

            if let Some(scope_val) = map.get(serde_yaml::Value::String("scope".to_string())) {
                if scope_val.is_null() {
                    // explicit empty scope
                } else if let Some(seq) = scope_val.as_sequence() {
                    for v in seq {
                        let name = v.as_str().ok_or_else(|| ExtensionError::InvalidConfig {
                            extension: Self::NAME.to_string(),
                            message: "scope entries must be strings".to_string(),
                        })?;
                        scope.push(name.to_string());
                    }
                } else {
                    return Err(ExtensionError::InvalidConfig {
                        extension: Self::NAME.to_string(),
                        message: "scope must be a list of collection names".to_string(),
                    });
                }
            }

            if let Some(ob_val) = map.get(serde_yaml::Value::String("on_broken".to_string())) {
                let ob = ob_val
                    .as_str()
                    .ok_or_else(|| ExtensionError::InvalidConfig {
                        extension: Self::NAME.to_string(),
                        message: "on_broken must be a string".to_string(),
                    })?;
                on_broken = match ob {
                    "warn" => OnBroken::Warn,
                    "ignore" => OnBroken::Ignore,
                    other => {
                        return Err(ExtensionError::InvalidConfig {
                            extension: Self::NAME.to_string(),
                            message: format!(
                                "invalid on_broken value `{other}` (expected `warn` or `ignore`)"
                            ),
                        })
                    }
                };
            }
        }

        Ok(WikiLinks { scope, on_broken })
    }

    /// Whether this extension applies to the given collection.
    fn in_scope(&self, collection: Option<&str>) -> bool {
        if self.scope.is_empty() {
            return true;
        }
        match collection {
            Some(c) => self.scope.iter().any(|s| s == c),
            // Standalone pages are not in any collection: out of scope when a
            // non-empty scope is configured.
            None => false,
        }
    }

    /// Render a single `[[inner]]` body into an anchor or broken-link span.
    /// Pushes a warning when the target is unresolved and `on_broken == Warn`.
    fn render_link(
        &self,
        inner: &str,
        index: &SiteIndex,
        baseurl: &str,
        warnings: &mut Vec<String>,
    ) -> Option<String> {
        let (target, label) = match inner.split_once('|') {
            Some((t, l)) => (t.trim(), Some(l.trim().to_string())),
            None => (inner.trim(), None),
        };

        if target.is_empty() {
            return None;
        }

        let label = label.unwrap_or_else(|| humanize(target));

        match index.resolve(target) {
            Some(url) => {
                let href = apply_baseurl(url, baseurl);
                Some(format!("<a href=\"{href}\">{label}</a>"))
            }
            None => {
                if self.on_broken == OnBroken::Warn {
                    warnings.push(format!("wikilinks: unresolved link `[[{inner}]]`"));
                }
                Some(format!("<span class=\"broken-link\">{label}</span>"))
            }
        }
    }
}

impl HtmlTransform for WikiLinks {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn transform(&self, html: &str, index: &SiteIndex, ctx: &TransformContext) -> TransformResult {
        if !self.in_scope(ctx.collection) {
            return TransformResult {
                html: html.to_string(),
                warnings: Vec::new(),
            };
        }

        let mut result = String::with_capacity(html.len());
        let mut warnings: Vec<String> = Vec::new();
        let bytes = html.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        let mut skip_depth: usize = 0;

        while i < len {
            // Handle HTML tags: track code/pre nesting so we can skip inside them.
            if bytes[i] == b'<' {
                if let Some(tag_end) = find_tag_end(bytes, i) {
                    let tag = &html[i..=tag_end];
                    let tag_lower = tag.to_ascii_lowercase();
                    if is_opening_skip_tag(&tag_lower) {
                        skip_depth += 1;
                    } else if is_closing_skip_tag(&tag_lower) {
                        skip_depth = skip_depth.saturating_sub(1);
                    }
                    result.push_str(tag);
                    i = tag_end + 1;
                    continue;
                }
            }

            // Look for `[[` outside code/pre.
            if skip_depth == 0 && bytes[i] == b'[' && i + 1 < len && bytes[i + 1] == b'[' {
                if let Some(close) = find_wiki_close(bytes, i + 2) {
                    let inner = &html[i + 2..close];
                    if let Some(rendered) =
                        self.render_link(inner, index, ctx.baseurl, &mut warnings)
                    {
                        result.push_str(&rendered);
                        i = close + 2;
                        continue;
                    }
                }
            }

            // Default: copy one UTF-8 char.
            let ch = html[i..].chars().next().expect("valid char boundary");
            result.push(ch);
            i += ch.len_utf8();
        }

        TransformResult {
            html: result,
            warnings,
        }
    }
}

/// Humanize a target: replace dashes with spaces (`event-tracking` ->
/// `event tracking`). Case is preserved.
fn humanize(target: &str) -> String {
    target.replace('-', " ")
}

/// Prepend `baseurl` to a resolved URL, matching `relative_url` semantics.
fn apply_baseurl(url: &str, baseurl: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    let base = baseurl.trim_end_matches('/');
    if base.is_empty() {
        return url.to_string();
    }
    if url.starts_with('/') {
        format!("{base}{url}")
    } else {
        format!("{base}/{url}")
    }
}

/// Find the index of the `]` that begins the closing `]]` for a `[[` opened at
/// `from`. Bails out (returns `None`) on `<`, newline, or a nested `[[` so a
/// wikilink cannot span tags or lines.
fn find_wiki_close(bytes: &[u8], from: usize) -> Option<usize> {
    let len = bytes.len();
    let mut i = from;
    while i + 1 < len {
        match bytes[i] {
            b'<' | b'\n' | b'\r' => return None,
            b'[' if bytes[i + 1] == b'[' => return None,
            b']' if bytes[i + 1] == b']' => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

/// Find the index of the closing `>` for a tag starting at `start`.
fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut in_quote = false;
    let mut quote_char = b'"';
    while i < bytes.len() {
        if in_quote {
            if bytes[i] == quote_char {
                in_quote = false;
            }
        } else {
            match bytes[i] {
                b'"' | b'\'' => {
                    in_quote = true;
                    quote_char = bytes[i];
                }
                b'>' => return Some(i),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Whether a lowercase tag string opens a `code` or `pre` element.
fn is_opening_skip_tag(tag: &str) -> bool {
    is_tag_match(tag, "<code", 5) || is_tag_match(tag, "<pre", 4)
}

/// Whether a lowercase tag string closes a `code` or `pre` element.
fn is_closing_skip_tag(tag: &str) -> bool {
    tag.starts_with("</code>") || tag.starts_with("</pre>")
}

/// Whether `tag` starts with `prefix` and the next char is a space or `>`.
fn is_tag_match(tag: &str, prefix: &str, prefix_len: usize) -> bool {
    tag.starts_with(prefix)
        && (tag.len() == prefix_len
            || tag
                .as_bytes()
                .get(prefix_len)
                .is_some_and(|&b| b == b' ' || b == b'>'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> SiteIndex {
        let mut idx = SiteIndex::new();
        idx.insert(
            "event-tracking",
            Some("Event Tracking"),
            "/wiki/event-tracking/",
        );
        idx
    }

    fn ctx_all<'a>(baseurl: &'a str) -> TransformContext<'a> {
        TransformContext {
            baseurl,
            collection: None,
        }
    }

    fn warn_ext() -> WikiLinks {
        WikiLinks {
            scope: Vec::new(),
            on_broken: OnBroken::Warn,
        }
    }

    // --- rendering ---

    #[test]
    fn test_default_label_humanized() {
        let ext = warn_ext();
        let out = ext.transform("<p>[[event-tracking]]</p>", &index(), &ctx_all(""));
        assert_eq!(
            out.html,
            "<p><a href=\"/wiki/event-tracking/\">event tracking</a></p>"
        );
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn test_explicit_label_preserved() {
        let ext = warn_ext();
        let out = ext.transform(
            "<p>[[event-tracking|Event Tracking Guide]]</p>",
            &index(),
            &ctx_all(""),
        );
        assert_eq!(
            out.html,
            "<p><a href=\"/wiki/event-tracking/\">Event Tracking Guide</a></p>"
        );
    }

    #[test]
    fn test_resolves_via_title() {
        let mut idx = SiteIndex::new();
        idx.insert("evt-track", Some("Event Tracking"), "/wiki/evt-track/");
        let ext = warn_ext();
        let out = ext.transform("[[event-tracking]]", &idx, &ctx_all(""));
        assert_eq!(out.html, "<a href=\"/wiki/evt-track/\">event tracking</a>");
    }

    #[test]
    fn test_case_insensitive_resolution() {
        let ext = warn_ext();
        let out = ext.transform("[[Event-Tracking]]", &index(), &ctx_all(""));
        assert!(out.html.contains("href=\"/wiki/event-tracking/\""));
    }

    #[test]
    fn test_baseurl_applied() {
        let ext = warn_ext();
        let out = ext.transform("[[event-tracking]]", &index(), &ctx_all("/podwiki"));
        assert_eq!(
            out.html,
            "<a href=\"/podwiki/wiki/event-tracking/\">event tracking</a>"
        );
    }

    #[test]
    fn test_baseurl_trailing_slash_trimmed() {
        let ext = warn_ext();
        let out = ext.transform("[[event-tracking]]", &index(), &ctx_all("/podwiki/"));
        assert!(out.html.contains("href=\"/podwiki/wiki/event-tracking/\""));
    }

    // --- code / pre skipping ---

    #[test]
    fn test_skip_inside_code() {
        let ext = warn_ext();
        let html = "<code>[[event-tracking]]</code>";
        let out = ext.transform(html, &index(), &ctx_all(""));
        assert_eq!(out.html, html, "wikilinks inside <code> must be untouched");
    }

    #[test]
    fn test_skip_inside_pre() {
        let ext = warn_ext();
        let html = "<pre>[[x]]</pre>";
        let out = ext.transform(html, &index(), &ctx_all(""));
        assert_eq!(out.html, html, "wikilinks inside <pre> must be untouched");
    }

    #[test]
    fn test_link_adjacent_to_code_still_transformed() {
        let ext = warn_ext();
        let html = "<code>x</code> [[event-tracking]]";
        let out = ext.transform(html, &index(), &ctx_all(""));
        assert!(out
            .html
            .contains("<code>x</code> <a href=\"/wiki/event-tracking/\">"));
    }

    #[test]
    fn test_nested_code_in_pre_skipped() {
        let ext = warn_ext();
        let html = "<pre><code>[[event-tracking]]</code></pre>";
        let out = ext.transform(html, &index(), &ctx_all(""));
        assert_eq!(out.html, html);
    }

    // --- on_broken ---

    #[test]
    fn test_broken_warn_emits_span_and_warning() {
        let ext = warn_ext();
        let out = ext.transform("[[missing-page]]", &index(), &ctx_all(""));
        assert_eq!(out.html, "<span class=\"broken-link\">missing page</span>");
        assert_eq!(out.warnings.len(), 1, "warn mode must record a warning");
        assert!(out.warnings[0].contains("missing-page"));
    }

    #[test]
    fn test_broken_ignore_emits_span_no_warning() {
        let ext = WikiLinks {
            scope: Vec::new(),
            on_broken: OnBroken::Ignore,
        };
        let out = ext.transform("[[missing-page]]", &index(), &ctx_all(""));
        assert_eq!(out.html, "<span class=\"broken-link\">missing page</span>");
        assert!(out.warnings.is_empty(), "ignore mode must not warn");
    }

    #[test]
    fn test_on_broken_defaults_to_warn() {
        // configure with no on_broken key -> Warn.
        let ext = WikiLinks::configure(&serde_yaml::Value::Null).unwrap();
        assert_eq!(ext.on_broken, OnBroken::Warn);
    }

    #[test]
    fn test_broken_explicit_label_preserved() {
        let ext = warn_ext();
        let out = ext.transform("[[missing|Look Here]]", &index(), &ctx_all(""));
        assert_eq!(out.html, "<span class=\"broken-link\">Look Here</span>");
    }

    // --- scope ---

    #[test]
    fn test_scope_limits_collections() {
        let ext = WikiLinks {
            scope: vec!["wiki".to_string()],
            on_broken: OnBroken::Warn,
        };
        // In-scope collection: transformed.
        let in_ctx = TransformContext {
            baseurl: "",
            collection: Some("wiki"),
        };
        let out = ext.transform("[[event-tracking]]", &index(), &in_ctx);
        assert!(out.html.contains("<a href="));

        // Out-of-scope collection: untouched.
        let out_ctx = TransformContext {
            baseurl: "",
            collection: Some("people"),
        };
        let out = ext.transform("[[event-tracking]]", &index(), &out_ctx);
        assert_eq!(out.html, "[[event-tracking]]");

        // Standalone page (no collection) with non-empty scope: untouched.
        let page_ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let out = ext.transform("[[event-tracking]]", &index(), &page_ctx);
        assert_eq!(out.html, "[[event-tracking]]");
    }

    #[test]
    fn test_empty_scope_applies_to_pages() {
        let ext = warn_ext();
        let page_ctx = TransformContext {
            baseurl: "",
            collection: None,
        };
        let out = ext.transform("[[event-tracking]]", &index(), &page_ctx);
        assert!(out.html.contains("<a href="));
    }

    // --- config parsing ---

    #[test]
    fn test_configure_parses_scope_and_on_broken() {
        let yaml: serde_yaml::Value =
            serde_yaml::from_str("scope: [wiki, people]\non_broken: ignore\n").unwrap();
        let ext = WikiLinks::configure(&yaml).unwrap();
        assert_eq!(ext.scope, vec!["wiki".to_string(), "people".to_string()]);
        assert_eq!(ext.on_broken, OnBroken::Ignore);
    }

    #[test]
    fn test_configure_invalid_on_broken_errors() {
        let yaml: serde_yaml::Value = serde_yaml::from_str("on_broken: explode\n").unwrap();
        let err = WikiLinks::configure(&yaml).expect_err("invalid on_broken");
        match err {
            ExtensionError::InvalidConfig { extension, .. } => assert_eq!(extension, "wikilinks"),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    // --- misc / unicode ---

    #[test]
    fn test_multiple_links_and_unicode_context() {
        let mut idx = SiteIndex::new();
        idx.insert("café-culture", None, "/wiki/cafe/");
        idx.insert("event-tracking", None, "/wiki/event-tracking/");
        let ext = warn_ext();
        let html = "<p>日本 [[event-tracking]] and [[café-culture|Café]] 世界</p>";
        let out = ext.transform(html, &idx, &ctx_all(""));
        assert!(out
            .html
            .contains("<a href=\"/wiki/event-tracking/\">event tracking</a>"));
        assert!(out.html.contains("<a href=\"/wiki/cafe/\">Café</a>"));
        assert!(out.html.contains("日本"));
        assert!(out.html.contains("世界"));
    }

    #[test]
    fn test_empty_wikilink_left_literal() {
        let ext = warn_ext();
        let out = ext.transform("[[]]", &index(), &ctx_all(""));
        assert_eq!(out.html, "[[]]");
    }
}

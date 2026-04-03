//! Tag implementations for Jekyll plugins.
//!
//! - `{% github_edit_link %}` -- produces an edit-on-GitHub link when
//!   `site.github.repository_url` and `site.github.source.branch` are available.
//!   Produces empty output otherwise.

use std::io::Write;

use liquid_core::model::ValueView;
use liquid_core::{
    BlockReflection, Language, ParseBlock, ParseTag, Renderable, Runtime, TagBlock, TagReflection,
    TagTokenIter,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Helper: drain all remaining tokens from the argument iterator.
fn consume_all_arguments(arguments: &mut TagTokenIter<'_>) {
    while arguments.expect_next("").is_ok() {}
}

/// Get a nested string value from the Liquid runtime context.
fn get_nested_str(runtime: &dyn Runtime, parts: &[&str]) -> Option<String> {
    if parts.is_empty() {
        return None;
    }
    let path: Vec<liquid_core::model::ScalarCow<'_>> = parts
        .iter()
        .map(|p| liquid_core::model::ScalarCow::new(*p))
        .collect();
    let val = runtime.try_get(&path)?;
    let s = val.to_kstr().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// HTML-escape a string for safe use in attribute values.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ---------------------------------------------------------------------------
// {% github_edit_link %} -- jekyll-github-metadata plugin
// ---------------------------------------------------------------------------

/// `{% github_edit_link %}` tag (jekyll-github-metadata plugin).
///
/// Produces `<a href="{repo_url}/edit/{branch}/{page_path}">{link_text}</a>`
/// when `site.github.repository_url` and `site.github.source.branch` are
/// available.  Produces empty output otherwise (matching Jekyll behavior
/// when the plugin is not active or github config is missing).
#[derive(Copy, Clone, Debug, Default)]
pub struct GithubEditLinkTag;

impl TagReflection for GithubEditLinkTag {
    fn tag(&self) -> &'static str {
        "github_edit_link"
    }

    fn description(&self) -> &'static str {
        "Generate an edit-on-GitHub link (jekyll-github-metadata plugin)"
    }
}

impl ParseTag for GithubEditLinkTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Extract the link text from arguments.
        // Usage: {% github_edit_link "Improve this page" %}
        let link_text = if let Ok(token) = arguments.expect_next("link text") {
            let raw = token.as_str().to_string();
            // Strip surrounding quotes if present
            if (raw.starts_with('"') && raw.ends_with('"'))
                || (raw.starts_with('\'') && raw.ends_with('\''))
            {
                raw[1..raw.len() - 1].to_string()
            } else {
                raw
            }
        } else {
            String::new()
        };
        consume_all_arguments(&mut arguments);
        Ok(Box::new(GithubEditLinkRenderable { link_text }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct GithubEditLinkRenderable {
    link_text: String,
}

impl std::fmt::Display for GithubEditLinkRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "github_edit_link")
    }
}

impl Renderable for GithubEditLinkRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        // Need repository_url and source.branch from site.github
        let repo_url = match get_nested_str(runtime, &["site", "github", "repository_url"]) {
            Some(url) => url,
            None => return Ok(()), // No github config -- produce empty output
        };

        let branch = get_nested_str(runtime, &["site", "github", "source", "branch"])
            .unwrap_or_else(|| "master".to_string());

        let page_path = get_nested_str(runtime, &["page", "path"]).unwrap_or_default();

        if self.link_text.is_empty() || page_path.is_empty() {
            return Ok(());
        }

        let repo_url = repo_url.trim_end_matches('/');
        let page_path = page_path.trim_start_matches('/');

        write!(
            writer,
            "<a href=\"{}/edit/{}/{}\">{}</a>",
            html_escape(repo_url),
            html_escape(&branch),
            html_escape(page_path),
            html_escape(&self.link_text),
        )
        .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Macro for no-op inline tags (produce empty output)
// ---------------------------------------------------------------------------

macro_rules! noop_inline_tag {
    ($struct_name:ident, $tag_name:expr, $desc:expr) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $struct_name;

        impl TagReflection for $struct_name {
            fn tag(&self) -> &'static str {
                $tag_name
            }
            fn description(&self) -> &'static str {
                $desc
            }
        }

        impl ParseTag for $struct_name {
            fn parse(
                &self,
                mut arguments: TagTokenIter<'_>,
                _options: &Language,
            ) -> liquid_core::Result<Box<dyn Renderable>> {
                consume_all_arguments(&mut arguments);
                Ok(Box::new(NoopRenderable))
            }
            fn reflection(&self) -> &dyn TagReflection {
                self
            }
        }
    };
}

/// Renderable that produces no output.
#[derive(Debug)]
struct NoopRenderable;

impl std::fmt::Display for NoopRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "noop")
    }
}

impl Renderable for NoopRenderable {
    fn render_to(
        &self,
        _writer: &mut dyn Write,
        _runtime: &dyn Runtime,
    ) -> liquid_core::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// No-op inline tags (jekyll-scholar and other plugins)
// ---------------------------------------------------------------------------

noop_inline_tag!(
    BibliographyTag,
    "bibliography",
    "No-op bibliography tag (jekyll-scholar)"
);
noop_inline_tag!(
    JupyterNotebookTag,
    "jupyter_notebook",
    "No-op jupyter_notebook tag"
);
noop_inline_tag!(SocialLinksTag, "social_links", "No-op social_links tag");
noop_inline_tag!(TwitterTag, "twitter", "No-op twitter tag");

// ---------------------------------------------------------------------------
// {% cite key1 key2 ... %} -- produces [key1, key2, ...]
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
pub struct CiteTag;

impl TagReflection for CiteTag {
    fn tag(&self) -> &'static str {
        "cite"
    }
    fn description(&self) -> &'static str {
        "Citation stub tag (jekyll-scholar)"
    }
}

impl ParseTag for CiteTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        let mut keys = Vec::new();
        while let Ok(token) = arguments.expect_next("") {
            keys.push(token.as_str().to_string());
        }
        Ok(Box::new(CiteRenderable { keys }))
    }
    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct CiteRenderable {
    keys: Vec<String>,
}

impl std::fmt::Display for CiteRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cite")
    }
}

impl Renderable for CiteRenderable {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> liquid_core::Result<()> {
        if self.keys.is_empty() {
            return Ok(());
        }
        write!(writer, "[{}]", self.keys.join(", "))
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// {% reference key %} -- produces [key]
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
pub struct ReferenceTag;

impl TagReflection for ReferenceTag {
    fn tag(&self) -> &'static str {
        "reference"
    }
    fn description(&self) -> &'static str {
        "Reference stub tag (jekyll-scholar)"
    }
}

impl ParseTag for ReferenceTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        let key = if let Ok(token) = arguments.expect_next("") {
            token.as_str().to_string()
        } else {
            String::new()
        };
        consume_all_arguments(&mut arguments);
        Ok(Box::new(ReferenceRenderable { key }))
    }
    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct ReferenceRenderable {
    key: String,
}

impl std::fmt::Display for ReferenceRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reference")
    }
}

impl Renderable for ReferenceRenderable {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> liquid_core::Result<()> {
        if self.key.is_empty() {
            return Ok(());
        }
        write!(writer, "[{}]", self.key)
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// {% quote [key] %}...{% endquote %} -- produces <blockquote>
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
pub struct QuoteBlock;

impl BlockReflection for QuoteBlock {
    fn start_tag(&self) -> &str {
        "quote"
    }
    fn end_tag(&self) -> &str {
        "endquote"
    }
    fn description(&self) -> &str {
        "Blockquote with optional citation (jekyll-scholar)"
    }
}

impl ParseBlock for QuoteBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut block: TagBlock<'_, '_>,
        options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Optional citation key argument
        let cite_key = if let Ok(token) = arguments.expect_next("") {
            Some(token.as_str().to_string())
        } else {
            None
        };
        consume_all_arguments(&mut arguments);

        // Parse the block body as Liquid template nodes
        let body = liquid_core::runtime::Template::new(block.parse_all(options)?);

        Ok(Box::new(QuoteRenderable { cite_key, body }))
    }
    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct QuoteRenderable {
    cite_key: Option<String>,
    body: liquid_core::Template,
}

impl std::fmt::Display for QuoteRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "quote")
    }
}

impl Renderable for QuoteRenderable {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        // Render body through Liquid
        let mut body_buf = Vec::new();
        self.body.render_to(&mut body_buf, runtime)?;
        let body_text = String::from_utf8_lossy(&body_buf).to_string();

        write!(writer, "<blockquote>{}", body_text.trim())
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        if let Some(ref key) = self.cite_key {
            write!(writer, "<cite>[{}]</cite>", html_escape(key))
                .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
        }

        write!(writer, "</blockquote>")
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// {% tabs group %}...{% endtabs %} / {% tab group name %}...{% endtab %}
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, Default)]
pub struct TabsBlock;

impl BlockReflection for TabsBlock {
    fn start_tag(&self) -> &str {
        "tabs"
    }
    fn end_tag(&self) -> &str {
        "endtabs"
    }
    fn description(&self) -> &str {
        "Tabbed content block (al-folio)"
    }
}

impl ParseBlock for TabsBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut block: TagBlock<'_, '_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // First argument is the group name
        let group = if let Ok(token) = arguments.expect_next("") {
            token.as_str().to_string()
        } else {
            "tabs".to_string()
        };
        consume_all_arguments(&mut arguments);

        // Get the raw body -- we'll parse tab blocks manually
        let body = block.escape_liquid(true)?.to_owned();

        Ok(Box::new(TabsRenderable { group, body }))
    }
    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct TabsRenderable {
    group: String,
    body: String,
}

impl std::fmt::Display for TabsRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tabs")
    }
}

/// Parse the raw body of a tabs block into (name, content) pairs.
///
/// Looks for `{% tab <group> <name> %}...{% endtab %}` sections.
fn parse_tab_sections(body: &str, group: &str) -> Vec<(String, String)> {
    let mut tabs = Vec::new();
    let mut remaining = body;

    // Pattern: {% tab <group> <name> %} ... {% endtab %}
    while let Some(start_idx) = remaining.find("{%") {
        let after_open = &remaining[start_idx + 2..];
        // Find the closing %}
        if let Some(close_idx) = after_open.find("%}") {
            let tag_content = after_open[..close_idx].trim();
            let parts: Vec<&str> = tag_content.split_whitespace().collect();

            // Check if this is a tab tag for our group
            if parts.len() >= 3 && parts[0] == "tab" && parts[1] == group {
                let tab_name = parts[2..].join(" ");
                let content_start = start_idx + 2 + close_idx + 2;
                let content_remaining = &remaining[content_start..];

                // Find matching {% endtab %}
                if let Some(end_idx) = find_endtab(content_remaining) {
                    let content = &content_remaining[..end_idx];
                    tabs.push((tab_name, content.trim().to_string()));
                    // Move past the {% endtab %}
                    let endtab_end = content_remaining[end_idx..]
                        .find("%}")
                        .map(|i| end_idx + i + 2)
                        .unwrap_or(content_remaining.len());
                    remaining = &remaining[content_start + endtab_end..];
                    continue;
                }
            }
        }
        // Not a tab tag, skip past this {% ... %}
        remaining = &remaining[start_idx + 2..];
    }

    tabs
}

/// Find the start position of `{% endtab %}` in the given text.
fn find_endtab(text: &str) -> Option<usize> {
    let mut search_from = 0;
    while search_from < text.len() {
        if let Some(idx) = text[search_from..].find("{%") {
            let abs_idx = search_from + idx;
            let after = &text[abs_idx + 2..];
            if let Some(close) = after.find("%}") {
                let tag = after[..close].trim();
                if tag == "endtab" {
                    return Some(abs_idx);
                }
            }
            search_from = abs_idx + 2;
        } else {
            break;
        }
    }
    None
}

impl Renderable for TabsRenderable {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let tabs = parse_tab_sections(&self.body, &self.group);
        if tabs.is_empty() {
            return Ok(());
        }

        let group_escaped = html_escape(&self.group);

        // Navigation: <ul id="group" class="tab" data-name="group">
        write!(
            writer,
            "<ul id=\"{}\" class=\"tab\" data-name=\"{}\">",
            group_escaped, group_escaped
        )
        .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        for (i, (name, _)) in tabs.iter().enumerate() {
            if i == 0 {
                write!(writer, "<li class=\"active\">{}</li>", html_escape(name))
            } else {
                write!(writer, "<li>{}</li>", html_escape(name))
            }
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
        }
        write!(writer, "</ul>")
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        // Content panels: <ul class="tab-content" data-name="group">
        write!(
            writer,
            "<ul class=\"tab-content\" data-name=\"{}\">",
            group_escaped
        )
        .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        for (i, (_, content)) in tabs.iter().enumerate() {
            // Render content through Markdown
            let rendered = crate::frontmatter::markdown_to_html(content);
            if i == 0 {
                write!(writer, "<li class=\"active\">{}</li>", rendered.trim())
            } else {
                write!(writer, "<li>{}</li>", rendered.trim())
            }
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;
        }
        write!(writer, "</ul>")
            .map_err(|e| liquid_core::Error::with_msg(format!("Write error: {}", e)))?;

        Ok(())
    }
}

/// The `{% tab %}` block is parsed inside `{% tabs %}` via raw body extraction.
/// This stub exists only so the Liquid parser does not reject `{% tab %}` when
/// encountered outside a `{% tabs %}` block (which should not happen in
/// well-formed content, but we handle it gracefully as a no-op).
#[derive(Copy, Clone, Debug, Default)]
pub struct TabBlock;

impl BlockReflection for TabBlock {
    fn start_tag(&self) -> &str {
        "tab"
    }
    fn end_tag(&self) -> &str {
        "endtab"
    }
    fn description(&self) -> &str {
        "Individual tab content block (al-folio)"
    }
}

impl ParseBlock for TabBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut block: TagBlock<'_, '_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        consume_all_arguments(&mut arguments);
        let _body = block.escape_liquid(true)?;
        Ok(Box::new(NoopRenderable))
    }
    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::model::Value;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_github_edit_link_no_output_without_context() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% github_edit_link %}", &ctx)
            .unwrap();
        assert_eq!(
            out, "",
            "github_edit_link should produce no output without context"
        );
    }

    #[test]
    fn test_github_edit_link_with_string_arg_no_github() {
        // With link text but no site.github context, should produce empty output
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(r#"{% github_edit_link "Help improve this page" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out, "",
            "github_edit_link with args but no github context should produce no output"
        );
    }

    #[test]
    fn test_github_edit_link_produces_link_with_context() {
        let eng = engine();

        let mut source = Object::new();
        source.insert("branch".into(), Value::scalar("master"));

        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            Value::scalar("https://github.com/pages-themes/primer"),
        );
        github.insert("source".into(), Value::Object(source));

        let mut site = Object::new();
        site.insert("github".into(), Value::Object(github));

        let mut page = Object::new();
        page.insert("path".into(), Value::scalar("index.md"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        ctx.insert("page".into(), Value::Object(page));

        let out = eng
            .parse_and_render(r#"{% github_edit_link "Improve this page" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out,
            r#"<a href="https://github.com/pages-themes/primer/edit/master/index.md">Improve this page</a>"#,
            "github_edit_link should produce correct anchor tag"
        );
    }

    #[test]
    fn test_github_edit_link_empty_repository_url() {
        // When repository_url is not present, produce empty output
        let eng = engine();

        let github = Object::new(); // no repository_url
        let mut site = Object::new();
        site.insert("github".into(), Value::Object(github));

        let mut page = Object::new();
        page.insert("path".into(), Value::scalar("index.md"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        ctx.insert("page".into(), Value::Object(page));

        let out = eng
            .parse_and_render(r#"{% github_edit_link "Edit" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out, "",
            "github_edit_link with empty repository_url should produce no output"
        );
    }

    #[test]
    fn test_github_edit_link_unicode_link_text() {
        let eng = engine();

        let mut source = Object::new();
        source.insert("branch".into(), Value::scalar("main"));

        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            Value::scalar("https://github.com/example/repo"),
        );
        github.insert("source".into(), Value::Object(source));

        let mut site = Object::new();
        site.insert("github".into(), Value::Object(github));

        let mut page = Object::new();
        page.insert("path".into(), Value::scalar("docs/page.md"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        ctx.insert("page".into(), Value::Object(page));

        let out = eng
            .parse_and_render(r#"{% github_edit_link "Seite verbessern" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out,
            r#"<a href="https://github.com/example/repo/edit/main/docs/page.md">Seite verbessern</a>"#,
            "github_edit_link should handle non-ASCII link text"
        );
    }

    #[test]
    fn test_github_edit_link_default_branch_master() {
        // When source.branch is not set, should default to "master"
        let eng = engine();

        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            Value::scalar("https://github.com/example/repo"),
        );
        // No source.branch set

        let mut site = Object::new();
        site.insert("github".into(), Value::Object(github));

        let mut page = Object::new();
        page.insert("path".into(), Value::scalar("README.md"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        ctx.insert("page".into(), Value::Object(page));

        let out = eng
            .parse_and_render(r#"{% github_edit_link "Edit" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out, r#"<a href="https://github.com/example/repo/edit/master/README.md">Edit</a>"#,
            "github_edit_link should default to master branch"
        );
    }

    #[test]
    fn test_github_edit_link_collection_item_path() {
        // Collection items (e.g., _licenses/mit.txt) need page.path set
        // to the full relative path including the collection directory prefix
        let eng = engine();

        let mut source = Object::new();
        source.insert("branch".into(), Value::scalar("gh-pages"));

        let mut github = Object::new();
        github.insert(
            "repository_url".into(),
            Value::scalar("https://github.com/github/choosealicense.com"),
        );
        github.insert("source".into(), Value::Object(source));

        let mut site = Object::new();
        site.insert("github".into(), Value::Object(github));

        let mut page = Object::new();
        page.insert("path".into(), Value::scalar("_licenses/mit.txt"));

        let mut ctx = Object::new();
        ctx.insert("site".into(), Value::Object(site));
        ctx.insert("page".into(), Value::Object(page));

        let out = eng
            .parse_and_render(r#"{% github_edit_link "Improve this page" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out,
            r#"<a href="https://github.com/github/choosealicense.com/edit/gh-pages/_licenses/mit.txt">Improve this page</a>"#,
            "github_edit_link should produce correct URL for collection item"
        );
    }

    // -----------------------------------------------------------------------
    // cite tag tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cite_single_key() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% cite mykey %}", &ctx).unwrap();
        assert_eq!(out, "[mykey]", "cite should produce [key]: {}", out);
    }

    #[test]
    fn test_cite_multiple_keys() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% cite key1 key2 key3 %}", &ctx)
            .unwrap();
        assert_eq!(
            out, "[key1, key2, key3]",
            "cite with multiple keys should produce [key1, key2, key3]: {}",
            out
        );
    }

    #[test]
    fn test_cite_no_arguments() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% cite %}", &ctx).unwrap();
        assert_eq!(out, "", "cite with no args should produce empty: {}", out);
    }

    #[test]
    fn test_cite_in_unicode_prose() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("Über die Arbeit von {% cite einstein1905 %} hinaus", &ctx)
            .unwrap();
        assert_eq!(
            out, "Über die Arbeit von [einstein1905] hinaus",
            "cite should work in Unicode prose: {}",
            out
        );
    }

    // -----------------------------------------------------------------------
    // reference tag tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_reference_single_key() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% reference mykey %}", &ctx).unwrap();
        assert_eq!(out, "[mykey]", "reference should produce [key]: {}", out);
    }

    #[test]
    fn test_reference_in_unicode_prose() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("Vgl. {% reference einstein1905 %} für Details", &ctx)
            .unwrap();
        assert_eq!(
            out, "Vgl. [einstein1905] für Details",
            "reference should work in Unicode prose: {}",
            out
        );
    }

    // -----------------------------------------------------------------------
    // quote block tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_quote_basic() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% quote %}Some text{% endquote %}", &ctx)
            .unwrap();
        assert!(
            out.contains("<blockquote>"),
            "quote should produce <blockquote>: {}",
            out
        );
        assert!(
            out.contains("Some text"),
            "quote should contain body text: {}",
            out
        );
        assert!(
            out.contains("</blockquote>"),
            "quote should close </blockquote>: {}",
            out
        );
    }

    #[test]
    fn test_quote_with_citation() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% quote einstein1905 %}Some text{% endquote %}", &ctx)
            .unwrap();
        assert!(
            out.contains("<blockquote>"),
            "quote should produce <blockquote>: {}",
            out
        );
        assert!(
            out.contains("<cite>[einstein1905]</cite>"),
            "quote with key should produce <cite>: {}",
            out
        );
    }

    #[test]
    fn test_quote_unicode_content() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% quote %}Über die Elektrodynamik bewegter Körper{% endquote %}",
                &ctx,
            )
            .unwrap();
        assert!(
            out.contains("Über die Elektrodynamik bewegter Körper"),
            "quote should preserve Unicode content: {}",
            out
        );
    }

    // -----------------------------------------------------------------------
    // tabs block tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_tabs_basic_structure() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% tabs mygroup %}{% tab mygroup Tab A %}Hello{% endtab %}{% tab mygroup Tab B %}World{% endtab %}{% endtabs %}",
                &ctx,
            )
            .unwrap();
        assert!(
            out.contains("<ul") && out.contains("class=\"tab\""),
            "tabs should produce <ul class=\"tab\">: {}",
            out
        );
        assert!(
            out.contains("Tab A"),
            "tabs should contain tab name 'Tab A': {}",
            out
        );
        assert!(
            out.contains("Tab B"),
            "tabs should contain tab name 'Tab B': {}",
            out
        );
        assert!(
            out.contains("Hello"),
            "tabs should contain tab content 'Hello': {}",
            out
        );
        assert!(
            out.contains("World"),
            "tabs should contain tab content 'World': {}",
            out
        );
        assert!(
            out.contains("class=\"tab-content\""),
            "tabs should produce tab-content section: {}",
            out
        );
    }

    #[test]
    fn test_tabs_first_active() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% tabs g %}{% tab g A %}First{% endtab %}{% tab g B %}Second{% endtab %}{% endtabs %}",
                &ctx,
            )
            .unwrap();
        // The first tab nav item should have class="active"
        // Find first <li in the tab nav section
        let tab_nav_start = out.find("class=\"tab\"").expect("should have tab nav");
        let after_nav = &out[tab_nav_start..];
        let first_li = after_nav.find("<li").expect("should have li in nav");
        let first_li_text = &after_nav[first_li..first_li + 80.min(after_nav.len() - first_li)];
        assert!(
            first_li_text.contains("active"),
            "First tab nav item should have active class: {}",
            first_li_text
        );
    }

    #[test]
    fn test_tabs_unicode_names() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "{% tabs g %}{% tab g Ünïcödé %}Inhalt{% endtab %}{% endtabs %}",
                &ctx,
            )
            .unwrap();
        assert!(
            out.contains("Ünïcödé"),
            "tabs should handle Unicode tab names: {}",
            out
        );
    }

    #[test]
    fn test_feed_meta_in_layout_context() {
        // feed_meta now emits a link tag, so it should appear in the output
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("<head>{% feed_meta %}<title>Test</title></head>", &ctx)
            .unwrap();
        assert!(
            out.contains("<link type=\"application/atom+xml\""),
            "feed_meta should produce link tag: {}",
            out
        );
        assert!(
            out.contains("<title>Test</title></head>"),
            "Rest of head should follow: {}",
            out
        );
    }
}

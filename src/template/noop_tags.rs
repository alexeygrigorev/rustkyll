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

// ---------------------------------------------------------------------------
// Macro for no-op block tags (produce empty output)
// ---------------------------------------------------------------------------

macro_rules! noop_block_tag {
    ($struct_name:ident, $start:expr, $end:expr, $desc:expr) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $struct_name;

        impl BlockReflection for $struct_name {
            fn start_tag(&self) -> &str {
                $start
            }
            fn end_tag(&self) -> &str {
                $end
            }
            fn description(&self) -> &str {
                $desc
            }
        }

        impl ParseBlock for $struct_name {
            fn parse(
                &self,
                mut arguments: TagTokenIter<'_>,
                mut block: TagBlock<'_, '_>,
                _options: &Language,
            ) -> liquid_core::Result<Box<dyn Renderable>> {
                consume_all_arguments(&mut arguments);
                // Consume block body without parsing
                let _body = block.escape_liquid(true)?;
                Ok(Box::new(NoopRenderable))
            }
            fn reflection(&self) -> &dyn BlockReflection {
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

noop_inline_tag!(CiteTag, "cite", "No-op cite tag (jekyll-scholar)");
noop_inline_tag!(
    ReferenceTag,
    "reference",
    "No-op reference tag (jekyll-scholar)"
);
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
noop_inline_tag!(
    BustFileCacheTag,
    "bust_file_cache",
    "No-op bust_file_cache tag"
);

// ---------------------------------------------------------------------------
// No-op block tags
// ---------------------------------------------------------------------------

noop_block_tag!(TabsBlock, "tabs", "endtabs", "No-op tabs block");
noop_block_tag!(TabBlock, "tab", "endtab", "No-op tab block");
noop_block_tag!(QuoteBlock, "quote", "endquote", "No-op quote block");

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

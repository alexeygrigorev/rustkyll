//! No-op tag stubs for Jekyll plugins whose output we cannot replicate
//! but whose presence would otherwise cause parse failures.
//!
//! Each tag consumes all arguments and renders nothing.  This lets
//! templates that use these tags parse and render successfully,
//! producing correct layout structure even though the specific
//! plugin output is absent.

use std::io::Write;

use liquid_core::{Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter};

// ---------------------------------------------------------------------------
// Generic no-op tag infrastructure
// ---------------------------------------------------------------------------

/// A no-op Liquid tag that consumes all arguments and renders nothing.
#[derive(Debug)]
struct NoopTagRenderable;

impl std::fmt::Display for NoopTagRenderable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "noop")
    }
}

impl Renderable for NoopTagRenderable {
    fn render_to(
        &self,
        _writer: &mut dyn Write,
        _runtime: &dyn Runtime,
    ) -> liquid_core::Result<()> {
        Ok(())
    }
}

/// Helper: drain all remaining tokens from the argument iterator.
fn consume_all_arguments(arguments: &mut TagTokenIter<'_>) {
    while arguments.expect_next("").is_ok() {}
}

// ---------------------------------------------------------------------------
// {% github_edit_link %} -- jekyll-github-metadata plugin
// ---------------------------------------------------------------------------

/// No-op `{% github_edit_link %}` tag (jekyll-github-metadata plugin).
///
/// In Jekyll this emits an edit-on-GitHub link.
/// We render nothing so that layouts containing this tag
/// can still be applied.
#[derive(Copy, Clone, Debug, Default)]
pub struct GithubEditLinkTag;

impl TagReflection for GithubEditLinkTag {
    fn tag(&self) -> &'static str {
        "github_edit_link"
    }

    fn description(&self) -> &'static str {
        "No-op stub for the jekyll-github-metadata {% github_edit_link %} tag"
    }
}

impl ParseTag for GithubEditLinkTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        consume_all_arguments(&mut arguments);
        Ok(Box::new(NoopTagRenderable))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_github_edit_link_no_output() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% github_edit_link %}", &ctx)
            .unwrap();
        assert_eq!(out, "", "github_edit_link should produce no output");
    }

    #[test]
    fn test_github_edit_link_with_string_arg() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(r#"{% github_edit_link "Help improve this page" %}"#, &ctx)
            .unwrap();
        assert_eq!(
            out, "",
            "github_edit_link with args should produce no output"
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

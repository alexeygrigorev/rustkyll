//! `{% details Summary %}...{% enddetails %}` block tag.
//!
//! Produces `<details><summary>Summary</summary>body</details>` HTML,
//! matching the al-folio Jekyll plugin (`_plugins/details.rb`).
//! The body content is rendered through Markdown conversion.

use std::io::Write;

use liquid_core::error::ResultLiquidReplaceExt;
use liquid_core::{
    BlockReflection, Language, ParseBlock, Renderable, Runtime, TagBlock, TagTokenIter,
};

/// The `{% details %}` block tag parser/reflection.
#[derive(Copy, Clone, Debug, Default)]
pub struct DetailsBlock;

impl BlockReflection for DetailsBlock {
    fn start_tag(&self) -> &str {
        "details"
    }

    fn end_tag(&self) -> &str {
        "enddetails"
    }

    fn description(&self) -> &str {
        "Collapsible details/summary block"
    }
}

impl ParseBlock for DetailsBlock {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        mut block: TagBlock<'_, '_>,
        options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        // Collect all argument tokens as the summary text
        let mut summary_parts = Vec::new();
        while let Ok(token) = arguments.expect_next("") {
            summary_parts.push(token.as_str().to_string());
        }
        let summary = summary_parts.join(" ");

        // Parse the block body as Liquid template nodes
        let body = liquid_core::runtime::Template::new(block.parse_all(options)?);

        Ok(Box::new(Details { summary, body }))
    }

    fn reflection(&self) -> &dyn BlockReflection {
        self
    }
}

#[derive(Debug)]
struct Details {
    summary: String,
    body: liquid_core::Template,
}

impl std::fmt::Display for Details {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "details")
    }
}

impl Renderable for Details {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        // Render summary through Markdown (strip wrapping <p> tags like Jekyll does)
        let summary_html = if self.summary.is_empty() {
            String::new()
        } else {
            let md_html = crate::frontmatter::markdown_to_html(&self.summary);
            // Strip wrapping <p>...</p> tags, matching Jekyll's gsub(/<\/?p[^>]*>/, '')
            strip_p_tags(&md_html).trim().to_string()
        };

        // Render body Liquid template
        let mut body_buf = Vec::new();
        self.body.render_to(&mut body_buf, runtime)?;
        let body_text = String::from_utf8_lossy(&body_buf).to_string();

        // Render body through Markdown
        let body_html = crate::frontmatter::markdown_to_html(&body_text);

        write!(
            writer,
            "<details><summary>{}</summary>{}</details>",
            summary_html, body_html
        )
        .replace("Failed to render")?;

        Ok(())
    }
}

/// Strip `<p>` and `</p>` tags from HTML, matching Jekyll's behavior.
fn strip_p_tags(html: &str) -> String {
    let mut result = html.to_string();
    // Remove opening <p> or <p ...> tags
    while let Some(start) = result.find("<p") {
        if let Some(end) = result[start..].find('>') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    // Remove closing </p> tags
    result = result.replace("</p>", "");
    result
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    fn render(template: &str) -> String {
        let eng = engine();
        let ctx = Object::new();
        eng.parse_and_render(template, &ctx).unwrap()
    }

    #[test]
    fn test_details_basic() {
        let output = render("{% details Summary text %}Body content{% enddetails %}");
        assert!(
            output.contains("<details>"),
            "Should contain <details>: {}",
            output
        );
        assert!(
            output.contains("<summary>Summary text</summary>"),
            "Should contain summary: {}",
            output
        );
        assert!(
            output.contains("</details>"),
            "Should contain </details>: {}",
            output
        );
        assert!(
            output.contains("Body content"),
            "Should contain body: {}",
            output
        );
    }

    #[test]
    fn test_details_empty_summary() {
        let output = render("{% details %}Some body{% enddetails %}");
        assert!(
            output.contains("<details>"),
            "Should contain <details>: {}",
            output
        );
        assert!(
            output.contains("<summary></summary>"),
            "Should contain empty summary: {}",
            output
        );
        assert!(
            output.contains("</details>"),
            "Should contain </details>: {}",
            output
        );
    }

    #[test]
    fn test_details_markdown_body() {
        let output = render("{% details Click me %}**bold** text{% enddetails %}");
        assert!(
            output.contains("<strong>bold</strong>"),
            "Body should render Markdown: {}",
            output
        );
    }

    #[test]
    fn test_details_unicode_summary() {
        let output = render("{% details Zusammenfassung %}Inhalt{% enddetails %}");
        assert!(
            output.contains("<summary>Zusammenfassung</summary>"),
            "Should handle Unicode summary: {}",
            output
        );
    }

    #[test]
    fn test_details_html_structure() {
        let output = render("{% details Summary %}Body{% enddetails %}");
        // Verify overall structure: <details><summary>...</summary>...</details>
        let details_start = output.find("<details>").expect("missing <details>");
        let summary_start = output.find("<summary>").expect("missing <summary>");
        let summary_end = output.find("</summary>").expect("missing </summary>");
        let details_end = output.find("</details>").expect("missing </details>");
        assert!(details_start < summary_start);
        assert!(summary_start < summary_end);
        assert!(summary_end < details_end);
    }
}

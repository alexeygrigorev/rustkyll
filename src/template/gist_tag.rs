//! `{% gist %}` tag support for the `jekyll-gist` plugin.
//!
//! The gist tag is handled during preprocessing (before the Liquid parser runs)
//! because its arguments (`user/id`, filenames with dots) are not valid Liquid
//! expressions. The tag is replaced with the equivalent HTML inline.
//!
//! ```liquid
//! {% gist 5555251 %}
//! {% gist 5555251 gist.md %}
//! {% gist parkr/931c1c8d465a04042403 %}
//! ```
//!
//! Output matches the `jekyll-gist` plugin:
//!
//! ```html
//! <noscript><pre>400: Invalid request</pre></noscript>
//! <script src="https://gist.github.com/<id>.js?file=<filename>"> </script>
//! ```

/// Render a gist embed given the raw arguments string after `gist `.
///
/// Returns the HTML `<noscript>` + `<script>` embed, or an empty string
/// if the arguments are empty.
pub fn render_gist(args: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return String::new();
    }

    let mut parts = args.split_whitespace();
    let gist_id = match parts.next() {
        Some(id) => id,
        None => return String::new(),
    };
    let filename = parts.next();

    let file_param = match filename {
        Some(f) => format!("?file={}", f),
        None => String::new(),
    };

    format!(
        "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/{}.js{}\"> </script>",
        gist_id, file_param
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use super::render_gist;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_render_gist_with_id_and_filename() {
        let html = render_gist("5555251 gist.md");
        assert_eq!(
            html,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/5555251.js?file=gist.md\"> </script>",
        );
    }

    #[test]
    fn test_render_gist_with_id_only() {
        let html = render_gist("abc123");
        assert_eq!(
            html,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/abc123.js\"> </script>",
        );
    }

    #[test]
    fn test_render_gist_user_slash_id() {
        let html = render_gist("parkr/931c1c8d465a04042403");
        assert_eq!(
            html,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/parkr/931c1c8d465a04042403.js\"> </script>",
        );
    }

    #[test]
    fn test_render_gist_empty() {
        assert_eq!(render_gist(""), "");
        assert_eq!(render_gist("  "), "");
    }

    #[test]
    fn test_gist_tag_in_template() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% gist 5555251 gist.md %}", &ctx)
            .unwrap();
        assert_eq!(
            out,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/5555251.js?file=gist.md\"> </script>",
            "gist tag with id and filename"
        );
    }

    #[test]
    fn test_gist_tag_id_only_in_template() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng.parse_and_render("{% gist abc123 %}", &ctx).unwrap();
        assert_eq!(
            out,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/abc123.js\"> </script>",
            "gist tag with id only"
        );
    }

    #[test]
    fn test_gist_tag_surrounding_content_preserved() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render(
                "<p>before</p>\n{% gist 5555251 gist.md %}\n<p>after</p>",
                &ctx,
            )
            .unwrap();
        assert!(out.contains("<p>before</p>"), "before preserved: {}", out);
        assert!(out.contains("<p>after</p>"), "after preserved: {}", out);
        assert!(
            out.contains("gist.github.com/5555251.js"),
            "gist present: {}",
            out
        );
    }

    #[test]
    fn test_gist_tag_user_slash_id_in_template() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% gist parkr/931c1c8d465a04042403 %}", &ctx)
            .unwrap();
        assert_eq!(
            out,
            "<noscript><pre>400: Invalid request</pre></noscript>\n<script src=\"https://gist.github.com/parkr/931c1c8d465a04042403.js\"> </script>",
            "gist tag with user/id format"
        );
    }

    #[test]
    fn test_gist_tag_unicode_filename_in_template() {
        let eng = engine();
        let ctx = Object::new();
        let out = eng
            .parse_and_render("{% gist abc123 t\u{00e9}st.md %}", &ctx)
            .unwrap();
        assert!(
            out.contains("?file=t\u{00e9}st.md"),
            "gist should handle unicode filename: {}",
            out
        );
    }
}

//! `{% octicon %}` tag support for the `jekyll-octicons` plugin.
//!
//! The octicon tag is handled during preprocessing (before the Liquid parser
//! runs) because its `key:value` argument syntax (e.g., `height:24`,
//! `class:"fill-green"`) is not valid Liquid syntax.
//!
//! The tag is replaced with the equivalent inline `<svg>` HTML.
//!
//! ```liquid
//! {% octicon mark-github height:24 class:"fill-gray-light d-inline" aria-label:github-logo %}
//! {% octicon check width:18 class:"fill-green" aria-label:check %}
//! {% octicon chevron-right height:18 class:"d-inline fill-blue ml-1" %}
//! ```

/// Static lookup table for octicon SVG path data.
/// All icons use viewBox "0 0 16 16" (the 16px variants from Octicons v2).
fn octicon_path(name: &str) -> Option<&'static str> {
    match name {
        "mark-github" => Some("M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"),
        "check" => Some("M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z"),
        "chevron-right" => Some("M6.22 3.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L9.94 8 6.22 4.28a.75.75 0 0 1 0-1.06Z"),
        "terminal" => Some("M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25Zm7.25 8a.75.75 0 0 1 .75-.75h1.5a.75.75 0 0 1 0 1.5h-1.5a.75.75 0 0 1-.75-.75Zm-7.25-1a.75.75 0 0 1 .22-.53l2.25-2.25a.75.75 0 0 1 1.06 0l.97.97.97-.97a.75.75 0 1 1 1.06 1.06l-1.5 1.5a.75.75 0 0 1-1.06 0l-.97-.97-1.72 1.72a.75.75 0 0 1-.53.22.75.75 0 0 1-.75-.75Z"),
        "server" => Some("M1.75 1h12.5c.966 0 1.75.784 1.75 1.75v4c0 .372-.116.717-.314 1 .198.283.314.628.314 1v4a1.75 1.75 0 0 1-1.75 1.75H1.75A1.75 1.75 0 0 1 0 12.75v-4c0-.372.116-.717.314-1A1.739 1.739 0 0 1 0 6.75v-4C0 1.784.784 1 1.75 1ZM1.5 2.75v4c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25v-4a.25.25 0 0 0-.25-.25H1.75a.25.25 0 0 0-.25.25Zm.25 5.75a.25.25 0 0 0-.25.25v4c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25v-4a.25.25 0 0 0-.25-.25ZM7 4.75A.75.75 0 0 1 7.75 4h4.5a.75.75 0 0 1 0 1.5h-4.5A.75.75 0 0 1 7 4.75ZM7.75 10h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM3 4.75A.75.75 0 0 1 3.75 4h.5a.75.75 0 0 1 0 1.5h-.5A.75.75 0 0 1 3 4.75ZM3.75 10h.5a.75.75 0 0 1 0 1.5h-.5a.75.75 0 0 1 0-1.5Z"),
        "beaker" => Some("M5 5.782V1h6v4.782l2.893 5.786A1.5 1.5 0 0 1 12.553 14H3.447a1.5 1.5 0 0 1-1.34-2.169Zm-1.4 7.218L6.1 7.784A.75.75 0 0 0 6.2 7.4V2.5h3.6v4.9a.75.75 0 0 0 .1.384L12.4 13Z"),
        "tools" => Some("M4.48 7.27c.26.26 1.28 1.33 1.28 1.33l.56-.58-.88-.91 1.75-1.8-.29-.3c-.31-.31-.85-.5-1.36-.5-.19 0-.38.03-.55.09 0 0-.71-1.11-.16-2.23C5.7 1.2 3.22.84 1.66 2.04 0 3.32-.16 5.47 1.52 6.63c.89.62 2.59.62 2.96.64ZM11.16 10 3.02 2.18a.752.752 0 0 0-1.06-.02.75.75 0 0 0-.02 1.06L9.94 11h.01l1.21-1Zm-8.78 3.38c-.19 0-.41-.04-.61-.11-.58-.21-.92-.57-1.03-.84-.24-.58.15-1.3 1.04-1.9L3.8 12.1c0 .04 0 .12.01.21.02.3.06.72-.22.98-.17.16-.43.09-.65.09h-.56Zm6.88-8.663 2.39-2.457a1.746 1.746 0 0 1 2.47 0 1.752 1.752 0 0 1 0 2.474L11.88 7.19l-1.03-1.004-.07-.068-.02-.02-.05-.05Zm1.99 4.873L8.68 7.078l.04-.04 1.06 1.034L12 5.849l1.06 1.034-1.74 1.787L12.64 10l-1.37 1.39Z"),
        "code" => Some("m11.28 3.22 4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.749.749 0 0 1-1.275-.326.749.749 0 0 1 .215-.734L13.94 8l-3.72-3.72a.749.749 0 0 1 .326-1.275.749.749 0 0 1 .734.215Zm-6.56 0a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042L2.06 8l3.72 3.72a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L.47 8.53a.75.75 0 0 1 0-1.06Z"),
        "lock" => Some("M4 4a4 4 0 0 1 8 0v2h.25c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 12.25 15h-8.5A1.75 1.75 0 0 1 2 13.25v-5.5C2 6.784 2.784 6 3.75 6H4Zm8.25 3.5h-8.5a.25.25 0 0 0-.25.25v5.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-5.5a.25.25 0 0 0-.25-.25ZM10.5 6V4a2.5 2.5 0 1 0-5 0v2Z"),
        "globe" => Some("M1.543 7.25h2.733c.144-2.074.866-3.756 1.58-4.948.12-.197.237-.381.348-.55a6.51 6.51 0 0 0-4.662 5.498Zm2.733 1.5H1.543a6.51 6.51 0 0 0 4.662 5.498 11.345 11.345 0 0 1-.348-.55c-.714-1.192-1.437-2.874-1.58-4.948Zm1.504 0h4.44a9.637 9.637 0 0 1-1.363 4.177c-.306.51-.612.919-.857 1.215a9.978 9.978 0 0 1-.857-1.215A9.637 9.637 0 0 1 5.78 8.75Zm4.44-1.5H5.78a9.637 9.637 0 0 1 1.363-4.177c.306-.51.612-.919.857-1.215.245.296.55.705.857 1.215A9.638 9.638 0 0 1 10.22 7.25Zm1.504 1.5c-.144 2.074-.866 3.756-1.58 4.948-.12.197-.237.381-.348.55a6.51 6.51 0 0 0 4.662-5.498Zm0-1.5a6.51 6.51 0 0 0-4.662-5.498c.121.169.239.353.348.55.714 1.192 1.437 2.874 1.58 4.948ZM8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0Z"),
        "checklist" => Some("M2.5 1.75v11.5c0 .138.112.25.25.25h3.17a.75.75 0 0 1 0 1.5H2.75A1.75 1.75 0 0 1 1 13.25V1.75C1 .784 1.784 0 2.75 0h8.5C12.216 0 13 .784 13 1.75v7.736a.75.75 0 0 1-1.5 0V1.75a.25.25 0 0 0-.25-.25h-8.5a.25.25 0 0 0-.25.25Zm5.75 3.5h-4a.75.75 0 0 1 0-1.5h4a.75.75 0 0 1 0 1.5ZM4.5 6.75a.75.75 0 0 1 .75-.75h4a.75.75 0 0 1 0 1.5h-4a.75.75 0 0 1-.75-.75Zm8.28 2.97a.75.75 0 0 1 1.06 0l1.97 1.97 3.47-3.47a.75.75 0 0 1 1.06 1.06l-4 4a.75.75 0 0 1-1.06 0l-2.5-2.5a.75.75 0 0 1 0-1.06Z"),
        "person" => Some("M10.561 8.073a6.005 6.005 0 0 1 3.432 5.142.75.75 0 1 1-1.498.07 4.5 4.5 0 0 0-8.99 0 .75.75 0 0 1-1.498-.07 6.004 6.004 0 0 1 3.431-5.142 3.999 3.999 0 1 1 5.123 0ZM10.5 5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z"),
        "book" => Some("M0 1.75A.75.75 0 0 1 .75 1h4.253c1.227 0 2.317.59 3 1.501A3.743 3.743 0 0 1 11.006 1h4.245a.75.75 0 0 1 .75.75v10.5a.75.75 0 0 1-.75.75h-4.507a2.25 2.25 0 0 0-1.591.659l-.622.621a.75.75 0 0 1-1.06 0l-.622-.621A2.25 2.25 0 0 0 5.258 13H.75a.75.75 0 0 1-.75-.75Zm7.251 10.324.004-5.073-.002-2.253A2.25 2.25 0 0 0 5.003 2.5H1.5v9h3.757a3.75 3.75 0 0 1 1.994.574ZM8.755 4.75l-.004 7.322a3.752 3.752 0 0 1 1.992-.572H14.5v-9h-3.495a2.25 2.25 0 0 0-2.25 2.25Z"),
        _ => None,
    }
}

/// Render the SVG HTML for an octicon tag given the raw arguments string.
///
/// Returns the inline `<svg>` element, or an empty string for unknown icons.
pub fn render_octicon(args: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return String::new();
    }

    // Parse arguments: icon-name followed by key:value pairs
    // e.g. "mark-github height:24 class:\"fill-gray-light d-inline\" aria-label:github-logo"
    let mut icon_name = String::new();
    let mut height: Option<u32> = None;
    let mut width: Option<u32> = None;
    let mut class: Option<String> = None;
    let mut aria_label: Option<String> = None;

    // Tokenize with quote awareness
    let mut tokens: Vec<String> = Vec::new();
    let mut chars = args.chars().peekable();
    let mut current = String::new();

    while let Some(&ch) = chars.peek() {
        if ch == '"' {
            // Quoted string -- consume until closing quote, include content in current token
            chars.next(); // skip opening quote
            while let Some(&c) = chars.peek() {
                if c == '"' {
                    chars.next(); // skip closing quote
                    break;
                }
                current.push(c);
                chars.next();
            }
        } else if ch == ' ' || ch == '\t' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            chars.next();
        } else {
            current.push(ch);
            chars.next();
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    // First token is the icon name
    if let Some(name) = tokens.first() {
        icon_name.clone_from(name);
    }

    // Remaining tokens are key:value pairs
    for token in tokens.iter().skip(1) {
        if let Some(colon_pos) = token.find(':') {
            let key = &token[..colon_pos];
            let value = &token[colon_pos + 1..];
            // Strip surrounding quotes from value if present
            let value = value.trim_start_matches('"').trim_end_matches('"');
            match key {
                "height" => height = value.parse().ok(),
                "width" => width = value.parse().ok(),
                "class" => class = Some(value.to_string()),
                "aria-label" => aria_label = Some(value.to_string()),
                _ => {}
            }
        }
    }

    let path_data = match octicon_path(&icon_name) {
        Some(p) => p,
        None => return String::new(), // Unknown icon -- graceful degradation
    };

    // Determine dimensions: if only one of height/width is given, use it for both.
    // Default to 16 if neither is specified.
    let (w, h) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w),
        (None, Some(h)) => (h, h),
        (None, None) => (16, 16),
    };

    // Build class: always starts with "octicon octicon-{name}"
    let class_str = match &class {
        Some(user_class) => format!("octicon octicon-{} {}", icon_name, user_class),
        None => format!("octicon octicon-{}", icon_name),
    };

    let aria_attr = match &aria_label {
        Some(label) => format!(" aria-label=\"{}\"", label),
        None => String::new(),
    };

    // Build the SVG element matching Jekyll's jekyll-octicons output format.
    // Jekyll outputs dimension attributes in a specific order:
    // - If width was specified (and height was not), width comes first
    // - Otherwise height comes first
    let dimension_first_is_width = width.is_some() && height.is_none();

    if dimension_first_is_width {
        format!(
            "<svg width=\"{}\" class=\"{}\"{} viewBox=\"0 0 16 16\" version=\"1.1\" height=\"{}\" role=\"img\"><path d=\"{}\"></path></svg>",
            w, class_str, aria_attr, h, path_data
        )
    } else {
        format!(
            "<svg height=\"{}\" class=\"{}\"{} viewBox=\"0 0 16 16\" version=\"1.1\" width=\"{}\" role=\"img\"><path d=\"{}\"></path></svg>",
            h, class_str, aria_attr, w, path_data
        )
    }
}

/// Preprocess `{% octicon ... %}` tags in a template, replacing them with
/// inline SVG HTML before the Liquid parser sees them.
pub fn preprocess_octicon_tags(template: &str) -> String {
    let mut result = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{%") {
        result.push_str(&remaining[..start]);

        let after_open = &remaining[start + 2..];
        if let Some(end) = after_open.find("%}") {
            let tag_content = after_open[..end].trim();

            // Check for whitespace-control markers
            let tag_content = tag_content.strip_prefix('-').unwrap_or(tag_content).trim();
            let tag_content = tag_content.strip_suffix('-').unwrap_or(tag_content).trim();

            if let Some(args) = tag_content.strip_prefix("octicon") {
                // Make sure "octicon" is followed by whitespace or end (not "octicons" etc.)
                if args.is_empty() || args.starts_with(char::is_whitespace) {
                    let svg = render_octicon(args);
                    result.push_str(&svg);
                    remaining = &after_open[end + 2..];
                    continue;
                }
            }

            // Not an octicon tag -- pass through unchanged
            result.push_str(&remaining[start..start + 2 + end + 2]);
            remaining = &after_open[end + 2..];
        } else {
            // No closing %} -- pass through the rest
            result.push_str(remaining);
            return result;
        }
    }

    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_octicon_mark_github() {
        let svg = render_octicon(
            r#"mark-github height:24 class:"fill-gray-light d-inline" aria-label:github-logo"#,
        );
        let expected = r#"<svg height="24" class="octicon octicon-mark-github fill-gray-light d-inline" aria-label="github-logo" viewBox="0 0 16 16" version="1.1" width="24" role="img"><path d="M8 0c4.42 0 8 3.58 8 8a8.013 8.013 0 0 1-5.45 7.59c-.4.08-.55-.17-.55-.38 0-.27.01-1.13.01-2.2 0-.75-.25-1.23-.54-1.48 1.78-.2 3.65-.88 3.65-3.95 0-.88-.31-1.59-.82-2.15.08-.2.36-1.02-.08-2.12 0 0-.67-.22-2.2.82-.64-.18-1.32-.27-2-.27-.68 0-1.36.09-2 .27-1.53-1.03-2.2-.82-2.2-.82-.44 1.1-.16 1.92-.08 2.12-.51.56-.82 1.28-.82 2.15 0 3.06 1.86 3.75 3.64 3.95-.23.2-.44.55-.51 1.07-.46.21-1.61.55-2.33-.66-.15-.24-.6-.83-1.23-.82-.67.01-.27.38.01.53.34.19.73.9.82 1.13.16.45.68 1.31 2.69.94 0 .67.01 1.3.01 1.49 0 .21-.15.45-.55.38A7.995 7.995 0 0 1 0 8c0-4.42 3.58-8 8-8Z"></path></svg>"#;
        assert_eq!(
            svg, expected,
            "Should match Jekyll cached mark-github output"
        );
    }

    #[test]
    fn test_render_octicon_check_width() {
        let svg = render_octicon(r#"check width:18 class:"fill-green" aria-label:check"#);
        let expected = r#"<svg width="18" class="octicon octicon-check fill-green" aria-label="check" viewBox="0 0 16 16" version="1.1" height="18" role="img"><path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z"></path></svg>"#;
        assert_eq!(svg, expected, "Should match Jekyll cached check output");
    }

    #[test]
    fn test_render_octicon_book() {
        let svg = render_octicon(
            r#"book height:22 class:"fill-gray-light d-inline mr-2" aria-label:book"#,
        );
        let expected = r#"<svg height="22" class="octicon octicon-book fill-gray-light d-inline mr-2" aria-label="book" viewBox="0 0 16 16" version="1.1" width="22" role="img"><path d="M0 1.75A.75.75 0 0 1 .75 1h4.253c1.227 0 2.317.59 3 1.501A3.743 3.743 0 0 1 11.006 1h4.245a.75.75 0 0 1 .75.75v10.5a.75.75 0 0 1-.75.75h-4.507a2.25 2.25 0 0 0-1.591.659l-.622.621a.75.75 0 0 1-1.06 0l-.622-.621A2.25 2.25 0 0 0 5.258 13H.75a.75.75 0 0 1-.75-.75Zm7.251 10.324.004-5.073-.002-2.253A2.25 2.25 0 0 0 5.003 2.5H1.5v9h3.757a3.75 3.75 0 0 1 1.994.574ZM8.755 4.75l-.004 7.322a3.752 3.752 0 0 1 1.992-.572H14.5v-9h-3.495a2.25 2.25 0 0 0-2.25 2.25Z"></path></svg>"#;
        assert_eq!(svg, expected, "Should match Jekyll cached book output");
    }

    #[test]
    fn test_render_octicon_unknown_empty() {
        let svg = render_octicon("nonexistent-icon height:24");
        assert_eq!(svg, "", "Unknown octicon should produce empty output");
    }

    #[test]
    fn test_render_octicon_no_aria_label() {
        let svg = render_octicon(r#"chevron-right height:18 class:"d-inline fill-blue ml-1""#);
        assert!(svg.contains("height=\"18\""), "Should have height: {}", svg);
        assert!(
            svg.contains("octicon octicon-chevron-right d-inline fill-blue ml-1"),
            "Should have correct class: {}",
            svg
        );
        assert!(
            !svg.contains("aria-label"),
            "Should not have aria-label: {}",
            svg
        );
    }

    #[test]
    fn test_all_government_github_icons_have_paths() {
        let icons = [
            "mark-github",
            "check",
            "chevron-right",
            "terminal",
            "server",
            "beaker",
            "tools",
            "code",
            "lock",
            "globe",
            "checklist",
            "person",
            "book",
        ];
        for icon in &icons {
            let svg = render_octicon(&format!("{} height:16", icon));
            assert!(
                svg.contains("<svg"),
                "Icon '{}' should produce SVG output, got: {}",
                icon,
                svg
            );
            assert!(
                svg.contains("<path d="),
                "Icon '{}' should have path data, got: {}",
                icon,
                svg
            );
        }
    }

    #[test]
    fn test_preprocess_octicon_tags_in_template() {
        let template = r#"<a>{% octicon mark-github height:24 class:"fill-gray-light" aria-label:github %}</a>"#;
        let result = preprocess_octicon_tags(template);
        assert!(
            result.starts_with("<a><svg"),
            "SVG should be inside anchor: {}",
            result
        );
        assert!(
            result.ends_with("</svg></a>"),
            "SVG should close before anchor: {}",
            result
        );
        assert!(
            result.contains("octicon-mark-github"),
            "Should have octicon class: {}",
            result
        );
    }

    #[test]
    fn test_preprocess_preserves_non_octicon_tags() {
        let template = "{% if true %}yes{% endif %}{% octicon check height:16 %}done";
        let result = preprocess_octicon_tags(template);
        assert!(
            result.starts_with("{% if true %}yes{% endif %}"),
            "if/endif preserved: {}",
            result
        );
        assert!(result.contains("<svg"), "octicon rendered: {}", result);
        assert!(
            result.ends_with("done"),
            "trailing text preserved: {}",
            result
        );
    }

    #[test]
    fn test_preprocess_multiple_octicon_tags() {
        let template = "{% octicon check height:16 %}{% octicon lock height:16 %}";
        let result = preprocess_octicon_tags(template);
        assert_eq!(
            result.matches("<svg").count(),
            2,
            "Should have 2 SVGs: {}",
            result
        );
    }

    #[test]
    fn test_preprocess_octicon_whitespace_control() {
        let template = "{%- octicon check height:16 -%}";
        let result = preprocess_octicon_tags(template);
        assert!(
            result.contains("<svg"),
            "Should render SVG with dash markers: {}",
            result
        );
    }

    #[test]
    fn test_render_octicon_empty_args() {
        let svg = render_octicon("");
        assert_eq!(svg, "", "Empty args should produce empty output");
    }

    #[test]
    fn test_render_octicon_default_dimensions() {
        let svg = render_octicon("check");
        assert!(
            svg.contains("height=\"16\""),
            "Default height should be 16: {}",
            svg
        );
        assert!(
            svg.contains("width=\"16\""),
            "Default width should be 16: {}",
            svg
        );
    }

    #[test]
    fn test_preprocess_octicon_with_unicode_surroundings() {
        let template = "<p>Z\u{00fc}rich</p>{% octicon check height:16 %}<p>caf\u{00e9}</p>";
        let result = preprocess_octicon_tags(template);
        assert!(
            result.contains("Z\u{00fc}rich"),
            "Unicode before tag preserved: {}",
            result
        );
        assert!(
            result.contains("caf\u{00e9}"),
            "Unicode after tag preserved: {}",
            result
        );
        assert!(result.contains("<svg"), "SVG rendered: {}", result);
    }
}

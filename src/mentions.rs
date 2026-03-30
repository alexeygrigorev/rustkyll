//! Jekyll-mentions plugin: convert `@username` patterns to GitHub profile links.
//!
//! This module implements the `jekyll-mentions` plugin behavior. When the plugin
//! is listed in a site's `plugins` or `gems` configuration, `@username` patterns
//! in rendered HTML are converted to `<a href="https://github.com/username" class="user-mention">@username</a>`.
//!
//! Mentions inside `<code>`, `<pre>`, and `<a>` tags are NOT converted.
//! Email addresses (e.g., `user@example.com`) are NOT converted.

use crate::config::SiteConfig;

/// Default base URL for GitHub profiles.
const DEFAULT_BASE_URL: &str = "https://github.com";

/// Check if the site has `jekyll-mentions` in its `plugins` or `gems` list.
pub fn has_mentions_plugin(config: &SiteConfig) -> bool {
    for key in &["plugins", "gems"] {
        if let Some(val) = config.extras.get(*key) {
            if let Some(seq) = val.as_sequence() {
                if seq
                    .iter()
                    .any(|v| v.as_str().map(|s| s == "jekyll-mentions").unwrap_or(false))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Get the configured base URL for mentions, defaulting to `https://github.com`.
///
/// The config key is `jekyll-mentions` with a `base_url` sub-key:
/// ```yaml
/// jekyll-mentions:
///   base_url: https://gitlab.com
/// ```
pub fn mentions_base_url(config: &SiteConfig) -> &str {
    if let Some(val) = config.extras.get("jekyll-mentions") {
        if let Some(map) = val.as_mapping() {
            for (k, v) in map {
                if k.as_str() == Some("base_url") {
                    if let Some(url) = v.as_str() {
                        return url;
                    }
                }
            }
        }
    }
    DEFAULT_BASE_URL
}

/// Process HTML content, replacing `@username` patterns with GitHub profile links.
///
/// This function:
/// - Skips content inside `<code>`, `<pre>`, and `<a>` tags
/// - Does not replace email addresses (`word@username`)
/// - Replaces `@username` where username is `[a-zA-Z0-9][a-zA-Z0-9-]*`
pub fn process_mentions(html: &str, base_url: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut chars = html.char_indices().peekable();
    let bytes = html.as_bytes();
    let len = bytes.len();

    // Track nesting depth of tags where mentions should not be processed.
    let mut skip_depth: usize = 0;

    while let Some(&(i, ch)) = chars.peek() {
        if ch == '<' {
            // Parse the tag
            if let Some(tag_end) = find_tag_end(html, i) {
                let tag_content = &html[i..=tag_end];
                let tag_lower = tag_content.to_ascii_lowercase();

                if is_opening_skip_tag(&tag_lower) {
                    skip_depth += 1;
                } else if is_closing_skip_tag(&tag_lower) {
                    skip_depth = skip_depth.saturating_sub(1);
                }

                result.push_str(tag_content);
                // Advance past the tag
                while let Some(&(j, _)) = chars.peek() {
                    if j > tag_end {
                        break;
                    }
                    chars.next();
                }
                continue;
            }
        }

        if skip_depth > 0 {
            result.push(ch);
            chars.next();
            continue;
        }

        // Look for @username pattern
        if ch == '@' {
            // Check that this is not preceded by an alphanumeric character (email address)
            let preceded_by_alnum = if i > 0 {
                html.as_bytes()[i - 1].is_ascii_alphanumeric()
                    || html.as_bytes()[i - 1] == b'.'
                    || html.as_bytes()[i - 1] == b'_'
            } else {
                false
            };

            if !preceded_by_alnum {
                // Try to parse username: [a-zA-Z0-9][a-zA-Z0-9-]*
                let after_at = i + 1;
                if after_at < len && (bytes[after_at].is_ascii_alphanumeric()) {
                    let username_start = after_at;
                    let mut username_end = username_start;
                    while username_end < len
                        && (bytes[username_end].is_ascii_alphanumeric()
                            || bytes[username_end] == b'-')
                    {
                        username_end += 1;
                    }
                    // Username must not end with a hyphen (GitHub restriction)
                    let mut actual_end = username_end;
                    while actual_end > username_start && bytes[actual_end - 1] == b'-' {
                        actual_end -= 1;
                    }
                    // Skip team mentions like @jekyll/core (slash after username)
                    let followed_by_slash = actual_end < len && bytes[actual_end] == b'/';
                    if actual_end > username_start && !followed_by_slash {
                        let username = &html[username_start..actual_end];
                        let trimmed_base = base_url.trim_end_matches('/');
                        result.push_str(&format!(
                            "<a href=\"{}/{}\" class=\"user-mention\">@{}</a>",
                            trimmed_base, username, username
                        ));
                        // Advance chars past the username
                        while let Some(&(j, _)) = chars.peek() {
                            if j >= actual_end {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
            }
        }

        result.push(ch);
        chars.next();
    }

    result
}

/// Find the index of the closing `>` for a tag starting at `start`.
fn find_tag_end(html: &str, start: usize) -> Option<usize> {
    let bytes = html.as_bytes();
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

/// Check if a lowercase tag string is an opening tag for code/pre/a/script.
fn is_opening_skip_tag(tag: &str) -> bool {
    is_tag_match(tag, "<code", 5)
        || is_tag_match(tag, "<pre", 4)
        || is_tag_match(tag, "<a", 2)
        || is_tag_match(tag, "<script", 7)
}

/// Check if `tag` starts with `prefix` and the character after the prefix
/// (if any) is a space or `>` (i.e., it's a real tag, not a prefix of something else).
fn is_tag_match(tag: &str, prefix: &str, prefix_len: usize) -> bool {
    tag.starts_with(prefix)
        && (tag.len() == prefix_len
            || tag
                .as_bytes()
                .get(prefix_len)
                .is_some_and(|&b| b == b' ' || b == b'>'))
}

/// Check if a lowercase tag string is a closing tag for code/pre/a/script.
fn is_closing_skip_tag(tag: &str) -> bool {
    tag.starts_with("</code>")
        || tag.starts_with("</pre>")
        || tag.starts_with("</a>")
        || tag.starts_with("</script>")
}

/// Apply mentions processing to HTML if the plugin is enabled in config.
///
/// Returns the HTML unchanged if `jekyll-mentions` is not in the plugins list.
pub fn apply_mentions_if_enabled(html: &str, config: &SiteConfig) -> String {
    if has_mentions_plugin(config) {
        let base_url = mentions_base_url(config);
        process_mentions(html, base_url)
    } else {
        html.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config_with_plugins(plugins: Vec<&str>) -> SiteConfig {
        let mut extras = HashMap::new();
        let plugins_val: Vec<serde_yaml::Value> = plugins
            .into_iter()
            .map(|s| serde_yaml::Value::String(s.to_string()))
            .collect();
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(plugins_val),
        );
        SiteConfig {
            extras,
            ..SiteConfig::default()
        }
    }

    fn make_config_with_gems(gems: Vec<&str>) -> SiteConfig {
        let mut extras = HashMap::new();
        let gems_val: Vec<serde_yaml::Value> = gems
            .into_iter()
            .map(|s| serde_yaml::Value::String(s.to_string()))
            .collect();
        extras.insert("gems".to_string(), serde_yaml::Value::Sequence(gems_val));
        SiteConfig {
            extras,
            ..SiteConfig::default()
        }
    }

    fn make_config_with_base_url(base_url: &str) -> SiteConfig {
        let mut extras = HashMap::new();
        let plugins_val: Vec<serde_yaml::Value> =
            vec![serde_yaml::Value::String("jekyll-mentions".to_string())];
        extras.insert(
            "plugins".to_string(),
            serde_yaml::Value::Sequence(plugins_val),
        );
        let mut mentions_map = serde_yaml::Mapping::new();
        mentions_map.insert(
            serde_yaml::Value::String("base_url".to_string()),
            serde_yaml::Value::String(base_url.to_string()),
        );
        extras.insert(
            "jekyll-mentions".to_string(),
            serde_yaml::Value::Mapping(mentions_map),
        );
        SiteConfig {
            extras,
            ..SiteConfig::default()
        }
    }

    // --- Plugin detection tests ---

    #[test]
    fn test_mentions_plugin_detected_in_plugins() {
        let config = make_config_with_plugins(vec!["jekyll-mentions", "jekyll-feed"]);
        assert!(has_mentions_plugin(&config));
    }

    #[test]
    fn test_mentions_plugin_detected_in_gems() {
        let config = make_config_with_gems(vec!["jekyll-mentions"]);
        assert!(has_mentions_plugin(&config));
    }

    #[test]
    fn test_mentions_plugin_not_detected_when_absent() {
        let config = make_config_with_plugins(vec!["jekyll-feed", "jekyll-seo-tag"]);
        assert!(!has_mentions_plugin(&config));
    }

    #[test]
    fn test_mentions_plugin_not_detected_empty_config() {
        let config = SiteConfig::default();
        assert!(!has_mentions_plugin(&config));
    }

    // --- Base URL tests ---

    #[test]
    fn test_default_base_url() {
        let config = make_config_with_plugins(vec!["jekyll-mentions"]);
        assert_eq!(mentions_base_url(&config), "https://github.com");
    }

    #[test]
    fn test_custom_base_url() {
        let config = make_config_with_base_url("https://gitlab.com");
        assert_eq!(mentions_base_url(&config), "https://gitlab.com");
    }

    // --- Mention replacement tests ---

    #[test]
    fn test_simple_mention() {
        let html = "<p>Hello @jekyllbot world</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result,
            "<p>Hello <a href=\"https://github.com/jekyllbot\" class=\"user-mention\">@jekyllbot</a> world</p>"
        );
    }

    #[test]
    fn test_email_not_replaced() {
        let html = "<p>Contact user@example.com for info</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(result, html, "Email addresses must not be converted");
    }

    #[test]
    fn test_mention_inside_code_not_replaced() {
        let html = "<p>Use <code>@username</code> in your config</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(result, html, "Mentions inside <code> must not be converted");
    }

    #[test]
    fn test_mention_inside_pre_not_replaced() {
        let html = "<pre>@username should stay</pre>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(result, html, "Mentions inside <pre> must not be converted");
    }

    #[test]
    fn test_mention_inside_anchor_not_replaced() {
        let html = "<a href=\"/profile\">@username</a>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(result, html, "Mentions inside <a> must not be converted");
    }

    #[test]
    fn test_multiple_mentions() {
        let html = "<p>@alice and @bob worked together</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert!(result.contains("href=\"https://github.com/alice\""));
        assert!(result.contains("href=\"https://github.com/bob\""));
        assert!(result.contains("class=\"user-mention\">@alice</a>"));
        assert!(result.contains("class=\"user-mention\">@bob</a>"));
    }

    #[test]
    fn test_hyphenated_username() {
        let html = "<p>@hyphenated-name is valid</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result,
            "<p><a href=\"https://github.com/hyphenated-name\" class=\"user-mention\">@hyphenated-name</a> is valid</p>"
        );
    }

    #[test]
    fn test_unicode_context() {
        let html = "<p>Erstellt von @jekyllbot f\u{00fc}r alle</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert!(
            result.contains("class=\"user-mention\">@jekyllbot</a>"),
            "Mention should be converted in Unicode context"
        );
        assert!(
            result.contains("Erstellt von"),
            "Surrounding Unicode text should be preserved"
        );
        assert!(
            result.contains("f\u{00fc}r alle"),
            "Surrounding Unicode text should be preserved"
        );
    }

    #[test]
    fn test_mention_at_start_of_text() {
        let html = "<p>@jekyllbot is a bot</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert!(result.contains("class=\"user-mention\">@jekyllbot</a>"));
    }

    #[test]
    fn test_custom_base_url_in_link() {
        let html = "<p>@user</p>";
        let result = process_mentions(html, "https://gitlab.com");
        assert_eq!(
            result,
            "<p><a href=\"https://gitlab.com/user\" class=\"user-mention\">@user</a></p>"
        );
    }

    #[test]
    fn test_apply_mentions_disabled() {
        let config = make_config_with_plugins(vec!["jekyll-feed"]);
        let html = "<p>@user should not be converted</p>";
        let result = apply_mentions_if_enabled(html, &config);
        assert_eq!(
            result, html,
            "Mentions should not be processed when plugin is not enabled"
        );
    }

    #[test]
    fn test_apply_mentions_enabled() {
        let config = make_config_with_plugins(vec!["jekyll-mentions"]);
        let html = "<p>@user should be converted</p>";
        let result = apply_mentions_if_enabled(html, &config);
        assert!(result.contains("class=\"user-mention\">@user</a>"));
    }

    #[test]
    fn test_nested_code_in_pre() {
        let html = "<pre><code>@user</code></pre>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result, html,
            "Mentions in nested code/pre must not be converted"
        );
    }

    #[test]
    fn test_mention_after_closing_code() {
        let html = "<p><code>code</code> @user</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert!(
            result.contains("class=\"user-mention\">@user</a>"),
            "Mention after closing code tag should be converted"
        );
    }

    #[test]
    fn test_dot_before_at_not_converted() {
        // foo.@bar should not convert (email-like pattern)
        let html = "<p>foo.@bar</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(result, html, "Dot before @ should prevent conversion");
    }

    #[test]
    fn test_mention_with_code_attributes() {
        let html = "<code class=\"highlight\">@user</code>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result, html,
            "Mentions in code with attributes must not be converted"
        );
    }

    #[test]
    fn test_mention_inside_script_not_replaced() {
        let html = r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"BlogPosting"}</script>"#;
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result, html,
            "Mentions inside <script> (JSON-LD) must not be converted"
        );
    }

    #[test]
    fn test_mention_after_script_is_replaced() {
        let html = r#"<script>var x = "@skip";</script><p>@user</p>"#;
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert!(
            !result.contains("class=\"user-mention\">@skip"),
            "Mention inside script should not be converted"
        );
        assert!(
            result.contains("class=\"user-mention\">@user</a>"),
            "Mention after script should be converted"
        );
    }

    #[test]
    fn test_team_mention_not_replaced() {
        let html = "<p>pushing it all onto @jekyll/core.</p>";
        let result = process_mentions(html, DEFAULT_BASE_URL);
        assert_eq!(
            result, html,
            "Team mentions (@org/team) must not be converted"
        );
    }
}

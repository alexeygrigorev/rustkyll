//! Kramdown compatibility post-processing for pulldown-cmark HTML output.
//!
//! Jekyll uses kramdown as its Markdown engine, which supports features
//! not present in pulldown-cmark (CommonMark). This module post-processes
//! pulldown-cmark's HTML output to match kramdown's behavior.

use std::collections::HashMap;

/// Apply all kramdown compatibility transformations to HTML output.
///
/// This is the main entry point. It applies, in order:
/// 1. Auto-generated heading IDs
/// 2. Inline attribute lists (`{:target="_blank"}`, `{:.class}`, `{:#id}`)
/// 3. Inline code classes (`language-plaintext highlighter-rouge`)
/// 4. Paragraph spacing (extra newlines after block elements)
pub fn postprocess(html: &str) -> String {
    let html = add_heading_ids(html);
    let html = apply_inline_attributes(&html);
    let html = add_inline_code_classes(&html);
    add_block_spacing(&html)
}

// ============================================================================
// 1. Inline attribute lists (IAL)
// ============================================================================

/// Apply kramdown inline attribute lists found in HTML output.
///
/// Finds patterns like `</a>{:target="_blank"}` and moves the attributes
/// onto the preceding HTML element. Supports:
/// - `{:target="_blank"}` - arbitrary attributes
/// - `{:.class-name}` - CSS classes
/// - `{:#id-name}` - element IDs
/// - Multiple attributes: `{:target="_blank" rel="noopener"}`
fn apply_inline_attributes(html: &str) -> String {
    // We need to find `{:...}` patterns that follow a closing or self-closing tag,
    // possibly with whitespace in between (but typically immediately after).
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for `{:` which starts an IAL
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b':' {
            // Find the closing `}`
            if let Some(close) = find_closing_brace(html, i) {
                let attr_str = &html[i + 2..close]; // content between {: and }
                                                    // Check if this IAL is inside a <pre> or <code> block - skip if so
                if !is_inside_code_block(&result) {
                    // Find the preceding closing tag in result to apply attributes to
                    if apply_attributes_to_last_tag(&mut result, attr_str) {
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        // Default: copy character as-is
        result.push(html[i..].chars().next().unwrap());
        i += html[i..].chars().next().unwrap().len_utf8();
    }

    result
}

/// Find the closing `}` for an IAL starting at `start` (position of `{`).
/// Returns the index of `}`, or None if not found on the same line.
fn find_closing_brace(html: &str, start: usize) -> Option<usize> {
    let rest = &html[start..];
    // Find closing brace, but don't cross newlines (IALs are inline)
    for (i, ch) in rest.char_indices() {
        if ch == '}' && i > 0 {
            return Some(start + i);
        }
        if ch == '\n' {
            return None;
        }
    }
    None
}

/// Check if the current output position is inside a `<pre>` or `<code>` block.
fn is_inside_code_block(html: &str) -> bool {
    // Count opens and closes of <pre> tags
    let pre_opens = html.matches("<pre").count();
    let pre_closes = html.matches("</pre>").count();
    if pre_opens > pre_closes {
        return true;
    }

    // Also check for <code> blocks that are inside <pre> (fenced code)
    // We only care about <pre><code> combos, standalone <code> is inline
    false
}

/// Parse an IAL attribute string and apply the attributes to the last
/// opening tag in `html`. Returns true if attributes were applied.
fn apply_attributes_to_last_tag(html: &mut String, attr_str: &str) -> bool {
    let attrs = parse_ial_attributes(attr_str);
    if attrs.is_empty() {
        return false;
    }

    // Find the last closing tag in html, then find its matching opening tag
    // We look for the last `</tagname>` and then find `<tagname` before it
    if let Some(close_tag_end) = html.rfind('>') {
        // Check if this is a closing tag (</...>)
        let before_close = &html[..=close_tag_end];
        if let Some(close_tag_start) = before_close.rfind("</") {
            let tag_name = html[close_tag_start + 2..close_tag_end].to_string();
            // Find the matching opening tag before the closing tag
            let search_area = &html[..close_tag_start];
            if let Some(open_pos) = find_last_opening_tag(search_area, &tag_name) {
                insert_attributes_at(html, open_pos, &attrs);
                return true;
            }
        }
    }

    false
}

/// Find the position of the last opening tag `<tagname` in the given string.
fn find_last_opening_tag(html: &str, tag_name: &str) -> Option<usize> {
    let pattern = format!("<{}", tag_name);
    // Search backwards for the pattern
    let mut search_from = html.len();
    while search_from > 0 {
        if let Some(pos) = html[..search_from].rfind(&pattern) {
            // Verify it's actually an opening tag (not `<tagname_other`)
            let after_tag = pos + pattern.len();
            if after_tag < html.len() {
                let next_ch = html[after_tag..].chars().next().unwrap();
                if next_ch == ' ' || next_ch == '>' || next_ch == '/' {
                    return Some(pos);
                }
            } else {
                return Some(pos);
            }
            search_from = pos;
        } else {
            break;
        }
    }
    None
}

/// Insert parsed attributes into the opening tag at `open_pos`.
fn insert_attributes_at(html: &mut String, open_pos: usize, attrs: &[(String, String)]) {
    // Find the end of the opening tag (the `>`)
    let tag_start = &html[open_pos..];
    if let Some(gt_offset) = tag_start.find('>') {
        let gt_pos = open_pos + gt_offset;
        let existing_tag = &html[open_pos..gt_pos];

        // Build new attributes string
        let mut new_attrs = String::new();
        for (key, value) in attrs {
            if key == "class" {
                // Check if class already exists on the tag
                if let Some(class_start) = existing_tag.find("class=\"") {
                    // Append to existing class
                    let class_val_start = open_pos + class_start + 7; // after `class="`
                    if let Some(class_val_end) = html[class_val_start..].find('"') {
                        let insert_pos = class_val_start + class_val_end;
                        html.insert_str(insert_pos, &format!(" {}", value));
                        return; // We modified html directly, so return
                    }
                } else {
                    new_attrs.push_str(&format!(" class=\"{}\"", value));
                }
            } else if key == "id" {
                // Check if id already exists (e.g., from heading ID generation)
                if existing_tag.contains("id=\"") {
                    // Replace existing id
                    // For now, just skip - the explicit IAL id should win
                    // This is handled in heading ID generation
                    new_attrs.push_str(&format!(" id=\"{}\"", value));
                } else {
                    new_attrs.push_str(&format!(" id=\"{}\"", value));
                }
            } else {
                new_attrs.push_str(&format!(" {}=\"{}\"", key, value));
            }
        }

        // Insert before the `>`
        html.insert_str(gt_pos, &new_attrs);
    }
}

/// Parse IAL attribute string into key-value pairs.
///
/// Supports:
/// - `.class-name` -> ("class", "class-name")
/// - `#id-name` -> ("id", "id-name")
/// - `key="value"` -> ("key", "value")
/// - `key='value'` -> ("key", "value")
fn parse_ial_attributes(attr_str: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut remaining = attr_str.trim();

    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        if remaining.starts_with('.') {
            // Class shorthand
            remaining = &remaining[1..];
            let end = remaining
                .find(|c: char| c.is_whitespace() || c == '}')
                .unwrap_or(remaining.len());
            let class_name = &remaining[..end];
            if !class_name.is_empty() {
                attrs.push(("class".to_string(), class_name.to_string()));
            }
            remaining = &remaining[end..];
        } else if remaining.starts_with('#') {
            // ID shorthand
            remaining = &remaining[1..];
            let end = remaining
                .find(|c: char| c.is_whitespace() || c == '}')
                .unwrap_or(remaining.len());
            let id_name = &remaining[..end];
            if !id_name.is_empty() {
                attrs.push(("id".to_string(), id_name.to_string()));
            }
            remaining = &remaining[end..];
        } else {
            // key="value" or key='value'
            if let Some(eq_pos) = remaining.find('=') {
                let key = remaining[..eq_pos].trim();
                let after_eq = remaining[eq_pos + 1..].trim_start();
                if after_eq.starts_with('"') {
                    // Double-quoted value
                    let value_start = 1;
                    if let Some(end_quote) = after_eq[value_start..].find('"') {
                        let value = &after_eq[value_start..value_start + end_quote];
                        attrs.push((key.to_string(), value.to_string()));
                        remaining = &after_eq[value_start + end_quote + 1..];
                    } else {
                        break; // Malformed
                    }
                } else if after_eq.starts_with('\'') {
                    // Single-quoted value
                    let value_start = 1;
                    if let Some(end_quote) = after_eq[value_start..].find('\'') {
                        let value = &after_eq[value_start..value_start + end_quote];
                        attrs.push((key.to_string(), value.to_string()));
                        remaining = &after_eq[value_start + end_quote + 1..];
                    } else {
                        break; // Malformed
                    }
                } else {
                    // Unquoted value (take until whitespace)
                    let end = after_eq
                        .find(|c: char| c.is_whitespace())
                        .unwrap_or(after_eq.len());
                    let value = &after_eq[..end];
                    attrs.push((key.to_string(), value.to_string()));
                    remaining = &after_eq[end..];
                }
            } else {
                // Unknown format, skip a word
                let end = remaining
                    .find(|c: char| c.is_whitespace())
                    .unwrap_or(remaining.len());
                remaining = &remaining[end..];
            }
        }
    }

    attrs
}

// ============================================================================
// 2. Auto-generated heading IDs
// ============================================================================

/// Add auto-generated `id` attributes to heading tags.
///
/// Matches kramdown's algorithm:
/// - Lowercase the heading text
/// - Replace spaces with hyphens
/// - Strip non-alphanumeric characters (except hyphens)
/// - Handle duplicates by appending `-1`, `-2`, etc.
fn add_heading_ids(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut used_ids: HashMap<String, usize> = HashMap::new();
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find next heading tag
        if let Some(h_pos) = find_next_heading(remaining) {
            // Copy everything before the heading
            result.push_str(&remaining[..h_pos]);

            let after = &remaining[h_pos..];
            // Parse the heading tag: <hN> or <hN ...>
            if let Some(gt_pos) = after.find('>') {
                let tag = &after[..gt_pos + 1];
                let level_char = after.as_bytes()[2]; // h1, h2, etc.

                // Find closing tag
                let close_tag = format!("</h{}>", level_char as char);
                if let Some(close_pos) = after.find(&close_tag) {
                    let inner_html = &after[gt_pos + 1..close_pos];

                    // Extract text content (strip HTML tags)
                    let text = strip_html_tags(inner_html);
                    let slug = slugify(&text);

                    // Handle duplicates
                    let id = get_unique_id(&mut used_ids, &slug);

                    // Only add IDs to headings generated by pulldown-cmark
                    // (simple <hN> tags with no existing attributes).
                    // Raw HTML headings passed through will already have
                    // attributes like class="...", so we skip those.
                    let is_simple_tag = tag == format!("<h{}>", level_char as char);
                    if !is_simple_tag {
                        // Has existing attributes or id -- leave as-is
                        result.push_str(&after[..close_pos + close_tag.len()]);
                    } else {
                        // Simple tag: <hN> -> <hN id="...">
                        result.push_str(&after[..3]);
                        result.push_str(&format!(" id=\"{}\"", id));
                        result.push_str(&after[3..close_pos + close_tag.len()]);
                    }

                    remaining = &after[close_pos + close_tag.len()..];
                    continue;
                }
            }

            // Couldn't parse heading, copy the `<` and continue
            result.push('<');
            remaining = &remaining[h_pos + 1..];
        } else {
            // No more headings
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Find the next heading tag in the string, returning its byte position.
fn find_next_heading(html: &str) -> Option<usize> {
    let bytes = html.as_bytes();
    for i in 0..bytes.len().saturating_sub(3) {
        if bytes[i] == b'<'
            && bytes[i + 1] == b'h'
            && bytes[i + 2].is_ascii_digit()
            && (1..=6).contains(&(bytes[i + 2] - b'0'))
        {
            // Check next char is '>' or ' ' (not a closing tag or other tag)
            if i + 3 < bytes.len() && (bytes[i + 3] == b'>' || bytes[i + 3] == b' ') {
                return Some(i);
            }
        }
    }
    None
}

/// Strip HTML tags from a string, returning just the text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// Convert heading text to a URL-friendly slug matching kramdown's algorithm.
fn slugify(text: &str) -> String {
    let lower = text.to_lowercase();
    let mut slug = String::with_capacity(lower.len());

    for ch in lower.chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
        } else if (ch == ' ' || ch == '-') && !slug.ends_with('-') {
            slug.push('-');
        }
        // Other characters are stripped
    }

    // Trim trailing hyphens
    while slug.ends_with('-') {
        slug.pop();
    }
    // Trim leading hyphens
    while slug.starts_with('-') {
        slug.remove(0);
    }

    slug
}

/// Get a unique ID, appending `-1`, `-2`, etc. for duplicates.
fn get_unique_id(used: &mut HashMap<String, usize>, base: &str) -> String {
    let count = used.entry(base.to_string()).or_insert(0);
    let id = if *count == 0 {
        base.to_string()
    } else {
        format!("{}-{}", base, count)
    };
    *count += 1;
    id
}

// ============================================================================
// 3. Inline code classes
// ============================================================================

/// Add `class="language-plaintext highlighter-rouge"` to inline `<code>` elements.
///
/// Only modifies `<code>` tags that:
/// - Don't already have a class attribute (i.e., not language-tagged fenced blocks)
/// - Are NOT inside a `<pre>` tag (fenced code blocks)
fn add_inline_code_classes(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;
    let mut in_pre = false;

    while !remaining.is_empty() {
        if in_pre {
            // Inside a <pre> block, look for </pre>
            if let Some(close_pos) = remaining.find("</pre>") {
                let end = close_pos + 6;
                result.push_str(&remaining[..end]);
                remaining = &remaining[end..];
                in_pre = false;
            } else {
                result.push_str(remaining);
                break;
            }
        } else if remaining.starts_with("<pre") {
            in_pre = true;
            // Copy the <pre tag
            if let Some(gt) = remaining.find('>') {
                result.push_str(&remaining[..=gt]);
                remaining = &remaining[gt + 1..];
            } else {
                result.push_str(remaining);
                break;
            }
        } else if remaining.starts_with("<code>") {
            // Inline code without class - add kramdown classes
            result.push_str("<code class=\"language-plaintext highlighter-rouge\">");
            remaining = &remaining[6..]; // skip past "<code>"
        } else {
            // Find next interesting point
            let next_pre = remaining.find("<pre");
            let next_code = remaining.find("<code>");
            let next = match (next_pre, next_code) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            if let Some(pos) = next {
                if pos > 0 {
                    result.push_str(&remaining[..pos]);
                    remaining = &remaining[pos..];
                } else {
                    // Already at the position, push one char to avoid infinite loop
                    // This shouldn't happen given the if/else above, but be safe
                    let ch = remaining.chars().next().unwrap();
                    result.push(ch);
                    remaining = &remaining[ch.len_utf8()..];
                }
            } else {
                result.push_str(remaining);
                break;
            }
        }
    }

    result
}

// ============================================================================
// 4. Paragraph spacing
// ============================================================================

/// Add extra newlines after closing block-level tags to match kramdown output.
fn add_block_spacing(html: &str) -> String {
    let block_tags = [
        "</p>",
        "</h1>",
        "</h2>",
        "</h3>",
        "</h4>",
        "</h5>",
        "</h6>",
        "</ul>",
        "</ol>",
        "</blockquote>",
        "</div>",
        "</pre>",
        "</table>",
    ];

    let mut result = String::with_capacity(html.len() + html.len() / 10);
    let mut remaining = html;

    while !remaining.is_empty() {
        let mut earliest: Option<(usize, usize)> = None; // (position, tag_len)

        for tag in &block_tags {
            if let Some(pos) = remaining.find(tag) {
                let tag_end = pos + tag.len();
                match earliest {
                    Some((ep, _)) if pos < ep => {
                        earliest = Some((pos, tag_end));
                    }
                    None => {
                        earliest = Some((pos, tag_end));
                    }
                    _ => {}
                }
            }
        }

        if let Some((_, tag_end)) = earliest {
            result.push_str(&remaining[..tag_end]);
            remaining = &remaining[tag_end..];

            // Add extra newline if not already followed by two newlines
            if !remaining.starts_with("\n\n") && remaining.starts_with('\n') {
                result.push('\n'); // Add one extra newline (already has one from pulldown-cmark)
            } else if remaining.is_empty() || !remaining.starts_with('\n') {
                // No newline at all after the tag; add two (block-level separation)
                // But only if there's more content
                if !remaining.is_empty() {
                    result.push('\n');
                }
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- IAL parsing tests ---

    #[test]
    fn test_parse_ial_target_blank() {
        let attrs = parse_ial_attributes("target=\"_blank\"");
        assert_eq!(attrs, vec![("target".into(), "_blank".into())]);
    }

    #[test]
    fn test_parse_ial_class_shorthand() {
        let attrs = parse_ial_attributes(".highlight");
        assert_eq!(attrs, vec![("class".into(), "highlight".into())]);
    }

    #[test]
    fn test_parse_ial_id_shorthand() {
        let attrs = parse_ial_attributes("#my-id");
        assert_eq!(attrs, vec![("id".into(), "my-id".into())]);
    }

    #[test]
    fn test_parse_ial_multiple_attrs() {
        let attrs = parse_ial_attributes("target=\"_blank\" rel=\"noopener\"");
        assert_eq!(
            attrs,
            vec![
                ("target".into(), "_blank".into()),
                ("rel".into(), "noopener".into()),
            ]
        );
    }

    #[test]
    fn test_parse_ial_mixed() {
        let attrs = parse_ial_attributes(".my-class target=\"_blank\"");
        assert_eq!(
            attrs,
            vec![
                ("class".into(), "my-class".into()),
                ("target".into(), "_blank".into()),
            ]
        );
    }

    // --- Slugify tests ---

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("What's New?"), "whats-new");
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("Step 1: Setup"), "step-1-setup");
    }

    #[test]
    fn test_slugify_exclamation() {
        assert_eq!(slugify("Title!"), "title");
    }

    #[test]
    fn test_slugify_hyphens() {
        assert_eq!(slugify("hello - world"), "hello-world");
    }

    // --- Unique ID tests ---

    #[test]
    fn test_unique_id_first_use() {
        let mut used = HashMap::new();
        assert_eq!(get_unique_id(&mut used, "faq"), "faq");
    }

    #[test]
    fn test_unique_id_duplicate() {
        let mut used = HashMap::new();
        assert_eq!(get_unique_id(&mut used, "faq"), "faq");
        assert_eq!(get_unique_id(&mut used, "faq"), "faq-1");
        assert_eq!(get_unique_id(&mut used, "faq"), "faq-2");
    }

    // --- Full postprocess integration tests ---

    #[test]
    fn test_postprocess_link_with_target_blank() {
        let html = "<p><a href=\"https://example.com\">text</a>{:target=\"_blank\"}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "Should contain target attribute. Got: {}",
            result
        );
        assert!(
            !result.contains("{:target"),
            "Should not contain raw IAL. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"https://example.com\" target=\"_blank\">text</a>"),
            "Attribute should be on the a tag. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_link_with_class() {
        let html = "<p><a href=\"url\">text</a>{:.highlight}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"highlight\""),
            "Should contain class. Got: {}",
            result
        );
        assert!(
            !result.contains("{:.highlight}"),
            "Should not contain raw IAL. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_link_with_id() {
        let html = "<p><a href=\"url\">text</a>{:#link-id}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"link-id\""),
            "Should contain id. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_link_multiple_attrs() {
        let html = "<p><a href=\"url\">text</a>{:target=\"_blank\" rel=\"noopener\"}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "Should have target. Got: {}",
            result
        );
        assert!(
            result.contains("rel=\"noopener\""),
            "Should have rel. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_id() {
        let html = "<h2>Hello World</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<h2 id=\"hello-world\">Hello World</h2>"),
            "Should have auto-generated id. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_special_chars() {
        let html = "<h1>Title!</h1>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"title\""),
            "Should strip special chars from id. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_duplicate_ids() {
        let html = "<h2>FAQ</h2>\n<h2>FAQ</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"faq\""),
            "First FAQ should have id='faq'. Got: {}",
            result
        );
        assert!(
            result.contains("id=\"faq-1\""),
            "Second FAQ should have id='faq-1'. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_all_heading_levels() {
        let html = "<h1>H1</h1>\n<h2>H2</h2>\n<h3>H3</h3>\n<h4>H4</h4>\n<h5>H5</h5>\n<h6>H6</h6>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<h1 id=\"h1\">"),
            "h1 missing id. Got: {}",
            result
        );
        assert!(
            result.contains("<h2 id=\"h2\">"),
            "h2 missing id. Got: {}",
            result
        );
        assert!(
            result.contains("<h3 id=\"h3\">"),
            "h3 missing id. Got: {}",
            result
        );
        assert!(
            result.contains("<h4 id=\"h4\">"),
            "h4 missing id. Got: {}",
            result
        );
        assert!(
            result.contains("<h5 id=\"h5\">"),
            "h5 missing id. Got: {}",
            result
        );
        assert!(
            result.contains("<h6 id=\"h6\">"),
            "h6 missing id. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_inline_code_class() {
        let html = "<p>Use <code>pip install</code> to install.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Inline code should have kramdown classes. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_fenced_code_not_modified() {
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"language-python\""),
            "Fenced code class should not be modified. Got: {}",
            result
        );
        assert!(
            !result.contains("language-plaintext"),
            "Should not add plaintext class to fenced code. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_paragraph_spacing() {
        let html = "<p>First paragraph.</p>\n<p>Second paragraph.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("</p>\n\n"),
            "Should have extra newline after </p>. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_spacing() {
        let html = "<h2>Title</h2>\n<p>Content.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("</h2>\n\n"),
            "Should have extra newline after </h2>. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_postprocess_list_spacing() {
        let html = "<ul>\n<li>Item</li>\n</ul>\n<p>After list.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("</ul>\n\n"),
            "Should have extra newline after </ul>. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_postprocess_malformed_ial_left_alone() {
        let html = "<p>Some text {: incomplete</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("{: incomplete"),
            "Malformed IAL should be left as-is. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_ial_in_code_block_ignored() {
        let html = "<pre><code>{:target=\"_blank\"}\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("{:target=\"_blank\"}"),
            "IAL inside code block should not be processed. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_combined() {
        // Test all four features working together
        let html = "<h2>Hello World</h2>\n<p>Visit <a href=\"url\">site</a>{:target=\"_blank\"} and use <code>pip</code>.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"hello-world\""),
            "Heading should have id. Got: {}",
            result
        );
        assert!(
            result.contains("target=\"_blank\""),
            "Link should have target. Got: {}",
            result
        );
        assert!(
            !result.contains("{:target"),
            "Raw IAL should be removed. Got: {}",
            result
        );
        assert!(
            result.contains("language-plaintext"),
            "Inline code should have class. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_ial_in_list_item() {
        let html =
            "<ul>\n<li><a href=\"/slack.html\">Register</a>{:target=\"_blank\"}</li>\n</ul>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "Should apply target in list item. Got: {}",
            result
        );
        assert!(
            !result.contains("{:target"),
            "Should remove raw IAL. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_ial_in_blockquote() {
        let html =
            "<blockquote>\n<p><a href=\"url\">quote link</a>{:target=\"_blank\"}</p>\n</blockquote>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "Should apply target in blockquote. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_whats_new() {
        let html = "<h2>What's New?</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"whats-new\""),
            "Should generate correct slug. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_heading_step_1_setup() {
        let html = "<h2>Step 1: Setup</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"step-1-setup\""),
            "Should handle numbers and colons. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("Hello <em>World</em>"), "Hello World");
        assert_eq!(strip_html_tags("Plain text"), "Plain text");
        assert_eq!(strip_html_tags("<a href=\"x\">link</a>"), "link");
    }

    #[test]
    fn test_fenced_code_no_language() {
        // Fenced code without language - <pre><code> should not get plaintext class
        let html = "<pre><code>plain code\n</code></pre>\n";
        let result = postprocess(html);
        // The <code> inside <pre> should NOT get language-plaintext class
        assert!(
            !result.contains("language-plaintext"),
            "Code inside pre should not get plaintext class. Got: {}",
            result
        );
    }
}

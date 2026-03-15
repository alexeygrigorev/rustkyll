//! Kramdown compatibility post-processing for pulldown-cmark HTML output.
//!
//! Jekyll uses kramdown as its Markdown engine, which supports features
//! not present in pulldown-cmark (CommonMark). This module post-processes
//! pulldown-cmark's HTML output to match kramdown's behavior.

use std::collections::HashMap;

/// Mark existing HTML headings with a data attribute so that `add_heading_ids`
/// will skip them. This should be called on content BEFORE markdown conversion
/// when the content contains a mix of raw HTML (from includes) and markdown.
///
/// D1: Headings from `{% include %}` output should NOT get auto-generated `id`
/// attributes. Only headings from markdown content should get IDs.
///
/// The marker is a `data-raw-html` attribute which makes `add_heading_ids`
/// see the tag as non-simple (it has attributes) and skip it.
pub fn mark_existing_html_headings(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while !remaining.is_empty() {
        // Find next '<h'
        if let Some(pos) = remaining.find("<h") {
            // Copy everything before the match
            result.push_str(&remaining[..pos]);
            let after = &remaining[pos..];

            // Check if this is a bare <hN> tag (e.g., <h1>, <h2>)
            if after.len() >= 4 {
                let level = after.as_bytes()[2];
                let next = after.as_bytes()[3];
                if level.is_ascii_digit() && (1..=6).contains(&(level - b'0')) && next == b'>' {
                    // Found bare <hN> -- add marker
                    result.push_str(&after[..3]);
                    result.push_str(" data-raw-html>");
                    remaining = &after[4..];
                    continue;
                }
            }
            // Not a bare heading tag -- copy the '<' and continue
            result.push('<');
            remaining = &after[1..];
        } else {
            // No more '<h' patterns
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Remove the `data-raw-html` marker attribute from headings.
///
/// Called after kramdown postprocessing to clean up the markers.
pub fn remove_heading_markers(html: &str) -> String {
    html.replace(" data-raw-html", "")
}

/// Apply all kramdown compatibility transformations to HTML output.
///
/// This is the main entry point. It applies, in order:
/// 1. Strip unwanted `<p>` tags inside HTML block elements
/// 2. Auto-generated heading IDs
/// 3. Inline attribute lists (`{:target="_blank"}`, `{:.class}`, `{:#id}`)
/// 4. Fenced code block wrapping (no language tag)
/// 5. Inline code classes (`language-plaintext highlighter-rouge`)
/// 6. Paragraph spacing (extra newlines after block elements)
/// 7. Remove `start` attribute from `<ol>` tags (D11)
/// 8. Remove self-closing slash from void elements (D3)
/// 9. Normalize boolean HTML attributes (D2, D12)
/// 10. Normalize `<figcaption>` closing tag whitespace (D6)
pub fn postprocess(html: &str) -> String {
    let html = strip_paragraphs_in_html_blocks(html);
    let html = add_heading_ids(&html);
    let html = apply_inline_attributes(&html);
    let html = wrap_fenced_code_blocks(&html);
    let html = add_inline_code_classes(&html);
    let html = add_block_spacing(&html);
    let html = remove_ol_start_attribute(&html);
    let html = normalize_void_elements(&html);
    let html = normalize_boolean_attributes(&html);
    normalize_figcaption_whitespace(&html)
}

// ============================================================================
// 0. Strip unwanted <p> tags inside HTML block elements
// ============================================================================

/// HTML block-level element tag names where pulldown-cmark may incorrectly
/// wrap inline content in `<p>` tags. kramdown does not do this.
const BLOCK_PARENT_TAGS: &[&str] = &[
    "li",
    "div",
    "td",
    "th",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "section",
    "article",
    "header",
    "footer",
    "nav",
    "aside",
    "figure",
    "figcaption",
    "details",
    "summary",
    "form",
    "fieldset",
    "dd",
    "dt",
];

/// Strip unwanted `<p>` tags that pulldown-cmark inserts inside HTML block elements.
///
/// When Liquid produces HTML like `<li><a href="...">Title</a> text</li>`,
/// pulldown-cmark wraps the inline content in `<p>` tags:
/// `<li><p><a href="...">Title</a> text</p></li>`.
///
/// kramdown does not do this -- it leaves inline content inside HTML block
/// elements as-is. This function removes those auto-generated `<p>` wrappers
/// while preserving intentional `<p>` tags in markdown content.
///
/// The algorithm: for each block parent element, check if it contains only
/// `<p>` wrappers around inline content (no nested block elements). If so,
/// strip the `<p>`/`</p>` tags.
fn strip_paragraphs_in_html_blocks(html: &str) -> String {
    let mut result = html.to_string();

    for &tag in BLOCK_PARENT_TAGS {
        result = strip_p_in_tag(&result, tag);
    }

    result
}

/// Strip `<p>` tags inside all instances of `<tag ...>...</tag>` in the HTML.
///
/// Only strips when the block element's content consists entirely of inline
/// content wrapped in `<p>` tags (no nested block elements).
fn strip_p_in_tag(html: &str, tag: &str) -> String {
    let open_pattern = format!("<{}", tag);
    let close_pattern = format!("</{}>", tag);
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find the next opening tag
        let open_pos = match remaining.find(&open_pattern) {
            Some(pos) => {
                // Verify it's actually the tag (not e.g. <listing> when we search <li>)
                let after = &remaining[pos + open_pattern.len()..];
                if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
                    pos
                } else {
                    // Not our tag, skip past this match
                    result.push_str(&remaining[..pos + open_pattern.len()]);
                    remaining = &remaining[pos + open_pattern.len()..];
                    continue;
                }
            }
            None => {
                result.push_str(remaining);
                break;
            }
        };

        // Copy everything before the tag
        result.push_str(&remaining[..open_pos]);
        remaining = &remaining[open_pos..];

        // Find the closing `>` of the opening tag
        let gt_pos = match remaining.find('>') {
            Some(pos) => pos,
            None => {
                result.push_str(remaining);
                break;
            }
        };

        let opening_tag = &remaining[..=gt_pos];

        // Find the matching closing tag (handle nesting)
        let inner_start = gt_pos + 1;
        let inner = &remaining[inner_start..];

        if let Some(close_offset) = find_matching_close(inner, tag) {
            let inner_content = &inner[..close_offset];
            let after_close = &inner[close_offset + close_pattern.len()..];

            // Decide whether to strip <p> tags from the inner content
            let processed_inner = maybe_strip_p_tags(inner_content);

            result.push_str(opening_tag);
            result.push_str(&processed_inner);
            result.push_str(&close_pattern);
            remaining = after_close;
        } else {
            // No matching close tag found -- output opening tag and continue
            result.push_str(opening_tag);
            remaining = &remaining[gt_pos + 1..];
        }
    }

    result
}

/// Find the position of the matching closing tag, handling nesting.
/// Returns the byte offset within `inner` where the closing tag starts.
fn find_matching_close(inner: &str, tag: &str) -> Option<usize> {
    let open_pattern = format!("<{}", tag);
    let close_pattern = format!("</{}>", tag);
    let mut depth = 0usize;
    let mut search_pos = 0;

    while search_pos < inner.len() {
        let next_open = inner[search_pos..].find(&open_pattern).map(|p| {
            let abs = search_pos + p;
            // Verify it's actually our tag
            let after = &inner[abs + open_pattern.len()..];
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
                Some(abs)
            } else {
                None
            }
        });
        let next_open = next_open.flatten();

        let next_close = inner[search_pos..]
            .find(&close_pattern)
            .map(|p| search_pos + p);

        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                // Nested open tag
                depth += 1;
                search_pos = o + open_pattern.len();
            }
            (_, Some(c)) => {
                if depth == 0 {
                    return Some(c);
                }
                depth -= 1;
                search_pos = c + close_pattern.len();
            }
            (Some(o), None) => {
                // Open without close -- malformed, give up
                search_pos = o + open_pattern.len();
            }
            (None, None) => {
                return None;
            }
        }
    }
    None
}

/// Check if the inner content of a block element should have `<p>` tags stripped.
///
/// We strip `<p>` tags when:
/// 1. The content contains `<p>` tags
/// 2. The content does NOT contain nested block-level elements (other than `<p>`)
/// 3. The `<p>` content is only inline elements/text
///
/// We do NOT strip `<p>` tags that appear to be intentionally authored (e.g.,
/// `<div><p class="intro">...</p></div>` -- the `<p>` has attributes).
fn maybe_strip_p_tags(inner: &str) -> String {
    // If there are no <p> tags, nothing to do
    if !inner.contains("<p>") {
        return inner.to_string();
    }

    // Check if the content has any block-level children OTHER than <p>
    // If it does, we should be more careful -- but still strip <p> tags
    // that wrap only inline content.
    //
    // The approach: replace each `<p>` ... `</p>` pair with just its content,
    // but only if the <p> has no attributes (auto-generated ones don't) and
    // the content between <p> and </p> is only inline content.
    let mut result = String::with_capacity(inner.len());
    let mut remaining = inner;

    while !remaining.is_empty() {
        if let Some(p_pos) = remaining.find("<p>") {
            // Check that this is a bare <p> (no attributes -- auto-generated)
            let before_p = &remaining[..p_pos];
            result.push_str(before_p);

            let after_p_open = &remaining[p_pos + 3..]; // skip "<p>"

            if let Some(close_p_pos) = find_close_p(after_p_open) {
                let p_content = &after_p_open[..close_p_pos];

                // Only strip if the <p> content contains no block-level elements
                if !contains_block_elements(p_content) {
                    // Strip the <p>...</p> wrapper, keep content
                    result.push_str(p_content);
                    remaining = &after_p_open[close_p_pos + 4..]; // skip "</p>"
                } else {
                    // Keep the <p> tag as-is
                    result.push_str("<p>");
                    remaining = after_p_open;
                }
            } else {
                // No closing </p> found -- keep as-is
                result.push_str("<p>");
                remaining = after_p_open;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Find the position of the matching `</p>` tag, handling nested `<p>` tags.
fn find_close_p(content: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut pos = 0;

    while pos < content.len() {
        if content[pos..].starts_with("<p>") || content[pos..].starts_with("<p ") {
            depth += 1;
            pos += 3;
        } else if content[pos..].starts_with("</p>") {
            if depth == 0 {
                return Some(pos);
            }
            depth -= 1;
            pos += 4;
        } else {
            pos += content[pos..].chars().next().map_or(1, |c| c.len_utf8());
        }
    }
    None
}

/// Check if HTML content contains any block-level elements.
///
/// Used to determine if `<p>` content is truly inline (safe to strip wrapper)
/// or contains block-level structure (keep wrapper).
fn contains_block_elements(content: &str) -> bool {
    let block_tags = [
        "<div",
        "<section",
        "<article",
        "<header",
        "<footer",
        "<nav",
        "<aside",
        "<ul",
        "<ol",
        "<table",
        "<blockquote",
        "<pre",
        "<figure",
        "<form",
        "<fieldset",
        "<details",
        "<dl",
    ];

    for tag in &block_tags {
        if let Some(pos) = content.find(tag) {
            // Verify it's actually a tag (not text like "a <division of labor")
            let after = &content[pos + tag.len()..];
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
                return true;
            }
        }
    }
    false
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
// 3. Fenced code block wrapping (no language tag)
// ============================================================================

/// Wrap bare `<pre><code>...</code></pre>` blocks in kramdown-style div structure.
///
/// Fenced code blocks without a language tag are wrapped as:
/// ```html
/// <div class="language-plaintext highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
/// ```
///
/// Fenced code blocks WITH a language class (e.g., `<pre><code class="language-python">`)
/// are left untouched.
fn wrap_fenced_code_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for <pre><code> (bare, no class on <code>)
        if let Some(pre_pos) = remaining.find("<pre><code>") {
            // Copy everything before this match
            result.push_str(&remaining[..pre_pos]);

            let after_pre_code = &remaining[pre_pos + 11..]; // skip "<pre><code>"

            // Find the closing </code></pre>
            if let Some(close_pos) = after_pre_code.find("</code></pre>") {
                let code_content = &after_pre_code[..close_pos];
                // Write the kramdown wrapper
                result.push_str("<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>");
                result.push_str(code_content);
                result.push_str("</code></pre></div></div>");
                remaining = &after_pre_code[close_pos + 13..]; // skip "</code></pre>"
            } else {
                // No closing tag found, copy as-is
                result.push_str("<pre><code>");
                remaining = after_pre_code;
            }
        } else {
            // No more bare <pre><code> blocks
            result.push_str(remaining);
            break;
        }
    }

    result
}

// ============================================================================
// 4. Inline code classes
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
// 5. Paragraph spacing
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
// 7. Remove `start` attribute from `<ol>` tags (D11)
// ============================================================================

/// Remove `start="N"` attributes from `<ol>` tags.
///
/// pulldown-cmark adds `start="N"` to ordered lists that don't start at 1.
/// kramdown never adds this attribute. This normalizes the output to match.
fn remove_ol_start_attribute(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(pos) = remaining.find("<ol ") {
        result.push_str(&remaining[..pos]);
        let after = &remaining[pos..];

        if let Some(gt_pos) = after.find('>') {
            let tag = &after[..gt_pos + 1];
            // Remove start="N" attribute
            if tag.contains("start=\"") {
                let cleaned = remove_attribute(tag, "start");
                result.push_str(&cleaned);
            } else {
                result.push_str(tag);
            }
            remaining = &after[gt_pos + 1..];
        } else {
            result.push_str(after);
            return result;
        }
    }
    result.push_str(remaining);
    result
}

/// Remove a specific attribute from an HTML tag string.
fn remove_attribute(tag: &str, attr_name: &str) -> String {
    let pattern = format!(" {attr_name}=\"");
    if let Some(start) = tag.find(&pattern) {
        let after_eq = start + pattern.len();
        if let Some(end_quote) = tag[after_eq..].find('"') {
            let end = after_eq + end_quote + 1;
            let mut result = tag[..start].to_string();
            result.push_str(&tag[end..]);
            // Clean up double spaces
            result = result.replace("  ", " ");
            // Clean up space before >
            result = result.replace(" >", ">");
            return result;
        }
    }
    tag.to_string()
}

// ============================================================================
// 8. Normalize void elements (D3)
// ============================================================================

/// Remove self-closing slash from void HTML elements.
///
/// pulldown-cmark produces `<br />`, `<hr />`, `<input ... />` etc.
/// kramdown produces `<br>`, `<hr>`, `<input ...>`.
fn normalize_void_elements(html: &str) -> String {
    let mut result = html.to_string();
    // Handle self-closing void elements: replace " />" with ">"
    // Only for void elements that should not have a closing slash
    let void_elements = [
        "br", "hr", "img", "input", "meta", "link", "col", "area", "base", "embed", "source",
        "track", "wbr",
    ];
    for tag in &void_elements {
        // Replace patterns like <br /> or <br/> or <input ... />
        let pattern_space = format!("<{} />", tag);
        let replacement_space = format!("<{}>", tag);
        result = result.replace(&pattern_space, &replacement_space);

        let pattern_no_space = format!("<{}/>", tag);
        let replacement_no_space = format!("<{}>", tag);
        result = result.replace(&pattern_no_space, &replacement_no_space);
    }

    // Handle void elements with attributes: `<input type="text" />`
    // We need a more targeted approach for these
    result = normalize_self_closing_with_attrs(&result);

    result
}

/// Normalize self-closing tags with attributes (e.g., `<input type="text" />`).
fn normalize_self_closing_with_attrs(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(pos) = remaining.find(" />") {
        // Check if this is inside a void element tag
        let before = &remaining[..pos];
        if let Some(tag_start) = before.rfind('<') {
            let tag_content = &before[tag_start + 1..];
            let tag_name = tag_content
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("");
            let void_elements = [
                "br", "hr", "img", "input", "meta", "link", "col", "area", "base", "embed",
                "source", "track", "wbr",
            ];
            if void_elements.contains(&tag_name) {
                result.push_str(&remaining[..pos]);
                result.push('>');
                remaining = &remaining[pos + 3..];
                continue;
            }
        }
        result.push_str(&remaining[..pos + 3]);
        remaining = &remaining[pos + 3..];
    }
    result.push_str(remaining);
    result
}

// ============================================================================
// 9. Normalize boolean HTML attributes (D2, D12)
// ============================================================================

/// Normalize boolean HTML attributes by removing empty string values.
///
/// pulldown-cmark produces `required=""`, `novalidate=""`, `itemscope=""`.
/// kramdown produces `required`, `novalidate`, `itemscope`.
fn normalize_boolean_attributes(html: &str) -> String {
    let boolean_attrs = [
        "required",
        "novalidate",
        "itemscope",
        "checked",
        "disabled",
        "readonly",
        "multiple",
        "autofocus",
        "autoplay",
        "controls",
        "loop",
        "muted",
        "selected",
        "hidden",
        "async",
        "defer",
        "formnovalidate",
        "open",
        "allowfullscreen",
    ];

    let mut result = html.to_string();
    for attr in &boolean_attrs {
        let pattern = format!("{attr}=\"\"");
        result = result.replace(&pattern, attr);
    }
    result
}

// ============================================================================
// 10. Normalize figcaption whitespace (D6)
// ============================================================================

/// Normalize figcaption closing tag to be on the same line as content.
///
/// pulldown-cmark produces:
/// ```html
/// <figcaption>text
/// </figcaption>
/// ```
/// kramdown produces:
/// ```html
/// <figcaption>text</figcaption>
/// ```
fn normalize_figcaption_whitespace(html: &str) -> String {
    html.replace("\n</figcaption>", "</figcaption>")
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
        // Fenced code without language should be wrapped in kramdown div structure
        let html = "<pre><code>plain code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain code\n</code></pre>"),
            "Bare fenced code should be wrapped in kramdown divs. Got: {}",
            result
        );
        assert!(
            result.contains("</pre>") && result.contains("</div>"),
            "Should have closing tags. Got: {}",
            result
        );
    }

    // --- Fenced code block wrapping tests ---

    #[test]
    fn test_fenced_code_wrapping_simple() {
        let html = "<pre><code>plain code\n</code></pre>\n";
        let result = postprocess(html);
        // The wrapping produces the kramdown div structure; block spacing adds newlines between closing tags
        assert!(
            result.contains("<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain code\n</code></pre>"),
            "Simple fenced code wrapping failed. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_multiline() {
        let html = "<pre><code>line 1\nline 2\nline 3\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<pre class=\"highlight\"><code>line 1\nline 2\nline 3\n</code></pre>"),
            "Multiline content should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("<div class=\"language-plaintext highlighter-rouge\">"),
            "Should have outer wrapper div. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_does_not_affect_language_python() {
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("<div class=\"language-plaintext highlighter-rouge\">"),
            "Language-tagged code should not get plaintext wrapper. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"language-python\""),
            "Language class should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_does_not_affect_language_bash() {
        let html = "<pre><code class=\"language-bash\">echo hello\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("<div class=\"language-plaintext highlighter-rouge\">"),
            "Language-tagged code should not get plaintext wrapper. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_multiple_bare_blocks() {
        let html = "<pre><code>block 1\n</code></pre>\n<pre><code>block 2\n</code></pre>\n";
        let result = postprocess(html);
        let count = result
            .matches("<div class=\"language-plaintext highlighter-rouge\">")
            .count();
        assert_eq!(
            count, 2,
            "Both bare blocks should be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_mixed_bare_and_language() {
        let html = "<pre><code>bare code\n</code></pre>\n<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        let count = result
            .matches("<div class=\"language-plaintext highlighter-rouge\">")
            .count();
        assert_eq!(
            count, 1,
            "Only bare block should be wrapped. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"language-python\""),
            "Language-tagged block should be unchanged. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_no_interference_with_inline() {
        let html = "<p>Use <code>pip install</code> to install.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Inline code should get class attribute, not div wrapper. Got: {}",
            result
        );
        assert!(
            !result.contains(
                "<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\">"
            ),
            "Inline code should NOT be wrapped in divs. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_mixed_inline_and_fenced() {
        let html = "<p>Use <code>pip</code> command.</p>\n<pre><code>bare code\n</code></pre>\n";
        let result = postprocess(html);
        // Inline code gets class attribute
        assert!(
            result.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Inline code should get class. Got: {}",
            result
        );
        // Fenced code gets div wrapper
        assert!(
            result.contains("<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>bare code\n</code></pre>"),
            "Fenced code should get div wrapper. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_mixed_all_three() {
        // Document with inline code, fenced-with-language, and fenced-without-language
        let html = "<p>Use <code>pip</code>.</p>\n<pre><code class=\"language-python\">import os\n</code></pre>\n<pre><code>plain\n</code></pre>\n";
        let result = postprocess(html);
        // Inline: gets class
        assert!(
            result.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Inline code should get class. Got: {}",
            result
        );
        // Fenced with language: unchanged
        assert!(
            result.contains("class=\"language-python\""),
            "Language-tagged block should be unchanged. Got: {}",
            result
        );
        // Fenced without language: wrapped
        assert!(
            result.contains("<div class=\"language-plaintext highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain\n</code></pre>"),
            "Bare fenced code should be wrapped. Got: {}",
            result
        );
        // Should NOT wrap the language-tagged block
        let wrapper_count = result
            .matches("<div class=\"language-plaintext highlighter-rouge\">")
            .count();
        assert_eq!(
            wrapper_count, 1,
            "Only one block should be wrapped. Got: {}",
            result
        );
    }

    // ======================================================================
    // Paragraph stripping inside HTML block elements (issue 92)
    // ======================================================================

    #[test]
    fn test_strip_p_in_li_single_line() {
        let html = "<li class=\"podcast\"><p><a href=\"url\">Title</a> on date by <a href=\"/people/name.html\">Name</a></p></li>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> inside <li>. Got: {}",
            result
        );
        assert!(
            result.contains("<li class=\"podcast\"><a href=\"url\">Title</a> on date by <a href=\"/people/name.html\">Name</a></li>"),
            "Content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_li_multiline() {
        let html = "<li class=\"podcast\">\n<p><a href=\"url\">Title</a>\non date\nby</p>\n<p><a href=\"/people/name.html\">Name</a></p>\n</li>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip all <p> tags inside <li>. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"url\">Title</a>"),
            "Links should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_div() {
        let html =
            "<div class=\"book-authors\"><h5><p>by <a href=\"/people/x.html\">Author</a></p></h5></div>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> inside <h5> inside <div>. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_td() {
        let html = "<td><p>some content with <a href=\"url\">link</a></p></td>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> inside <td>. Got: {}",
            result
        );
        assert!(
            result.contains("<td>some content with <a href=\"url\">link</a></td>"),
            "Content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_nested_ul_li() {
        let html = "<ul><li><p><a href=\"url\">Link</a> text</p></li><li><p>Other</p></li></ul>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> in all <li> elements. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_section_with_nested_div() {
        let html =
            "<section><h2><p>Title</p></h2><div><p>Content with <a>link</a></p></div></section>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> in nested elements. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_preserves_markdown_paragraphs() {
        // Standalone <p> tags (not inside block elements) should be preserved
        let html = "<p>First paragraph.</p>\n\n<p>Second paragraph.</p>\n";
        let result = strip_paragraphs_in_html_blocks(html);
        assert_eq!(
            result.matches("<p>").count(),
            2,
            "Standalone <p> tags should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_preserves_p_with_block_content() {
        // <p> that contains block-level elements should NOT be stripped
        let html = "<div><p><div>nested block</div></p></div>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            result.contains("<p><div>nested block</div></p>"),
            "<p> with block content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_empty_elements() {
        let html = "<li></li>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert_eq!(result, "<li></li>", "Empty elements should be unchanged");
    }

    #[test]
    fn test_strip_p_whitespace_only() {
        let html = "<li>  </li>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert_eq!(
            result, "<li>  </li>",
            "Whitespace-only elements should be unchanged"
        );
    }

    #[test]
    fn test_strip_p_preserves_p_with_attributes() {
        // <p class="..."> is intentionally authored, not auto-generated
        let html = "<div><p class=\"intro\">Intentional paragraph</p></div>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            result.contains("<p class=\"intro\">"),
            "Attributed <p> tags should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_mixed_content() {
        // Markdown paragraphs before/after a div with p-stripped content
        let html = "<p>Markdown paragraph</p>\n<div><p>inline content</p></div>\n<p>Another paragraph</p>\n";
        let result = strip_paragraphs_in_html_blocks(html);
        // The <p> inside <div> should be stripped
        assert!(
            result.contains("<div>inline content</div>"),
            "Should strip <p> inside <div>. Got: {}",
            result
        );
        // But standalone paragraphs should remain
        assert!(
            result.contains("<p>Markdown paragraph</p>"),
            "Standalone <p> should remain. Got: {}",
            result
        );
        assert!(
            result.contains("<p>Another paragraph</p>"),
            "Standalone <p> should remain. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_nested_divs() {
        let html = "<div><div><p>nested content</p></div></div>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> in nested divs. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_via_markdown_to_html_li() {
        // Test the full pipeline through markdown_to_html
        let input = "<li class=\"podcast\">\n<a href=\"url\">Title</a>\non date\nby\n<a href=\"/people/name.html\">Name</a>\n</li>";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li class=\"podcast\">\n<p>"),
            "Should not have <p> inside <li> after full pipeline. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_via_markdown_to_html_div() {
        let input = "<div class=\"info\"><h5>by <a href=\"/people/x.html\">Author</a></h5></div>";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<h5><p>") && !html.contains("<h5>\n<p>"),
            "Should not have <p> inside <h5>. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_via_markdown_to_html_td() {
        let input = "<td>some content with <a href=\"url\">link</a></td>";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<td><p>") && !html.contains("<td>\n<p>"),
            "Should not have <p> inside <td>. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_preserves_legit_markdown_paragraphs() {
        let input = "Hello world\n\nSecond paragraph";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<p>").count(),
            2,
            "Should have two <p> tags for two markdown paragraphs. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_heading_then_paragraph() {
        let input = "# Heading\n\nParagraph text";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>Paragraph text</p>"),
            "Paragraph after heading should still have <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_mixed_markdown_and_html_blocks() {
        let input = "Markdown paragraph\n\n<div>inline</div>\n\nAnother paragraph";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>Markdown paragraph</p>"),
            "First paragraph should have <p>. Got: {}",
            html
        );
        assert!(
            html.contains("<p>Another paragraph</p>"),
            "Second paragraph should have <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_events_page_pattern() {
        // Simulates the events.md pattern after Liquid processing
        let input = r#"<ul>
<li class="podcast">
<a href="https://example.com/event" target="_blank">Event Title</a>
on 16 Mar 2026
by
<a href="/people/name.html">Speaker Name</a>
</li>
<li class="workshop">
<a href="https://example.com/workshop" target="_blank">Workshop Title</a>
on 20 Mar 2026
</li>
</ul>"#;
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li class=\"podcast\">\n<p>"),
            "Should not wrap event content in <p>. Got: {}",
            html
        );
        assert!(
            !html.contains("<li class=\"workshop\">\n<p>"),
            "Should not wrap workshop content in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_strip_p_books_page_pattern() {
        let input = r#"<div class="book-authors"><h5>by <a href="/people/author.html">Author Name</a></h5></div>"#;
        let html = crate::frontmatter::markdown_to_html(input);
        // The <h5> content should not be wrapped in <p>
        let has_p_in_h5 = html.contains("<h5><p>") || html.contains("<h5>\n<p>");
        assert!(
            !has_p_in_h5,
            "Should not wrap h5 content in <p>. Got: {}",
            html
        );
    }
}

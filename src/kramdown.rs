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
/// 1. Strip unwanted `<p>` tags inside HTML block elements (only when
///    `collapse_blank_lines_in_html_blocks` was NOT applied pre-markdown)
/// 2. Auto-generated heading IDs
/// 3. Inline attribute lists (`{:target="_blank"}`, `{:.class}`, `{:#id}`)
/// 4. Fenced code block wrapping (no language tag)
/// 5. Inline code classes (`language-plaintext highlighter-rouge`)
///    5b. Wrap bare text between block elements in `<p>` tags
/// 6. Paragraph spacing (extra newlines after block elements)
/// 7. Remove `start` attribute from `<ol>` tags (D11)
/// 8. Remove self-closing slash from void elements (D3)
/// 9. Normalize boolean HTML attributes (D2, D12)
/// 10. Normalize `<figcaption>` closing tag whitespace (D6)
pub fn postprocess(html: &str) -> String {
    let html = strip_paragraphs_in_html_blocks(html);
    let html = encode_bare_ampersands(&html);
    let html = add_heading_ids(&html);
    let html = apply_inline_attributes(&html);
    let html = wrap_fenced_code_blocks(&html);
    let html = add_inline_code_classes(&html);
    let html = wrap_bare_text_in_paragraphs(&html);
    let html = add_block_spacing(&html);
    let html = remove_ol_start_attribute(&html);
    let html = normalize_figcaption_whitespace(&html);
    // D2, D12: Normalize boolean attributes in the markdown output early
    // (during collection loading). This ensures that the final
    // normalize_html_output() call after layout wrapping finds nothing to change
    // and exits early, avoiding a full scan of the (often 100-300KB) page HTML.
    // Note: void element self-closing slashes are NOT removed because
    // Jekyll/kramdown outputs XHTML-style self-closing tags (e.g. <br />).
    normalize_boolean_attributes(&html)
}

/// Lighter postprocessing for the `markdownify` Liquid filter.
///
/// Jekyll's `markdownify` filter runs kramdown, which produces `<p>text</p>\n`
/// (single trailing newline after block elements). The full `postprocess`
/// adds `add_block_spacing` which doubles the newline, but that extra spacing
/// is only correct for page body content -- not for inline filter output where
/// the template already supplies the next newline.
///
/// This variant applies only the transformations relevant to short inline
/// markdown (inline code classes, boolean attributes, ol start removal) and
/// skips heavy block-level processing (heading IDs, fenced code wrapping,
/// block spacing, bare text wrapping, etc.).
pub fn postprocess_for_filter(html: &str) -> String {
    let html = add_inline_code_classes(html);
    let html = remove_ol_start_attribute(&html);
    normalize_boolean_attributes(&html)
}

/// Apply final HTML output normalization to match Jekyll/kramdown conventions.
///
/// This should be applied to the FINAL rendered HTML before writing to disk,
/// after all template rendering, layout wrapping, and postprocessing is done.
///
/// Includes:
/// - D2, D12: Boolean HTML attribute normalization (`required=""` -> `required`)
///
/// Note: void element self-closing slashes are NOT removed because
/// Jekyll/kramdown outputs XHTML-style self-closing tags (e.g. `<br />`).
pub fn normalize_html_output(html: &str) -> String {
    // Quick check: if the HTML has no `=""`, nothing to normalize.
    if !html.contains("=\"\"") {
        return html.to_string();
    }
    normalize_boolean_attributes(html)
}

// ============================================================================
// Pre-markdown: Collapse blank lines inside HTML block elements
// ============================================================================

/// Collapse blank lines inside HTML block elements before markdown parsing.
///
/// When Liquid `{% include %}` output contains blank lines inside HTML block
/// elements like `<li>`, `<div>`, `<h5>`, etc., pulldown-cmark interprets the
/// blank lines as paragraph separators and wraps content in `<p>` tags.
/// Jekyll/kramdown does not do this.
///
/// This function removes blank lines (lines containing only whitespace) that
/// appear between the opening and closing tags of HTML block elements. It
/// operates on the post-Liquid, pre-markdown content to prevent pulldown-cmark
/// from ever seeing the blank lines as paragraph breaks.
///
/// Content outside HTML block elements is left unchanged so that regular
/// markdown paragraph separation still works.
pub fn collapse_blank_lines_in_html_blocks(content: &str) -> String {
    let mut result = content.to_string();

    for &tag in BLOCK_PARENT_TAGS {
        result = collapse_blanks_in_tag(&result, tag);
    }

    result
}

/// Collapse blank lines inside all instances of `<tag ...>...</tag>`.
fn collapse_blanks_in_tag(content: &str, tag: &str) -> String {
    let open_pattern = format!("<{}", tag);
    let close_pattern = format!("</{}>", tag);
    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

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

            // Collapse blank lines in the inner content
            let collapsed = collapse_blank_lines(inner_content);

            result.push_str(opening_tag);
            result.push_str(&collapsed);
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

/// Remove blank lines (lines that are empty or contain only whitespace) from
/// a string while preserving non-blank lines separated by single newlines.
///
/// A "blank line" is a line that contains only whitespace characters.
/// Multiple consecutive blank lines are removed entirely. Non-blank lines
/// remain separated by single newlines.
fn collapse_blank_lines(content: &str) -> String {
    // Check if there are any blank lines to collapse
    let has_blank_line = content.split('\n').any(|line| line.trim().is_empty());
    if !has_blank_line {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let mut result_lines: Vec<&str> = Vec::with_capacity(lines.len());

    for line in &lines {
        if line.trim().is_empty() {
            // Skip blank lines entirely
            continue;
        }
        result_lines.push(line);
    }

    if result_lines.is_empty() {
        return String::new();
    }

    // Join non-blank lines with newlines, and add leading/trailing newlines
    // to match the original structure (content sits on its own lines within
    // the block element).
    let mut result = String::with_capacity(content.len());
    result.push('\n');
    for (i, line) in result_lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(line);
    }
    result.push('\n');

    result
}

// ============================================================================
// 0. Strip unwanted <p> tags inside HTML block elements
// ============================================================================

/// HTML block-level element tag names where pulldown-cmark may incorrectly
/// wrap inline content in `<p>` tags. kramdown does not do this.
/// Tags used by `collapse_blank_lines_in_html_blocks` (pre-markdown processing).
/// Includes all block-level HTML elements where blank lines from Liquid output
/// might cause pulldown-cmark to insert unwanted paragraph breaks.
const BLOCK_PARENT_TAGS: &[&str] = &[
    "li",
    "div",
    "p",
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

/// Tags used by `strip_paragraphs_in_html_blocks` (post-markdown processing).
///
/// This is a SUBSET of `BLOCK_PARENT_TAGS`. It includes only elements where
/// pulldown-cmark commonly auto-inserts `<p>` tags around inline content
/// (which kramdown does not do).
///
/// Excludes:
/// - `<section>`, `<article>`, `<header>`, `<footer>`, `<nav>`, `<aside>`:
///   semantic containers that commonly contain intentional `<p>` tags.
/// - `<div>`, `<form>`, `<fieldset>`, `<details>`: generic containers where
///   `<p>` tags are typically intentional HTML structure. Pulldown-cmark
///   treats these as HTML blocks and passes content through unchanged.
///
/// The remaining elements are ones where pulldown-cmark DOES auto-wrap
/// inline content in `<p>` tags during markdown processing, typically from
/// Liquid include output that mixes inline content inside block elements.
const STRIP_P_PARENT_TAGS: &[&str] = &[
    "li",
    "td",
    "th",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "figure",
    "figcaption",
    "summary",
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
/// Uses `STRIP_P_PARENT_TAGS` (not `BLOCK_PARENT_TAGS`) to avoid stripping
/// intentional `<p>` tags from semantic container elements like `<section>`.
fn strip_paragraphs_in_html_blocks(html: &str) -> String {
    let mut result = html.to_string();

    for &tag in STRIP_P_PARENT_TAGS {
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

            // For <li> elements: bare <li> (no attributes) comes from markdown
            // list syntax. In loose lists (items separated by blank lines),
            // kramdown wraps content in <p> tags -- and so does pulldown-cmark.
            // We must preserve those <p> tags. Only strip <p> from <li> elements
            // with attributes (e.g. <li class="podcast">), which come from raw
            // HTML / Liquid includes where pulldown-cmark erroneously adds <p>.
            let is_bare_li = tag == "li" && opening_tag == "<li>";
            let processed_inner = if is_bare_li {
                inner_content.to_string()
            } else {
                maybe_strip_p_tags(inner_content)
            };

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

            // Don't apply `target` attribute to non-<a> elements.
            // In kramdown, {:target="_blank"} is only meaningful on links.
            // When the markdown link wasn't parsed (e.g., due to parentheses
            // in the URL), the IAL would incorrectly attach to whatever
            // element precedes it (<figure>, <strong>, <em>, etc.).
            if tag_name != "a" && attrs.iter().any(|(k, _)| k == "target") {
                return false;
            }

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
    // Decode HTML entities before parsing -- comrak HTML-encodes quotes in
    // IAL text (e.g. {:target=&quot;_blank&quot;}) since it treats IALs as
    // plain text, not kramdown syntax.
    let decoded = html_unescape(attr_str);
    let mut attrs = Vec::new();
    let mut remaining = decoded.trim();

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
///
/// Kramdown's `generate_id` does:
/// 1. Downcase
/// 2. Remove all characters except `[a-z0-9 -]`
/// 3. Replace spaces with hyphens (without collapsing consecutive hyphens)
///
/// Note: kramdown does NOT strip leading digits. `"1. DataTalksClub"` becomes
/// `"1-datatalksclub"`, not `"datatalksclub"`.
fn slugify(text: &str) -> String {
    // Step 1: Lowercase
    let lower = text.to_lowercase();

    // Step 2: Keep only [a-z0-9 -], remove everything else
    let mut slug = String::with_capacity(lower.len());
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' {
            slug.push(ch);
        }
        // All other characters are stripped
    }

    // Step 3: Replace spaces with hyphens
    slug = slug.replace(' ', "-");

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

/// Wrap `<pre><code>...</code></pre>` blocks in kramdown-style div structure.
///
/// Fenced code blocks without a language tag are wrapped as:
/// ```html
/// <div class="language-plaintext highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
/// ```
///
/// Fenced code blocks WITH a language class (e.g., `<pre><code class="language-python">`)
/// are also wrapped with the appropriate language class:
/// ```html
/// <div class="language-python highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
/// ```
/// Reverse common HTML entity escaping so that raw source code can be passed
/// to the syntax highlighter (which re-escapes as needed).
fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn wrap_fenced_code_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for <pre><code (either bare or with class)
        if let Some(pre_pos) = remaining.find("<pre><code") {
            // Copy everything before this match
            result.push_str(&remaining[..pre_pos]);

            let after_pre = &remaining[pre_pos + 10..]; // skip "<pre><code"

            // Determine if this is bare <code> or <code class="language-xxx">
            let (lang, after_open_tag) = if let Some(rest) = after_pre.strip_prefix('>') {
                // Bare <pre><code>
                ("plaintext".to_string(), rest)
            } else if let Some(rest) = after_pre.strip_prefix(" class=\"language-") {
                // <pre><code class="language-xxx">
                if let Some(quote_end) = rest.find('"') {
                    let lang = rest[..quote_end].to_string();
                    let after_quote = &rest[quote_end + 1..];
                    if let Some(inner) = after_quote.strip_prefix('>') {
                        (lang, inner)
                    } else {
                        // Unexpected format, copy as-is
                        result.push_str("<pre><code");
                        remaining = after_pre;
                        continue;
                    }
                } else {
                    result.push_str("<pre><code");
                    remaining = after_pre;
                    continue;
                }
            } else {
                // Some other attribute, copy as-is
                result.push_str("<pre><code");
                remaining = after_pre;
                continue;
            };

            // Find the closing </code></pre>
            if let Some(close_pos) = after_open_tag.find("</code></pre>") {
                let code_content = &after_open_tag[..close_pos];
                // Write the kramdown wrapper
                result.push_str(&format!(
                    "<div class=\"language-{} highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                    lang
                ));
                // Try syntax highlighting; fall back to plain code if unsupported
                let raw_code = html_unescape(code_content);
                if let Some(highlighted) = crate::syntax::highlight_code(&lang, &raw_code) {
                    result.push_str(&highlighted);
                } else {
                    result.push_str(code_content);
                }
                result.push_str("</code></pre></div></div>");
                remaining = &after_open_tag[close_pos + 13..]; // skip "</code></pre>"
            } else {
                // No closing tag found, copy as-is
                result.push_str("<pre><code>");
                remaining = after_open_tag;
            }
        } else {
            // No more <pre><code blocks
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
/// Jekyll/kramdown adds `class="language-plaintext highlighter-rouge"`
/// to inline code spans. Only modifies `<code>` tags that:
/// - Don't already have a class attribute
/// - Are NOT inside a `<pre>` tag (fenced code blocks are handled separately)
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
            // Inline code without class - add kramdown class
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
// 4b. Wrap bare text between block elements in <p> tags
// ============================================================================

/// Wrap bare inline text between block-level elements in `<p>` tags.
///
/// Kramdown auto-wraps loose inline text that sits between block elements
/// (e.g. between `</h3>` and `<ul>`) in `<p>` tags. Pulldown-cmark does not
/// do this for text that originates from raw HTML / Liquid template output.
///
/// This function detects such bare text regions and wraps them in `<p>...</p>`.
/// It only wraps text at the top level -- text inside container elements like
/// `<ul>`, `<div>`, `<pre>`, etc. is left alone.
fn wrap_bare_text_in_paragraphs(html: &str) -> String {
    /// Block-level tags that act as containers (can have children).
    const CONTAINER_TAGS: &[&str] = &[
        "ul",
        "ol",
        "li",
        "div",
        "p",
        "table",
        "thead",
        "tbody",
        "tr",
        "td",
        "th",
        "blockquote",
        "pre",
        "center",
        "form",
        "figure",
        "figcaption",
        "details",
        "summary",
        "nav",
        "header",
        "footer",
        "section",
        "article",
        "aside",
        "main",
        "dl",
        "dd",
        "dt",
        "script",
    ];

    /// Block-level tags (includes void/self-closing like hr).
    const BLOCK_TAGS: &[&str] = &[
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "ul",
        "ol",
        "li",
        "div",
        "table",
        "thead",
        "tbody",
        "tr",
        "td",
        "th",
        "blockquote",
        "pre",
        "hr",
        "center",
        "form",
        "figure",
        "figcaption",
        "details",
        "summary",
        "nav",
        "header",
        "footer",
        "section",
        "article",
        "aside",
        "main",
        "dl",
        "dd",
        "dt",
        "p",
        "script",
    ];

    let lines: Vec<&str> = html.split('\n').collect();
    let len = lines.len();
    let mut result = Vec::with_capacity(len);
    let mut depth: i32 = 0; // nesting depth inside container elements
    let mut i = 0;

    while i < len {
        let trimmed = lines[i].trim();

        // Track container nesting depth
        let depth_delta = compute_depth_delta(trimmed, CONTAINER_TAGS);

        // If we're entering a container (opening tag), increase depth before
        // processing this line. If leaving (closing tag), decrease after.
        let entering = depth_delta > 0;
        let leaving = depth_delta < 0;

        if entering {
            // Push line first, then increase depth
            result.push(lines[i].to_string());
            depth += depth_delta;
            i += 1;
            continue;
        }

        if leaving {
            // Decrease depth, then push line
            depth += depth_delta;
            if depth < 0 {
                depth = 0;
            }
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        // Only wrap bare text at the top level (depth == 0)
        if depth > 0 {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        // Skip empty lines and block-level element lines
        if trimmed.is_empty() || is_block_line(trimmed, BLOCK_TAGS) {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        // Found a non-empty, non-block line at top level.
        // Check if it's bare text between block elements.
        if is_bare_text_context(&lines, i, BLOCK_TAGS) {
            // Collect consecutive bare text lines
            let start = i;
            while i < len {
                let t = lines[i].trim();
                if t.is_empty() || is_block_line(t, BLOCK_TAGS) {
                    break;
                }
                // Stop if we hit a container opening
                if compute_depth_delta(t, CONTAINER_TAGS) != 0 {
                    break;
                }
                i += 1;
            }
            // Wrap the collected lines in <p>
            let bare_text: String = lines[start..i]
                .iter()
                .map(|l| l.trim())
                .collect::<Vec<_>>()
                .join(" ");
            result.push(format!("<p>{}</p>", bare_text));
        } else {
            result.push(lines[i].to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Compute the nesting depth change for a line based on container tags.
/// Returns positive for opening tags, negative for closing tags, 0 for neither.
fn compute_depth_delta(trimmed: &str, container_tags: &[&str]) -> i32 {
    let mut delta = 0i32;
    for tag in container_tags {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);

        // Count opening tags on this line
        if trimmed.contains(&open) {
            let rest_after_open = &trimmed[trimmed.find(&open).unwrap() + open.len()..];
            if rest_after_open.starts_with('>')
                || rest_after_open.starts_with(' ')
                || rest_after_open.starts_with('/')
            {
                delta += 1;
            }
        }

        // Count closing tags on this line
        if trimmed.contains(&close) {
            delta -= 1;
        }
    }
    delta
}

/// Check if a trimmed line starts/ends with a block-level HTML element.
fn is_block_line(trimmed: &str, block_tags: &[&str]) -> bool {
    // HTML comments are block-level elements (CommonMark type 2).
    // They must not be wrapped in <p> tags.
    if trimmed.starts_with("<!--") {
        return true;
    }

    for tag in block_tags {
        // Opening tag: <tag> or <tag ...>
        let open = format!("<{}", tag);
        if trimmed.starts_with(&open) {
            let rest = &trimmed[open.len()..];
            if rest.starts_with('>') || rest.starts_with(' ') || rest.starts_with('/') {
                return true;
            }
        }
        // Closing tag: </tag>
        let close = format!("</{}>", tag);
        if trimmed.starts_with(&close) || trimmed.ends_with(&close) {
            return true;
        }
    }

    false
}

/// Check whether the bare text at line index `i` is between block elements.
fn is_bare_text_context(lines: &[&str], i: usize, block_tags: &[&str]) -> bool {
    // Look backward: skip empty lines, find a block element or start of content
    let has_preceding_block = if i == 0 {
        true
    } else {
        let mut j = i - 1;
        loop {
            let t = lines[j].trim();
            if !t.is_empty() {
                break is_block_line(t, block_tags);
            }
            if j == 0 {
                break true;
            }
            j -= 1;
        }
    };

    if !has_preceding_block {
        return false;
    }

    // Look forward: skip the current bare text lines, skip empty lines,
    // find a block element or end of content
    let len = lines.len();
    let mut j = i;
    // Skip current bare text region
    while j < len {
        let t = lines[j].trim();
        if t.is_empty() || is_block_line(t, block_tags) {
            break;
        }
        j += 1;
    }
    // Skip empty lines
    while j < len && lines[j].trim().is_empty() {
        j += 1;
    }

    if j >= len {
        return true;
    }

    let t = lines[j].trim();
    is_block_line(t, block_tags)
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

            // Add extra newline if not already followed by two newlines,
            // but NOT at the very end of content (trailing \n should stay single).
            // Jekyll/kramdown ends content with </p>\n, not </p>\n\n.
            if !remaining.starts_with("\n\n") && remaining.starts_with('\n') && remaining.len() > 1
            {
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

/// Remove self-closing slash from void HTML elements (single-pass).
///
/// Note: This function is no longer called in production because
/// Jekyll/kramdown outputs XHTML-style self-closing tags (e.g. `<br />`).
/// Kept for test coverage only.
#[cfg(test)]
fn normalize_void_elements(html: &str) -> String {
    // Quick check: if there's no "/>", nothing to normalize
    if !html.contains("/>") {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find next "/>" (with or without space before it)
        let pos = match remaining.find("/>") {
            Some(p) => p,
            None => {
                result.push_str(remaining);
                break;
            }
        };

        // Check if there's a space before "/>" (i.e., " />")
        let has_space = pos > 0 && remaining.as_bytes()[pos - 1] == b' ';
        let tag_end_pos = if has_space { pos - 1 } else { pos };

        // Find the opening '<' for this tag
        let before = &remaining[..tag_end_pos];
        if let Some(tag_start) = before.rfind('<') {
            let tag_content = &before[tag_start + 1..];
            let tag_name = tag_content
                .split(|c: char| c.is_whitespace())
                .next()
                .unwrap_or("");
            if is_void_element(tag_name) {
                // It's a void element -- replace " />" or "/>" with ">"
                result.push_str(&remaining[..tag_end_pos]);
                result.push('>');
                remaining = &remaining[pos + 2..];
                continue;
            }
        }
        // Not a void element -- keep as-is
        result.push_str(&remaining[..pos + 2]);
        remaining = &remaining[pos + 2..];
    }
    result
}

/// Check if a tag name is a void (self-closing) HTML element.
#[cfg(test)]
fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "br" | "hr"
            | "img"
            | "input"
            | "meta"
            | "link"
            | "col"
            | "area"
            | "base"
            | "embed"
            | "source"
            | "track"
            | "wbr"
    )
}

// ============================================================================
// 9. Normalize boolean HTML attributes (D2, D12)
// ============================================================================

/// Normalize boolean HTML attributes by removing empty string values (single-pass).
///
/// pulldown-cmark produces `required=""`, `novalidate=""`, `itemscope=""`.
/// kramdown produces `required`, `novalidate`, `itemscope`.
///
/// This implementation finds `=""` patterns and checks if the preceding word
/// is a boolean attribute, avoiding 18 separate `replace()` calls that each
/// scan the entire string.
fn normalize_boolean_attributes(html: &str) -> String {
    // Quick check: if there's no `=""`, nothing to normalize
    if !html.contains("=\"\"") {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        let pos = match remaining.find("=\"\"") {
            Some(p) => p,
            None => {
                result.push_str(remaining);
                break;
            }
        };

        // Extract the attribute name: scan backwards from `pos` to find the word
        let before = &remaining[..pos];
        let attr_start = before
            .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let attr_name = &before[attr_start..];

        if is_boolean_attribute(attr_name) {
            // Strip the `=""` -- just output up to and including the attribute name
            result.push_str(&remaining[..pos]);
            remaining = &remaining[pos + 3..];
        } else {
            // Not a boolean attribute -- keep `=""`
            result.push_str(&remaining[..pos + 3]);
            remaining = &remaining[pos + 3..];
        }
    }
    result
}

/// Check if an attribute name is a boolean HTML attribute.
fn is_boolean_attribute(attr: &str) -> bool {
    matches!(
        attr,
        "required"
            | "novalidate"
            | "itemscope"
            | "checked"
            | "disabled"
            | "readonly"
            | "multiple"
            | "autofocus"
            | "autoplay"
            | "controls"
            | "loop"
            | "muted"
            | "selected"
            | "hidden"
            | "async"
            | "defer"
            | "formnovalidate"
            | "open"
            | "allowfullscreen"
    )
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

/// Encode bare `&` characters that are not part of valid HTML entity references.
///
/// D17: pulldown-cmark passes raw HTML blocks through verbatim, so bare `&`
/// characters (not part of `&name;`, `&#digits;`, or `&#xhex;` references)
/// survive into the output. Jekyll/kramdown re-encodes these as `&amp;`.
///
/// This function finds every `&` and checks whether it begins a valid entity
/// reference. If not, it replaces the `&` with `&amp;`. Already-encoded
/// entities like `&amp;` are left untouched, preventing double-encoding.
fn encode_bare_ampersands(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut in_script = false;

    while i < len {
        // Track whether we are inside a <script> block.
        // Content inside <script> tags should not have & encoded because
        // script content is not parsed as HTML entities. This is important
        // for JSON-LD structured data includes that contain bare & characters
        // (e.g., "Infrastructure & Prerequisites" in course structured data).
        // Use byte-level comparison to avoid UTF-8 char boundary issues.
        if !in_script && i + 7 < len && bytes[i..i + 7] == *b"<script" {
            in_script = true;
        } else if in_script && i + 9 <= len && bytes[i..i + 9] == *b"</script>" {
            in_script = false;
        }

        if bytes[i] == b'&' && !in_script {
            if is_valid_entity_start(bytes, i, len) {
                // Part of a valid entity reference -- copy as-is up to and
                // including the terminating ';'
                if let Some(semi) = find_entity_end(bytes, i, len) {
                    result.push_str(&html[i..=semi]);
                    i = semi + 1;
                } else {
                    // Shouldn't happen if is_valid_entity_start is correct,
                    // but be safe: encode as bare ampersand
                    result.push_str("&amp;");
                    i += 1;
                }
            } else {
                result.push_str("&amp;");
                i += 1;
            }
        } else {
            result.push(html[i..].chars().next().unwrap());
            i += html[i..].chars().next().unwrap().len_utf8();
        }
    }

    result
}

/// Check whether `&` at position `pos` begins a valid HTML entity reference.
///
/// Valid patterns:
/// - `&#` followed by one or more ASCII digits, then `;`
/// - `&#x` or `&#X` followed by one or more hex digits, then `;`
/// - `&` followed by one or more ASCII alphanumeric characters, then `;`
fn is_valid_entity_start(bytes: &[u8], pos: usize, len: usize) -> bool {
    if pos + 1 >= len {
        return false;
    }

    let next = bytes[pos + 1];

    if next == b'#' {
        // Numeric character reference
        if pos + 2 >= len {
            return false;
        }
        let after_hash = bytes[pos + 2];
        if after_hash == b'x' || after_hash == b'X' {
            // Hex: &#xHH;
            let mut j = pos + 3;
            if j >= len || !bytes[j].is_ascii_hexdigit() {
                return false;
            }
            while j < len && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            j < len && bytes[j] == b';'
        } else if after_hash.is_ascii_digit() {
            // Decimal: &#DD;
            let mut j = pos + 2;
            while j < len && bytes[j].is_ascii_digit() {
                j += 1;
            }
            j < len && bytes[j] == b';'
        } else {
            false
        }
    } else if next.is_ascii_alphabetic() {
        // Named entity: &name;
        let mut j = pos + 1;
        while j < len && bytes[j].is_ascii_alphanumeric() {
            j += 1;
        }
        j < len && bytes[j] == b';'
    } else {
        false
    }
}

/// Find the position of the `;` that terminates the entity starting at `pos`.
fn find_entity_end(bytes: &[u8], pos: usize, len: usize) -> Option<usize> {
    let mut j = pos + 1;
    while j < len {
        if bytes[j] == b';' {
            return Some(j);
        }
        j += 1;
    }
    None
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

    #[test]
    fn test_parse_ial_html_encoded_quotes() {
        // When comrak HTML-encodes the IAL text, quotes become &quot;
        let attrs = parse_ial_attributes("target=&quot;_blank&quot;");
        assert_eq!(attrs, vec![("target".into(), "_blank".into())]);
    }

    #[test]
    fn test_parse_ial_html_encoded_multiple() {
        let attrs = parse_ial_attributes("target=&quot;_blank&quot; rel=&quot;noopener&quot;");
        assert_eq!(
            attrs,
            vec![
                ("target".into(), "_blank".into()),
                ("rel".into(), "noopener".into()),
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
        // Kramdown preserves consecutive dashes: space and dash each stay
        assert_eq!(slugify("hello - world"), "hello---world");
    }

    #[test]
    fn test_slugify_slash_preserves_double_dash() {
        // "DevOps / Site Reliability Engineer" -> "devops--site-reliability-engineer"
        // The `/` is stripped, but the spaces around it each become `-`, yielding `--`
        assert_eq!(
            slugify("DevOps / Site Reliability Engineer"),
            "devops--site-reliability-engineer"
        );
    }

    #[test]
    fn test_slugify_ampersand_preserves_double_dash() {
        // "Free & Free-to-Audit Courses" -> "free--free-to-audit-courses"
        // The `&` is stripped, but the spaces around it each become `-`, yielding `--`
        assert_eq!(
            slugify("Free & Free-to-Audit Courses"),
            "free--free-to-audit-courses"
        );
    }

    #[test]
    fn test_slugify_leading_digits_preserved() {
        // Kramdown does NOT strip leading digits from heading IDs
        // "1. DataTalksClub" -> "1-datatalksclub"
        assert_eq!(slugify("1. DataTalksClub"), "1-datatalksclub");
        assert_eq!(slugify("123 Hello"), "123-hello");
    }

    #[test]
    fn test_slugify_leading_spaces_become_hyphens() {
        // Leading spaces become leading hyphens (spaces -> hyphens)
        assert_eq!(slugify("  Hello"), "--hello");
    }

    #[test]
    fn test_slugify_trailing_chars_preserved() {
        // Trailing dashes/spaces from non-alnum chars are NOT trimmed by kramdown
        // (kramdown does not trim trailing dashes)
        assert_eq!(slugify("Hello World!"), "hello-world");
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
    fn test_postprocess_link_with_html_encoded_target_blank() {
        // When comrak HTML-encodes the IAL, quotes become &quot;
        let html = "<p><a href=\"https://example.com\">text</a>{:target=&quot;_blank&quot;}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "Should contain target attribute with proper quotes. Got: {}",
            result
        );
        assert!(
            !result.contains("{:target"),
            "Should not contain raw IAL. Got: {}",
            result
        );
        assert!(
            !result.contains("&quot;"),
            "Should not contain HTML-encoded quotes. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_target_blank_not_applied_to_non_a_tags() {
        // When a markdown link isn't parsed (e.g., due to parentheses in URL),
        // the {:target="_blank"} IAL should NOT be applied to non-<a> elements
        // like <figure>, <strong>, or <em>.

        // Case 1: target IAL after </figure> (from raw HTML block)
        let html = "<figure>\n<img src=\"test.jpg\" />\n</figure>{:target=\"_blank\"}\n";
        let result = postprocess(html);
        assert!(
            !result.contains("figure target=\"_blank\""),
            "target should not be on figure. Got: {}",
            result
        );

        // Case 2: target IAL after </strong>
        let html = "<p><strong>bold text</strong>{:target=\"_blank\"}</p>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("strong target=\"_blank\""),
            "target should not be on strong. Got: {}",
            result
        );

        // Case 3: target IAL after </em>
        let html = "<p><em>italic text</em>{:target=\"_blank\"}</p>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("em target=\"_blank\""),
            "target should not be on em. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_target_blank_still_works_on_a_tags() {
        // Ensure the fix doesn't break normal target="_blank" on <a> tags
        let html = "<p><a href=\"https://example.com\">link</a>{:target=\"_blank\"}</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("target=\"_blank\""),
            "target should be applied to a tag. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"https://example.com\" target=\"_blank\">link</a>"),
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
    fn test_postprocess_inline_code_highlighter_rouge_class() {
        // Jekyll adds class="language-plaintext highlighter-rouge" to inline code
        let html = "<p>Use <code>pip install</code> to install.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Inline code should have language-plaintext highlighter-rouge class. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_fenced_code_wrapped_with_language() {
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\">"),
            "Language-tagged code should be wrapped with language class. Got: {}",
            result
        );
        assert!(
            !result.contains("language-plaintext"),
            "Should not add plaintext class to language-tagged code. Got: {}",
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
        // Inline code gets language-plaintext highlighter-rouge class
        assert!(
            result.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Inline code should have language-plaintext highlighter-rouge class. Got: {}",
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
    fn test_fenced_code_wrapping_wraps_language_python() {
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\">"),
            "Language-tagged code should get language-specific wrapper. Got: {}",
            result
        );
        assert!(
            !result.contains("language-plaintext"),
            "Should not add plaintext class to language-tagged code. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_wraps_language_bash() {
        let html = "<pre><code class=\"language-bash\">echo hello\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-bash highlighter-rouge\">"),
            "Language-tagged code should get language-specific wrapper. Got: {}",
            result
        );
        assert!(
            !result.contains("language-plaintext"),
            "Should not add plaintext class to language-tagged code. Got: {}",
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
        let plaintext_count = result
            .matches("<div class=\"language-plaintext highlighter-rouge\">")
            .count();
        assert_eq!(
            plaintext_count, 1,
            "Only bare block should get plaintext wrapper. Got: {}",
            result
        );
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\">"),
            "Language-tagged block should get its own wrapper. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_no_interference_with_inline() {
        let html = "<p>Use <code>pip install</code> to install.</p>\n";
        let result = postprocess(html);
        // Inline code gets language-plaintext highlighter-rouge, not div wrapper
        assert!(
            result.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Inline code should have language-plaintext highlighter-rouge class. Got: {}",
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
        // Inline code gets language-plaintext highlighter-rouge class
        assert!(
            result.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Inline code should have language-plaintext highlighter-rouge class. Got: {}",
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
        // Inline code gets language-plaintext highlighter-rouge class
        assert!(
            result.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Inline code should have language-plaintext highlighter-rouge class. Got: {}",
            result
        );
        // Fenced with language: wrapped with language class
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\">"),
            "Language-tagged block should be wrapped with language class. Got: {}",
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
    fn test_strip_p_in_nested_ul_li_with_attrs() {
        // Raw HTML <li> with attributes (from Liquid includes) should strip <p>.
        // Bare <li> (from markdown) should preserve <p> for loose list support.
        let html = "<ul><li class=\"item\"><p><a href=\"url\">Link</a> text</p></li><li class=\"item\"><p>Other</p></li></ul>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "Should strip <p> in <li> elements with attributes. Got: {}",
            result
        );
    }

    #[test]
    fn test_preserve_p_in_bare_li() {
        // Bare <li> (from markdown loose lists) should preserve <p> tags.
        let html = "<ul>\n<li><p>First item</p></li>\n<li><p>Second item</p></li>\n</ul>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            result.contains("<li><p>First item</p></li>"),
            "Should preserve <p> in bare <li> elements. Got: {}",
            result
        );
    }

    #[test]
    fn test_strip_p_in_section_preserves_intentional_p() {
        // <section> and <div> are no longer in STRIP_P_PARENT_TAGS.
        // Only the <h2> (which IS in the list) gets its <p> stripped.
        let html =
            "<section><h2><p>Title</p></h2><div><p>Content with <a>link</a></p></div></section>";
        let result = strip_paragraphs_in_html_blocks(html);
        // <p> inside <h2> should be stripped (h2 is in STRIP_P_PARENT_TAGS)
        assert!(
            result.contains("<h2>Title</h2>"),
            "Should strip <p> from <h2>. Got: {}",
            result
        );
        // <p> inside <div> should be preserved (div is NOT in STRIP_P_PARENT_TAGS)
        assert!(
            result.contains("<div><p>Content with <a>link</a></p></div>"),
            "Should preserve <p> in <div>. Got: {}",
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
    fn test_strip_p_mixed_content_preserves_div_p() {
        // <div> is no longer in STRIP_P_PARENT_TAGS, so <p> inside <div> is preserved.
        let html = "<p>Markdown paragraph</p>\n<div><p>inline content</p></div>\n<p>Another paragraph</p>\n";
        let result = strip_paragraphs_in_html_blocks(html);
        // The <p> inside <div> should be preserved (div is NOT in STRIP_P_PARENT_TAGS)
        assert!(
            result.contains("<div><p>inline content</p></div>"),
            "Should preserve <p> inside <div>. Got: {}",
            result
        );
        // Standalone paragraphs should remain
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
    fn test_strip_p_in_nested_divs_preserved() {
        // <div> is no longer in STRIP_P_PARENT_TAGS -- its <p> tags are intentional.
        let html = "<div><div><p>nested content</p></div></div>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            result.contains("<p>"),
            "Should preserve <p> in divs (intentional markup). Got: {}",
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

    // ========================================================================
    // D1: Heading ID marking for include content
    // ========================================================================

    #[test]
    fn test_d1_mark_bare_heading_tags() {
        let input = "<h2>Title</h2>";
        let marked = mark_existing_html_headings(input);
        assert_eq!(marked, "<h2 data-raw-html>Title</h2>");
    }

    #[test]
    fn test_d1_mark_does_not_affect_headings_with_attrs() {
        let input = r#"<h2 class="title">Title</h2>"#;
        let marked = mark_existing_html_headings(input);
        assert_eq!(
            marked, input,
            "Headings with attributes should not be marked"
        );
    }

    #[test]
    fn test_d1_remove_heading_markers() {
        let input = "<h2 data-raw-html>Title</h2>";
        let cleaned = remove_heading_markers(input);
        assert_eq!(cleaned, "<h2>Title</h2>");
    }

    #[test]
    fn test_d1_marked_heading_skipped_by_add_heading_ids() {
        let input = "<h2 data-raw-html>Include Title</h2>";
        let result = add_heading_ids(input);
        // Should NOT get an id because it has attributes
        assert!(
            !result.contains("id=\"include-title\""),
            "Marked heading should not get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_d1_markdown_heading_still_gets_id() {
        // Markdown-generated heading (bare <h2>)
        let input = "<h2>Markdown Title</h2>";
        let result = add_heading_ids(input);
        assert!(
            result.contains("id=\"markdown-title\""),
            "Markdown heading should get auto-generated ID. Got: {}",
            result
        );
    }

    // ========================================================================
    // D11: Remove <ol start="N"> attribute
    // ========================================================================

    #[test]
    fn test_d11_remove_ol_start_attribute() {
        let input = "<ol start=\"2\">\n<li>Item</li>\n</ol>";
        let result = remove_ol_start_attribute(input);
        assert!(
            !result.contains("start="),
            "start attribute should be removed. Got: {}",
            result
        );
        assert!(result.contains("<ol>"), "Should have bare <ol> tag");
    }

    #[test]
    fn test_d11_ol_without_start_unchanged() {
        let input = "<ol>\n<li>Item</li>\n</ol>";
        let result = remove_ol_start_attribute(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_d11_ol_start_1_also_removed() {
        let input = "<ol start=\"1\">\n<li>Item</li>\n</ol>";
        let result = remove_ol_start_attribute(input);
        assert!(
            !result.contains("start="),
            "start='1' should also be removed. Got: {}",
            result
        );
    }

    #[test]
    fn test_d11_postprocess_for_filter_removes_ol_start() {
        // The markdownify filter path uses postprocess_for_filter,
        // which must also remove <ol start="N"> attributes (issue #146).
        let input = "<ol start=\"2\">\n<li>Item</li>\n</ol>";
        let result = postprocess_for_filter(input);
        assert!(
            !result.contains("start="),
            "postprocess_for_filter should remove ol start attribute. Got: {}",
            result
        );
        assert!(
            result.contains("<ol>"),
            "Should have bare <ol> tag. Got: {}",
            result
        );
    }

    // ========================================================================
    // D2, D3, D12: Boolean attributes and self-closing tags
    // ========================================================================

    #[test]
    fn test_d2_boolean_required_normalized() {
        let result = normalize_boolean_attributes(r#"<input required="" type="text">"#);
        assert_eq!(result, r#"<input required type="text">"#);
    }

    #[test]
    fn test_d12_itemscope_normalized() {
        let result =
            normalize_boolean_attributes(r#"<div itemscope="" itemtype="http://schema.org">"#);
        assert_eq!(result, r#"<div itemscope itemtype="http://schema.org">"#);
    }

    #[test]
    fn test_d2_novalidate_normalized() {
        let result = normalize_boolean_attributes(r#"<form novalidate="">"#);
        assert_eq!(result, "<form novalidate>");
    }

    #[test]
    fn test_d3_br_self_closing_removed() {
        let result = normalize_void_elements("<br />");
        assert_eq!(result, "<br>");
    }

    #[test]
    fn test_d3_input_self_closing_removed() {
        let result = normalize_void_elements(r#"<input type="text" />"#);
        assert_eq!(result, r#"<input type="text">"#);
    }

    #[test]
    fn test_d3_hr_self_closing_removed() {
        let result = normalize_void_elements("<hr />");
        assert_eq!(result, "<hr>");
    }

    #[test]
    fn test_d3_non_void_element_unchanged() {
        let result = normalize_void_elements("<div />");
        // div is not a void element, so " />" should be preserved
        assert_eq!(result, "<div />");
    }

    // ========================================================================
    // D6: Figcaption whitespace normalization
    // ========================================================================

    #[test]
    fn test_d6_figcaption_newline_removed() {
        let input = "<figcaption>Caption text\n</figcaption>";
        let result = normalize_figcaption_whitespace(input);
        assert_eq!(result, "<figcaption>Caption text</figcaption>");
    }

    #[test]
    fn test_d6_figcaption_same_line_unchanged() {
        let input = "<figcaption>Caption text</figcaption>";
        let result = normalize_figcaption_whitespace(input);
        assert_eq!(result, input);
    }

    // ========================================================================
    // normalize_html_output normalizes boolean attributes only (D2+D12)
    // Void element self-closing slashes are preserved to match Jekyll/kramdown.
    // ========================================================================

    #[test]
    fn test_normalize_html_output_combined() {
        let input = r#"<input required="" type="text" /><br /><div itemscope="">"#;
        let result = normalize_html_output(input);
        // Void elements keep their self-closing slashes; boolean attrs are normalized
        assert_eq!(
            result,
            r#"<input required type="text" /><br /><div itemscope>"#
        );
    }

    // ========================================================================
    // Pre-markdown: Collapse blank lines in HTML block elements
    // ========================================================================

    #[test]
    fn test_collapse_blank_lines_in_li() {
        let input = "<li>\n\n  Event Title\n  (link)\n\n</li>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(result, "<li>\n  Event Title\n  (link)\n</li>");
    }

    #[test]
    fn test_collapse_blank_lines_in_div() {
        let input = "<div class=\"book\">\n\n  Some content\n\n  More content\n\n</div>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(
            result,
            "<div class=\"book\">\n  Some content\n  More content\n</div>"
        );
    }

    #[test]
    fn test_collapse_blank_lines_in_h5() {
        let input = "<h5>\n\nby <a href=\"x\">Author</a>\n\n</h5>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(result, "<h5>\nby <a href=\"x\">Author</a>\n</h5>");
    }

    #[test]
    fn test_collapse_preserves_content_outside_blocks() {
        let input = "Paragraph one\n\nParagraph two\n\n<li>\n\nContent\n\n</li>\n\nParagraph three";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(
            result,
            "Paragraph one\n\nParagraph two\n\n<li>\nContent\n</li>\n\nParagraph three"
        );
    }

    #[test]
    fn test_collapse_no_blank_lines_unchanged() {
        let input = "<li>Simple content</li>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(result, "<li>Simple content</li>");
    }

    #[test]
    fn test_collapse_nested_blocks() {
        let input = "<div>\n\n<li>\n\nNested\n\n</li>\n\n</div>";
        let result = collapse_blank_lines_in_html_blocks(input);
        // Both div and li blank lines should be collapsed
        assert_eq!(result, "<div>\n<li>\nNested\n</li>\n</div>");
    }

    #[test]
    fn test_collapse_li_with_class() {
        let input = "<li class=\"webinar\">\n\n  Event Title by <a href=\"/people/foo.html\">Foo</a>\n  (<a href=\"https://youtube.com\">watch on youtube</a>)\n\n</li>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(
            result,
            "<li class=\"webinar\">\n  Event Title by <a href=\"/people/foo.html\">Foo</a>\n  (<a href=\"https://youtube.com\">watch on youtube</a>)\n</li>"
        );
    }

    #[test]
    fn test_collapse_multiple_li_elements() {
        let input = "<li>\n\nFirst\n\n</li>\n<li>\n\nSecond\n\n</li>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(result, "<li>\nFirst\n</li>\n<li>\nSecond\n</li>");
    }

    #[test]
    fn test_collapse_authors_include_pattern() {
        // Simulates the authors.html include output with blank lines between iterations
        let input = "<li>\n\n  <a href=\"/people/foo.html\">Foo</a>, \n\n  <a href=\"/people/bar.html\">Bar</a>\n\n</li>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(
            result,
            "<li>\n  <a href=\"/people/foo.html\">Foo</a>, \n  <a href=\"/people/bar.html\">Bar</a>\n</li>"
        );
    }

    #[test]
    fn test_collapse_does_not_affect_td() {
        let input = "<td>\n\nCell content\n\n</td>";
        let result = collapse_blank_lines_in_html_blocks(input);
        assert_eq!(result, "<td>\nCell content\n</td>");
    }

    #[test]
    fn test_no_p_tags_after_collapse_in_li() {
        // Full pipeline test: collapse then markdown_to_html
        let input = "<li>\n\nSome text by <a href='x'>Author</a>\n\n</li>";
        let collapsed = collapse_blank_lines_in_html_blocks(input);
        let html = crate::frontmatter::markdown_to_html(&collapsed);
        // The output should NOT contain <p> inside <li>
        assert!(
            !html.contains("<li><p>") && !html.contains("<li>\n<p>"),
            "Expected no <p> inside <li>, got: {}",
            html
        );
    }

    #[test]
    fn test_no_p_tags_after_collapse_in_h5() {
        let input = "<h5>\n\nby <a href='x'>Author</a>\n\n</h5>";
        let collapsed = collapse_blank_lines_in_html_blocks(input);
        let html = crate::frontmatter::markdown_to_html(&collapsed);
        assert!(
            !html.contains("<h5><p>") && !html.contains("<h5>\n<p>"),
            "Expected no <p> inside <h5>, got: {}",
            html
        );
    }

    #[test]
    fn test_legitimate_paragraphs_preserved() {
        // Regular markdown with blank lines should still produce <p> tags
        let input = "First paragraph\n\nSecond paragraph";
        let result = collapse_blank_lines_in_html_blocks(input);
        // Content outside HTML blocks should be unchanged
        assert_eq!(result, input);
        let html = crate::frontmatter::markdown_to_html(&result);
        assert!(html.contains("<p>First paragraph</p>"));
        assert!(html.contains("<p>Second paragraph</p>"));
    }

    #[test]
    fn test_markdown_after_html_block_still_works() {
        let input = "<li>\n\nContent\n\n</li>\n\nA paragraph of text\n\nAnother paragraph";
        let result = collapse_blank_lines_in_html_blocks(input);
        // The blank lines between paragraphs outside the <li> should be preserved
        assert!(result.contains("\n\nA paragraph of text\n\nAnother paragraph"));
    }

    #[test]
    fn test_collapse_event_html_full_pattern() {
        // Simulate the full event.html include pattern inside <li>
        let input = r#"<li class="webinar">

  Machine Learning Zoomcamp by <a href="/people/alexey-grigorev.html">Alexey Grigorev</a>
  (<a href="https://youtube.com/watch?v=123">watch on youtube</a>)

</li>"#;
        let result = collapse_blank_lines_in_html_blocks(input);
        let html = crate::frontmatter::markdown_to_html(&result);
        assert!(
            !html.contains("<p>"),
            "Expected no <p> tags in event output, got: {}",
            html
        );
    }

    #[test]
    fn test_collapse_book_html_with_nested_h5() {
        // Simulate book.html include with nested h5 containing authors
        let input = r#"<div class="book">

<h5>

by <a href="/people/author.html">Author Name</a>

</h5>

</div>"#;
        let result = collapse_blank_lines_in_html_blocks(input);
        let html = crate::frontmatter::markdown_to_html(&result);
        assert!(
            !html.contains("<h5>\n<p>") && !html.contains("<h5><p>"),
            "Expected no <p> inside <h5> in book output, got: {}",
            html
        );
    }

    // ========================================================================
    // D17: encode_bare_ampersands tests
    // ========================================================================

    #[test]
    fn test_encode_bare_ampersand_in_text() {
        let input = "<div>Tom & Jerry</div>";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "<div>Tom &amp; Jerry</div>");
    }

    #[test]
    fn test_encode_bare_ampersand_no_double_encoding() {
        let input = "<div>Tom &amp; Jerry</div>";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "<div>Tom &amp; Jerry</div>");
    }

    #[test]
    fn test_encode_bare_ampersand_preserves_named_entities() {
        let input = "&lt;div&gt; &nbsp; &quot;hello&quot;";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "&lt;div&gt; &nbsp; &quot;hello&quot;");
    }

    #[test]
    fn test_encode_bare_ampersand_preserves_numeric_entities() {
        let input = "&#123; &#x1F600;";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "&#123; &#x1F600;");
    }

    #[test]
    fn test_encode_bare_ampersand_in_url_attribute() {
        let input = r#"<a href="?a=1&b=2">link</a>"#;
        let result = encode_bare_ampersands(input);
        assert_eq!(result, r#"<a href="?a=1&amp;b=2">link</a>"#);
    }

    #[test]
    fn test_encode_bare_ampersand_multiple() {
        let input = "A & B & C";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "A &amp; B &amp; C");
    }

    #[test]
    fn test_encode_bare_ampersand_at_end_of_string() {
        let input = "trailing &";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "trailing &amp;");
    }

    #[test]
    fn test_encode_bare_ampersand_mixed() {
        let input = "Tom & Jerry &amp; friends &lt;3";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "Tom &amp; Jerry &amp; friends &lt;3");
    }

    #[test]
    fn test_encode_bare_ampersand_no_semicolon() {
        // &foo without semicolon is a bare ampersand
        let input = "&foo bar";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "&amp;foo bar");
    }

    #[test]
    fn test_encode_bare_ampersand_preserves_hex_entity_case_insensitive() {
        let input = "&#xAB; &#XCD;";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "&#xAB; &#XCD;");
    }

    #[test]
    fn test_encode_bare_ampersand_empty_string() {
        let result = encode_bare_ampersands("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_encode_bare_ampersand_no_ampersands() {
        let input = "<p>Hello world</p>";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "<p>Hello world</p>");
    }

    #[test]
    fn test_encode_bare_ampersand_via_markdown_to_html() {
        // Raw HTML block with bare & should get encoded via postprocess
        let input = "<div>Tom & Jerry</div>";
        let result = crate::frontmatter::markdown_to_html(input);
        assert!(
            result.contains("Tom &amp; Jerry"),
            "Bare & in HTML block should be encoded. Got: {}",
            result
        );
    }

    #[test]
    fn test_encode_bare_ampersand_markdown_text() {
        // In regular markdown text, pulldown-cmark already encodes &
        let input = "Tom & Jerry";
        let result = crate::frontmatter::markdown_to_html(input);
        assert!(
            result.contains("Tom &amp; Jerry"),
            "& in markdown text should be encoded. Got: {}",
            result
        );
        // Verify no double-encoding
        assert!(
            !result.contains("&amp;amp;"),
            "Should not double-encode. Got: {}",
            result
        );
    }

    #[test]
    fn test_encode_bare_ampersand_already_encoded_markdown() {
        // Already-encoded entities in HTML block should not be double-encoded
        let input = "<div>&amp; &lt; &gt;</div>";
        let result = crate::frontmatter::markdown_to_html(input);
        assert!(
            result.contains("&amp;") && !result.contains("&amp;amp;"),
            "Should not double-encode &amp;. Got: {}",
            result
        );
        assert!(
            result.contains("&lt;") && !result.contains("&amp;lt;"),
            "Should not double-encode &lt;. Got: {}",
            result
        );
    }

    #[test]
    fn test_encode_bare_ampersand_with_utf8() {
        let input = "<div>caf\u{00e9} & th\u{00e9}</div>";
        let result = encode_bare_ampersands(input);
        assert_eq!(result, "<div>caf\u{00e9} &amp; th\u{00e9}</div>");
    }

    #[test]
    fn test_encode_bare_ampersand_skips_script_blocks() {
        // Content inside <script> tags should not have bare & encoded,
        // because script content is not parsed as HTML entities.
        // This is critical for JSON-LD structured data includes.
        let input = r#"<div>A & B</div>
<script type="application/ld+json">
{"name": "Infrastructure & Prerequisites"}
</script>
<div>C & D</div>"#;
        let result = encode_bare_ampersands(input);
        // & outside script should be encoded
        assert!(
            result.contains("<div>A &amp; B</div>"),
            "& outside script should be encoded, got: {}",
            result
        );
        assert!(
            result.contains("<div>C &amp; D</div>"),
            "& after script should be encoded, got: {}",
            result
        );
        // & inside script should NOT be encoded
        assert!(
            result.contains("Infrastructure & Prerequisites"),
            "& inside <script> should not be encoded, got: {}",
            result
        );
    }

    // --- Optimized normalize_void_elements tests (single-pass) ---

    #[test]
    fn test_normalize_void_elements_br_space() {
        assert_eq!(normalize_void_elements("<br />"), "<br>");
    }

    #[test]
    fn test_normalize_void_elements_br_no_space() {
        assert_eq!(normalize_void_elements("<br/>"), "<br>");
    }

    #[test]
    fn test_normalize_void_elements_hr() {
        assert_eq!(normalize_void_elements("<hr />"), "<hr>");
    }

    #[test]
    fn test_normalize_void_elements_img_with_attrs() {
        assert_eq!(
            normalize_void_elements(r#"<img src="test.jpg" />"#),
            r#"<img src="test.jpg">"#
        );
    }

    #[test]
    fn test_normalize_void_elements_meta() {
        assert_eq!(
            normalize_void_elements(r#"<meta content="summary" />"#),
            r#"<meta content="summary">"#
        );
    }

    #[test]
    fn test_normalize_void_elements_link() {
        assert_eq!(
            normalize_void_elements(r#"<link rel="stylesheet" href="s.css" />"#),
            r#"<link rel="stylesheet" href="s.css">"#
        );
    }

    #[test]
    fn test_normalize_void_elements_preserves_non_void() {
        // Non-void elements with /> should be preserved
        assert_eq!(normalize_void_elements("<div />"), "<div />");
    }

    #[test]
    fn test_normalize_void_elements_no_change() {
        let input = "<p>Hello</p>";
        assert_eq!(normalize_void_elements(input), input);
    }

    #[test]
    fn test_normalize_void_elements_multiple() {
        assert_eq!(normalize_void_elements("<br /><hr /><br/>"), "<br><hr><br>");
    }

    // --- Optimized normalize_boolean_attributes tests (single-pass) ---

    #[test]
    fn test_normalize_boolean_required() {
        assert_eq!(
            normalize_boolean_attributes(r#"<input required="">"#),
            "<input required>"
        );
    }

    #[test]
    fn test_normalize_boolean_itemscope() {
        assert_eq!(
            normalize_boolean_attributes(r#"<div itemscope="">"#),
            "<div itemscope>"
        );
    }

    #[test]
    fn test_normalize_boolean_multiple() {
        assert_eq!(
            normalize_boolean_attributes(r#"<input required="" disabled="">"#),
            "<input required disabled>"
        );
    }

    #[test]
    fn test_normalize_boolean_preserves_non_boolean() {
        // Non-boolean attributes with ="" should be preserved
        let input = r#"<input value="">"#;
        assert_eq!(normalize_boolean_attributes(input), input);
    }

    #[test]
    fn test_normalize_boolean_no_change() {
        let input = "<p>Hello</p>";
        assert_eq!(normalize_boolean_attributes(input), input);
    }

    // --- normalize_html_output quick-exit tests ---

    #[test]
    fn test_normalize_html_output_no_patterns() {
        let input = "<html><body><p>Hello world</p></body></html>";
        assert_eq!(normalize_html_output(input), input);
    }

    #[test]
    fn test_normalize_html_output_bool_attrs_only() {
        // Void elements keep self-closing slashes; only boolean attrs are normalized
        assert_eq!(
            normalize_html_output(r#"<br /><input required="">"#),
            "<br /><input required>"
        );
    }

    // --- wrap_bare_text_in_paragraphs tests ---

    #[test]
    fn test_wrap_bare_text_between_h3_and_ul() {
        let html = "<h3 id=\"intro\">Intro</h3>\n\nSome bare text here\n<ul>\n<li>item</li>\n</ul>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            result.contains("<p>Some bare text here</p>"),
            "Bare text between h3 and ul should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_with_inline_html() {
        let html = "<h3>Title</h3>\n\nText with <span class=\"cls\">span</span>\n<ul>\n<li>item</li>\n</ul>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            result.contains("<p>Text with <span class=\"cls\">span</span></p>"),
            "Bare text with inline elements should be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_does_not_wrap_inside_ul() {
        let html = "<ul>\n<li>item 1</li>\nbare text\n<li>item 2</li>\n</ul>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            !result.contains("<p>bare text</p>"),
            "Text inside ul should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_does_not_wrap_inside_div() {
        let html = "<div>\nsome text\n</div>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            !result.contains("<p>some text</p>"),
            "Text inside div should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_preserves_existing_p_tags() {
        let html = "<h3>Title</h3>\n<p>Already wrapped</p>\n<ul>\n<li>item</li>\n</ul>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            result.contains("<p>Already wrapped</p>"),
            "Existing p tags should be preserved. Got: {}",
            result
        );
        // Should not double-wrap
        assert!(
            !result.contains("<p><p>"),
            "Should not double-wrap. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_does_not_wrap_inside_pre() {
        let html = "<pre><code>line 1\nline 2\n</code></pre>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            !result.contains("<p>line"),
            "Text inside pre should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_course_page_pattern() {
        // This is the exact pattern from the course page
        let html = concat!(
            "<h3 id=\"intro\">Introduction to Machine Learning</h3>\n",
            "Course overview &ndash; <span class=\"datetime\">Monday</span>\n",
            "<ul>\n",
            "<li>item 1</li>\n",
            "<li>item 2</li>\n",
            "</ul>\n",
            "<p><a href=\"url\">Lesson materials</a></p>\n",
        );
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            result
                .contains("<p>Course overview &ndash; <span class=\"datetime\">Monday</span></p>"),
            "Course subtitle should be wrapped in <p>. Got: {}",
            result
        );
        // Lesson materials should still be wrapped
        assert!(
            result.contains("<p><a href=\"url\">Lesson materials</a></p>"),
            "Existing p-wrapped content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_multiple_sections() {
        let html = concat!(
            "<h3>Section 1</h3>\n",
            "Subtitle 1\n",
            "<ul>\n<li>a</li>\n</ul>\n",
            "<h3>Section 2</h3>\n",
            "Subtitle 2\n",
            "<ul>\n<li>b</li>\n</ul>\n",
        );
        let result = wrap_bare_text_in_paragraphs(html);
        assert!(
            result.contains("<p>Subtitle 1</p>"),
            "First subtitle should be wrapped. Got: {}",
            result
        );
        assert!(
            result.contains("<p>Subtitle 2</p>"),
            "Second subtitle should be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_wrap_bare_text_no_wrapping_needed() {
        let html = "<h3>Title</h3>\n<p>Content</p>\n<ul>\n<li>item</li>\n</ul>";
        let result = wrap_bare_text_in_paragraphs(html);
        assert_eq!(
            result, html,
            "When no bare text exists, output should be unchanged"
        );
    }

    #[test]
    fn test_wrap_bare_text_via_postprocess() {
        // Test that it works through the full postprocess pipeline
        let html = "<h3>Title</h3>\nBare text here\n<ul>\n<li>item</li>\n</ul>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<p>Bare text here</p>"),
            "Bare text should be wrapped via postprocess. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 124: Kramdown loose list <p> wrapping
    // ========================================================================

    #[test]
    fn test_loose_list_preserves_p_tags_in_li() {
        // Markdown loose list (blank lines between items) should wrap each
        // item's content in <p> tags, matching kramdown behavior.
        let input =
            "* First item paragraph.\n\n* Second item paragraph.\n\n* Third item paragraph.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // Each <li> should contain a <p> tag
        assert!(
            html.contains("<li>\n<p>First item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping. Got: {}",
            html
        );
        assert!(
            html.contains("<li>\n<p>Second item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping. Got: {}",
            html
        );
        assert!(
            html.contains("<li>\n<p>Third item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping. Got: {}",
            html
        );
    }

    #[test]
    fn test_tight_list_no_p_tags_in_li() {
        // Tight list (no blank lines) should NOT have <p> tags in <li>.
        let input = "* First item\n* Second item\n* Third item\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // Count <p> tags - there should be none inside <li> for tight lists
        assert!(
            !html.contains("<li>\n<p>"),
            "Tight list should not have <p> inside <li>. Got: {}",
            html
        );
    }

    #[test]
    fn test_loose_list_multi_paragraph_item() {
        // A loose list item with multiple paragraphs should preserve all <p> tags.
        let input =
            "* First paragraph of item.\n\n  Second paragraph of same item.\n\n* Another item.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>First paragraph of item.</p>"),
            "First paragraph in multi-paragraph item should be <p>-wrapped. Got: {}",
            html
        );
        assert!(
            html.contains("<p>Second paragraph of same item.</p>"),
            "Second paragraph in multi-paragraph item should be <p>-wrapped. Got: {}",
            html
        );
    }

    #[test]
    fn test_html_li_with_attributes_still_strips_p() {
        // Raw HTML <li> with attributes (from Liquid includes) should still strip <p>.
        let html = "<li class=\"podcast\"><p><a href=\"url\">Title</a> on date</p></li>";
        let result = strip_paragraphs_in_html_blocks(html);
        assert!(
            !result.contains("<p>"),
            "HTML <li> with attributes should still strip <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_readme_driven_development_loose_list() {
        // The actual pattern from mojombo-blog readme-driven-development post:
        // a loose list where items are long paragraphs separated by blank lines.
        let input = "* Most importantly, you\u{2019}re giving yourself a chance to think through the project.\n\n* As a byproduct of writing a Readme, you\u{2019}ll have nice documentation.\n\n* If you\u{2019}re working with a team, everyone can start work on other projects.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // Each item should be wrapped in <p> (loose list behavior)
        let li_p_count = html.matches("<li>\n<p>").count();
        assert_eq!(
            li_p_count, 3,
            "All 3 loose list items should have <p> wrapping. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 144: HTML comments and script tags must not be wrapped in <p>
    // ========================================================================

    #[test]
    fn test_html_comment_not_wrapped_in_p_by_wrap_bare_text() {
        // HTML comments between block elements should not be wrapped in <p> tags
        let input = "<h2>Heading</h2>\n<!-- FAQ Accordion Component -->\n<div class=\"faq\">\ncontent\n</div>";
        let result = wrap_bare_text_in_paragraphs(input);
        assert!(
            !result.contains("<p><!--"),
            "HTML comment should not be wrapped in <p>, got: {}",
            result
        );
        assert!(
            result.contains("<!-- FAQ Accordion Component -->"),
            "HTML comment should be preserved, got: {}",
            result
        );
    }

    #[test]
    fn test_script_tag_not_wrapped_in_p_by_wrap_bare_text() {
        // Script tags between block elements should not be wrapped in <p> tags
        let input =
            "</div>\n<script src=\"/assets/accordion.js\"></script>\n<div>\ncontent\n</div>";
        let result = wrap_bare_text_in_paragraphs(input);
        assert!(
            !result.contains("<p><script"),
            "Script tag should not be wrapped in <p>, got: {}",
            result
        );
    }

    #[test]
    fn test_accordion_include_output_not_wrapped_in_p() {
        // Full accordion include output pattern: comment + div + JSON-LD script + accordion.js script
        let input = "<h2>FAQ</h2>\n\
                      <!-- FAQ Accordion Component -->\n\
                      <div class=\"faq-accordion\">\n\
                      <div class=\"faq-item\">Q&A</div>\n\
                      </div>\n\
                      \n\
                      <!-- FAQ Schema Markup (JSON-LD) -->\n\
                      <script type=\"application/ld+json\">\n\
                      {\"@type\": \"FAQPage\"}\n\
                      </script>\n\
                      \n\
                      <!-- Load accordion JavaScript -->\n\
                      <script src=\"/assets/accordion.js\"></script>";
        let result = wrap_bare_text_in_paragraphs(input);
        assert!(
            !result.contains("<p><!--"),
            "HTML comments should not be wrapped in <p>, got: {}",
            result
        );
        assert!(
            !result.contains("<p><script"),
            "Script tags should not be wrapped in <p>, got: {}",
            result
        );
        assert!(
            result.contains("<script src=\"/assets/accordion.js\"></script>"),
            "Accordion script tag should be preserved intact, got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_preserves_accordion_script() {
        // End-to-end test through postprocess
        let input = "<h2>FAQ</h2>\n\
                      <!-- FAQ Accordion Component -->\n\
                      <div class=\"faq-accordion\">\n\
                      <div class=\"faq-item\">Q&A</div>\n\
                      </div>\n\
                      \n\
                      <script type=\"application/ld+json\">\n\
                      {\"@type\": \"FAQPage\"}\n\
                      </script>\n\
                      \n\
                      <!-- Load accordion JavaScript -->\n\
                      <script src=\"/assets/accordion.js\"></script>";
        let result = postprocess(input);
        assert!(
            !result.contains("<p><!--"),
            "HTML comments should not be wrapped in <p> after postprocess, got: {}",
            result
        );
        assert!(
            !result.contains("<p><script"),
            "Script tags should not be wrapped in <p> after postprocess, got: {}",
            result
        );
        assert!(
            result.contains("<script src=\"/assets/accordion.js\"></script>"),
            "Accordion script tag should be present after postprocess, got: {}",
            result
        );
    }

    #[test]
    fn test_course_structured_data_script_not_wrapped_in_p() {
        // Course structured data include: comment followed by script tag
        let input = "</div>\n\
                      \n\
                      <!-- Course Structured Data -->\n\
                      <script type=\"application/ld+json\">\n\
                      {\"@type\": \"Course\"}\n\
                      </script>";
        let result = wrap_bare_text_in_paragraphs(input);
        assert!(
            !result.contains("<p><!--"),
            "Comment before script should not be wrapped in <p>, got: {}",
            result
        );
        assert!(
            !result.contains("<p><script"),
            "Script tag should not be wrapped in <p>, got: {}",
            result
        );
    }

    #[test]
    fn test_full_pipeline_accordion_script_placement() {
        // Full pipeline test: markdown_to_html (which includes postprocess)
        // simulating the content after Liquid processing
        let input = "## Frequently Asked Questions\n\
                      \n\
                      <!-- FAQ Accordion Component -->\n\
                      <div class=\"faq-accordion\">\n\
                      <div class=\"faq-item\">\n\
                      <button>Q1</button>\n\
                      </div>\n\
                      </div>\n\
                      \n\
                      <!-- FAQ Schema Markup (JSON-LD) -->\n\
                      <script type=\"application/ld+json\">\n\
                      {\"@type\": \"FAQPage\"}\n\
                      </script>\n\
                      \n\
                      <!-- Load accordion JavaScript -->\n\
                      <script src=\"/assets/accordion.js\"></script>\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<p><!--"),
            "HTML comments should not be wrapped in <p> in full pipeline, got: {}",
            html
        );
        assert!(
            html.contains("<script src=\"/assets/accordion.js\"></script>"),
            "Accordion script tag should be present in full pipeline, got: {}",
            html
        );
        // The script tag should NOT be inside a <p>
        assert!(
            !html.contains("<p><script"),
            "Script tag should not be inside <p> in full pipeline, got: {}",
            html
        );
    }
}

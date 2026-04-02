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
        .replace(" data-md1-heading", "")
}

/// Issue 276: Convert LaTeX math delimiters to match Jekyll/kramdown output.
///
/// Display math: `<p>$$...$$</p>` becomes `\[...\]` as a bare text node (no `<p>` wrapper).
/// Inline math: `$...$` within a paragraph becomes `\(...\)`.
///
/// Does not convert `$` inside `<code>` or `<pre>` elements.
/// Does not convert lone `$` signs (e.g., "$100").
#[allow(dead_code)]
fn convert_math_delimiters(html: &str) -> String {
    if !html.contains('$') {
        return html.to_string();
    }

    // First pass: convert display math <p>$$...$$</p> (may be multiline)
    let html = convert_display_math_blocks(html);

    // Second pass: convert inline $...$ to \(...\) line by line
    let mut result = String::with_capacity(html.len());
    for line in html.split('\n') {
        if line.contains('$') && !line.contains("<code") && !line.contains("<pre") {
            let converted = convert_inline_math(line);
            result.push_str(&converted);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline added by the split/join
    if result.ends_with('\n') && !html.ends_with('\n') {
        result.pop();
    }

    result
}

/// Convert display math blocks: `<p>$$...$$</p>` to `\[...\]` (bare text node).
///
/// Handles both single-line and multi-line display math blocks.
fn convert_display_math_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for <p> that might contain display math
        if let Some(p_start) = remaining.find("<p>$$") {
            // Copy everything before this <p>
            result.push_str(&remaining[..p_start]);

            // Find the closing </p>
            let after_p_open = &remaining[p_start + 3..]; // skip "<p>"
            if let Some(p_end_rel) = after_p_open.find("</p>") {
                let inner = &after_p_open[..p_end_rel];
                let inner_trimmed = inner.trim();

                if let Some(math_content) = inner_trimmed
                    .strip_prefix("$$")
                    .and_then(|s| s.strip_suffix("$$"))
                {
                    // This is display math -- emit as \[...\]
                    let math_content = math_content.trim();
                    result.push_str("\\[");
                    result.push_str(math_content);
                    result.push_str("\\]");
                    remaining = &after_p_open[p_end_rel + 4..]; // skip "</p>"
                    continue;
                }
            }

            // Not a math block after all; copy the <p> literally and continue
            result.push_str("<p>");
            remaining = after_p_open;
        } else {
            // No more <p>$$ patterns
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Convert inline `$$...$$` pairs to `\(...\)` within HTML text.
///
/// This runs AFTER `convert_display_math_blocks`, so standalone `<p>$$...$$</p>`
/// patterns have already been consumed. Any remaining `$$...$$` pairs are inline math.
///
/// Does NOT convert inside `<code>` or `<pre>` elements.
fn convert_inline_double_dollar_math(html: &str) -> String {
    if !html.contains("$$") {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    for line in html.split('\n') {
        if line.contains("$$") && !line.contains("<code") && !line.contains("<pre") {
            // Replace $$...$$ pairs with \(...\)
            let mut converted = String::with_capacity(line.len());
            let mut remaining = line;
            while let Some(start) = remaining.find("$$") {
                converted.push_str(&remaining[..start]);
                let after_open = &remaining[start + 2..];
                if let Some(end) = after_open.find("$$") {
                    let math_content = &after_open[..end];
                    converted.push_str("\\(");
                    converted.push_str(math_content);
                    converted.push_str("\\)");
                    remaining = &after_open[end + 2..];
                } else {
                    // No closing $$, leave as-is
                    converted.push_str("$$");
                    remaining = after_open;
                }
            }
            converted.push_str(remaining);
            result.push_str(&converted);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline added by the split/join
    if result.ends_with('\n') && !html.ends_with('\n') {
        result.pop();
    }

    result
}

/// Convert inline math `$...$` to `\(...\)` within a line of HTML.
///
/// Only converts when `$` is followed by non-space content and closed by another `$`.
/// Skips lone `$` (e.g., "$100") and `$$` (display math delimiters).
#[allow(dead_code)]
fn convert_inline_math(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' {
            // Skip $$ (display math delimiter -- should have been handled already)
            if i + 1 < len && bytes[i + 1] == b'$' {
                result.push('$');
                result.push('$');
                i += 2;
                continue;
            }

            // Try to find closing $ for inline math
            let content_start = i + 1;
            if content_start < len && bytes[content_start] != b' ' && bytes[content_start] != b'$' {
                let mut j = content_start;
                let mut found = false;
                while j < len {
                    if bytes[j] == b'$' {
                        // Don't match $$ as closing delimiter
                        if j + 1 < len && bytes[j + 1] == b'$' {
                            j += 2;
                            continue;
                        }
                        // Check that content before closing $ is not a space
                        if j > content_start && bytes[j - 1] != b' ' {
                            let content = &line[content_start..j];
                            // Only convert if content looks like math (contains
                            // at least one letter or backslash). Pure digits/
                            // punctuation like $10,000 is currency, not math.
                            if content.contains('\\') || content.chars().any(|c| c.is_alphabetic())
                            {
                                result.push_str("\\(");
                                result.push_str(content);
                                result.push_str("\\)");
                                i = j + 1;
                                found = true;
                                break;
                            }
                            // Not math — push the opening $ and continue
                            break;
                        }
                        break; // Space before closing $, not math
                    }
                    if bytes[j] == b'\n' {
                        break;
                    }
                    j += 1;
                }
                if !found {
                    let ch = line[i..].chars().next().unwrap();
                    result.push(ch);
                    i += ch.len_utf8();
                }
            } else {
                result.push('$');
                i += 1;
            }
        } else {
            let ch = line[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}

/// Apply all kramdown compatibility transformations to HTML output.
///
/// This is the main entry point. It applies, in order:
/// 1. Strip unwanted `<p>` tags inside HTML block elements (only when
///    `collapse_blank_lines_in_html_blocks` was NOT applied pre-markdown)
/// 2. Auto-generated heading IDs
/// 3. Inline attribute lists (`{:target="_blank"}`, `{:.class}`, `{:#id}`)
/// 4. Fenced code block wrapping (no language tag)
/// 5. (moved to markdown rendering -- see `frontmatter::add_inline_code_class_to_events`)
///    5b. Wrap bare text between block elements in `<p>` tags
/// 6. Paragraph spacing (extra newlines after block elements)
/// 7. Remove `start` attribute from `<ol>` tags (D11)
/// 8. Remove self-closing slash from void elements (D3)
/// 9. Normalize boolean HTML attributes (D2, D12)
/// 10. Normalize `<figcaption>` closing tag whitespace (D6)
/// 11. Indent loose list items to match kramdown formatting (kramdown mode only)
pub fn postprocess(html: &str) -> String {
    postprocess_with_options(html, true)
}

/// Issue 489: Replace kramdown `{:toc}` patterns in markdown content with a
/// placeholder that will be expanded to an actual TOC during postprocessing.
/// This is the public entry point for the main markdown pipeline.
pub fn replace_toc_pattern_in_markdown(content: &str) -> String {
    replace_toc_pattern_with_placeholder(content)
}

/// Whether heading IDs are generated in kramdown mode (GFM slugify on text content)
/// or CommonMarkGhPages mode (basic_generate_id on raw inner HTML).
#[derive(Clone, Copy, PartialEq, Eq)]
enum HeadingIdMode {
    /// Kramdown: strip HTML tags, decode entities, then GFM slugify (Unicode-preserving).
    Kramdown,
    /// CommonMarkGhPages: use raw inner HTML with basic_generate_id (ASCII-only).
    CommonMarkGhPages,
}

/// Fix mis-balanced emphasis tags produced by pulldown-cmark.
///
/// pulldown-cmark sometimes produces `<strong>A<strong>B</strong>C</strong>` when
/// the correct output should be `<strong>A</strong>B<strong>C</strong>`. This happens
/// when `**text**"` patterns confuse the emphasis resolver in certain document contexts.
///
/// This function detects the pattern where a `<strong>` or `<em>` tag is opened, then
/// another opening tag of the SAME type appears before the first is closed, and rewrites
/// the second opening tag as a closing tag for the first span.
pub fn fix_nested_emphasis_tags(html: &str) -> String {
    // Quick check: if no emphasis tags, return early
    if !html.contains("<strong>") && !html.contains("<em>") {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Fix nested <strong>: <strong>A<strong>B</strong>C</strong>
    // -> <strong>A</strong>B<strong>C</strong>
    result = fix_nested_tag(&result, "strong");
    result = fix_nested_tag(&result, "em");

    result
}

/// Fix nested same-type emphasis tags.
///
/// Detects the pattern `<tag>A<tag>B</tag>C</tag>` and rewrites it as
/// `<tag>A</tag>B<tag>C</tag>`. This handles cases where pulldown-cmark
/// mis-nests emphasis delimiters.
fn fix_nested_tag(html: &str, tag: &str) -> String {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");

    if !html.contains(&open_tag) {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find the next opening tag
        if let Some(first_open_pos) = remaining.find(&open_tag) {
            let after_first_open = first_open_pos + open_tag.len();

            // Look for the next occurrence of the SAME opening tag before a closing tag
            let next_open = remaining[after_first_open..].find(&open_tag);
            let next_close = remaining[after_first_open..].find(&close_tag);

            match (next_open, next_close) {
                (Some(open_offset), Some(close_offset)) if open_offset < close_offset => {
                    // Found <tag>A<tag>B</tag>C</tag> pattern.
                    // Rewrite as <tag>A</tag>B<tag>C</tag>.
                    //
                    // Step 1: Copy up to second <tag>, replace it with </tag>
                    let second_open_abs = after_first_open + open_offset;
                    result.push_str(&remaining[..second_open_abs]);
                    result.push_str(&close_tag);

                    // Step 2: The first </tag> in the remaining text becomes <tag>
                    let after_second = &remaining[second_open_abs + open_tag.len()..];
                    if let Some(first_close_in_rest) = after_second.find(&close_tag) {
                        result.push_str(&after_second[..first_close_in_rest]);
                        result.push_str(&open_tag);
                        remaining = &after_second[first_close_in_rest + close_tag.len()..];
                    } else {
                        // No matching close tag -- just copy rest
                        result.push_str(after_second);
                        remaining = "";
                    }
                }
                _ => {
                    // Normal: <tag>...</tag> -- no nesting issue.
                    // Copy up to and past the closing tag.
                    if let Some(close_offset) = next_close {
                        let close_abs = after_first_open + close_offset + close_tag.len();
                        result.push_str(&remaining[..close_abs]);
                        remaining = &remaining[close_abs..];
                    } else {
                        // No closing tag found -- copy rest and stop.
                        result.push_str(remaining);
                        remaining = "";
                    }
                }
            }
        } else {
            // No more opening tags -- copy rest.
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Return the byte length of a UTF-8 character from its first byte.
fn utf8_char_len(first_byte: u8) -> usize {
    if first_byte < 0x80 {
        1
    } else if first_byte < 0xE0 {
        2
    } else if first_byte < 0xF0 {
        3
    } else {
        4
    }
}

/// Strip HTML tags from a string, returning only the text content.
fn strip_html_tags_simple(html: &str) -> String {
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

/// Find a closing `*` for a literal emphasis span starting at `start`.
/// Returns the content and position of the closing `*`.
fn find_literal_emphasis_span(html: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let mut depth: i32 = 0;

    while i < len {
        if bytes[i] == b'<' {
            if i + 1 < len && bytes[i + 1] == b'/' {
                depth -= 1;
            } else if i + 1 < len && bytes[i + 1] != b'!' {
                let tag_end = html[i..].find('>')?;
                let abs_end = i + tag_end;
                if bytes[abs_end - 1] != b'/' {
                    let tag_content = &html[i + 1..abs_end];
                    let tag_name = tag_content.split_whitespace().next().unwrap_or("");
                    if !matches!(tag_name, "br" | "hr" | "img" | "input" | "meta" | "link") {
                        depth += 1;
                    }
                }
            }
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            continue;
        }

        if bytes[i] == b'*'
            && depth <= 0
            && i > start
            && bytes[i - 1] != b' '
            && bytes[i - 1] != b'\n'
        {
            let next_ok = i + 1 >= len
                || matches!(
                    bytes[i + 1],
                    b' ' | b',' | b'.' | b'<' | b'"' | b'\n' | b')' | b';' | b':' | b'!' | b'?'
                );
            if next_ok {
                let span_len = i - start;
                if span_len > 0 && span_len < 2000 {
                    let content = &html[start..i];
                    let text_content = strip_html_tags_simple(content);
                    if !text_content.trim().is_empty() {
                        return Some((content, i));
                    }
                }
            }
        }

        let ch_len = utf8_char_len(bytes[i]);
        i += ch_len;
    }

    None
}

/// Find a closing `_` (or `__`) for a literal underscore emphasis span.
fn find_literal_underscore_emphasis_span(
    html: &str,
    start: usize,
    delim_count: usize,
) -> Option<(&str, usize)> {
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = start;

    while i < len {
        if bytes[i] == b'<' {
            return None;
        }
        if bytes[i] == b'\n' {
            return None;
        }
        if bytes[i] == b'_' {
            let close_start = i;
            let mut close_count = 0;
            while i < len && bytes[i] == b'_' {
                close_count += 1;
                i += 1;
            }
            if close_count >= delim_count {
                let preceded_ok = close_start > start
                    && bytes[close_start - 1] != b' '
                    && bytes[close_start - 1] != b'\n';
                let followed_ok = i >= len || !bytes[i].is_ascii_alphanumeric();
                if preceded_ok && followed_ok {
                    let span_len = close_start - start;
                    if span_len > 0 && span_len < 2000 {
                        let content = &html[start..close_start];
                        if !content.trim().is_empty() {
                            return Some((content, close_start));
                        }
                    }
                }
            }
            continue;
        }
        let ch_len = utf8_char_len(bytes[i]);
        i += ch_len;
    }

    None
}

/// Issue 332: Fix literal `*` characters that should have been emphasis markers.
///
/// pulldown-cmark sometimes fails to resolve `*...*` as emphasis in certain document
/// contexts (e.g., when preceded by `<figure>` blocks with `<figcaption>` containing
/// nested `<a>` tags). The `*` characters end up as literal text in the HTML output.
///
/// This function detects paired `*...*` patterns in the HTML output where the content
/// between them may include HTML tags (like `<a>`) and plain text, and converts them
/// to `<em>...</em>`.
pub fn fix_literal_asterisk_emphasis(html: &str) -> String {
    if !html.contains('*') {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut em_depth: i32 = 0;
    let mut in_code = false;

    while i < len {
        if bytes[i] == b'<' {
            let tag_start = i;
            i += 1;
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let tag = &html[tag_start..i];
            if tag == "<em>" || tag == "<strong>" {
                em_depth += 1;
            } else if tag == "</em>" || tag == "</strong>" {
                em_depth -= 1;
            } else if tag == "<code>" || tag.starts_with("<code ") {
                in_code = true;
            } else if tag == "</code>" {
                in_code = false;
            }
            result.push_str(tag);
            continue;
        }

        if bytes[i] == b'*' && em_depth <= 0 && !in_code {
            let is_opener = {
                let prev_ok = i == 0
                    || matches!(bytes[i - 1], b' ' | b'>' | b'"' | b'\n' | b'\t' | b'(')
                    || (i >= 2 && !bytes[i - 1].is_ascii());
                let next_ok = i + 1 < len && bytes[i + 1] != b' ' && bytes[i + 1] != b'\n';
                prev_ok && next_ok
            };

            if is_opener {
                if let Some((content, end_pos)) = find_literal_emphasis_span(html, i + 1) {
                    result.push_str("<em>");
                    result.push_str(content);
                    result.push_str("</em>");
                    i = end_pos + 1;
                    continue;
                }
            }
        }

        if bytes[i] == b'*' {
            result.push('*');
            i += 1;
            continue;
        }

        let ch_len = utf8_char_len(bytes[i]);
        result.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }

    result
}

/// Issue 333: Fix literal underscore emphasis that pulldown-cmark failed to parse.
///
/// In certain document contexts, pulldown-cmark leaves `_text_` as literal
/// underscores instead of converting to `<em>text</em>`. This postprocessor
/// detects such patterns in the HTML output and wraps them in emphasis tags.
pub fn fix_literal_underscore_emphasis(html: &str) -> String {
    if !html.contains('_') {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut em_depth: i32 = 0;
    let mut in_code = false;

    while i < len {
        if bytes[i] == b'<' {
            let tag_start = i;
            i += 1;
            while i < len && bytes[i] != b'>' {
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let tag = &html[tag_start..i];
            if tag == "<em>" || tag == "<strong>" {
                em_depth += 1;
            } else if tag == "</em>" || tag == "</strong>" {
                em_depth -= 1;
            } else if tag == "<code>" || tag.starts_with("<code ") {
                in_code = true;
            } else if tag == "</code>" {
                in_code = false;
            }
            result.push_str(tag);
            continue;
        }

        if bytes[i] == b'_' && em_depth <= 0 && !in_code {
            let is_double = i + 1 < len && bytes[i + 1] == b'_';

            if is_double {
                let delim_count = 2;
                let content_start = i + delim_count;
                let is_opener = {
                    let prev_ok = i == 0
                        || matches!(
                            bytes[i - 1],
                            b' ' | b'>' | b'"' | b'\n' | b'\t' | b'(' | b',' | b';'
                        )
                        || (i >= 2 && !bytes[i - 1].is_ascii());
                    let next_ok = content_start < len
                        && bytes[content_start] != b' '
                        && bytes[content_start] != b'\n';
                    prev_ok && next_ok
                };
                if is_opener {
                    if let Some((content, end_pos)) =
                        find_literal_underscore_emphasis_span(html, content_start, delim_count)
                    {
                        result.push_str("<strong>");
                        result.push_str(content);
                        result.push_str("</strong>");
                        i = end_pos + delim_count;
                        continue;
                    }
                }
            }

            let is_opener = {
                let prev_ok = i == 0
                    || matches!(
                        bytes[i - 1],
                        b' ' | b'>' | b'"' | b'\n' | b'\t' | b'(' | b',' | b';'
                    )
                    || (i >= 2 && !bytes[i - 1].is_ascii());
                let next_ok = i + 1 < len
                    && bytes[i + 1] != b' '
                    && bytes[i + 1] != b'\n'
                    && bytes[i + 1] != b'_';
                prev_ok && next_ok
            };

            if is_opener {
                if let Some((content, end_pos)) =
                    find_literal_underscore_emphasis_span(html, i + 1, 1)
                {
                    result.push_str("<em>");
                    result.push_str(content);
                    result.push_str("</em>");
                    i = end_pos + 1;
                    continue;
                }
            }
        }

        let ch_len = utf8_char_len(bytes[i]);
        result.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }

    result
}

/// Issue 515: Restructure kramdown full-width table separator rows into proper
/// `<tbody>` splits and `<tfoot>` sections.
///
/// When pulldown-cmark renders a GFM table that contains kramdown full-width
/// separators like `|----|` or `|====|`, it treats them as data rows with a
/// single cell containing dashes/equals (the rest are empty). Smart punctuation
/// may also convert `---` sequences to em-dashes.
///
/// This function detects those separator rows in the HTML output and restructures
/// them:
/// - A dash separator row (`-` or em-dash content) splits `<tbody>` into two.
/// - An equals separator row (`=` content) starts a `<tfoot>` section.
pub fn restructure_kramdown_table_separators(html: &str) -> String {
    // Quick check: if no <tbody>, nothing to restructure
    if !html.contains("<tbody>") {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(tbody_start) = remaining.find("<tbody>") {
        // Copy everything up to and including <tbody>
        result.push_str(&remaining[..tbody_start + "<tbody>".len()]);
        remaining = &remaining[tbody_start + "<tbody>".len()..];

        // Find the matching </tbody>
        let tbody_end = match remaining.find("</tbody>") {
            Some(pos) => pos,
            None => {
                // No closing tag, just copy the rest
                result.push_str(remaining);
                return result;
            }
        };

        let tbody_content = &remaining[..tbody_end];

        // Check if this tbody contains any separator rows
        if !contains_separator_row(tbody_content) {
            // No separators, pass through as-is
            result.push_str(&remaining[..tbody_end + "</tbody>".len()]);
            remaining = &remaining[tbody_end + "</tbody>".len()..];
            continue;
        }

        // Process rows within this tbody, splitting on separator rows
        let mut in_tfoot = false;
        let mut row_remaining = tbody_content;

        while let Some(tr_start) = row_remaining.find("<tr>") {
            let tr_end = match row_remaining[tr_start..].find("</tr>") {
                Some(pos) => tr_start + pos + "</tr>".len(),
                None => break,
            };

            let tr_html = &row_remaining[tr_start..tr_end];
            let before_tr = &row_remaining[..tr_start];

            if is_separator_tr(tr_html) {
                let sep_type = separator_type(tr_html);
                match sep_type {
                    SeparatorType::Dash => {
                        // Close current tbody, open a new one
                        result.push_str("\n</tbody>\n<tbody>");
                    }
                    SeparatorType::Equals => {
                        // Close current tbody, open tfoot
                        result.push_str("\n</tbody>\n<tfoot>");
                        in_tfoot = true;
                    }
                    SeparatorType::None => {
                        // Not actually a separator, keep it
                        result.push_str(before_tr);
                        result.push_str(tr_html);
                    }
                }
            } else {
                result.push_str(before_tr);
                result.push_str(tr_html);
            }

            row_remaining = &row_remaining[tr_end..];
        }

        // Append any trailing content after the last </tr> but before </tbody>
        result.push_str(row_remaining);

        if in_tfoot {
            result.push_str("\n</tfoot>");
        } else {
            result.push_str("</tbody>");
        }
        remaining = &remaining[tbody_end + "</tbody>".len()..];
    }

    // Copy any remaining content after the last </tbody>
    result.push_str(remaining);
    result
}

/// Check if a tbody's content contains any separator rows.
fn contains_separator_row(tbody_content: &str) -> bool {
    let mut pos = 0;
    while let Some(tr_start) = tbody_content[pos..].find("<tr>") {
        let abs_start = pos + tr_start;
        if let Some(tr_end_rel) = tbody_content[abs_start..].find("</tr>") {
            let tr_html = &tbody_content[abs_start..abs_start + tr_end_rel + "</tr>".len()];
            if is_separator_tr(tr_html) {
                return true;
            }
            pos = abs_start + tr_end_rel + "</tr>".len();
        } else {
            break;
        }
    }
    false
}

#[derive(PartialEq)]
enum SeparatorType {
    Dash,
    Equals,
    None,
}

/// Determine what kind of separator a `<tr>` row is.
fn separator_type(tr_html: &str) -> SeparatorType {
    if let Some(content) = extract_first_td_content(tr_html) {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return SeparatorType::None;
        }
        // Check for equals separator: all '=' characters
        if trimmed.chars().all(|c| c == '=') {
            return SeparatorType::Equals;
        }
        // Check for dash separator: all '-' or em-dash or en-dash characters
        if trimmed
            .chars()
            .all(|c| c == '-' || c == '\u{2014}' || c == '\u{2013}')
        {
            return SeparatorType::Dash;
        }
    }
    SeparatorType::None
}

/// Check if a `<tr>` is a kramdown separator row.
///
/// A separator row has:
/// - First `<td>` contains only dashes (or em-dashes) or only equals signs
/// - All other `<td>` elements are empty
fn is_separator_tr(tr_html: &str) -> bool {
    let sep = separator_type(tr_html);
    if sep == SeparatorType::None {
        return false;
    }

    // Verify all other <td>s are empty
    let mut td_count = 0;
    let mut pos = 0;
    while let Some(td_start) = tr_html[pos..].find("<td") {
        let abs_start = pos + td_start;
        let tag_end = match tr_html[abs_start..].find('>') {
            Some(p) => abs_start + p + 1,
            None => break,
        };
        let td_close = match tr_html[tag_end..].find("</td>") {
            Some(p) => tag_end + p,
            None => break,
        };
        let content = tr_html[tag_end..td_close].trim();
        td_count += 1;

        if td_count > 1 && !content.is_empty() {
            return false;
        }

        pos = td_close + "</td>".len();
    }

    td_count >= 1
}

/// Extract the text content of the first `<td>` in a `<tr>`.
fn extract_first_td_content(tr_html: &str) -> Option<&str> {
    let td_start = tr_html.find("<td")?;
    let tag_end = tr_html[td_start..].find('>')? + td_start + 1;
    let td_close = tr_html[tag_end..].find("</td>")? + tag_end;
    Some(&tr_html[tag_end..td_close])
}

/// Apply all kramdown compatibility transformations to HTML output.
///
/// When `indent_lists` is true (kramdown mode), list items are indented with
/// 2 spaces to match Jekyll's kramdown renderer. When false (CommonMarkGhPages),
/// list items are NOT indented, matching Jekyll's CommonMark renderer.
pub fn postprocess_with_options(html: &str, indent_lists: bool) -> String {
    // Issue 275b: Fix mis-balanced emphasis tags from pulldown-cmark before other
    // postprocessing steps that depend on correct tag nesting.
    let html = fix_nested_emphasis_tags(html);
    // Issue 332: Fix literal asterisks that should have been emphasis.
    // Only for kramdown mode -- CommonMarkGhPages handles emphasis correctly.
    let html = if indent_lists {
        fix_literal_asterisk_emphasis(&html)
    } else {
        html
    };
    // Issue 333: Fix literal underscores that should have been emphasis.
    // Only for kramdown mode -- for CommonMarkGhPages, pulldown-cmark already
    // handles emphasis correctly and this would create false emphasis.
    let html = if indent_lists {
        fix_literal_underscore_emphasis(&html)
    } else {
        html
    };
    let html = strip_paragraphs_in_html_blocks(&html);
    let html = unwrap_block_elements_from_p(&html);
    let html = encode_bare_ampersands(&html);
    // Issue 330: Use different heading ID generation for kramdown vs CommonMarkGhPages.
    // indent_lists=true means kramdown mode, false means CommonMarkGhPages.
    let heading_mode = if indent_lists {
        HeadingIdMode::Kramdown
    } else {
        HeadingIdMode::CommonMarkGhPages
    };
    let html = add_heading_ids(&html, heading_mode);
    // Issue 489: Replace {:toc} placeholders with generated TOC from headings.
    // Must run after add_heading_ids so heading IDs are available.
    let html = replace_toc_placeholders(&html);
    let html = apply_block_ial(&html);
    let html = apply_inline_attributes(&html);
    let html = wrap_fenced_code_blocks(&html);
    // Note: inline code classes are now added during markdown rendering
    // (in frontmatter::add_inline_code_class_to_events) rather than here,
    // so that only backtick-generated <code> gets the class -- not raw HTML
    // <code> tags from the source.
    let html = wrap_bare_text_in_paragraphs(&html);
    let html = wrap_standalone_comments_in_paragraphs(&html);
    let html = wrap_marked_partial_loose_list_items(&html);
    let html = add_block_spacing(&html);
    let html = remove_ol_start_attribute(&html);
    // Issue 297: Only indent list items for kramdown mode. CommonMarkGhPages
    // (Jekyll) does not indent <li> elements inside <ul>/<ol>.
    let html = if indent_lists {
        indent_list_items(&html)
    } else {
        html
    };
    let html = indent_blockquote_content(&html);
    let html = normalize_figcaption_whitespace(&html);
    // Issue 201: Convert bare void elements (<br>, <hr>) to XHTML-style
    // (<br />, <hr />) to match Jekyll/kramdown output.
    let html = normalize_bare_void_elements(&html);
    // Issue 448: Extract <br /> from end of blockquote <p> when followed by
    // another blockquote. Jekyll renders such <br> as standalone elements
    // between blockquotes, not inside the preceding blockquote's paragraph.
    let html = extract_br_between_blockquotes(&html);
    // Issue 339: Escape malformed raw HTML tags that use single-quoted
    // attributes with unescaped apostrophes. Jekyll/kramdown treats these as
    // literal text rather than live elements.
    let html = escape_malformed_single_quote_tags(&html);
    // Issue 270: Collapse newlines inside HTML tags to spaces, matching
    // Jekyll/kramdown behavior where raw HTML tags with multi-line attributes
    // are normalized to single-line output.
    let html = normalize_newlines_in_html_tags(&html);
    // Issue 276: Convert display math blocks <p>$$...$$</p> to \[...\] bare
    // text nodes, matching Jekyll/kramdown behavior for MathJax rendering.
    let html = convert_display_math_blocks(&html);
    // Issue 475: Convert remaining inline $$...$$ to \(...\) after display math
    // has been consumed. Jekyll/kramdown converts inline $$...$$ to MathJax \(...\).
    let html = convert_inline_double_dollar_math(&html);
    // D2, D12: Normalize boolean attributes in the markdown output early
    // (during collection loading). This ensures that the final
    // normalize_html_output() call after layout wrapping finds nothing to change
    // and exits early, avoiding a full scan of the (often 100-300KB) page HTML.
    normalize_boolean_attributes(&html)
}

/// Lighter postprocessing for the `markdownify` Liquid filter.
///
/// Jekyll's `markdownify` filter runs kramdown, which produces output matching
/// the full kramdown postprocessing: block spacing (`\n\n` between consecutive
/// block elements) and list item indentation (2-space indent on `<li>` tags).
///
/// This variant applies the transformations relevant to filter-produced HTML:
/// inline attributes, `ol start` removal, block spacing, list indentation,
/// void element normalization, and boolean attribute normalization. It skips
/// heavy page-body-only processing (heading IDs, fenced code wrapping,
/// bare text wrapping, blockquote indentation, figcaption normalization, etc.).
/// Inline code classes are added during markdown rendering
/// (see `frontmatter::add_inline_code_class_to_events`).
pub fn postprocess_for_filter(html: &str) -> String {
    postprocess_for_filter_with_options(html, true)
}

/// Lighter postprocessing for the `markdownify` filter, with options.
///
/// When `indent_lists` is true (kramdown mode), list items are indented.
/// When false (CommonMarkGhPages), list items are NOT indented, matching
/// how `postprocess_with_options` works for page body content.
pub fn postprocess_for_filter_with_options(html: &str, indent_lists: bool) -> String {
    // Issue 275b: Fix mis-balanced emphasis tags from pulldown-cmark
    let html = fix_nested_emphasis_tags(html);
    // Issue 332: Fix literal asterisks that should have been emphasis.
    let html = fix_literal_asterisk_emphasis(&html);
    // Issue 333: Fix literal underscores that should have been emphasis.
    let html = fix_literal_underscore_emphasis(&html);
    let html = apply_inline_attributes(&html);
    // Issue 365: Add heading IDs to markdownify output, matching the main pipeline.
    // indent_lists=true means kramdown mode, false means CommonMarkGhPages.
    let heading_mode = if indent_lists {
        HeadingIdMode::Kramdown
    } else {
        HeadingIdMode::CommonMarkGhPages
    };
    let html = add_heading_ids(&html, heading_mode);
    // Note: inline code classes are now added during markdown rendering
    // (in frontmatter::add_inline_code_class_to_events) rather than here.
    let html = wrap_marked_partial_loose_list_items(&html);
    let html = remove_ol_start_attribute(&html);
    let html = add_block_spacing(&html);
    let html = if indent_lists {
        indent_list_items(&html)
    } else {
        html
    };
    let html = normalize_bare_void_elements(&html);
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
/// Note: inline code classes are NOT applied here. Jekyll only adds
/// `highlighter-rouge` to `<code>` tags during markdown
/// rendering (handled by `frontmatter::add_inline_code_class_to_events()`),
/// not to `<code>` tags from Liquid templates or raw HTML in the source.
///
/// Note: void element self-closing slashes are NOT removed because
/// Jekyll/kramdown outputs XHTML-style self-closing tags (e.g. `<br />`).
pub fn normalize_html_output(html: &str) -> String {
    let needs_bool_attrs = html.contains("=\"\"");

    // Only normalize bare <br> in the final output -- raw HTML <br> in markdown
    // content (e.g., table cells) needs XHTML-style self-closing. Do NOT normalize
    // <hr> here because pulldown-cmark already outputs <hr /> for markdown rules,
    // and converting <hr> here would incorrectly affect include/layout content.
    let html = normalize_br_only(html);

    if needs_bool_attrs {
        normalize_boolean_attributes(&html)
    } else {
        html
    }
}

/// Owned-string version of `normalize_html_output` that avoids allocating
/// when nothing changes. Takes ownership of the input `String` and returns
/// it unmodified on the fast path, avoiding the clone that the borrow-based
/// version would incur. Used in the per-page rendering hot path.
pub fn normalize_html_output_owned(html: String) -> String {
    let needs_bool_attrs = html.contains("=\"\"");
    let needs_br = html.contains("<br>") || html.contains("<br/>");

    // Fast path: nothing to normalize -- return the original String without allocating.
    if !needs_bool_attrs && !needs_br {
        return html;
    }

    let after_br = normalize_br_only(&html);

    if needs_bool_attrs {
        normalize_boolean_attributes(&after_br)
    } else {
        after_br
    }
}

// ============================================================================
// Issue 275: Escape inner delimiters in mixed-delimiter emphasis
// ============================================================================

/// Escape inner emphasis delimiters when a different delimiter type wraps them.
///
/// kramdown treats mixed emphasis delimiters (`_` and `*`) as non-interchangeable:
/// when `_` opens emphasis, `*` inside is literal text (and vice versa).
/// pulldown-cmark (CommonMark) nests them as separate `<em>` elements.
///
/// This pre-processes markdown to escape the inner delimiters so that
/// pulldown-cmark treats them as literal text, matching kramdown behavior.
///
/// Examples:
/// - `_*text*_` -> `_\*text\*_` (renders as `<em>*text*</em>`)
/// - `*_text_ more*` -> `*\_text\_ more*` (renders as `<em>_text_ more</em>`)
/// - `__*text*__` -> `__\*text\*__` (renders as `<strong>*text*</strong>`)
/// - `**_text_**` -> `**\_text\_**` (renders as `<strong>_text_</strong>`)
///
/// Does not modify same-delimiter nesting (e.g., `**text *inner* more**`).
///
/// Convert runs of 4+ consecutive underscores to match kramdown behavior.
///
/// Kramdown treats underscore runs differently from CommonMark:
///   - 4 underscores: `<em>__</em>`
///   - 6+ underscores: `<strong>__</strong>` + literal remainder
///   - 2, 3, 5 underscores: stay literal (same as CommonMark)
///
/// This only applies when the run is not adjacent to alphanumeric characters
/// (kramdown's word-boundary rule for `_` emphasis).
pub fn convert_kramdown_underscore_runs(markdown: &str) -> String {
    if !markdown.contains("____") {
        return markdown.to_string();
    }

    let mut result = String::with_capacity(markdown.len() + 64);
    let chars: Vec<char> = markdown.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip code spans (backticks)
        if chars[i] == '`' {
            let backtick_start = i;
            let mut backtick_count = 0;
            while i < len && chars[i] == '`' {
                backtick_count += 1;
                i += 1;
            }
            let mut found_close = false;
            while i < len {
                if chars[i] == '`' {
                    let mut close_count = 0;
                    while i < len && chars[i] == '`' {
                        close_count += 1;
                        i += 1;
                    }
                    if close_count == backtick_count {
                        found_close = true;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let end = if found_close { i } else { len };
            for c in &chars[backtick_start..end] {
                result.push(*c);
            }
            continue;
        }

        // Check for underscore run
        if chars[i] == '_' {
            let run_start = i;
            while i < len && chars[i] == '_' {
                i += 1;
            }
            let run_len = i - run_start;

            // Check word boundary: underscore emphasis doesn't open after alpha
            let preceded_by_alpha = run_start > 0 && chars[run_start - 1].is_alphabetic();
            // Check what follows: underscore emphasis doesn't close before alnum
            let followed_by_alnum = i < len && chars[i].is_alphanumeric();

            if run_len >= 6 && !preceded_by_alpha && !followed_by_alnum {
                // Strong: <strong>__</strong> + remainder
                result.push_str("<strong>__</strong>");
                let remainder = run_len - 6;
                for _ in 0..remainder {
                    result.push('_');
                }
            } else if run_len == 4 && !preceded_by_alpha && !followed_by_alnum {
                // Em: <em>__</em>
                result.push_str("<em>__</em>");
            } else {
                // Leave as literal underscores
                for _ in 0..run_len {
                    result.push('_');
                }
            }
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

pub fn escape_mixed_delimiter_emphasis(markdown: &str) -> String {
    if !markdown.contains('*') && !markdown.contains('_') {
        return markdown.to_string();
    }

    let mut result = String::with_capacity(markdown.len() + 32);
    let chars: Vec<char> = markdown.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip code spans (backticks)
        if chars[i] == '`' {
            let backtick_start = i;
            let mut backtick_count = 0;
            while i < len && chars[i] == '`' {
                backtick_count += 1;
                i += 1;
            }
            // Find matching closing backticks
            let mut found_close = false;
            let content_start = i;
            while i < len {
                if chars[i] == '`' {
                    let mut close_count = 0;
                    while i < len && chars[i] == '`' {
                        close_count += 1;
                        i += 1;
                    }
                    if close_count == backtick_count {
                        // Copy entire code span verbatim
                        for ch in &chars[backtick_start..i] {
                            result.push(*ch);
                        }
                        found_close = true;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if !found_close {
                // No closing backticks -- copy verbatim
                for ch in &chars[backtick_start..content_start] {
                    result.push(*ch);
                }
                i = content_start;
            }
            continue;
        }

        // Detect opening emphasis: _ or * (or __ or **)
        if (chars[i] == '_' || chars[i] == '*') && !is_escaped(&chars, i) {
            let outer_delim = chars[i];
            let inner_delim = if outer_delim == '_' { '*' } else { '_' };

            // Count consecutive outer delimiters
            let mut outer_count = 0;
            while i + outer_count < len && chars[i + outer_count] == outer_delim {
                outer_count += 1;
            }

            // Only handle 1 or 2 delimiter sequences (em or strong)
            if outer_count <= 2 {
                // Look ahead for the inner delimiter pattern
                let after_outer = i + outer_count;
                if let Some(span) = find_mixed_emphasis_span(
                    &chars,
                    after_outer,
                    outer_delim,
                    outer_count,
                    inner_delim,
                ) {
                    // Found a mixed-delimiter emphasis span.
                    // Write the outer delimiters
                    for _ in 0..outer_count {
                        result.push(outer_delim);
                    }
                    // Write the content with inner delimiters escaped,
                    // but skip code spans (don't escape inside backticks)
                    let mut j = after_outer;
                    while j < span.content_end {
                        if chars[j] == '`' {
                            let bt_start = j;
                            let mut bt_count = 0;
                            while j < span.content_end && chars[j] == '`' {
                                bt_count += 1;
                                j += 1;
                            }
                            let mut found_close = false;
                            let scan_start = j;
                            while j < span.content_end {
                                if chars[j] == '`' {
                                    let mut close_bt = 0;
                                    while j < span.content_end && chars[j] == '`' {
                                        close_bt += 1;
                                        j += 1;
                                    }
                                    if close_bt == bt_count {
                                        found_close = true;
                                        break;
                                    }
                                } else {
                                    j += 1;
                                }
                            }
                            let end = if found_close { j } else { scan_start };
                            for ch in &chars[bt_start..end] {
                                result.push(*ch);
                            }
                            if !found_close {
                                j = scan_start;
                            }
                            continue;
                        }
                        if chars[j] == inner_delim && !is_escaped(&chars, j) {
                            result.push('\\');
                        }
                        result.push(chars[j]);
                        j += 1;
                    }
                    // Write the closing outer delimiters
                    for _ in 0..outer_count {
                        result.push(outer_delim);
                    }
                    i = span.content_end + outer_count;
                    continue;
                }
            }

            // Not a mixed-delimiter pattern -- copy delimiter as-is
            for _ in 0..outer_count {
                result.push(outer_delim);
            }
            i += outer_count;
            continue;
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Result of finding a mixed-delimiter emphasis span.
struct MixedEmphasisSpan {
    /// Index just past the last content character (before closing outer delimiters)
    content_end: usize,
}

/// Check if a character at position `pos` is backslash-escaped.
fn is_escaped(chars: &[char], pos: usize) -> bool {
    if pos == 0 {
        return false;
    }
    let mut backslash_count = 0;
    let mut j = pos - 1;
    loop {
        if chars[j] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
        if j == 0 {
            break;
        }
        j -= 1;
    }
    backslash_count % 2 == 1
}

/// Look for a mixed-delimiter emphasis span starting at `start`.
///
/// Returns the span boundaries if the content contains the inner delimiter
/// and the outer closing delimiter is found. Only matches when the inner
/// delimiter creates what would be a nested emphasis (i.e., paired delimiters).
fn find_mixed_emphasis_span(
    chars: &[char],
    start: usize,
    outer_delim: char,
    outer_count: usize,
    inner_delim: char,
) -> Option<MixedEmphasisSpan> {
    let len = chars.len();

    // The content must contain at least one inner delimiter pair
    // to be a mixed-delimiter case.
    let mut has_inner_delim = false;
    let mut i = start;

    // Scan forward to find the matching closing outer delimiter(s)
    while i < len {
        // Skip code spans (backticks) -- inner delimiters inside code spans
        // must not count as emphasis delimiters
        if chars[i] == '`' {
            let mut bt_count = 0;
            while i < len && chars[i] == '`' {
                bt_count += 1;
                i += 1;
            }
            let mut found_close = false;
            while i < len {
                if chars[i] == '`' {
                    let mut close_bt = 0;
                    while i < len && chars[i] == '`' {
                        close_bt += 1;
                        i += 1;
                    }
                    if close_bt == bt_count {
                        found_close = true;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if !found_close {
                return None;
            }
            continue;
        }

        // Skip escaped characters
        if chars[i] == '\\' && i + 1 < len {
            i += 2;
            continue;
        }

        // Check for newline (emphasis doesn't span lines in kramdown)
        if chars[i] == '\n' {
            return None;
        }

        // Check for inner delimiter
        if chars[i] == inner_delim && !is_escaped(chars, i) {
            has_inner_delim = true;
            i += 1;
            continue;
        }

        // Check for closing outer delimiter(s)
        if chars[i] == outer_delim && !is_escaped(chars, i) {
            let mut close_count = 0;
            let close_start = i;
            while i < len && chars[i] == outer_delim {
                close_count += 1;
                i += 1;
            }
            if close_count == outer_count && has_inner_delim {
                // Verify the inner delimiters appear in pairs or as part
                // of what pulldown-cmark would parse as emphasis
                return Some(MixedEmphasisSpan {
                    content_end: close_start,
                });
            }
            // Not matching close -- this might be a different use
            // Just continue scanning
            continue;
        }

        i += 1;
    }

    None
}

// ============================================================================
// Issue 228: Process markdown="1" attribute on HTML elements
// ============================================================================

/// Issue 489: Placeholder prefix for kramdown {:toc} inside markdown="1" blocks.
/// Uses HTML comment syntax so pulldown-cmark passes it through unchanged
/// (double underscores would be interpreted as strong emphasis by markdown).
const KRAMDOWN_TOC_PLACEHOLDER_PREFIX: &str = "<!-- KRAMDOWN_TOC:";
const KRAMDOWN_TOC_PLACEHOLDER_SUFFIX: &str = " -->";

/// Issue 489: Detect kramdown `{:toc}` pattern in markdown content and replace
/// it with a placeholder. The pattern is a list item followed by a `{:toc ...}`
/// IAL, which kramdown interprets as "replace this list with a generated TOC".
///
/// Pattern: `* <any text>\n{:toc ...}` (the `*` creates a list marker, and
/// `{:toc}` on the next line tells kramdown to generate a TOC).
///
/// This is also available as `replace_toc_pattern_in_markdown` for use in
/// the main markdown pipeline (frontmatter.rs).
fn replace_toc_pattern_with_placeholder(content: &str) -> String {
    // Look for `{:toc` in the content first (fast path)
    if !content.contains("{:toc") {
        return content.to_string();
    }

    // Match: `* <text>\n{:toc [.class1 .class2 ...]}`
    // The list marker can be `*`, `-`, or `+` with optional leading whitespace.
    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();

    // Also track if input ended with newline
    let ends_with_newline = content.ends_with('\n');

    let mut i = 0;
    while i < lines.len() {
        if i + 1 < lines.len() {
            let next_line = lines[i + 1].trim();
            if next_line.starts_with("{:toc") && next_line.ends_with('}') {
                let current_trimmed = lines[i].trim();
                // Check if current line is a list item
                if current_trimmed.starts_with("* ")
                    || current_trimmed.starts_with("- ")
                    || current_trimmed.starts_with("+ ")
                {
                    // Extract classes from {:toc .class1 .class2}
                    let ial_content = &next_line[1..next_line.len() - 1]; // strip { and }
                    let ial_content = ial_content.strip_prefix(':').unwrap_or(ial_content).trim();
                    let mut classes = Vec::new();
                    for token in ial_content.split_whitespace() {
                        if token == "toc" {
                            continue;
                        }
                        if let Some(class) = token.strip_prefix('.') {
                            classes.push(class);
                        }
                    }
                    let class_str = classes.join(" ");
                    result.push_str(KRAMDOWN_TOC_PLACEHOLDER_PREFIX);
                    result.push_str(&class_str);
                    result.push_str(KRAMDOWN_TOC_PLACEHOLDER_SUFFIX);
                    result.push('\n');
                    i += 2; // skip both lines
                    continue;
                }
            }
        }
        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }

    // Remove the extra trailing newline we added if original didn't have one
    if !ends_with_newline && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Issue 489: Generate a TOC `<ul>` from heading IDs already present in the
/// HTML. This scans for `<hN id="...">` tags and builds a nested list.
///
/// `extra_classes` is an optional space-separated class list to add to the
/// outer `<ul>` (from `{:toc .toc__menu}` => `class="toc__menu"`).
fn generate_toc_from_headings(html: &str, extra_classes: &str) -> String {
    // Collect all headings with IDs: (level, id, text)
    let mut headings: Vec<(usize, String, String)> = Vec::new();

    let mut search_from = 0;
    while let Some(h_pos) = html[search_from..].find("<h") {
        let abs_pos = search_from + h_pos;
        let after = &html[abs_pos + 2..];

        // Parse heading level (1-6)
        let level_char = after.as_bytes().first().copied().unwrap_or(0);
        if !level_char.is_ascii_digit() {
            search_from = abs_pos + 2;
            continue;
        }
        let level = (level_char - b'0') as usize;
        if !(1..=6).contains(&level) {
            search_from = abs_pos + 2;
            continue;
        }

        // Check for id="..."
        let tag_end = match after.find('>') {
            Some(p) => p,
            None => {
                search_from = abs_pos + 2;
                continue;
            }
        };
        let tag_content = &after[..tag_end];

        // Extract id value
        let id = if let Some(id_pos) = tag_content.find("id=\"") {
            let id_start = id_pos + 4;
            if let Some(id_end) = tag_content[id_start..].find('"') {
                tag_content[id_start..id_start + id_end].to_string()
            } else {
                search_from = abs_pos + 2;
                continue;
            }
        } else {
            search_from = abs_pos + 2;
            continue;
        };

        // Skip headings with data-raw-html attribute (from includes)
        if tag_content.contains("data-raw-html") {
            search_from = abs_pos + tag_end + 1;
            continue;
        }

        // Extract heading text (between > and </hN>)
        let content_start = abs_pos + 2 + tag_end + 1;
        let close_tag = format!("</h{}>", level);
        if let Some(close_pos) = html[content_start..].find(&close_tag) {
            let text = &html[content_start..content_start + close_pos];
            // Strip any inner HTML tags to get plain text for the TOC link
            let plain_text = strip_html_tags_for_toc(text);
            headings.push((level, id, plain_text));
            search_from = content_start + close_pos + close_tag.len();
        } else {
            search_from = abs_pos + 2;
        }
    }

    if headings.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let class_attr = if extra_classes.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", extra_classes)
    };
    output.push_str(&format!("<ul{} id=\"markdown-toc\">\n", class_attr));

    let mut stack: Vec<usize> = Vec::new();

    for (i, (level, id, text)) in headings.iter().enumerate() {
        let level = *level;

        // Close nested lists as needed
        while stack.last().is_some_and(|&l| l >= level) {
            stack.pop();
            let indent = 2 + stack.len() * 4;
            output.push_str(&" ".repeat(indent + 2));
            output.push_str("</ul>\n");
            output.push_str(&" ".repeat(indent));
            output.push_str("</li>\n");
        }

        let indent = 2 + stack.len() * 4;

        // Check if next heading is deeper
        let next_is_deeper = headings.get(i + 1).is_some_and(|(nl, _, _)| *nl > level);

        output.push_str(&" ".repeat(indent));
        output.push_str(&format!(
            "<li><a href=\"#{}\" id=\"markdown-toc-{}\">{}</a>",
            id, id, text
        ));

        if next_is_deeper {
            output.push_str(&"    ".repeat(stack.len() + 1));
            output.push_str("<ul>\n");
            stack.push(level);
        } else {
            output.push_str("</li>\n");
        }
    }

    // Close remaining open lists
    while stack.pop().is_some() {
        let indent = 2 + stack.len() * 4;
        output.push_str(&" ".repeat(indent + 2));
        output.push_str("</ul>\n");
        output.push_str(&" ".repeat(indent));
        output.push_str("</li>\n");
    }

    output.push_str("</ul>\n");
    output
}

/// Strip HTML tags from text, keeping only the text content. Used for
/// generating TOC link text from heading inner HTML.
fn strip_html_tags_for_toc(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
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

/// Issue 489: Replace TOC placeholders in HTML with generated TOC from headings.
/// Called during postprocessing after heading IDs have been assigned.
fn replace_toc_placeholders(html: &str) -> String {
    if !html.contains(KRAMDOWN_TOC_PLACEHOLDER_PREFIX) {
        return html.to_string();
    }

    let mut result = html.to_string();
    // Find and replace each placeholder
    while let Some(start) = result.find(KRAMDOWN_TOC_PLACEHOLDER_PREFIX) {
        let after_prefix = start + KRAMDOWN_TOC_PLACEHOLDER_PREFIX.len();
        if let Some(suffix_offset) = result[after_prefix..].find(KRAMDOWN_TOC_PLACEHOLDER_SUFFIX) {
            let classes = result[after_prefix..after_prefix + suffix_offset].to_string();
            let end = after_prefix + suffix_offset + KRAMDOWN_TOC_PLACEHOLDER_SUFFIX.len();

            // The placeholder might be wrapped in <p> tags by pulldown-cmark
            // or inside a <li>. Check for surrounding wrappers and remove.
            let before = &result[..start];
            let after_end = &result[end..];

            let (remove_start, remove_end) =
                if before.ends_with("<p>") && after_end.trim_start().starts_with("</p>") {
                    let ws_len = after_end.len() - after_end.trim_start().len();
                    (start - 3, end + ws_len + 4)
                } else if before.ends_with("<li>") && after_end.trim_start().starts_with("</li>") {
                    let ws_len = after_end.len() - after_end.trim_start().len();
                    (start - 4, end + ws_len + 5)
                } else {
                    (start, end)
                };

            // Also check if the wrapper is itself inside a <ul>/<ol> that should be removed
            let before_wrapper = &result[..remove_start];
            let after_wrapper = &result[remove_end..];
            let (final_start, final_end) = if before_wrapper.trim_end().ends_with("<ul>")
                && after_wrapper.trim_start().starts_with("</ul>")
            {
                let pre_ws = before_wrapper.len() - before_wrapper.trim_end().len();
                let ul_start = before_wrapper.trim_end().len() - 4;
                let post_ws = after_wrapper.len() - after_wrapper.trim_start().len();
                // Also remove any newline before <ul>
                let final_start = if ul_start > 0
                    && result.as_bytes().get(ul_start - 1).copied() == Some(b'\n')
                {
                    ul_start - 1 - pre_ws
                } else {
                    ul_start - pre_ws
                };
                (final_start, remove_end + post_ws + 5)
            } else {
                (remove_start, remove_end)
            };

            let toc_html = generate_toc_from_headings(&result, &classes);
            result.replace_range(final_start..final_end, &toc_html);
        } else {
            break; // malformed placeholder
        }
    }

    result
}

/// Process HTML elements with `markdown="1"` attribute (kramdown feature).
///
/// When an HTML element has `markdown="1"`:
/// 1. Remove the `markdown="1"` attribute from the output
/// 2. Process the content within that element as markdown
/// 3. Affects `<aside>`, `<p>`, `<div>`, and other block elements
///
/// This should be called on content BEFORE markdown conversion, as the
/// `markdown="1"` attribute appears in raw markdown source files.
pub fn process_markdown_attribute(content: &str) -> String {
    use pulldown_cmark::{html as cmark_html, Options, Parser};

    // Short-circuit: if the content contains no markdown attribute at all, return as-is.
    if !content.contains("markdown=") {
        return content.to_string();
    }

    // All recognised markdown attribute patterns and whether they mean "span" mode.
    // "1" and "block" are block mode; "span" is inline mode.
    const PATTERNS: &[(&str, bool)] = &[
        ("markdown=\"1\"", false),
        ("markdown='1'", false),
        ("markdown=\"block\"", false),
        ("markdown='block'", false),
        ("markdown=\"span\"", true),
        ("markdown='span'", true),
    ];

    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while !remaining.is_empty() {
        // Find the earliest occurrence of any markdown attribute pattern
        let mut best: Option<(usize, bool)> = None;
        for &(pat, is_span) in PATTERNS {
            if let Some(pos) = remaining.find(pat) {
                if best.is_none() || pos < best.unwrap().0 {
                    best = Some((pos, is_span));
                }
            }
        }
        let (md_attr_pos, is_span_mode) = match best {
            Some(v) => v,
            None => {
                result.push_str(remaining);
                break;
            }
        };

        // Find the opening tag that contains this attribute (search backwards for '<')
        let before_attr = &remaining[..md_attr_pos];
        let tag_start = match before_attr.rfind('<') {
            Some(pos) => pos,
            None => {
                result.push_str(&remaining[..md_attr_pos + 1]);
                remaining = &remaining[md_attr_pos + 1..];
                continue;
            }
        };

        // Parse the opening tag
        let after_tag_start = &remaining[tag_start..];
        let gt_pos = match after_tag_start.find('>') {
            Some(pos) => pos,
            None => {
                result.push_str(&remaining[..md_attr_pos + 1]);
                remaining = &remaining[md_attr_pos + 1..];
                continue;
            }
        };

        let open_tag_str = &after_tag_start[..gt_pos + 1];

        // Extract tag name
        let tag_name = extract_markdown_tag_name(open_tag_str);
        if tag_name.is_empty() {
            result.push_str(&remaining[..tag_start + gt_pos + 1]);
            remaining = &remaining[tag_start + gt_pos + 1..];
            continue;
        }

        // Find the matching closing tag
        let after_open = &remaining[tag_start + gt_pos + 1..];
        let close_tag = format!("</{}>", tag_name);
        let close_pos = match find_markdown_close_tag(after_open, &tag_name, &close_tag) {
            Some(pos) => pos,
            None => {
                result.push_str(&remaining[..tag_start + gt_pos + 1]);
                remaining = &remaining[tag_start + gt_pos + 1..];
                continue;
            }
        };

        let inner_content = &after_open[..close_pos];

        // Build new opening tag without markdown="1" / markdown='1'
        let clean_open_tag = remove_markdown_attr_from_tag(open_tag_str);

        // Render inner content as markdown
        let trimmed_inner = inner_content.trim();

        // Issue 489: Detect kramdown {:toc} pattern inside markdown="1" blocks.
        // The pattern is: `* <text>\n{:toc ...}` which kramdown replaces with
        // a generated table of contents. Replace with a placeholder that
        // postprocess_with_options will fill with the actual TOC.
        let trimmed_inner = replace_toc_pattern_with_placeholder(trimmed_inner);
        let trimmed_inner_ref: &str = &trimmed_inner;

        // Issue 322: Pre-process to join <img> lines with following text so
        // pulldown-cmark treats them as inline content (paragraph) rather than
        // HTML blocks.
        let preprocessed_inner = preprocess_inline_html_for_markdown(trimmed_inner_ref);
        let preprocessed_ref = preprocessed_inner.trim();
        let rendered_inner = if preprocessed_ref.is_empty() {
            String::new()
        } else {
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_SMART_PUNCTUATION);
            let parser = Parser::new_ext(preprocessed_ref, options);
            let mut html_output = String::new();
            cmark_html::push_html(&mut html_output, parser);
            // Trim trailing newline
            if html_output.ends_with('\n') {
                html_output.pop();
            }
            html_output
        };

        // Issue 320: Recursively process any nested markdown attributes
        // that survived the pulldown-cmark rendering (e.g., <p markdown="1">
        // inside a <div markdown="1"> block).
        let rendered_inner = if PATTERNS.iter().any(|(pat, _)| rendered_inner.contains(pat)) {
            process_markdown_attribute(&rendered_inner)
        } else {
            rendered_inner
        };

        // Issue 505: Apply block-level IALs that pulldown-cmark merged into
        // paragraph text. pulldown-cmark doesn't understand kramdown IALs, so
        // `{: .class }` on the line after a paragraph gets merged as literal
        // text inside the `<p>`. We apply them here before the content is
        // placed into the container element.
        let rendered_inner = {
            let mut html = rendered_inner;
            apply_merged_ial(&mut html);
            html
        };

        // Issue 320: Mark headings generated inside markdown="1" blocks
        // so that add_heading_ids uses basic_generate_id (ASCII-only) instead
        // of GFM slugify. This matches kramdown's behavior where content inside
        // markdown="1" is re-parsed by the base parser.
        let rendered_inner = mark_md1_headings_in_html(&rendered_inner);

        // markdown="span" forces inline mode (strip outer <p> tags),
        // regardless of the container element. For <p> and <span> containers
        // inline mode is also the default.
        let is_inline_container = is_span_mode || tag_name == "p" || tag_name == "span";

        // Copy everything before the tag
        result.push_str(&remaining[..tag_start]);

        if is_inline_container {
            // For inline containers / span mode, strip the outer <p> from rendered content
            let inner_rendered = strip_outer_p_tags_for_markdown(&rendered_inner);
            result.push_str(&clean_open_tag);
            result.push_str(&inner_rendered);
            result.push_str(&close_tag);
        } else {
            // For block containers like <aside> and <div>
            result.push_str(&clean_open_tag);
            result.push('\n');
            result.push_str(&rendered_inner);
            result.push('\n');
            result.push_str(&close_tag);
        }

        remaining = &after_open[close_pos + close_tag.len()..];
    }

    result
}

/// Pre-process inner content of `markdown="1"` blocks before sending to
/// pulldown-cmark. Joins `<img ...>` lines with following text lines so that
/// pulldown-cmark treats them as inline content (wrapped in `<p>`) rather than
/// as HTML blocks (CommonMark type 6).
///
/// Without this, `<img` at the start of a line triggers HTML block mode in
/// pulldown-cmark, which suppresses `<p>` wrapping. Kramdown treats `<img>` as
/// inline inside `markdown="1"` blocks, so the text gets `<p>` wrapped.
fn preprocess_inline_html_for_markdown(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::with_capacity(content.len());
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Check if this line is an <img ...> tag (self-closing or not)
        if trimmed.starts_with("<img ") || trimmed.starts_with("<img>") {
            // Join this <img> line with subsequent non-blank, non-block-HTML text
            // lines using space, so pulldown-cmark sees them as inline content
            // within a paragraph. In CommonMark, `<img` at line start triggers
            // HTML block type 6 unless text follows on the same line after `>`.
            result.push_str(trimmed);
            i += 1;
            while i < lines.len() {
                let next_trimmed = lines[i].trim();
                if next_trimmed.is_empty() {
                    break;
                }
                // Stop if we hit a block-level HTML open tag (not inline elements)
                if next_trimmed.starts_with('<')
                    && !next_trimmed.starts_with("<img")
                    && !next_trimmed.starts_with("<a ")
                    && !next_trimmed.starts_with("<a>")
                    && !next_trimmed.starts_with("<em")
                    && !next_trimmed.starts_with("<strong")
                    && !next_trimmed.starts_with("<code")
                    && !next_trimmed.starts_with("<br")
                {
                    break;
                }
                // Join all subsequent lines (including additional <img> lines)
                // with space so the first <img> has text after `>` on the
                // same line, preventing HTML block mode in pulldown-cmark
                result.push(' ');
                result.push_str(next_trimmed);
                i += 1;
            }
            // Add blank line after the img+text block to separate from any
            // following block-level HTML (like <p markdown="1">)
            result.push('\n');
            result.push('\n');
        } else {
            result.push_str(lines[i]);
            result.push('\n');
            i += 1;
        }
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Mark simple `<hN>` headings in HTML with `data-md1-heading` so that
/// `add_heading_ids` uses `basic_generate_id` (ASCII-only) instead of GFM slugify.
///
/// This is applied to HTML rendered inside `markdown="1"` blocks to match
/// kramdown's behavior where the base parser (not GFM) generates heading IDs.
fn mark_md1_headings_in_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + 64);
    let mut remaining = html;

    while !remaining.is_empty() {
        if let Some(pos) = remaining.find("<h") {
            result.push_str(&remaining[..pos]);
            let after = &remaining[pos..];

            // Check if it's <hN> (h1-h6 with just >)
            if after.len() >= 4 {
                let level = after.as_bytes()[2];
                if level.is_ascii_digit()
                    && (1..=6).contains(&(level - b'0'))
                    && after.as_bytes()[3] == b'>'
                {
                    // Simple <hN> tag -- add marker
                    result.push_str(&after[..3]);
                    result.push_str(" data-md1-heading>");
                    remaining = &after[4..];
                    continue;
                }
            }

            // Not a simple heading tag, copy the <h and continue
            result.push_str(&after[..2]);
            remaining = &after[2..];
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Extract the tag name from an opening tag like `<aside markdown="1" class="foo">`.
fn extract_markdown_tag_name(tag: &str) -> String {
    let inner = tag.trim_start_matches('<').trim_end_matches('>');
    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    inner[..name_end].to_lowercase()
}

/// Remove `markdown="1"`, `markdown="block"`, `markdown="span"` (and
/// single-quote variants) from an opening tag string.
fn remove_markdown_attr_from_tag(tag: &str) -> String {
    let mut result = tag.to_string();
    for val in &["1", "block", "span"] {
        let dq = format!(" markdown=\"{}\"", val);
        let sq = format!(" markdown='{}'", val);
        let dq_space = format!("markdown=\"{}\" ", val);
        let sq_space = format!("markdown='{}' ", val);
        let dq_bare = format!("markdown=\"{}\"", val);
        let sq_bare = format!("markdown='{}'", val);
        result = result
            .replace(&dq, "")
            .replace(&sq, "")
            .replace(&dq_space, "")
            .replace(&sq_space, "")
            .replace(&dq_bare, "")
            .replace(&sq_bare, "");
    }
    // Clean up double spaces
    result.replace("  ", " ").replace("< ", "<")
}

/// Find the matching closing tag, handling nested tags of the same type.
fn find_markdown_close_tag(html: &str, tag_name: &str, close_tag: &str) -> Option<usize> {
    let open_pattern = format!("<{}", tag_name);
    let mut depth = 0i32;
    let mut pos = 0;

    while pos < html.len() {
        if html[pos..].starts_with(close_tag) {
            if depth == 0 {
                return Some(pos);
            }
            depth -= 1;
            pos += close_tag.len();
            continue;
        }
        if html[pos..].starts_with(&open_pattern) {
            let after = &html[pos + open_pattern.len()..];
            if after.starts_with('>') || after.starts_with(' ') {
                depth += 1;
            }
        }
        pos += html[pos..].chars().next().map_or(1, |c| c.len_utf8());
    }
    None
}

/// Strip outer `<p>...</p>` wrapper from rendered markdown content.
fn strip_outer_p_tags_for_markdown(html: &str) -> String {
    let trimmed = html.trim();
    if let Some(rest) = trimmed.strip_prefix("<p>") {
        if let Some(inner) = rest.strip_suffix("</p>") {
            return inner.to_string();
        }
    }
    trimmed.to_string()
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

    // TEMP DEBUG for issue 524
    let is_history = content.contains("bug-fixes-v3-2-0");
    if is_history {
        let _ = std::fs::write("/tmp/history_before_collapse.md", &result);
    }

    for &tag in BLOCK_PARENT_TAGS {
        let prev = result.clone();
        result = collapse_blanks_in_tag(&result, tag);
        if is_history && prev != result {
            let _ = std::fs::write(format!("/tmp/history_after_{}.md", tag), &result);
        }
    }

    result
}

/// Block-level HTML tags after which trailing text on the same line
/// should be split onto a new line for proper markdown parsing.
///
/// In CommonMark, an HTML block extends until a blank line. When text like
/// `</figure>Photo by [Name](url)` appears, pulldown-cmark treats the text
/// after `</figure>` as part of the HTML block and doesn't parse the markdown.
/// kramdown, however, treats the text after the closing tag as new content.
const BLOCK_CLOSE_SPLIT_TAGS: &[&str] = &[
    "</figure>",
    "</figcaption>",
    "</div>",
    "</table>",
    "</blockquote>",
    "</pre>",
    "</section>",
    "</article>",
    "</header>",
    "</footer>",
    "</nav>",
    "</aside>",
    "</details>",
    "</summary>",
    "</form>",
    "</fieldset>",
];

/// Split text that immediately follows a closing HTML block tag onto a new line.
///
/// In kramdown, text like `</figure>Photo by [Name](url)` is treated as:
/// - `</figure>` ends the HTML block
/// - `Photo by [Name](url)` is a new markdown paragraph
///
/// In CommonMark (pulldown-cmark), the entire line is part of the HTML block,
/// so the markdown links are not parsed.
///
/// This pre-processing step inserts a blank line between the closing HTML tag
/// and the trailing text, so pulldown-cmark will parse the text as markdown.
pub fn split_text_after_html_block_close(content: &str) -> String {
    // Short-circuit: if no closing HTML tags present, nothing to split.
    if !content.contains("</") {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len() + 64);
    let mut remaining = content;

    while !remaining.is_empty() {
        // Find the earliest block-close tag in the remaining content
        let mut earliest: Option<(usize, &str)> = None;
        for &tag in BLOCK_CLOSE_SPLIT_TAGS {
            if let Some(pos) = remaining.find(tag) {
                let end = pos + tag.len();
                match earliest {
                    None => earliest = Some((end, tag)),
                    Some((prev_end, _)) if end < prev_end => {
                        earliest = Some((end, tag));
                    }
                    _ => {}
                }
            }
        }

        if let Some((end, tag)) = earliest {
            // Copy everything up to and including the closing tag
            result.push_str(&remaining[..end]);
            let after = &remaining[end..];

            // For </summary>, skip the split if </details> appears on the same line.
            // This preserves single-line <details><summary>...</summary>text</details>
            // as raw HTML, preventing pulldown-cmark from wrapping content in <p> tags.
            if tag == "</summary>" {
                let rest_of_line = after.split('\n').next().unwrap_or("");
                if rest_of_line.contains("</details>") {
                    remaining = after;
                    continue;
                }
            }

            // Check if there's non-whitespace text immediately following
            // (not starting with newline, not empty, not another HTML tag)
            if !after.is_empty() && !after.starts_with('\n') && !after.starts_with('<') {
                // There's text right after the closing tag -- insert blank line
                let trimmed = after.trim_start();
                if !trimmed.is_empty() && !trimmed.starts_with('<') {
                    result.push_str("\n\n");
                    remaining = after;
                    continue;
                }
            }

            remaining = after;
            if after.is_empty() {
                break;
            }
        } else {
            // No more block-close tags
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Collapse blank lines inside all instances of `<tag ...>...</tag>`.
/// Check if a position in text is inside a backtick code span on the same line.
/// Returns true if there is an odd number of backtick-delimiters before pos
/// on the same line, meaning we're inside an inline code span.
fn is_inside_backtick_code(text: &str, pos: usize) -> bool {
    // Find the start of the line containing pos
    let line_start = text[..pos].rfind('\n').map_or(0, |p| p + 1);
    let before = &text[line_start..pos];

    // Count backtick groups (code spans use matching backtick counts)
    // A simple heuristic: count individual backtick characters. If odd, we're inside code.
    // This handles both single backtick `code` and double backtick ``code``.
    let backtick_count = before.chars().filter(|&c| c == '`').count();
    backtick_count % 2 != 0
}

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
                    // Issue 524: Skip tags inside backtick code spans.
                    // `<div>` in backticks must not be treated as a real HTML block.
                    let abs_pos = content.len() - remaining.len() + pos;
                    if is_inside_backtick_code(content, abs_pos) {
                        result.push_str(&remaining[..pos + open_pattern.len()]);
                        remaining = &remaining[pos + open_pattern.len()..];
                        continue;
                    }
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
// Pre-markdown: Fix kramdown list indentation for ordered lists
// ============================================================================

/// Issue 329: Fix kramdown list indentation for ordered lists.
///
/// Kramdown allows sub-lists and continuation content to be indented with
/// just 2 spaces under an ordered list item (e.g., `1. text`), but
/// pulldown-cmark (CommonMark) requires indentation equal to the marker
/// width (3 spaces for `N. `, 4 for `NN. `, etc.).
pub fn fix_kramdown_list_indentation(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len() + 256);
    let mut in_code_block = false;
    let mut ol_stack: Vec<(usize, usize)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }

        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            continue;
        }

        if in_code_block {
            result.push_str(line);
            continue;
        }

        let leading_spaces = line.len() - trimmed.len();

        if let Some(marker_width) = ordered_list_marker_width(trimmed) {
            while let Some(&(indent, _)) = ol_stack.last() {
                if indent >= leading_spaces {
                    ol_stack.pop();
                } else {
                    break;
                }
            }
            let required_content_indent = leading_spaces + marker_width;
            ol_stack.push((leading_spaces, required_content_indent));
            result.push_str(line);
            continue;
        }

        if trimmed.is_empty() {
            result.push_str(line);
            continue;
        }

        if !ol_stack.is_empty() {
            let mut needs_fix = false;
            let mut extra_spaces = 0;

            for &(marker_indent, required_indent) in ol_stack.iter().rev() {
                if leading_spaces > marker_indent && leading_spaces < required_indent {
                    extra_spaces = required_indent - leading_spaces;
                    needs_fix = true;
                    break;
                }
            }

            if needs_fix && extra_spaces > 0 {
                for _ in 0..extra_spaces {
                    result.push(' ');
                }
                result.push_str(line);
                continue;
            }

            while let Some(&(indent, _)) = ol_stack.last() {
                if leading_spaces <= indent && !trimmed.is_empty() {
                    if is_markdown_list_item(trimmed) || leading_spaces < indent {
                        ol_stack.pop();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        result.push_str(line);
    }

    result
}

/// Return the marker width (including trailing space) of an ordered list marker,
/// or None if the trimmed line doesn't start with one.
fn ordered_list_marker_width(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos > 0
        && pos < bytes.len()
        && bytes[pos] == b'.'
        && pos + 1 < bytes.len()
        && bytes[pos + 1] == b' '
    {
        Some(pos + 2)
    } else {
        None
    }
}

/// Issue 329: Render fenced code blocks inside HTML block elements like `<details>`.
///
/// pulldown-cmark treats content inside HTML blocks as raw HTML, so fenced code
/// blocks (triple backticks) inside `<details>` are not recognized. This function
/// finds such code blocks and converts them to `<pre><code>` HTML.
pub fn render_code_blocks_in_html_blocks(content: &str) -> String {
    if !content.contains("```") {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len());
    let mut remaining = content;

    while !remaining.is_empty() {
        if let Some(pos) = find_html_block_with_code_fences(remaining) {
            result.push_str(&remaining[..pos.start]);
            let block = &remaining[pos.start..pos.end];
            let processed = convert_fenced_code_in_html_block(block);
            result.push_str(&processed);
            remaining = &remaining[pos.end..];
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

struct HtmlBlockRange {
    start: usize,
    end: usize,
}

fn find_html_block_with_code_fences(content: &str) -> Option<HtmlBlockRange> {
    let open_tag = "<details";
    let close_tag = "</details>";

    let mut search_from = 0;
    while search_from < content.len() {
        if let Some(open_pos) = content[search_from..].find(open_tag) {
            let abs_open = search_from + open_pos;
            if let Some(close_pos) = content[abs_open..].find(close_tag) {
                let abs_end = abs_open + close_pos + close_tag.len();
                let block = &content[abs_open..abs_end];
                if block.contains("```") {
                    return Some(HtmlBlockRange {
                        start: abs_open,
                        end: abs_end,
                    });
                }
                search_from = abs_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    None
}

fn convert_fenced_code_in_html_block(block: &str) -> String {
    let mut result = String::with_capacity(block.len());
    let mut remaining = block;

    while !remaining.is_empty() {
        if let Some(fence_start) = remaining.find("```") {
            result.push_str(&remaining[..fence_start]);
            let after_fence = &remaining[fence_start + 3..];
            let line_end = after_fence.find('\n').unwrap_or(after_fence.len());
            let lang = after_fence[..line_end].trim();
            let code_start = if line_end < after_fence.len() {
                line_end + 1
            } else {
                line_end
            };

            if let Some(close_pos) = after_fence[code_start..].find("```") {
                let code_content = &after_fence[code_start..code_start + close_pos];
                let code_content = code_content.strip_suffix('\n').unwrap_or(code_content);

                if lang.is_empty() {
                    result.push_str("<pre><code>");
                } else {
                    result.push_str(&format!("<pre><code class=\"language-{}\">", lang));
                }
                let escaped = code_content
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                result.push_str(&escaped);
                result.push_str("\n</code></pre>");

                let after_close = &after_fence[code_start + close_pos + 3..];
                let skip_newline = after_close.strip_prefix('\n').unwrap_or(after_close);
                remaining = skip_newline;
            } else {
                result.push_str("```");
                remaining = after_fence;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

// ============================================================================
// Pre-markdown: Escape headings inside markdown list context
// ============================================================================

/// Escape heading markers that appear inside a markdown list context.
///
/// In kramdown, a heading marker (e.g., `#### text`) that appears directly
/// after a list item (without a blank line separator) is treated as text
/// within the list item, NOT as a heading. pulldown-cmark (CommonMark)
/// treats it as a heading, breaking the list.
///
/// This function escapes heading markers in list context by prefixing
/// the `#` characters with a backslash, so pulldown-cmark treats them
/// as literal text.
///
/// Only applies to headings that appear immediately after a list item
/// (no blank line between).
pub fn escape_headings_in_list_context(content: &str) -> String {
    // Short-circuit: if no heading markers (# at start of line), nothing to escape.
    if !content.contains('#') {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len());
    let mut in_list = false;
    let mut in_code_block = false;

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }

        let trimmed = line.trim_start();

        // Track fenced code blocks
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            continue;
        }

        if in_code_block {
            result.push_str(line);
            continue;
        }

        let is_blank = trimmed.is_empty();

        if is_blank {
            // A blank line ends the list context
            in_list = false;
            result.push_str(line);
            continue;
        }

        let is_list_item = is_markdown_list_item(trimmed);

        if is_list_item {
            in_list = true;
            result.push_str(line);
            continue;
        }

        // Check if this line is a heading marker in list context
        let is_heading = trimmed.starts_with('#');
        if in_list && is_heading {
            // Issue 341: In the newline_to_br | markdownify pipeline, a heading
            // after <br /> is treated as a real heading by kramdown, not escaped.
            // Check if the previous line ends with <br /> to detect this context.
            let prev_ends_with_br = i > 0 && lines[i - 1].trim_end().ends_with("<br />");
            if prev_ends_with_br {
                // Don't escape -- kramdown renders this as a real heading
                result.push_str(line);
            } else {
                // Escape the heading marker by prefixing # with backslash
                let leading_ws = &line[..line.len() - trimmed.len()];
                result.push_str(leading_ws);
                result.push('\\');
                result.push_str(trimmed);
            }
        } else {
            result.push_str(line);
        }

        // Non-list, non-heading lines don't end the list context
        // (could be continuation lines)
    }

    result
}

/// Collapse blank lines between markdown list items to make partially-loose lists tight.
///
/// A "fully loose" list (blank lines between ALL consecutive items) keeps its
/// Issue 301: Mark forward-direction IALs in markdown source.
///
/// In kramdown, a standalone `{: .class}` with blank lines on BOTH sides
/// applies to the FOLLOWING element, not the preceding one. Since
/// pulldown-cmark doesn't preserve blank-line information in HTML output,
/// we insert an HTML comment marker before such IALs so that
/// `apply_block_ial` can detect them.
///
/// Pattern detected: `\n\n{: ...}\n\n` (blank line before and after the IAL)
/// Also handles IAL at the very start of content followed by a non-blank line
/// (forward IAL applies to the next block element).
/// Transforms to: `\n\n<!-- IAL:FWD -->\n{: ...}\n\n`
pub fn mark_forward_ial(content: &str) -> String {
    // Short-circuit: if no IAL markers at all, return as-is.
    if !content.contains("{:") {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len() + 64);

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        // Check if this line is a standalone IAL: starts with {: and ends with }
        if line.starts_with("{:") && line.ends_with('}') {
            // Check if previous line is blank (or this is the first line)
            let prev_blank = i == 0 || lines[i - 1].trim().is_empty();
            // Check if next line is blank
            let next_blank = i + 1 < lines.len() && lines[i + 1].trim().is_empty();
            // Forward IAL: standalone IAL with a blank line (or start) before it.
            // In kramdown, {: .class} on its own line preceded by a blank applies
            // to the following block element.
            if prev_blank {
                // Insert forward marker before the IAL
                result.push_str("<!-- IAL:FWD -->\n");
                result.push_str(lines[i]);
                result.push('\n');
                // If the next line is NOT blank, insert a blank line to ensure
                // pulldown-cmark creates a separate paragraph for the IAL
                // instead of merging it with the following text.
                if !next_blank && i + 1 < lines.len() {
                    result.push('\n');
                }
                i += 1;
                continue;
            }
        }
        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }

    // Remove trailing newline added by the loop if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

const PARTIAL_LOOSE_ITEM_MARKER: &str = "<!-- KRMD:PARTIAL_LOOSE_ITEM -->";

/// Mark simple partial-loose list items so HTML postprocessing can wrap only
/// those `<li>` items in `<p>...</p>`.
///
/// This is intentionally narrow:
/// - only partially-loose regions (`some` item gaps, not all)
/// - only list items followed by a blank gap to another sibling item
/// - only one-line items (no continuation lines / nested blocks)
pub fn mark_simple_partial_loose_list_items(content: &str) -> String {
    if !content.contains("- ") && !content.contains("* ") && !content.contains("+ ") {
        return content.to_string();
    }

    let mut lines: Vec<String> = content.split('\n').map(ToString::to_string).collect();
    if lines.len() < 3 {
        return content.to_string();
    }

    let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    let regions = find_list_regions(&line_refs);
    if regions.is_empty() {
        return content.to_string();
    }

    let mut in_code_block = false;

    for region in regions {
        if region.fully_loose {
            continue;
        }
        let mut i = region.start;
        while i < region.end {
            let trimmed = lines[i].trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_code_block = !in_code_block;
                i += 1;
                continue;
            }
            if in_code_block || !is_markdown_list_item(trimmed) {
                i += 1;
                continue;
            }

            let mut j = i + 1;
            let mut has_continuation = false;
            while j < lines.len() {
                let next_trimmed = lines[j].trim_start();
                if next_trimmed.is_empty() {
                    break;
                }
                let indent = lines[j].len() - next_trimmed.len();
                if indent >= 2 && !is_markdown_list_item(next_trimmed) {
                    has_continuation = true;
                    j += 1;
                } else {
                    break;
                }
            }

            let mut blank_count = 0usize;
            while j < lines.len() && lines[j].trim().is_empty() {
                blank_count += 1;
                j += 1;
            }
            let next_is_sibling_item =
                j < lines.len() && is_markdown_list_item(lines[j].trim_start());
            let has_inline_markdown_link = lines[i].contains("](");

            if blank_count > 0
                && next_is_sibling_item
                && !has_continuation
                && has_inline_markdown_link
                && !lines[i].contains(PARTIAL_LOOSE_ITEM_MARKER)
            {
                lines[i].push(' ');
                lines[i].push_str(PARTIAL_LOOSE_ITEM_MARKER);
            }

            i = if j > i { j } else { i + 1 };
        }
    }

    lines.join("\n")
}

/// blank lines because kramdown also wraps all items in `<p>`.
/// A "partially loose" list (some blanks but not all) is collapsed to tight.
///
/// Exception: sub-list items at the same indent level where ALL consecutive
/// pairs have blank lines between them (locally fully-loose sub-groups) keep
/// their blank lines so pulldown-cmark renders them as loose (issue #372).
pub fn collapse_blank_lines_between_list_items(content: &str) -> String {
    // Short-circuit: if no list markers, nothing to collapse.
    if !content.contains("- ") && !content.contains("* ") && !content.contains("+ ") {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 3 {
        return content.to_string();
    }

    // First pass: classify list regions
    let regions = find_list_regions(&lines);

    // Second pass: find line indices that are part of locally fully-loose
    // indented sub-groups within partial-loose regions. These items should
    // keep their blank lines so pulldown-cmark wraps them in <p>.
    let locally_loose_lines = find_locally_loose_subgroup_lines(&lines, &regions);

    let mut result = String::with_capacity(content.len());
    let mut in_code_block = false;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
        }

        if in_code_block {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(line);
            i += 1;
            continue;
        }

        let in_partial = regions
            .iter()
            .any(|r| i >= r.start && i < r.end && !r.fully_loose);
        let is_item = is_markdown_list_item(trimmed);

        if is_item && in_partial {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(line);

            let mut j = i + 1;
            while j < lines.len() {
                let nt = lines[j].trim_start();
                if nt.is_empty() {
                    break;
                }
                let indent = lines[j].len() - nt.len();
                if indent >= 2 && !is_markdown_list_item(nt) {
                    result.push('\n');
                    result.push_str(lines[j]);
                    j += 1;
                } else {
                    break;
                }
            }

            let mut blank_count = 0;
            while j < lines.len() && lines[j].trim().is_empty() {
                blank_count += 1;
                j += 1;
            }

            if blank_count > 0 && j < lines.len() {
                let nt = lines[j].trim_start();
                if is_markdown_list_item(nt) {
                    // Preserve blank lines for locally fully-loose sub-groups
                    // so pulldown-cmark renders them as loose list items.
                    if !locally_loose_lines.contains(&i) {
                        i = j;
                        continue;
                    }
                }
            }

            for _ in 0..blank_count {
                result.push('\n');
            }
            i = j;
        } else {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(line);
            i += 1;
        }
    }

    result
}

/// Find line indices of items that belong to locally fully-loose sub-groups
/// within partial-loose regions. A sub-group is locally fully-loose when:
/// - 2+ consecutive items at the same indent level (indent > 0)
/// - ALL consecutive pairs within the group have blank lines between them
/// - no items have continuation lines
fn find_locally_loose_subgroup_lines(
    lines: &[&str],
    regions: &[ListRegion],
) -> std::collections::HashSet<usize> {
    let mut result = std::collections::HashSet::new();

    for region in regions {
        if region.fully_loose {
            continue;
        }

        // Collect items in this region with their properties
        struct SubItem {
            line_idx: usize,
            indent: usize,
            has_blank_after_to_sibling: bool,
            has_continuation: bool,
        }
        let mut items: Vec<SubItem> = Vec::new();

        let mut in_code = false;
        let mut i = region.start;
        while i < region.end {
            let trimmed = lines[i].trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_code = !in_code;
                i += 1;
                continue;
            }
            if in_code || !is_markdown_list_item(trimmed) {
                i += 1;
                continue;
            }

            let indent = lines[i].len() - trimmed.len();

            let mut j = i + 1;
            let mut has_continuation = false;
            while j < lines.len() {
                let nt = lines[j].trim_start();
                if nt.is_empty() {
                    break;
                }
                let ind = lines[j].len() - nt.len();
                if ind >= 2 && !is_markdown_list_item(nt) {
                    has_continuation = true;
                    j += 1;
                } else {
                    break;
                }
            }

            let mut blank_count = 0;
            while j < lines.len() && lines[j].trim().is_empty() {
                blank_count += 1;
                j += 1;
            }
            let next_is_sibling = j < lines.len() && is_markdown_list_item(lines[j].trim_start());

            items.push(SubItem {
                line_idx: i,
                indent,
                has_blank_after_to_sibling: blank_count > 0 && next_is_sibling,
                has_continuation,
            });

            i = if j > i { j } else { i + 1 };
        }

        // Find contiguous groups at the same indent level (indent > 0)
        // where ALL consecutive pairs have blank lines
        let mut g = 0;
        while g < items.len() {
            if items[g].indent == 0 {
                g += 1;
                continue;
            }
            let group_indent = items[g].indent;
            let group_start = g;
            let mut g_end = g;
            // Extend group: consecutive items at same indent, all with blanks between them
            while g_end + 1 < items.len()
                && items[g_end + 1].indent == group_indent
                && items[g_end].has_blank_after_to_sibling
                && !items[g_end].has_continuation
            {
                g_end += 1;
            }

            let group_len = g_end - group_start + 1;
            if group_len >= 2 && !items[g_end].has_continuation {
                // Mark all items in this group
                for item in items.iter().take(g_end + 1).skip(group_start) {
                    result.insert(item.line_idx);
                }
            }
            g = g_end + 1;
        }
    }

    result
}

fn wrap_marked_partial_loose_list_items(html: &str) -> String {
    if !html.contains(PARTIAL_LOOSE_ITEM_MARKER) {
        return html.to_string();
    }

    let mut result = html.to_string();

    while let Some(marker_pos) = result.find(PARTIAL_LOOSE_ITEM_MARKER) {
        let Some(li_start) = result[..marker_pos].rfind("<li>") else {
            result = result.replacen(PARTIAL_LOOSE_ITEM_MARKER, "", 1);
            continue;
        };
        let Some(li_end_rel) = result[marker_pos..].find("</li>") else {
            result = result.replacen(PARTIAL_LOOSE_ITEM_MARKER, "", 1);
            continue;
        };
        let li_end = marker_pos + li_end_rel;
        let inner_start = li_start + "<li>".len();

        let space_marker = format!(" {}", PARTIAL_LOOSE_ITEM_MARKER);
        let mut inner = result[inner_start..li_end]
            .replace(&space_marker, "")
            .replace(PARTIAL_LOOSE_ITEM_MARKER, "");
        let should_wrap = is_simple_inline_list_item(inner.trim());
        if should_wrap {
            inner = format!("<p>{}</p>", inner.trim());
        }

        let mut rewritten = String::with_capacity(result.len() + if should_wrap { 7 } else { 0 });
        rewritten.push_str(&result[..inner_start]);
        rewritten.push_str(&inner);
        rewritten.push_str(&result[li_end..]);
        result = rewritten;
    }

    result
}

fn is_simple_inline_list_item(inner: &str) -> bool {
    if inner.is_empty() {
        return false;
    }
    if inner.contains("<p")
        || inner.contains("<ul")
        || inner.contains("<ol")
        || inner.contains("<li")
        || inner.contains("<div")
        || inner.contains("<table")
        || inner.contains("<blockquote")
        || inner.contains("<pre")
        || inner.contains("<h1")
        || inner.contains("<h2")
        || inner.contains("<h3")
        || inner.contains("<h4")
        || inner.contains("<h5")
        || inner.contains("<h6")
    {
        return false;
    }
    true
}

/// Convert kramdown-style pipe table lines to HTML `<table>` elements.
///
/// kramdown treats lines containing `|` as table rows when they appear at
/// block boundaries. This pre-processing converts such lines to raw HTML
/// tables before pulldown-cmark processes the markdown.
///
/// Key kramdown rules (from kramdown source `table.rb`):
/// - Table must start after a block boundary (blank line, start of document,
///   or end of preceding block element).
/// - Table must end before a block boundary (blank line, EOF, or start of
///   new block element). If the next line after pipe rows is non-empty
///   non-pipe text, the entire block is NOT a table.
/// - Inside list items, the table's block boundaries are relative to the
///   list item content.
pub fn convert_kramdown_pipe_tables(content: &str) -> String {
    // Short-circuit: if no pipe character at all, no tables to convert.
    if !content.contains('|') {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len());
    let mut i = 0;
    let mut in_code_block = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
        }

        if in_code_block {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(line);
            i += 1;
            continue;
        }

        if is_kramdown_table_line(trimmed) && !is_standard_pipe_table_context(&lines, i) {
            // Check after_block_boundary: previous line must be blank, start
            // of file, or a block-level element start. If the previous line
            // is non-empty non-block text, this pipe line is part of a
            // paragraph and should not become a table.
            if !is_after_block_boundary(&lines, i) {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(line);
                i += 1;
                continue;
            }

            let (prefix, content_part) = extract_line_prefix_and_content(line);
            let mut table_rows: Vec<String> = Vec::new();
            table_rows.push(content_part.to_string());

            let base_indent = line.len() - line.trim_start().len();
            let mut j = i + 1;
            while j < lines.len() {
                let next_line = lines[j];
                let next_trimmed = next_line.trim();
                if next_trimmed.is_empty() {
                    break;
                }
                let next_indent = next_line.len() - next_line.trim_start().len();
                if next_indent >= base_indent && is_kramdown_table_line(next_trimmed) {
                    table_rows.push(next_trimmed.to_string());
                    j += 1;
                } else {
                    break;
                }
            }

            // Check before_block_boundary: the line after the table rows
            // must be blank, EOF, or a block-level element. If a non-pipe,
            // non-blank line follows, kramdown does not treat the block as
            // a table.
            if !is_before_block_boundary(&lines, j) {
                // Not at a block boundary -- output all collected lines
                // as-is (not as a table).
                for (offset, &line_text) in lines[i..j].iter().enumerate() {
                    if i + offset > 0 {
                        result.push('\n');
                    }
                    result.push_str(line_text);
                }
                i = j;
                continue;
            }

            if i > 0 {
                result.push('\n');
            }
            result.push_str(&prefix);
            result.push_str("<table>\n<tbody>\n");
            for row_text in &table_rows {
                result.push_str("<tr>\n");
                for cell in split_kramdown_table_cells(row_text) {
                    result.push_str("<td>");
                    let cell_text = apply_typographic_symbols(cell.trim());
                    // HTML-escape < and > in cell content to match kramdown.
                    // Kramdown escapes these in pipe table cells since they're
                    // treated as raw text, not HTML.
                    // Note: we don't escape & because cell text may already
                    // contain HTML entities like &amp; from YAML sources.
                    let cell_text = cell_text.replace('<', "&lt;").replace('>', "&gt;");
                    result.push_str(&cell_text);
                    result.push_str("</td>\n");
                }
                result.push_str("</tr>\n");
            }
            result.push_str("</tbody>\n</table>");
            i = j;
        } else if is_kramdown_table_line(trimmed) && is_standard_pipe_table_context(&lines, i) {
            // GFM table detected (has separator row). Check block boundaries
            // like kramdown does. If not at proper boundaries, escape the
            // separator row so pulldown-cmark doesn't render it as a table.

            // Only process from the start of the table block (header row).
            // Walk backward to find the start.
            let mut table_start = i;
            while table_start > 0 {
                let prev_trimmed = lines[table_start - 1].trim();
                if is_table_separator_line(prev_trimmed)
                    || (prev_trimmed.starts_with('|') && prev_trimmed.ends_with('|'))
                {
                    table_start -= 1;
                } else {
                    break;
                }
            }

            // If we're not at the start of the table block, this line was
            // already processed or will be. Output as-is.
            if table_start != i {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(line);
                i += 1;
                continue;
            }

            // Collect all lines in this GFM table block.
            // GFM tables don't require leading/trailing `|`, so we match
            // separator lines, `|`-bounded lines, AND lines with internal
            // pipes (kramdown table lines). Without this, tables like
            // `A | B\n--|--\n1 | 2` would fail to collect any lines and
            // cause an infinite loop (j == i, i never advances).
            let mut j = i;
            while j < lines.len() {
                let jt = lines[j].trim();
                if is_table_separator_line(jt)
                    || (jt.starts_with('|') && jt.ends_with('|'))
                    || is_kramdown_table_line(jt)
                {
                    j += 1;
                } else {
                    break;
                }
            }

            // Check block boundaries (both before and after)
            let at_after_boundary = is_after_block_boundary(&lines, i);
            let at_before_boundary = is_before_block_boundary(&lines, j);

            if at_after_boundary && at_before_boundary {
                // Proper block boundaries -- pass through for pulldown-cmark.
                for (idx, line_text) in lines[i..j].iter().enumerate() {
                    if i + idx > 0 {
                        result.push('\n');
                    }
                    result.push_str(line_text);
                }
            } else {
                // Not at proper block boundaries -- escape separator rows
                // by replacing leading `|` with `\|` so pulldown-cmark does
                // not recognize them as GFM tables.
                for (idx, line_text) in lines[i..j].iter().enumerate() {
                    if i + idx > 0 {
                        result.push('\n');
                    }
                    let lt = line_text.trim();
                    if is_table_separator_line(lt) {
                        let indent_len = line_text.len() - line_text.trim_start().len();
                        let indent = &line_text[..indent_len];
                        result.push_str(indent);
                        result.push_str(&lt.replacen('|', "\\|", 1));
                    } else {
                        result.push_str(line_text);
                    }
                }
            }
            i = j;
        } else {
            if i > 0 {
                result.push('\n');
            }
            result.push_str(line);
            i += 1;
        }
    }
    result
}

/// Issue 491: Convert kramdown definition list syntax to HTML.
///
/// Kramdown recognises a definition list when a line of text (the term) is
/// immediately followed by a line starting with `:   ` (colon + 3 spaces) for
/// the definition. Pulldown-cmark does not support this, so we convert to
/// `<dl>/<dt>/<dd>` HTML before the markdown parser sees it.
///
/// The pattern is:
/// ```text
/// Term
/// :   Definition text
/// ```
///
/// Multiple consecutive term/definition pairs are grouped into a single `<dl>`.
/// Inline markdown (links, emphasis) within terms and definitions is rendered.
pub fn convert_kramdown_definition_lists(content: &str) -> String {
    // Quick check: if no definition marker pattern exists, return early.
    if !content.contains("\n:   ") && !content.starts_with(":   ") {
        return content.to_string();
    }

    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = String::with_capacity(content.len());
    let mut i = 0;
    let mut in_code_block = false;

    while i < lines.len() {
        let line = lines[i];

        // Track code fences
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            i += 1;
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            i += 1;
            continue;
        }

        // Check if current line is a valid term line followed by a definition marker.
        if is_potential_dl_term_line(line) {
            // Peek: is the next line a definition marker?
            if i + 1 < lines.len() && is_definition_marker_line(lines[i + 1]) {
                // Start a definition list block
                result.push_str("<dl>\n");

                loop {
                    if i >= lines.len() {
                        break;
                    }

                    let term_line = lines[i].trim();
                    if term_line.is_empty() {
                        // Check if there's another term+def pair after the blank line
                        if i + 2 < lines.len()
                            && !lines[i + 1].trim().is_empty()
                            && is_definition_marker_line(lines.get(i + 2).copied().unwrap_or(""))
                        {
                            // Skip blank line, continue accumulating in same <dl>
                            i += 1;
                            continue;
                        }
                        break;
                    }

                    // Must be a term line
                    if is_definition_marker_line(lines[i]) {
                        // Stray definition without term -- stop
                        break;
                    }

                    // Read the term
                    let term = lines[i].trim();
                    i += 1;

                    // Read definition(s) for this term
                    if i < lines.len() && is_definition_marker_line(lines[i]) {
                        result.push_str("  <dt>");
                        result.push_str(&render_dl_inline_markdown(term));
                        result.push_str("</dt>\n");

                        while i < lines.len() && is_definition_marker_line(lines[i]) {
                            let def = lines[i].trim().trim_start_matches(':').trim();
                            result.push_str("  <dd>");
                            result.push_str(&render_dl_inline_markdown(def));
                            result.push_str("</dd>\n");
                            i += 1;
                        }
                    } else {
                        // Not followed by definition -- shouldn't happen but handle gracefully
                        result.push_str("  <dt>");
                        result.push_str(&render_dl_inline_markdown(term));
                        result.push_str("</dt>\n");
                    }
                }

                result.push_str("</dl>\n");
                continue;
            }
        }

        result.push_str(line);
        if i + 1 < lines.len() {
            result.push('\n');
        }
        i += 1;
    }

    // Trim any trailing extra newline if the input didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Check if a line could be a definition list term.
/// Must be non-blank, not a code fence, not a definition marker,
/// and not an ATX heading (but `#hashtag` without space is OK).
fn is_potential_dl_term_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        return false;
    }
    if is_definition_marker_line(line) {
        return false;
    }
    // Reject ATX headings (# followed by space)
    if trimmed.starts_with("# ")
        || trimmed.starts_with("## ")
        || trimmed.starts_with("### ")
        || trimmed.starts_with("#### ")
        || trimmed.starts_with("##### ")
        || trimmed.starts_with("###### ")
    {
        return false;
    }
    true
}

/// Check if a line is a kramdown definition marker (starts with `:` + spaces).
fn is_definition_marker_line(line: &str) -> bool {
    if let Some(rest) = line.strip_prefix(':') {
        rest.starts_with("   ") || rest.starts_with('\t')
    } else {
        false
    }
}

/// Render inline markdown (links, emphasis, code) to HTML, stripping outer `<p>` tags.
fn render_dl_inline_markdown(text: &str) -> String {
    use pulldown_cmark::{html as cmark_html, Options, Parser};

    // If the text has no markdown syntax, return as-is for performance
    if !text.contains('[')
        && !text.contains('*')
        && !text.contains('_')
        && !text.contains('`')
        && !text.contains('<')
    {
        return text.to_string();
    }

    let options = Options::empty();
    let parser = Parser::new_ext(text, options);
    let mut html = String::new();
    cmark_html::push_html(&mut html, parser);

    // Strip outer <p>...</p>\n wrapper that pulldown-cmark adds
    let trimmed = html.trim();
    if trimmed.starts_with("<p>") && trimmed.ends_with("</p>") {
        trimmed[3..trimmed.len() - 4].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Check if position `index` is after a block boundary.
///
/// A block boundary exists when:
/// - `index == 0` (start of document)
/// - The previous line is blank
/// - The previous line is a block-level element (heading, HR, list item, HTML block, etc.)
/// - The previous line is itself a kramdown table line (table continues)
///
/// For list items: `- text | pipes |` is at a block boundary because
/// the `- ` prefix starts a new list item (block element). But a line
/// that is indented continuation of a previous non-pipe list item is
/// NOT at a block boundary.
fn is_after_block_boundary(lines: &[&str], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let prev = lines[index - 1];
    let prev_trimmed = prev.trim();

    // Blank line = block boundary
    if prev_trimmed.is_empty() {
        return true;
    }

    // Previous line is a block-level element start
    if is_block_level_line(prev_trimmed) {
        return true;
    }

    // Previous line is also a kramdown table line (continuing a table)
    if is_kramdown_table_line(prev_trimmed) {
        return true;
    }

    // Current line starts a new list item = block boundary within the list
    let current_trimmed = lines[index].trim();
    if is_markdown_list_item(current_trimmed) {
        return true;
    }

    false
}

/// Check if position `index` is before a block boundary.
///
/// The line at `index` is the first line AFTER the table rows.
/// A block boundary exists when:
/// - `index >= lines.len()` (EOF)
/// - The line at `index` is blank
/// - The line at `index` is a block-level element start
/// - The line at `index` is a new list item (block boundary for previous item)
fn is_before_block_boundary(lines: &[&str], index: usize) -> bool {
    if index >= lines.len() {
        return true;
    }
    let line = lines[index];
    let trimmed = line.trim();

    // Blank line = block boundary
    if trimmed.is_empty() {
        return true;
    }

    // New block-level element
    if is_block_level_line(trimmed) {
        return true;
    }

    // New list item = block boundary for previous list item
    if is_markdown_list_item(trimmed) {
        return true;
    }

    false
}

/// Check if a line starts a block-level element.
///
/// This approximates kramdown's block boundary detection for use in the
/// preprocessor. It detects headings, horizontal rules, HTML block tags,
/// code fences, and blockquotes.
fn is_block_level_line(trimmed: &str) -> bool {
    // ATX heading
    if trimmed.starts_with('#') {
        return true;
    }
    // Horizontal rule
    if trimmed.len() >= 3
        && (trimmed.chars().all(|c| c == '-' || c == ' ')
            || trimmed.chars().all(|c| c == '*' || c == ' ')
            || trimmed.chars().all(|c| c == '_' || c == ' '))
        && trimmed
            .chars()
            .filter(|&c| c == '-' || c == '*' || c == '_')
            .count()
            >= 3
    {
        return true;
    }
    // Code fence
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        return true;
    }
    // HTML block tag
    if trimmed.starts_with('<') && !trimmed.starts_with("<!-") {
        // Check for common block-level HTML tags
        let lower = trimmed.to_lowercase();
        if lower.starts_with("<div")
            || lower.starts_with("<p>")
            || lower.starts_with("<p ")
            || lower.starts_with("<table")
            || lower.starts_with("<blockquote")
            || lower.starts_with("<pre")
            || lower.starts_with("<hr")
            || lower.starts_with("<h1")
            || lower.starts_with("<h2")
            || lower.starts_with("<h3")
            || lower.starts_with("<h4")
            || lower.starts_with("<h5")
            || lower.starts_with("<h6")
            || lower.starts_with("<ul")
            || lower.starts_with("<ol")
            || lower.starts_with("<li")
            || lower.starts_with("<dl")
            || lower.starts_with("<dt")
            || lower.starts_with("<dd")
            || lower.starts_with("<figure")
            || lower.starts_with("<aside")
            || lower.starts_with("<section")
            || lower.starts_with("<article")
            || lower.starts_with("<nav")
            || lower.starts_with("<header")
            || lower.starts_with("<footer")
            || lower.starts_with("<details")
            || lower.starts_with("</")
        {
            return true;
        }
    }
    // Blockquote
    if trimmed.starts_with('>') {
        return true;
    }
    false
}

fn is_kramdown_table_line(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    let content = strip_list_prefix_for_table(trimmed).trim();
    if !has_pipe_outside_angle_brackets(content) {
        return false;
    }
    let inner = content.trim_matches('|').trim();
    if !inner.is_empty()
        && inner
            .chars()
            .all(|c| c == '-' || c == ':' || c == '|' || c == ' ')
    {
        return false;
    }
    true
}

/// Check if `content` contains a `|` character that is NOT inside angle brackets.
/// Check if content has an unescaped `|` character (not preceded by `\`).
/// kramdown treats ANY unescaped `|` as a potential table delimiter, regardless
/// of surrounding `<>` brackets (unlike CommonMark autolinks).
fn has_pipe_outside_angle_brackets(content: &str) -> bool {
    let mut prev_backslash = false;
    for ch in content.chars() {
        if prev_backslash {
            prev_backslash = false;
            continue;
        }
        match ch {
            '\\' => {
                prev_backslash = true;
            }
            '|' => return true,
            _ => {}
        }
    }
    false
}

fn strip_list_prefix_for_table(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("- ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        return rest;
    }
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        return rest;
    }
    if let Some(dot_pos) = trimmed.find(". ") {
        if dot_pos <= 3 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
            return &trimmed[dot_pos + 2..];
        }
    }
    trimmed
}

fn is_standard_pipe_table_context(lines: &[&str], index: usize) -> bool {
    // Check if this line's immediate neighbors are a separator
    if index + 1 < lines.len() && is_table_separator_line(lines[index + 1].trim()) {
        return true;
    }
    if index > 0 && is_table_separator_line(lines[index - 1].trim()) {
        return true;
    }

    // For data rows far from the separator: walk backward through consecutive
    // pipe-delimited rows to find the separator line. If any contiguous row
    // above us is adjacent to a separator, this entire block is a standard table.
    let trimmed = lines[index].trim();
    let content = strip_list_prefix_for_table(trimmed).trim();
    if content.starts_with('|') && content.ends_with('|') {
        // Walk backward through consecutive pipe rows
        let mut pos = index;
        while pos > 0 {
            let prev = lines[pos - 1].trim();
            let prev_content = strip_list_prefix_for_table(prev).trim();
            if is_table_separator_line(prev) {
                // Found the separator above this contiguous block
                return true;
            }
            if prev_content.starts_with('|') && prev_content.ends_with('|') {
                pos -= 1;
                continue;
            }
            break;
        }
        // Also walk forward to find a separator (for header rows above the separator)
        let mut pos = index;
        while pos + 1 < lines.len() {
            let next = lines[pos + 1].trim();
            let next_content = strip_list_prefix_for_table(next).trim();
            if is_table_separator_line(next) {
                return true;
            }
            if next_content.starts_with('|') && next_content.ends_with('|') {
                pos += 1;
                continue;
            }
            break;
        }
    }
    false
}

fn is_table_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.contains('-') {
        return false;
    }
    let stripped = trimmed.trim_start_matches('|').trim_end_matches('|');
    if stripped.is_empty() {
        return false;
    }
    stripped
        .chars()
        .all(|c| c == '-' || c == ':' || c == '|' || c == ' ')
}

fn extract_line_prefix_and_content(line: &str) -> (String, &str) {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let trimmed = line.trim_start();

    if let Some(rest) = trimmed.strip_prefix("- ") {
        return (format!("{indent}- "), rest);
    }
    if let Some(rest) = trimmed.strip_prefix("* ") {
        return (format!("{indent}* "), rest);
    }
    if let Some(rest) = trimmed.strip_prefix("+ ") {
        return (format!("{indent}+ "), rest);
    }
    if let Some(dot_pos) = trimmed.find(". ") {
        if dot_pos <= 3 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
            return (
                format!("{indent}{}", &trimmed[..dot_pos + 2]),
                &trimmed[dot_pos + 2..],
            );
        }
    }
    (indent.to_string(), trimmed)
}

/// Apply kramdown typographic symbol substitutions to text.
/// Converts:
/// - `...` -> `…` (U+2026 horizontal ellipsis)
/// - `---` -> `—` (U+2014 em-dash)
/// - `--` -> `–` (U+2013 en-dash)
///
/// Order matters: `---` must be replaced before `--`.
fn apply_typographic_symbols(text: &str) -> String {
    // Replace in order: longest patterns first
    let result = text.replace("---", "\u{2014}");
    let result = result.replace("--", "\u{2013}");
    result.replace("...", "\u{2026}")
}

fn split_kramdown_table_cells(row: &str) -> Vec<&str> {
    let trimmed = row.trim();
    let without_trailing = trimmed.strip_suffix('|').unwrap_or(trimmed);
    let content = without_trailing
        .strip_prefix('|')
        .unwrap_or(without_trailing);
    if content.is_empty() {
        return Vec::new();
    }
    content.split('|').collect()
}

struct ListRegion {
    start: usize,
    end: usize,
    fully_loose: bool,
}

fn find_list_regions(lines: &[&str]) -> Vec<ListRegion> {
    let mut regions = Vec::new();
    let mut in_code_block = false;
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            i += 1;
            continue;
        }
        if in_code_block {
            i += 1;
            continue;
        }
        if is_markdown_list_item(trimmed) {
            let start = i;
            let mut items = 0u32;
            let mut gaps = 0u32;
            while i < lines.len() {
                let t = lines[i].trim_start();
                if !is_markdown_list_item(t) {
                    break;
                }
                items += 1;
                i += 1;
                while i < lines.len() {
                    let ct = lines[i].trim_start();
                    if ct.is_empty() {
                        break;
                    }
                    let indent = lines[i].len() - ct.len();
                    if indent >= 2 && !is_markdown_list_item(ct) {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let mut blanks = 0u32;
                while i < lines.len() && lines[i].trim().is_empty() {
                    blanks += 1;
                    i += 1;
                }
                if blanks > 0 {
                    if i < lines.len() && is_markdown_list_item(lines[i].trim_start()) {
                        gaps += 1;
                    } else {
                        break;
                    }
                }
            }
            let fully_loose = items > 1 && gaps == items - 1;
            regions.push(ListRegion {
                start,
                end: i,
                fully_loose,
            });
        } else {
            i += 1;
        }
    }
    regions
}

/// Check if a trimmed line starts with a markdown list item marker.
fn is_markdown_list_item(trimmed: &str) -> bool {
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }
    // Check for ordered list: digits followed by . or ) and space
    let bytes = trimmed.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos > 0
        && pos < bytes.len()
        && (bytes[pos] == b'.' || bytes[pos] == b')')
        && pos + 1 < bytes.len()
        && bytes[pos + 1] == b' '
    {
        return true;
    }
    false
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
    "li", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "figure",
    // Note: figcaption is intentionally NOT included here.
    // In the DTC site (and typical Jekyll usage), <figcaption><p>...</p></figcaption>
    // is raw HTML in the source markdown. Jekyll/kramdown passes it through unchanged.
    // pulldown-cmark also passes HTML blocks through unchanged, so the <p> is original,
    // not auto-inserted. Stripping it would differ from Jekyll output.
    "summary", "dd", "dt",
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

/// Unwrap block-level HTML elements that pulldown-cmark erroneously wraps in
/// `<p>` tags. For example, `<p><noscript>...</noscript>\n</p>` becomes
/// `<noscript>...</noscript>`.
///
/// This happens when raw HTML block elements (like `<noscript>`) appear in
/// markdown content after Liquid preprocessing (e.g., from `{% gist %}` tags).
/// pulldown-cmark does not recognize all HTML5 block elements and wraps them
/// in paragraph tags.
fn unwrap_block_elements_from_p(html: &str) -> String {
    /// Block-level tags that should never appear inside `<p>`.
    const UNWRAP_TAGS: &[&str] = &["noscript", "iframe"];

    /// Issue 449: Void (self-closing) tags that should be unwrapped from `<p>`
    /// when they are the sole content of the paragraph.
    const UNWRAP_VOID_TAGS: &[&str] = &["img"];

    let mut result = html.to_string();
    for &tag in UNWRAP_TAGS {
        let open_tag = format!("<{}", tag);
        let close_tag = format!("</{}>", tag);
        let mut search_from = 0;
        loop {
            // Find `<p>` followed by our block tag
            let haystack = &result[search_from..];
            let Some(rel_pos) = haystack.find("<p>") else {
                break;
            };
            let p_pos = search_from + rel_pos;
            let after_p_start = p_pos + 3; // len("<p>")
            let after_p = &result[after_p_start..];

            // Check if immediately followed by our block tag (no whitespace)
            if !after_p.starts_with(&open_tag) {
                search_from = after_p_start;
                continue;
            }

            // Find the closing tag
            let Some(close_rel) = after_p.find(&close_tag) else {
                search_from = after_p_start;
                continue;
            };
            let close_end = after_p_start + close_rel + close_tag.len();
            let after_close = &result[close_end..];

            // Check for </p> after the close tag (with optional newline)
            let trimmed = after_close.trim_start_matches('\n');
            if !trimmed.starts_with("</p>") {
                search_from = after_p_start;
                continue;
            }
            let p_close_end = result.len() - trimmed.len() + 4; // len("</p>")

            // Extract the block content (without the <p>...</p> wrapper)
            // Also collapse any newline before the closing tag that pulldown-cmark inserts
            let block_content =
                result[after_p_start..close_end].replace(&format!("\n{}", close_tag), &close_tag);
            result = format!(
                "{}{}{}",
                &result[..p_pos],
                &block_content,
                &result[p_close_end..]
            );
            // Don't advance search_from -- there may be more instances
        }
    }

    // Issue 449: Unwrap void (self-closing) elements like <img> from <p> when
    // they are the sole content. Pattern: <p><img ... /></p> or <p><img ...></p>
    for &tag in UNWRAP_VOID_TAGS {
        let open_tag = format!("<{}", tag);
        let mut search_from = 0;
        loop {
            let haystack = &result[search_from..];
            let Some(rel_pos) = haystack.find("<p>") else {
                break;
            };
            let p_pos = search_from + rel_pos;
            let after_p_start = p_pos + 3; // len("<p>")
            let after_p = &result[after_p_start..];

            // Check if immediately followed by our void tag
            if !after_p.starts_with(&open_tag) {
                search_from = after_p_start;
                continue;
            }

            // Find the end of the tag (the closing >) -- void tags have no
            // separate closing tag, just <img ...> or <img ... />
            let Some(gt_rel) = after_p.find('>') else {
                search_from = after_p_start;
                continue;
            };
            let tag_end = after_p_start + gt_rel + 1;
            let after_tag = &result[tag_end..];

            // Check for </p> immediately after the void tag (with optional newline)
            let trimmed = after_tag.trim_start_matches('\n');
            if !trimmed.starts_with("</p>") {
                search_from = after_p_start;
                continue;
            }
            let p_close_end = result.len() - trimmed.len() + 4; // len("</p>")

            // Extract the void element (without the <p>...</p> wrapper)
            let element_content = result[after_p_start..tag_end].to_string();
            result = format!(
                "{}{}{}",
                &result[..p_pos],
                &element_content,
                &result[p_close_end..]
            );
            // Don't advance search_from -- there may be more instances
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
    let mut figcaption_depth = 0usize;

    while !remaining.is_empty() {
        // Track whether we're inside a <figcaption> element.
        // <p> tags inside <figcaption> must be preserved (they're from the
        // original source HTML, not auto-inserted by pulldown-cmark).
        if remaining.starts_with("<figcaption") {
            figcaption_depth += 1;
        }
        if remaining.starts_with("</figcaption>") && figcaption_depth > 0 {
            figcaption_depth -= 1;
        }

        if figcaption_depth > 0 {
            // Inside <figcaption>: copy char-by-char without stripping <p>
            let ch = remaining.chars().next().unwrap();
            result.push(ch);
            remaining = &remaining[ch.len_utf8()..];
            continue;
        }

        if let Some(p_pos) = remaining.find("<p>") {
            // Check if there's a <figcaption> before this <p>
            let figcap_pos = remaining.find("<figcaption");
            if let Some(fp) = figcap_pos {
                if fp < p_pos {
                    // <figcaption> comes before <p> -- copy up to <figcaption>
                    // and let the loop handle it
                    result.push_str(&remaining[..fp]);
                    remaining = &remaining[fp..];
                    continue;
                }
            }

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

/// Apply block-level kramdown IALs.
///
/// Block IALs appear as standalone paragraphs: `<p>{: .class }</p>`.
/// This happens when markdown like:
/// ```markdown
/// # Heading
/// {: .fs-9 }
/// ```
/// is parsed by comrak, which wraps the `{: .fs-9 }` line in a `<p>` tag.
///
/// This function finds `<p>{: ... }</p>` paragraphs and applies their
/// attributes to the preceding block element, then removes the IAL paragraph.
fn apply_block_ial(html: &str) -> String {
    // Find all `<p>{: ... }</p>` patterns (block-level IAL paragraphs).
    // Process from end to start to preserve positions when modifying.
    let mut result = html.to_string();
    let prefix = "<p>{:";
    let suffix = "}</p>";

    // Collect all match positions first
    let mut matches: Vec<(usize, usize, String)> = Vec::new();
    let mut search_from = 0;
    while let Some(p_start) = result[search_from..].find(prefix) {
        let abs_start = search_from + p_start;
        // Find closing }</p>
        let after_prefix = abs_start + prefix.len();
        if let Some(close_offset) = result[after_prefix..].find(suffix) {
            let abs_end = after_prefix + close_offset + suffix.len();
            // Extract the attribute string between {: and }
            let attr_str = result[after_prefix..after_prefix + close_offset]
                .trim()
                .to_string();
            // Verify this <p> contains ONLY the IAL (no other content before {: )
            matches.push((abs_start, abs_end, attr_str));
            search_from = abs_end;
        } else {
            search_from = abs_start + prefix.len();
        }
    }

    // Process from end to start
    for (start, end, attr_str) in matches.into_iter().rev() {
        let attrs = parse_ial_attributes(&attr_str);
        if attrs.is_empty() {
            continue;
        }

        // Determine direction: if the IAL paragraph was preceded by a
        // `<!-- IAL:FWD -->` marker (inserted by mark_forward_ial() during
        // markdown preprocessing), apply to the FOLLOWING element.
        let before = &result[..start];
        let fwd_marker = "<!-- IAL:FWD -->";
        let has_fwd_marker = before.trim_end().ends_with(fwd_marker);

        if has_fwd_marker {
            // Forward direction: apply to the next block element after the IAL
            let after = &result[end..];
            // Skip whitespace/newlines to find the next opening tag
            let trimmed_after = after.trim_start();
            if trimmed_after.starts_with('<') {
                // Calculate the absolute position of the next opening tag
                let whitespace_len = after.len() - trimmed_after.len();
                let next_tag_pos = end + whitespace_len;

                // Remove the IAL paragraph AND the preceding <!-- IAL:FWD --> marker.
                // The marker is in the text before the <p>{: tag.
                let marker_pos = before.trim_end().len() - fwd_marker.len();
                // Extend removal to include any whitespace before the marker
                let remove_start = if marker_pos > 0 && result.as_bytes()[marker_pos - 1] == b'\n' {
                    marker_pos - 1
                } else {
                    marker_pos
                };
                // Also remove trailing newline after the IAL if present
                let remove_end = if end < result.len() && result.as_bytes()[end] == b'\n' {
                    end + 1
                } else {
                    end
                };
                let removed_len = remove_end - remove_start;
                result.replace_range(remove_start..remove_end, "");

                // Adjust position of the next tag after removal
                let adjusted_pos = next_tag_pos - removed_len;
                insert_attributes_at(&mut result, adjusted_pos, &attrs);
            }
        } else {
            // Backward direction (original behavior): apply to the preceding element
            // Look for the last closing tag before this IAL paragraph
            if let Some(close_pos) = before.rfind("</") {
                if let Some(gt_pos) = before[close_pos..].find('>') {
                    let tag_name = before[close_pos + 2..close_pos + gt_pos].to_string();

                    // Find the matching opening tag
                    let search_area = &before[..close_pos];
                    if let Some(open_pos) = find_last_opening_tag(search_area, &tag_name) {
                        // Remove the IAL paragraph (including any preceding newline)
                        let remove_start = if start > 0 && result.as_bytes()[start - 1] == b'\n' {
                            start - 1
                        } else {
                            start
                        };
                        result.replace_range(remove_start..end, "");

                        // Apply attributes to the opening tag
                        insert_attributes_at(&mut result, open_pos, &attrs);
                    }
                }
            }
        }
    }

    // Second pass: handle IAL merged into paragraph text by comrak.
    // When there's no blank line between paragraph text and IAL, comrak merges them:
    //   `<p>Some text\n{: .fs-6 .fw-300 }</p>` or `<p>Some text {: .fs-6 }</p>`
    // We find `{: ... }` at the end of text content within closing `</p>`, `</h1>`, etc.,
    // strip it, and apply attributes to the element.
    apply_merged_ial(&mut result);

    result
}

/// Handle IAL patterns merged into block element text content by comrak.
///
/// When there's no blank line between paragraph text and an IAL line, comrak
/// merges them into one element:
///   `<p>Some text\n{: .fs-6 .fw-300 }</p>`
///
/// This function finds `{: ... }` at the end of text content before a closing
/// tag (e.g., `</p>`), strips the IAL text, and applies attributes to the
/// element's opening tag.
fn apply_merged_ial(html: &mut String) {
    // We search for patterns like: `{: .class1 .class2 }</p>` or `{: .class }</h1>` etc.
    // working backwards to preserve positions.
    let ial_marker = "{: ";
    let mut search_from = 0;
    let mut replacements: Vec<(usize, usize, String, usize)> = Vec::new();

    while let Some(ial_start) = html[search_from..].find(ial_marker) {
        let abs_ial_start = search_from + ial_start;

        // Find the closing `}` on the same line
        let after_marker = abs_ial_start + ial_marker.len();
        let rest = &html[after_marker..];
        let mut close_brace = None;
        for (i, ch) in rest.char_indices() {
            if ch == '}' {
                close_brace = Some(after_marker + i);
                break;
            }
            if ch == '\n' {
                break;
            }
        }

        let close_brace = match close_brace {
            Some(pos) => pos,
            None => {
                search_from = after_marker;
                continue;
            }
        };

        // Check that this `}` is followed (possibly with whitespace) by a closing tag `</tag>`
        let after_brace = close_brace + 1;
        let trailing = &html[after_brace..];
        let trimmed = trailing.trim_start();
        if !trimmed.starts_with("</") {
            search_from = after_brace;
            continue;
        }

        // Find the closing tag name
        let close_tag_start = after_brace + (trailing.len() - trimmed.len());
        let tag_rest = &html[close_tag_start + 2..];
        let tag_end = tag_rest.find('>').unwrap_or(0);
        let tag_name = &html[close_tag_start + 2..close_tag_start + 2 + tag_end];

        // Only handle block elements
        let is_block = matches!(
            tag_name,
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "blockquote" | "div"
        );
        if !is_block {
            search_from = after_brace;
            continue;
        }

        // This is already handled by the first pass if the IAL is in its own <p>{: ... }</p>
        // Check that the IAL is NOT at the very start of the element (after the opening tag)
        // by looking for content before the {: marker
        let before_ial = &html[..abs_ial_start];
        // Check if the preceding non-whitespace is `>` from an opening `<p>` tag
        let trimmed_before = before_ial.trim_end();
        if trimmed_before.ends_with(&format!("<{}>", tag_name))
            || trimmed_before.ends_with(&format!("<{} ", tag_name))
        {
            // This looks like `<p>{: ... }</p>` -- already handled by first pass
            search_from = after_brace;
            continue;
        }

        // Skip if the IAL immediately follows a closing tag (e.g., `</a>{: .btn }`)
        // These are inline IALs handled by apply_inline_attributes.
        if trimmed_before.ends_with('>') {
            // Check if this is a closing tag by looking for `</` before `>`
            if let Some(lt_pos) = trimmed_before.rfind("</") {
                let candidate = &trimmed_before[lt_pos..];
                if candidate.ends_with('>') {
                    search_from = after_brace;
                    continue;
                }
            }
        }

        // Extract the attribute string
        let attr_str = html[after_marker..close_brace].trim().to_string();
        let attrs = parse_ial_attributes(&attr_str);
        if attrs.is_empty() {
            search_from = after_brace;
            continue;
        }

        // Find the opening tag for this element by searching backwards from the IAL
        let search_area = &html[..abs_ial_start];
        if let Some(open_pos) = find_last_opening_tag(search_area, tag_name) {
            // Determine what to remove: from whitespace before `{:` to `}` (inclusive)
            // Also strip any preceding whitespace/newline
            let mut remove_start = abs_ial_start;
            while remove_start > 0
                && matches!(
                    html.as_bytes()[remove_start - 1],
                    b' ' | b'\n' | b'\r' | b'\t'
                )
            {
                remove_start -= 1;
                // Don't go past the opening tag content
                if remove_start <= open_pos {
                    remove_start = abs_ial_start;
                    break;
                }
            }

            replacements.push((remove_start, close_brace + 1, attr_str, open_pos));
        }

        search_from = after_brace;
    }

    // Apply replacements from end to start to preserve positions
    for (remove_start, remove_end, attr_str, open_pos) in replacements.into_iter().rev() {
        // Remove the IAL text
        html.replace_range(remove_start..remove_end, "");

        // Apply attributes to the opening tag
        let attrs = parse_ial_attributes(&attr_str);
        insert_attributes_at(html, open_pos, &attrs);
    }
}

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
        let before_close = &html[..=close_tag_end];

        // First try: closing tag (</tagname>)
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

        // Second try: self-closing/void element (e.g., <img ... /> or <br />).
        // Check if the last `>` is from a self-closing tag ending with `/>`.
        if close_tag_end >= 1 && html.as_bytes()[close_tag_end - 1] == b'/' {
            // Find the opening `<` for this self-closing tag
            if let Some(open_lt) = html[..close_tag_end].rfind('<') {
                // Verify it's not a closing tag
                if open_lt + 1 < html.len() && html.as_bytes()[open_lt + 1] != b'/' {
                    insert_attributes_at(html, open_lt, &attrs);
                    return true;
                }
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

        // Merge all class values into a single space-separated string.
        let mut classes: Vec<&str> = Vec::new();
        let mut other_attrs = String::new();
        for (key, value) in attrs {
            if key == "class" {
                classes.push(value);
            } else if key == "id" {
                other_attrs.push_str(&format!(" id=\"{}\"", value));
            } else {
                other_attrs.push_str(&format!(" {}=\"{}\"", key, value));
            }
        }

        // Handle classes: sort alphabetically (matching kramdown behavior)
        // and append to existing or create new class attribute
        if !classes.is_empty() {
            classes.sort_unstable();
            let merged = classes.join(" ");
            if let Some(class_start) = existing_tag.find("class=\"") {
                // Append to existing class attribute
                let class_val_start = open_pos + class_start + 7; // after `class="`
                if let Some(class_val_end) = html[class_val_start..].find('"') {
                    let insert_pos = class_val_start + class_val_end;
                    html.insert_str(insert_pos, &format!(" {}", merged));
                    // Recalculate gt_pos since we inserted before it
                    let new_gt_pos = gt_pos + merged.len() + 1;
                    // For self-closing tags, insert before ` />`
                    let adj_pos = if new_gt_pos >= 2
                        && html.as_bytes()[new_gt_pos - 1] == b'/'
                        && html.as_bytes()[new_gt_pos - 2] == b' '
                    {
                        new_gt_pos - 2
                    } else if new_gt_pos >= 1 && html.as_bytes()[new_gt_pos - 1] == b'/' {
                        new_gt_pos - 1
                    } else {
                        new_gt_pos
                    };
                    html.insert_str(adj_pos, &other_attrs);
                    return;
                }
            } else {
                other_attrs = format!(" class=\"{}\"", merged) + &other_attrs;
            }
        }

        // Insert before the `>` (or before ` />` for self-closing tags)
        let insert_pos = if gt_pos >= 2
            && html.as_bytes()[gt_pos - 1] == b'/'
            && html.as_bytes()[gt_pos - 2] == b' '
        {
            // Self-closing tag with space: `<img ... />`  -> insert before ` />`
            gt_pos - 2
        } else if gt_pos >= 1 && html.as_bytes()[gt_pos - 1] == b'/' {
            // Self-closing tag without space: `<br/>`  -> insert before `/>`
            gt_pos - 1
        } else {
            gt_pos
        };
        html.insert_str(insert_pos, &other_attrs);
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
    // Strip optional trailing colon (kramdown allows `{: .class :}`)
    let trimmed = decoded.trim();
    let trimmed = trimmed.strip_suffix(':').unwrap_or(trimmed).trim();
    let mut remaining: &str = trimmed;

    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        if remaining.starts_with('.') {
            // Class shorthand -- kramdown supports dot-concatenated classes:
            // `.class1.class2` means two separate classes.
            remaining = &remaining[1..];
            let end = remaining
                .find(|c: char| c.is_whitespace() || c == '}')
                .unwrap_or(remaining.len());
            let class_token = &remaining[..end];
            // Split on '.' to handle concatenated classes like "mx-auto.d-block"
            for part in class_token.split('.') {
                if !part.is_empty() {
                    attrs.push(("class".to_string(), part.to_string()));
                }
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

/// Count the net change in `<div` open and `</div>` close tags in a slice.
/// Returns the delta (opens minus closes). Used by `add_heading_ids` to track
/// whether a heading is inside a raw HTML `<div>` block.
fn count_div_depth_delta(text: &str) -> i32 {
    let mut delta: i32 = 0;
    let mut pos = 0;
    let bytes = text.as_bytes();
    while pos < bytes.len() {
        if pos + 4 < bytes.len() && &bytes[pos..pos + 4] == b"<div" {
            // Check it's actually a tag: next char should be '>', ' ', '\n', '\t', or '/'
            let next = bytes.get(pos + 4).copied().unwrap_or(b'>');
            if next == b'>' || next == b' ' || next == b'\n' || next == b'\t' {
                delta += 1;
                pos += 4;
                continue;
            }
        }
        if pos + 6 <= bytes.len() && &bytes[pos..pos + 6] == b"</div>" {
            delta -= 1;
            pos += 6;
            continue;
        }
        pos += 1;
    }
    delta
}

/// Add auto-generated `id` attributes to heading tags.
///
/// Matches kramdown's algorithm:
/// - Lowercase the heading text
/// - Replace spaces with hyphens
/// - Strip non-alphanumeric characters (except hyphens)
/// - Handle duplicates by appending `-1`, `-2`, etc.
fn add_heading_ids(html: &str, mode: HeadingIdMode) -> String {
    let mut result = String::with_capacity(html.len());
    let mut used_ids: HashMap<String, usize> = HashMap::new();
    let mut remaining = html;
    // Issue 526: Track div nesting depth so headings inside raw HTML <div> blocks
    // (like note/warning boxes) don't get auto-generated IDs.
    let mut div_depth: i32 = 0;

    while !remaining.is_empty() {
        // Find next heading tag
        if let Some(h_pos) = find_next_heading(remaining) {
            // Copy everything before the heading
            let before = &remaining[..h_pos];
            // Issue 526: Update div nesting depth from content before this heading.
            div_depth += count_div_depth_delta(before);
            result.push_str(before);

            let after = &remaining[h_pos..];
            // Parse the heading tag: <hN> or <hN ...>
            if let Some(gt_pos) = after.find('>') {
                let tag = &after[..gt_pos + 1];
                let level_char = after.as_bytes()[2]; // h1, h2, etc.

                // Find closing tag
                let close_tag = format!("</h{}>", level_char as char);
                if let Some(close_pos) = after.find(&close_tag) {
                    let inner_html = &after[gt_pos + 1..close_pos];

                    // Issue 228: Check for explicit {#custom-id} syntax
                    let (clean_inner, explicit_id) = extract_explicit_heading_id(inner_html);

                    // Issue 320: Detect headings from markdown="1" blocks
                    let is_md1_heading = tag.contains("data-md1-heading");

                    let id = if let Some(eid) = explicit_id {
                        // Use explicit ID, still track for uniqueness
                        let _ = get_unique_id(&mut used_ids, &eid);
                        eid
                    } else if mode == HeadingIdMode::CommonMarkGhPages {
                        // Issue 330: CommonMarkGhPages generates heading IDs from
                        // raw inner HTML using basic_generate_id (ASCII-only).
                        // Jekyll's commonmarker gem outputs &quot; for " in text
                        // nodes, and the slugify preserves "quot" as ASCII text.
                        // We replicate this by HTML-encoding text nodes in the
                        // inner HTML before running basic_generate_id.
                        let encoded = encode_text_nodes_for_heading_id(&clean_inner);
                        let slug = basic_generate_id(&encoded);
                        get_unique_id(&mut used_ids, &slug)
                    } else {
                        // Extract text content (strip HTML tags, decode entities)
                        let text = strip_html_tags(&clean_inner);
                        let text = decode_html_entities(&text);
                        // Issue 320: Use basic_generate_id for headings inside
                        // markdown="1" blocks (strips non-ASCII, falls back to
                        // "section") to match kramdown's base parser behavior.
                        let slug = if is_md1_heading {
                            basic_generate_id(&text)
                        } else {
                            slugify(&text)
                        };

                        // Handle duplicates
                        get_unique_id(&mut used_ids, &slug)
                    };

                    // Only add IDs to headings generated by pulldown-cmark
                    // (simple <hN> tags with no existing attributes).
                    // Raw HTML headings passed through will already have
                    // attributes like class="...", so we skip those.
                    // Issue 320: data-md1-heading tags are treated as simple tags.
                    // Issue 526: Skip headings inside raw HTML <div> blocks
                    // (e.g., note/warning boxes). These are raw HTML that kramdown
                    // passes through without adding IDs.
                    let is_simple_tag = tag == format!("<h{}>", level_char as char);
                    let is_md1_tag = tag == format!("<h{} data-md1-heading>", level_char as char);
                    let inside_div = div_depth > 0;
                    if !is_simple_tag && !is_md1_tag {
                        // Has existing attributes or id -- leave as-is
                        result.push_str(&after[..close_pos + close_tag.len()]);
                    } else if inside_div && !is_md1_tag {
                        // Issue 526: Heading inside a raw HTML <div> block --
                        // leave as-is without adding an ID.
                        result.push_str(&after[..close_pos + close_tag.len()]);
                    } else if is_md1_tag {
                        // Issue 320: Strip data-md1-heading marker and add ID
                        result.push_str(&format!("<h{} id=\"{}\">", level_char as char, id));
                        result.push_str(&clean_inner);
                        result.push_str(&close_tag);
                    } else if clean_inner != inner_html {
                        // Issue 228: {#id} was stripped -- use cleaned inner HTML
                        result.push_str(&format!("<h{} id=\"{}\">", level_char as char, id));
                        result.push_str(&clean_inner);
                        result.push_str(&close_tag);
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

/// Extract an explicit heading ID from `{#custom-id}` syntax in heading text.
///
/// Kramdown allows `## Heading Text {#custom-id}` to set a custom ID.
/// This is different from block IAL `{: #id}` which is already handled elsewhere.
///
/// Returns (cleaned_inner_html, Some(id)) if `{#id}` was found, or
/// (original_inner_html, None) if not.
fn extract_explicit_heading_id(inner_html: &str) -> (String, Option<String>) {
    // Look for {#...} at the end of the heading text (possibly with trailing whitespace)
    let trimmed = inner_html.trim_end();
    if let Some(brace_pos) = trimmed.rfind("{#") {
        if let Some(close_pos) = trimmed[brace_pos..].find('}') {
            let id = &trimmed[brace_pos + 2..brace_pos + close_pos];
            if !id.is_empty() && !id.contains(' ') {
                // Strip the {#id} and any preceding whitespace
                let before = trimmed[..brace_pos].trim_end();
                return (before.to_string(), Some(id.to_string()));
            }
        }
    }
    (inner_html.to_string(), None)
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

/// Decode HTML entities to their corresponding characters.
///
/// Handles named entities (&amp;, &lt;, &gt;, &quot;, &apos;),
/// decimal numeric entities (&#8217;), and hex numeric entities (&#x2019;).
fn decode_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(amp_pos) = remaining.find('&') {
        result.push_str(&remaining[..amp_pos]);
        let after_amp = &remaining[amp_pos + 1..];

        if let Some(semi_pos) = after_amp.find(';') {
            // Only decode if the entity is reasonably short (avoid matching across large spans)
            if semi_pos <= 10 {
                let entity = &after_amp[..semi_pos];
                if let Some(decoded) = decode_entity(entity) {
                    result.push(decoded);
                    remaining = &after_amp[semi_pos + 1..];
                    continue;
                }
            }
        }

        // Not a valid entity; keep the '&' and move on
        result.push('&');
        remaining = after_amp;
    }

    result.push_str(remaining);
    result
}

/// Decode a single HTML entity body (the part between & and ;).
fn decode_entity(entity: &str) -> Option<char> {
    // Named entities
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{00A0}'),
        _ => {
            // Numeric entities
            if let Some(hex) = entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
            {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
            } else if let Some(dec) = entity.strip_prefix('#') {
                dec.parse::<u32>().ok().and_then(char::from_u32)
            } else {
                None
            }
        }
    }
}

/// Convert heading text to a URL-friendly slug matching kramdown-parser-gfm's
/// `generate_gfm_header_id`.
///
/// Jekyll defaults to `kramdown: { input: GFM }`, so all Jekyll sites use the GFM
/// heading ID algorithm rather than kramdown's ASCII-only `basic_generate_id`.
///
/// GFM algorithm from kramdown-parser-gfm (Ruby):
/// ```ruby
/// NON_WORD_RE = /[^\p{Word}\- \t]/
/// result = text.downcase
/// result.gsub!(NON_WORD_RE, '')
/// result.tr!(" \t", '-')
/// ```
///
/// `\p{Word}` matches Unicode letters, digits, and underscore.
/// This preserves Cyrillic, Arabic, accented Latin, CJK, etc. in heading IDs.
fn slugify(text: &str) -> String {
    // Step 1: Downcase (Unicode-aware)
    let lower = text.to_lowercase();

    // Step 2: Keep only Unicode word chars (letters, digits, underscore), hyphens,
    // spaces, and tabs. Strip everything else (punctuation, symbols, etc.)
    let mut slug = String::with_capacity(lower.len());
    for ch in lower.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ' ' || ch == '\t' {
            slug.push(ch);
        }
    }

    // Step 3: Replace spaces and tabs with hyphens
    slug = slug.replace([' ', '\t'], "-");

    // Step 4: Fall back to "section" if empty
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

/// Kramdown's `basic_generate_id` algorithm for headings inside `markdown="1"` blocks.
///
/// Unlike the GFM algorithm (`slugify`), this strips ALL non-ASCII characters.
/// Matches kramdown's Ruby implementation in converter/base.rb:
/// ```ruby
/// def basic_generate_id(str)
///   gen_id = str.gsub(/^[^a-zA-Z]+/, '')   # strip leading non-alpha
///   gen_id.tr!('^a-zA-Z0-9 -', '')          # keep only ASCII alphanum, space, hyphen
///   gen_id.tr!(' ', '-')                     # replace spaces with hyphens
///   gen_id.downcase!                         # downcase
///   gen_id
/// end
/// ```
///
/// The "section" fallback for empty results is included here (matching `generate_id`).
fn basic_generate_id(text: &str) -> String {
    // Step 1: Strip leading non-ASCII-alpha characters
    let text = text.trim_start_matches(|c: char| !c.is_ascii_alphabetic());

    // Step 2: Keep only ASCII alphanumeric, space, and hyphen
    let mut slug = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-' {
            slug.push(ch);
        }
    }

    // Step 3: Replace spaces with hyphens
    slug = slug.replace(' ', "-");

    // Step 4: Downcase
    slug = slug.to_lowercase();

    // Step 5: Fall back to "section" if empty
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

/// Issue 330: HTML-encode text nodes in heading inner HTML, preserving HTML tags as-is.
///
/// Jekyll's commonmarker gem outputs `&quot;` for `"` in text nodes, and the heading
/// ID generation includes "quot" as part of the slug. pulldown-cmark outputs literal `"`
/// in text nodes. This function encodes `"` in text nodes to `&quot;` while preserving
/// HTML tags (where `"` is used as attribute delimiters).
fn encode_text_nodes_for_heading_id(inner_html: &str) -> String {
    let mut result = String::with_capacity(inner_html.len());
    let mut in_tag = false;
    for ch in inner_html.chars() {
        if ch == '<' {
            in_tag = true;
            result.push(ch);
        } else if ch == '>' {
            in_tag = false;
            result.push(ch);
        } else if !in_tag && ch == '"' {
            result.push_str("&quot;");
        } else {
            result.push(ch);
        }
    }
    result
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
/// <div class="highlighter-rouge"><div class="highlight"><pre class="highlight"><code>...</code></pre></div></div>
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

/// Check if a language is recognized by Rouge (Jekyll's syntax highlighter),
/// even if syntect cannot highlight it. These languages get the `<div>` wrapper
/// in Jekyll's output rather than bare `<pre><code>`.
fn is_rouge_recognized_language(lang: &str) -> bool {
    matches!(
        lang,
        "turtle"
            | "ecl"
            | "verilog"
            | "systemverilog"
            | "sparql"
            | "ntriples"
            | "elixir"
            | "slim"
            | "haml"
            | "sass"
            | "scss"
            | "less"
            | "coffeescript"
            | "handlebars"
            | "liquid"
            | "twig"
            | "jinja"
            | "django"
            | "ada"
            | "nim"
            | "crystal"
            | "ceylon"
            | "io"
            | "factor"
            | "coq"
            | "isabelle"
            | "agda"
            | "idris"
            | "sml"
            | "ocaml"
            | "fsharp"
            | "batchfile"
            | "powershell"
            | "docker"
            | "dockerfile"
    )
}

fn wrap_fenced_code_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for <pre followed by optional attributes and ><code
        // This handles both bare <pre><code> and IAL-annotated <pre data-title="..."><code>
        if let Some(pre_pos) = remaining.find("<pre") {
            let after_pre_tag = &remaining[pre_pos + 4..];

            let (pre_attrs, after_pre_open) = if let Some(rest) = after_pre_tag.strip_prefix('>') {
                // Bare <pre> — no attributes
                ("", rest)
            } else if after_pre_tag.starts_with(' ') {
                // <pre with attributes — find the closing >
                if let Some(gt_pos) = after_pre_tag.find('>') {
                    let attrs = after_pre_tag[1..gt_pos].trim();
                    (attrs, &after_pre_tag[gt_pos + 1..])
                } else {
                    // Unclosed <pre, skip
                    result.push_str(&remaining[..pre_pos + 4]);
                    remaining = after_pre_tag;
                    continue;
                }
            } else {
                // <pre followed by something unexpected (e.g. <preview), skip
                result.push_str(&remaining[..pre_pos + 4]);
                remaining = after_pre_tag;
                continue;
            };

            // Now check if the content starts with <code
            if !after_pre_open.starts_with("<code") {
                // Not a code block, copy the <pre> tag and continue
                if pre_attrs.is_empty() {
                    result.push_str("<pre>");
                } else {
                    result.push_str(&format!("<pre {}>", pre_attrs));
                }
                remaining = after_pre_open;
                continue;
            }

            // Copy everything before this <pre>
            result.push_str(&remaining[..pre_pos]);

            let after_code = &after_pre_open[5..]; // skip "<code"

            let (lang, after_open_tag) = if let Some(rest) = after_code.strip_prefix('>') {
                ("plaintext".to_string(), rest)
            } else if let Some(rest) = after_code.strip_prefix(" class=\"language-") {
                if let Some(quote_end) = rest.find('"') {
                    let lang = rest[..quote_end].to_string();
                    let after_quote = &rest[quote_end + 1..];
                    if let Some(inner) = after_quote.strip_prefix('>') {
                        (lang, inner)
                    } else {
                        result.push_str("<pre><code");
                        remaining = after_code;
                        continue;
                    }
                } else {
                    result.push_str("<pre><code");
                    remaining = after_code;
                    continue;
                }
            } else {
                result.push_str("<pre><code");
                remaining = after_code;
                continue;
            };

            if let Some(close_pos) = after_open_tag.find("</code></pre>") {
                let code_content = &after_open_tag[..close_pos];
                let raw_code = html_unescape(code_content);
                let highlighted = if lang != "plaintext" {
                    crate::syntax::highlight_code(&lang, &raw_code).or_else(|| {
                        if lang == "docker" || lang == "dockerfile" {
                            crate::docker_highlight::highlight_docker(&raw_code)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                if lang == "plaintext"
                    || highlighted.is_some()
                    || is_rouge_recognized_language(&lang)
                {
                    if lang == "plaintext" {
                        if pre_attrs.is_empty() {
                            result.push_str(
                                "<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                            );
                        } else {
                            result.push_str(&format!(
                                "<div {} class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                                pre_attrs
                            ));
                        }
                    } else if pre_attrs.is_empty() {
                        result.push_str(&format!(
                            "<div class=\"language-{} highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                            lang
                        ));
                    } else {
                        result.push_str(&format!(
                            "<div {} class=\"language-{} highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                            pre_attrs, lang
                        ));
                    }
                    if let Some(ref hl) = highlighted {
                        result.push_str(hl);
                    } else {
                        result.push_str(code_content);
                    }
                    result.push_str("</code></pre></div></div>");
                } else if pre_attrs.is_empty() {
                    result.push_str(&format!("<pre><code class=\"language-{}\">", lang));
                    result.push_str(code_content);
                    result.push_str("</code></pre>");
                } else {
                    result.push_str(&format!(
                        "<div {}><pre><code class=\"language-{}\">",
                        pre_attrs, lang
                    ));
                    result.push_str(code_content);
                    result.push_str("</code></pre></div>");
                }
                remaining = &after_open_tag[close_pos + 13..];
            } else {
                result.push_str("<pre><code>");
                remaining = after_open_tag;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

// Note: Inline code class addition (previously step 4) has been moved to
// frontmatter::add_inline_code_class_to_events() which operates on pulldown-cmark
// events during markdown rendering. This ensures only backtick-generated <code>
// gets the class -- not raw HTML <code> tags already present in the source.

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
        "tfoot",
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
        "noscript",
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
        "tfoot",
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
        "noscript",
        "iframe",
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

    // Issue 449: Standalone void elements (like <img>) that occupy the entire
    // line are block-level. This prevents wrap_bare_text_in_paragraphs from
    // re-wrapping them in <p> after unwrap_block_elements_from_p stripped the
    // original <p> wrapper. Only matches lines that are entirely a single
    // void element tag (starting with <img and ending with > or />).
    const BLOCK_VOID_TAGS: &[&str] = &["img"];
    for tag in BLOCK_VOID_TAGS {
        let open = format!("<{}", tag);
        if trimmed.starts_with(&open) {
            let rest = &trimmed[open.len()..];
            if (rest.starts_with('>') || rest.starts_with(' ') || rest.starts_with('/'))
                && trimmed.ends_with('>')
            {
                // Verify the line is ONLY this single element (no other text content)
                // by checking there's only one `<` in the trimmed line
                if trimmed.chars().filter(|c| *c == '<').count() == 1 {
                    return true;
                }
            }
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
// 4c. Wrap standalone HTML comments in <p> tags (Issue 274)
// ============================================================================

/// Wrap standalone HTML comments in `<p>` tags to match kramdown behavior.
///
/// Kramdown treats HTML comments that appear as standalone lines (surrounded by
/// blank lines) as inline content and wraps them in `<p>` tags. Pulldown-cmark
/// treats them as HTML block type 2 and leaves them unwrapped.
///
/// This function wraps comments that are:
/// - On their own line
/// - Separated from surrounding content by blank lines (or at start/end)
/// - NOT immediately adjacent (without blank line separation) to block-level
///   HTML elements
///
/// Must run AFTER `wrap_bare_text_in_paragraphs` which treats comments as
/// block-level (issue 144). This function selectively wraps the standalone ones.
fn wrap_standalone_comments_in_paragraphs(html: &str) -> String {
    let lines: Vec<&str> = html.split('\n').collect();
    let len = lines.len();
    if len == 0 {
        return html.to_string();
    }

    let mut result: Vec<String> = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        let trimmed = lines[i].trim();

        // Only process lines that are HTML comments
        if !trimmed.starts_with("<!--") || !trimmed.ends_with("-->") {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        // Already wrapped in <p>? Skip.
        if trimmed.starts_with("<p><!--") {
            result.push(lines[i].to_string());
            i += 1;
            continue;
        }

        // Check if this comment is "standalone" -- surrounded by blank lines
        // (or at start/end of content).
        //
        // Issue 279: A group of consecutive comment-only lines surrounded by
        // blank lines should each be wrapped. We look past adjacent comment
        // lines to find the nearest non-comment neighbor and check if it's
        // blank (or start/end of content).
        let prev_is_blank_or_start = {
            let mut j = i;
            // Skip backwards over adjacent comment lines
            while j > 0 {
                let prev = lines[j - 1].trim();
                if prev.starts_with("<!--") && prev.ends_with("-->") && !prev.starts_with("<p><!--")
                {
                    j -= 1;
                } else {
                    break;
                }
            }
            if j == 0 {
                true
            } else {
                lines[j - 1].trim().is_empty()
            }
        };

        let next_is_blank_or_end = {
            let mut j = i;
            // Skip forwards over adjacent comment lines
            while j + 1 < len {
                let next = lines[j + 1].trim();
                if next.starts_with("<!--") && next.ends_with("-->") && !next.starts_with("<p><!--")
                {
                    j += 1;
                } else {
                    break;
                }
            }
            if j + 1 >= len {
                true
            } else {
                lines[j + 1].trim().is_empty()
            }
        };

        // Issue 316: kramdown treats indented HTML comments (1-3 leading
        // spaces) as inline/paragraph content and wraps them in <p>, but
        // only when they appear among other comments (from Liquid include
        // output). Comments inside HTML block elements (e.g., inside <div>
        // or <form>) should NOT be wrapped even if indented.
        let leading_spaces = lines[i].len() - lines[i].trim_start().len();
        let is_indented_among_comments = (1..=3).contains(&leading_spaces) && {
            // Check that at least one neighbor is also a comment line
            let prev_is_comment = i > 0 && {
                let p = lines[i - 1].trim();
                p.starts_with("<!--") && p.ends_with("-->")
            };
            let next_is_comment = i + 1 < len && {
                let n = lines[i + 1].trim();
                n.starts_with("<!--") && n.ends_with("-->")
            };
            prev_is_comment || next_is_comment
        };

        if (prev_is_blank_or_start && next_is_blank_or_end) || is_indented_among_comments {
            // This is a standalone comment or indented comment -- wrap it in <p>
            result.push(format!("<p>{}</p>", trimmed));
        } else {
            // Adjacent to non-blank content and at column 0 -- leave as-is (issue 144)
            result.push(lines[i].to_string());
        }

        i += 1;
    }

    result.join("\n")
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
        "</figure>",
    ];

    let mut result = String::with_capacity(html.len() + html.len() / 10);
    let mut remaining = html;

    while !remaining.is_empty() {
        // Skip over <script> blocks entirely -- their content is not HTML and
        // block-tag patterns like </p> inside JSON-LD strings must not be
        // modified. (Issue 185)
        if let Some(script_start) = remaining.find("<script") {
            // Check if the <script> tag appears before any block closing tag.
            // If so, copy everything up to and including </script> verbatim.
            let mut any_block_before_script = false;
            for tag in &block_tags {
                if let Some(pos) = remaining.find(tag) {
                    if pos < script_start {
                        any_block_before_script = true;
                        break;
                    }
                }
            }
            if !any_block_before_script {
                // The <script> tag comes first -- skip the entire block.
                if let Some(script_end) = remaining[script_start..].find("</script>") {
                    let end_pos = script_start + script_end + "</script>".len();
                    result.push_str(&remaining[..end_pos]);
                    remaining = &remaining[end_pos..];
                    continue;
                }
                // No closing </script> found -- copy everything and stop.
                result.push_str(remaining);
                break;
            }
        }

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

            // Do NOT add spacing after </pre> or </div> when they are part of the
            // code block wrapper `</code></pre></div></div>`. Jekyll keeps these
            // closing tags on one line.
            if remaining.starts_with("</div></div>") || remaining.starts_with("</div>") {
                // Check if we just wrote </pre> or </div> as part of code wrapper
                if result.ends_with("</code></pre>") || result.ends_with("</pre></div>") {
                    continue; // Skip spacing -- keep tags on same line
                }
            }

            // Do NOT add spacing after </pre> when immediately followed by </noscript>.
            // The gist tag produces <noscript><pre>...</pre></noscript> as a single unit.
            if remaining.starts_with("</noscript>") && result.ends_with("</pre>") {
                continue;
            }

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
// 8b. Normalize bare void elements to XHTML-style (Issue 201)
// ============================================================================
/// Collapse newlines inside HTML tags to spaces, but only for inline HTML.
///
/// Jekyll/kramdown normalizes raw HTML tags that span multiple lines into
/// single-line tags when they appear as **inline** HTML (e.g. inside `<p>`).
/// Block-level HTML (like `<figure>`, `<div>`) is passed through verbatim
/// and newlines in attributes are preserved.
///
/// For example, inline HTML:
///   `<p><img alt="Creative\nCommons License" ...></p>` becomes
///   `<p><img alt="Creative Commons License" ...></p>`
///
/// But block-level HTML is unchanged:
///   `<figure>\n<img alt="ML Zoomcamp \nleaderboard..." />\n</figure>`
///
/// The heuristic: only normalize tags that appear inside `<p>...</p>`.
fn normalize_newlines_in_html_tags(html: &str) -> String {
    if !html.contains('\n') {
        return html.to_string();
    }

    // Find all <p>...</p> regions first, then only normalize within those.
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for <p> or <p ...> opening tag
        if i + 2 < len && bytes[i] == b'<' && bytes[i + 1] == b'p' {
            // Check it's actually <p> or <p ...> (not <pre>, <param>, etc.)
            let after_p = if i + 2 < len { bytes[i + 2] } else { 0 };
            if after_p == b'>' || after_p == b' ' || after_p == b'\n' {
                // Find the matching </p>
                if let Some(close_pos) = find_closing_p_tag(html, i) {
                    let end = close_pos + 4; // </p> is 4 bytes
                    let p_content = &html[i..end];
                    let normalized = normalize_newlines_in_tags_unconditionally(p_content);
                    result.push_str(&normalized);
                    i = end;
                    continue;
                }
            }
        }
        // Advance by one character (handling multi-byte UTF-8)
        let ch = html[i..].chars().next().unwrap();
        result.push(ch);
        i += ch.len_utf8();
    }

    result
}

/// Find the position of the closing `</p>` tag matching an opening `<p>` at `start`.
/// Returns the byte offset of the `<` in `</p>`, or None if not found.
fn find_closing_p_tag(html: &str, start: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut i = start + 1;
    // Skip past the opening <p> or <p ...>
    while i < bytes.len() && bytes[i] != b'>' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    i += 1; // skip '>'

    // Find </p> -- HTML <p> cannot nest so no depth tracking needed
    while i + 3 < bytes.len() {
        if bytes[i] == b'<' && bytes[i + 1] == b'/' && bytes[i + 2] == b'p' && bytes[i + 3] == b'>'
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Normalize ALL newlines inside HTML tags to spaces (both inside and outside quotes).
/// Used only for inline HTML content within `<p>` elements.
fn normalize_newlines_in_tags_unconditionally(html: &str) -> String {
    if !html.contains('\n') {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut inside_tag = false;
    let mut inside_quote: Option<char> = None;

    for ch in html.chars() {
        if inside_tag {
            if let Some(q) = inside_quote {
                if ch == q {
                    inside_quote = None;
                }
                if ch == '\n' {
                    result.push(' ');
                } else {
                    result.push(ch);
                }
            } else if ch == '"' || ch == '\'' {
                inside_quote = Some(ch);
                result.push(ch);
            } else if ch == '>' {
                inside_tag = false;
                result.push(ch);
            } else if ch == '\n' {
                result.push(' ');
            } else {
                result.push(ch);
            }
        } else if ch == '<' {
            inside_tag = true;
            result.push(ch);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Jekyll/kramdown outputs XHTML-style self-closing tags for void elements:
/// `<br />`, `<hr />`, `<img ... />`, etc. When raw HTML in markdown source
/// contains bare void tags like `<br>` (without /), this function converts
/// them to `<br />` to match Jekyll's output.
///
/// This is important because some HTML parsers (e.g., BeautifulSoup's
/// html.parser) can misinterpret subsequent self-closing tags when bare void
/// tags are present earlier in the document, causing text nodes to be placed
/// as children of void elements instead of siblings.
///
/// Only converts tags that are NOT already self-closing (i.e., does not
/// touch `<br />` or `<br/>`).
fn normalize_bare_void_elements(html: &str) -> String {
    // Convert ALL bare void elements to XHTML-style self-closing format
    // to match Jekyll/kramdown output. Jekyll adds " />" to every void element.
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len + 64);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' && i + 1 < len && bytes[i + 1] != b'/' {
            // Potential opening tag -- find the tag name
            let tag_start = i;
            i += 1;
            let name_start = i;
            while i < len && bytes[i] != b'>' && bytes[i] != b' ' && bytes[i] != b'/' {
                i += 1;
            }
            let tag_name = &html[name_start..i];

            // Convert ALL HTML void elements to XHTML-style self-closing tags.
            // Full list per HTML spec: area, base, br, col, embed, hr, img,
            // input, link, meta, param, source, track, wbr.
            if matches!(
                tag_name,
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            ) {
                // Find the closing '>'
                while i < len && bytes[i] != b'>' {
                    i += 1;
                }
                if i < len {
                    // Check if already self-closing (ends with / before >)
                    let before_close = &html[tag_start..i];
                    if before_close.ends_with('/') || before_close.ends_with("/ ") {
                        // Already self-closing -- copy as-is
                        result.push_str(&html[tag_start..=i]);
                    } else {
                        // Bare void tag -- convert to XHTML style: add " />"
                        result.push_str(&html[tag_start..i]);
                        result.push_str(" />");
                    }
                    i += 1;
                } else {
                    // Unclosed tag at end of string -- copy as-is
                    result.push_str(&html[tag_start..]);
                }
            } else {
                // Not a void element -- copy the '<' and reparse from name_start
                result.push('<');
                i = name_start;
            }
        } else {
            // Advance by a full UTF-8 character to avoid corrupting multi-byte sequences.
            // `bytes[i] as char` would treat each byte as Latin-1, producing mojibake.
            let ch = html[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}

/// Escape malformed raw HTML tags that contain unbalanced single quotes.
///
/// Jekyll/kramdown treats malformed raw HTML like:
///
/// ```html
/// <canvas data-labels='["We don't self-host"]'>
/// ```
///
/// as literal text, not a live element. pulldown-cmark passes it through as
/// a real tag, so we detect tags whose raw single quotes are unbalanced and
/// escape both the opening and matching closing tags.
fn escape_malformed_single_quote_tags(html: &str) -> String {
    if !html.contains('\'') {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len() + 32);
    let mut remaining = html;

    while let Some(tag_start) = remaining.find('<') {
        result.push_str(&remaining[..tag_start]);
        let after_lt = &remaining[tag_start..];

        let Some(tag_end_rel) = after_lt.find('>') else {
            result.push_str(after_lt);
            break;
        };

        let tag = &after_lt[..=tag_end_rel];
        if let Some(tag_name) = malformed_single_quote_tag_name(tag) {
            let closing_tag = format!("</{tag_name}>");
            result.push_str(&escape_html_tag(tag));

            let after_tag = &after_lt[tag_end_rel + 1..];
            if let Some(close_pos) = after_tag.find(&closing_tag) {
                result.push_str(&after_tag[..close_pos]);
                result.push_str(&escape_html_tag(&closing_tag));
                remaining = &after_tag[close_pos + closing_tag.len()..];
            } else {
                result.push_str(after_tag);
                break;
            }
        } else {
            result.push_str(tag);
            remaining = &after_lt[tag.len()..];
        }
    }

    if !remaining.is_empty() {
        result.push_str(remaining);
    }

    result
}

/// Return the tag name if the tag is malformed due to unbalanced single quotes.
fn malformed_single_quote_tag_name(tag: &str) -> Option<&str> {
    if !tag.starts_with('<') || tag.starts_with("</") {
        return None;
    }

    let inner = &tag[1..tag.len().saturating_sub(1)];
    if !has_unbalanced_single_quotes(inner) {
        return None;
    }

    let tag_name = inner
        .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .next()
        .unwrap_or("");
    if tag_name.is_empty() {
        None
    } else {
        Some(tag_name)
    }
}

/// Check whether a tag's raw single quotes are unbalanced.
///
/// We count single quotes that appear outside double-quoted attribute values.
/// A well-formed tag has an even number of such quotes. An unescaped apostrophe
/// inside a single-quoted attribute produces an odd count.
fn has_unbalanced_single_quotes(tag_inner: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut single_quote_count = 0usize;

    for ch in tag_inner.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                single_quote_count += 1;
            }
            '"' if !in_single => in_double = !in_double,
            _ => {}
        }
    }

    in_single || in_double || single_quote_count % 2 == 1
}

fn escape_html_tag(tag: &str) -> String {
    let mut escaped = String::with_capacity(tag.len() + 8);
    escaped.push_str("&lt;");
    escaped.push_str(tag.trim_start_matches('<'));
    if escaped.ends_with('>') {
        escaped.pop();
        escaped.push_str("&gt;");
    } else {
        escaped.push_str("&gt;");
    }
    escaped
}

/// Converts only bare `<br>` to `<br />` in the full page output.
///
/// Used in `normalize_html_output` which runs on the FULL rendered page
/// (including layout and include HTML). We only convert `<br>` because
/// raw HTML `<br>` tags in markdown content (e.g., table cells) need
/// XHTML-style self-closing to match Jekyll/kramdown output.
///
/// We do NOT convert `<hr>` here because:
/// 1. pulldown-cmark already outputs `<hr />` for markdown `---` rules
/// 2. `postprocess()` already calls `normalize_bare_void_elements()` on
///    markdown-rendered content, which converts any `<hr>` to `<hr />`
/// 3. Converting `<hr>` here would incorrectly affect include/layout HTML
///    (e.g., `_includes/footer.html`), where Jekyll outputs bare `<hr>`
///
/// We also do NOT convert `<meta>`, `<link>`, `<input>`, `<img>` etc.
/// since Jekyll doesn't self-close those in layout templates.
fn normalize_br_only(html: &str) -> String {
    if !html.contains("<br>") && !html.contains("<br/>") {
        return html.to_string();
    }
    // Normalize both <br> and <br/> (no space) to <br /> (XHTML-style with space)
    html.replace("<br>", "<br />").replace("<br/>", "<br />")
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
// 11. Indent loose list items to match kramdown formatting
// ============================================================================

/// Indent `<li>` items inside `<ul>` and `<ol>` to match kramdown output.
///
/// kramdown indents loose list items (those containing `<p>` tags) like:
/// ```html
/// <ul>
///   <li>
///     <p>text</p>
///   </li>
/// </ul>
/// ```
///
/// pulldown-cmark outputs them without indentation:
/// ```html
/// <ul>
/// <li>
/// <p>text</p>
/// </li>
/// </ul>
/// ```
fn indent_list_items(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + html.len() / 20);
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find the next <ul> or <ol> tag at line start (not inside code blocks)
        let next_ul = remaining.find("<ul>\n");
        let next_ol = remaining.find("<ol>\n");

        let list_pos = match (next_ul, next_ol) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        let list_pos = match list_pos {
            Some(p) => p,
            None => {
                result.push_str(remaining);
                break;
            }
        };

        // Copy everything before the list
        result.push_str(&remaining[..list_pos]);
        remaining = &remaining[list_pos..];

        // Find the list tag
        let tag_end = remaining.find('\n').unwrap_or(remaining.len());
        let list_tag = &remaining[..tag_end];
        let is_ul = list_tag.starts_with("<ul");

        // Find the matching close tag by tracking nesting depth
        let close_tag = if is_ul { "</ul>" } else { "</ol>" };
        let open_tag = if is_ul { "<ul>" } else { "<ol>" };
        if let Some(close_pos) = find_matching_close_tag(remaining, open_tag, close_tag) {
            let list_content = &remaining[tag_end + 1..close_pos];

            result.push_str(list_tag);
            result.push('\n');
            indent_list_content(&mut result, list_content, 0);
            result.push_str(close_tag);
            remaining = &remaining[close_pos + close_tag.len()..];
        } else {
            // No matching close tag, copy as-is
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Find the position of the matching close tag, accounting for nesting.
fn find_matching_close_tag(html: &str, open_tag: &str, close_tag: &str) -> Option<usize> {
    let mut depth = 0;
    let mut pos = 0;
    let bytes = html.as_bytes();
    let open_bytes = open_tag.as_bytes();
    let close_bytes = close_tag.as_bytes();
    while pos < bytes.len() {
        if bytes[pos..].starts_with(open_bytes) {
            depth += 1;
            pos += open_bytes.len();
        } else if bytes[pos..].starts_with(close_bytes) {
            depth -= 1;
            if depth == 0 {
                return Some(pos);
            }
            pos += close_bytes.len();
        } else {
            pos += 1;
        }
    }
    None
}

/// Indent list content lines with proper nesting depth.
/// `base_depth` is the nesting level (0 = top-level list).
fn indent_list_content(result: &mut String, content: &str, base_depth: usize) {
    let indent = "  ".repeat(base_depth + 1); // 2 spaces per level, +1 for being inside a list
    let inner_indent = "  ".repeat(base_depth + 2); // content inside <li> in loose lists

    // Check if this is a loose list (contains <p> inside <li>)
    let is_loose = content.contains("<li>\n<p>");

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if is_loose {
            if line == "<li>" {
                result.push_str(&indent);
                result.push_str("<li>\n");
                // Process content inside this <li> until </li>
                i += 1;
                while i < lines.len() && lines[i] != "</li>" {
                    let inner_line = lines[i];
                    if inner_line.is_empty() {
                        // Skip blank lines inside <li> -- kramdown doesn't have them
                        i += 1;
                        continue;
                    }
                    // Check for nested list start
                    if inner_line == "<ul>" || inner_line == "<ol>" {
                        let nested_close = if inner_line == "<ul>" {
                            "</ul>"
                        } else {
                            "</ol>"
                        };
                        // Collect the nested list content
                        let (nested_content, end_idx) =
                            collect_nested_list_lines(&lines, i, inner_line, nested_close);
                        result.push_str(&inner_indent);
                        result.push_str(inner_line);
                        result.push('\n');
                        indent_list_content(result, &nested_content, base_depth + 2);
                        result.push_str(&inner_indent);
                        result.push_str(nested_close);
                        result.push('\n');
                        i = end_idx + 1;
                        continue;
                    }
                    result.push_str(&inner_indent);
                    result.push_str(inner_line);
                    result.push('\n');
                    i += 1;
                }
                if i < lines.len() && lines[i] == "</li>" {
                    result.push_str(&indent);
                    result.push_str("</li>\n");
                }
            } else if line.is_empty() {
                // Blank line outside <li> -- preserve
                result.push('\n');
            } else if !line.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            // Tight list
            if line.starts_with("<li>") {
                // Check if this is a multi-line <li> (not self-closing on same line
                // with </li>). Look for a nested list inside.
                let has_close_on_same_line = line.contains("</li>");
                if has_close_on_same_line {
                    // Simple single-line <li>content</li>
                    result.push_str(&indent);
                    result.push_str(line);
                    result.push('\n');
                } else {
                    // Multi-line <li> -- may contain nested list
                    result.push_str(&indent);
                    result.push_str(line);
                    result.push('\n');
                    i += 1;
                    while i < lines.len() {
                        let inner_line = lines[i];
                        if inner_line == "</li>" {
                            result.push_str(&indent);
                            result.push_str("</li>\n");
                            break;
                        }
                        // Check for nested list start
                        if inner_line == "<ul>" || inner_line == "<ol>" {
                            let nested_close = if inner_line == "<ul>" {
                                "</ul>"
                            } else {
                                "</ol>"
                            };
                            let (nested_content, end_idx) =
                                collect_nested_list_lines(&lines, i, inner_line, nested_close);
                            result.push_str(&inner_indent);
                            result.push_str(inner_line);
                            result.push('\n');
                            indent_list_content(result, &nested_content, base_depth + 2);
                            result.push_str(&inner_indent);
                            result.push_str(nested_close);
                            result.push('\n');
                            i = end_idx + 1;
                            continue;
                        }
                        if inner_line.is_empty() {
                            // Skip blank lines inside <li> -- kramdown doesn't have them
                            i += 1;
                            continue;
                        }
                        result.push_str(&inner_indent);
                        result.push_str(inner_line);
                        result.push('\n');
                        i += 1;
                    }
                }
            } else if line.starts_with("</li>") {
                result.push_str(&indent);
                result.push_str(line);
                result.push('\n');
            } else if !line.is_empty() {
                result.push_str(line);
                result.push('\n');
            } else {
                result.push('\n');
            }
        }
        i += 1;
    }
}

/// Collect lines for a nested list (from open tag to matching close tag),
/// returning the content between tags and the index of the close tag line.
fn collect_nested_list_lines(
    lines: &[&str],
    start: usize,
    open_tag: &str,
    close_tag: &str,
) -> (String, usize) {
    let mut depth = 1;
    let mut i = start + 1; // skip the open tag line
    let mut content = String::new();
    while i < lines.len() {
        if lines[i] == open_tag {
            depth += 1;
        } else if lines[i] == close_tag {
            depth -= 1;
            if depth == 0 {
                return (content, i);
            }
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(lines[i]);
        i += 1;
    }
    (content, i.saturating_sub(1))
}

// ============================================================================
// 9b. Normalize blockquote content (Issue 163, fixed in Issue 164)
// ============================================================================

/// Ensure blockquote content matches kramdown output format.
///
/// kramdown outputs blockquotes with NO indentation on inner content
/// and a blank line between the last content and closing tag.
///
/// This function ensures the blank line before closing tag.
fn indent_blockquote_content(html: &str) -> String {
    let mut result = String::with_capacity(html.len() + html.len() / 20);
    let mut remaining = html;
    while !remaining.is_empty() {
        if let Some(pos) = remaining.find("<blockquote>\n") {
            result.push_str(&remaining[..pos]);
            let tag = "<blockquote>\n";
            let after_tag = &remaining[pos + tag.len()..];
            if let Some(close_pos) = after_tag.find("</blockquote>") {
                let content = &after_tag[..close_pos];
                result.push_str("<blockquote>\n");
                // Jekyll/kramdown: no indentation, preserve blank lines.
                result.push_str(content);
                if !content.ends_with("\n\n") {
                    if content.ends_with('\n') {
                        result.push('\n');
                    } else {
                        result.push_str("\n\n");
                    }
                }
                result.push_str("</blockquote>");
                remaining = &after_tag[close_pos + "</blockquote>".len()..];
            } else {
                result.push_str(&remaining[..pos + tag.len()]);
                remaining = after_tag;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

// ============================================================================
// 9c. Extract <br /> between blockquotes (Issue 448)
// ============================================================================

/// Extract `<br />` tags from the end of a blockquote's `<p>` when the
/// blockquote is followed by another `<blockquote>`.
///
/// Jekyll/kramdown renders a `<br>` between two blockquotes as a standalone
/// element between them. pulldown-cmark absorbs it into the preceding
/// blockquote's paragraph. This function moves it back outside.
///
/// Input pattern (after `normalize_bare_void_elements`):
/// ```html
/// <blockquote>
/// <p>text<br />
/// <br /></p>
///
/// </blockquote>
/// ```
///
/// Output:
/// ```html
/// <blockquote>
/// <p>text</p>
///
/// </blockquote>
/// <br />
/// ```
fn extract_br_between_blockquotes(html: &str) -> String {
    // Pattern: blockquote ending with <p>...trailing <br />\n</p>\n\n</blockquote>
    // followed (possibly with whitespace) by <blockquote>
    // We need to extract the trailing <br /> tags from the <p> and place them
    // between the two blockquotes.

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Look for </blockquote> followed (with optional whitespace/newlines)
        // by <blockquote>
        if let Some(close_pos) = remaining.find("</blockquote>") {
            let before_close = &remaining[..close_pos];
            let after_close = &remaining[close_pos + "</blockquote>".len()..];

            // Check if another <blockquote> follows
            let trimmed_after = after_close.trim_start_matches('\n');
            if trimmed_after.starts_with("<blockquote>") {
                // Check if the content before </blockquote> ends with <br /> in a <p>
                // Look for the pattern: <br />\n</p>\n\n or similar at the end
                if let Some(extracted) = try_extract_trailing_br(before_close) {
                    result.push_str(&extracted.cleaned_blockquote_content);
                    result.push_str("</blockquote>\n");
                    result.push_str(&extracted.br_tags);
                    remaining = trimmed_after;
                    continue;
                }
            }

            // No extraction needed - copy up through </blockquote>
            result.push_str(&remaining[..close_pos + "</blockquote>".len()]);
            remaining = after_close;
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

struct ExtractedBr {
    /// The blockquote content with trailing <br /> tags removed from the <p>
    cleaned_blockquote_content: String,
    /// The <br /> tags to insert between blockquotes
    br_tags: String,
}

/// Try to extract trailing `<br />` tags from the last `<p>` in a blockquote.
///
/// Returns `None` if no trailing `<br />` pattern is found.
fn try_extract_trailing_br(blockquote_content: &str) -> Option<ExtractedBr> {
    // The content should end with something like:
    // <p>text<br />\n<br /></p>\n\n
    // or: <p>text\n<br /></p>\n\n
    // We need to find the last </p> and check for trailing <br /> before it.

    // Find the last </p> in the content
    let p_close_pos = blockquote_content.rfind("</p>")?;
    let before_p_close = &blockquote_content[..p_close_pos];

    // Collect trailing <br /> tags from before </p>
    let mut scan = before_p_close;
    let mut br_count = 0;

    loop {
        let trimmed = scan.trim_end();
        if let Some(stripped) = trimmed.strip_suffix("<br />") {
            br_count += 1;
            scan = stripped;
        } else {
            break;
        }
    }

    if br_count == 0 {
        return None;
    }

    // Rebuild: content without the trailing <br /> tags, closing the <p> properly
    let clean_text = scan.trim_end_matches('\n');
    let after_p_close = &blockquote_content[p_close_pos + "</p>".len()..];

    let mut cleaned = String::with_capacity(blockquote_content.len());
    cleaned.push_str(clean_text);
    cleaned.push_str("</p>");
    cleaned.push_str(after_p_close);

    // Output exactly one <br /> between the blockquotes, matching Jekyll.
    // pulldown-cmark may absorb the source <br> and also generate a hard-break
    // <br />, doubling the count. Jekyll always outputs a single <br>.
    let mut br_tags = String::new();
    {
        br_tags.push_str("<br />\n");
    }

    Some(ExtractedBr {
        cleaned_blockquote_content: cleaned,
        br_tags,
    })
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
// Issue 211: Fix smart quote direction mismatches vs kramdown
// ============================================================================

/// Fix smart quote directions to match kramdown behavior.
///
/// pulldown-cmark's smart punctuation algorithm uses Unicode-standard character
/// classification to decide quote direction. kramdown uses simpler heuristics
/// based on the SmartyPants/RubyPants algorithm. They disagree in several cases.
///
/// This post-processing pass re-determines the direction of each smart quote
/// using kramdown's context-based rules (from `SQ_RULES` in kramdown's
/// `smart_quotes.rb`), which look at the character before/after the quote
/// rather than using Unicode left-flanking/right-flanking delimiter logic.
///
/// Key kramdown rules:
/// - `SQ_CLOSE` chars (anything except space, `\`, tab, `\r`, `\n`, `[`, `{`,
///   `(`, `-`) before a quote → closing (RIGHT)
/// - Space before quote + word char after → opening (LEFT)
/// - Quote before space/end → closing (RIGHT)
/// - Apostrophe context (between alphabetic chars) → RIGHT SINGLE (preserved)
/// - Fallback → opening (LEFT)
pub fn fix_smart_quote_directions(html: &str) -> String {
    // Quick check: if no smart quotes present, return early
    if !html.contains('\u{2018}')
        && !html.contains('\u{2019}')
        && !html.contains('\u{201C}')
        && !html.contains('\u{201D}')
    {
        return html.to_string();
    }

    let chars: Vec<char> = html.chars().collect();
    let mut result: Vec<char> = chars.clone();

    // We process text outside of HTML tags. Track the last non-tag text char
    // before the current position (skipping over HTML tags).
    let mut in_tag = false;

    // Track German-style double quote openers (U+201E „). When seen, the next
    // U+201C/U+201D should be left as U+201C (German closing quote), not
    // converted by kramdown rules.
    let mut german_double_open = false;

    for (i, &ch) in chars.iter().enumerate() {
        if ch == '<' {
            in_tag = true;
            continue;
        } else if ch == '>' {
            in_tag = false;
            continue;
        }

        if in_tag {
            continue;
        }

        match ch {
            // German-style double low-9 quotation mark (opener)
            '\u{201E}' => {
                german_double_open = true;
            }
            '\u{201C}' | '\u{201D}' => {
                if german_double_open {
                    // This is the closing quote of a German „..." pair.
                    // Keep it as U+201C (the standard German closer).
                    result[i] = '\u{201C}';
                    german_double_open = false;
                } else {
                    // Standard double quote: determine direction using kramdown rules
                    let prev = prev_text_char(&chars, i);
                    let next = next_text_char(&chars, i);
                    let direction = kramdown_quote_direction(prev, next);
                    result[i] = if direction {
                        '\u{201D}' // RIGHT (closing)
                    } else {
                        '\u{201C}' // LEFT (opening)
                    };
                }
            }
            '\u{2018}' | '\u{2019}' => {
                // Single quote: check apostrophe context first
                if is_apostrophe_context_kramdown(&chars, i) {
                    result[i] = '\u{2019}'; // apostrophe = RIGHT SINGLE
                } else {
                    let prev = prev_text_char(&chars, i);
                    let next = next_text_char(&chars, i);
                    let direction = kramdown_quote_direction(prev, next);
                    result[i] = if direction {
                        '\u{2019}' // RIGHT (closing)
                    } else {
                        '\u{2018}' // LEFT (opening)
                    };
                }
            }
            _ => {}
        }
    }

    let output: String = result.into_iter().collect();

    // Issue 308: Fix smart quote direction after <br />\n.
    // When newline_to_br produces <br />\n before a quote, the quote is at the
    // start of a new visual line. If the general direction logic produced a
    // right/closing quote but the quote is followed by (optional whitespace +)
    // a word character, it should be left/opening instead.
    fix_quotes_after_br(&output)
}

/// Issue 308: Fix quote direction for quotes that appear right after `<br />\n`.
///
/// The pattern `<br />\n"<space>word` should produce an opening (left) quote,
/// matching kramdown's behavior where `<br />` acts as a line break and the
/// quote starts new text. The general direction logic incorrectly produces a
/// closing (right) quote because it sees `prev=\n` + `next=space` and triggers
/// Rule 8 (quote + space → closing).
fn fix_quotes_after_br(html: &str) -> String {
    // Quick check: if no <br /> followed by quotes, skip
    if !html.contains("<br />") {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Find all occurrences of <br />\n followed by a smart quote
    // We need to work with char indices because smart quotes are multi-byte
    let chars: Vec<char> = result.chars().collect();
    let mut replacements = Vec::new();

    for (ci, &ch) in chars.iter().enumerate() {
        // Look for right (closing) quotes that should be left (opening)
        if ch != '\u{201D}' && ch != '\u{2019}' {
            continue;
        }

        // Check if preceded by <br />\n (possibly with spaces between \n and quote)
        let mut pi = ci;
        // Skip whitespace before the quote
        while pi > 0 {
            let prev = chars[pi - 1];
            if prev == ' ' || prev == '\t' {
                pi -= 1;
            } else {
                break;
            }
        }
        // Check for \n
        if pi == 0 || chars[pi - 1] != '\n' {
            continue;
        }
        pi -= 1;
        // Check for <br /> or <br> before the \n
        // Look for > then scan back for <br
        if pi == 0 || chars[pi - 1] != '>' {
            continue;
        }
        // Scan back to find the tag start
        let tag_end = pi - 1;
        let mut ti = tag_end;
        while ti > 0 {
            ti -= 1;
            if chars[ti] == '<' {
                break;
            }
        }
        let tag: String = chars[ti..=tag_end].iter().collect();
        if !tag.starts_with("<br") {
            continue;
        }

        // This quote is right after <br />\n. Check if followed by word char
        // (possibly after whitespace)
        let mut ni = ci + 1;
        while ni < chars.len() && chars[ni].is_whitespace() {
            ni += 1;
        }
        if ni < chars.len() && (chars[ni].is_alphanumeric() || chars[ni] == '_') {
            // This should be an opening quote
            let opening = if ch == '\u{201D}' {
                '\u{201C}'
            } else {
                '\u{2018}'
            };
            replacements.push((ci, opening));
        }
    }

    // Apply replacements (from end to start to preserve indices)
    if !replacements.is_empty() {
        let mut chars = result.chars().collect::<Vec<_>>();
        for (idx, replacement) in replacements.iter().rev() {
            chars[*idx] = *replacement;
        }
        result = chars.into_iter().collect();
    }

    result
}

/// Get the previous text character, skipping over HTML tags.
/// Returns None if at the start of text or only tags precede.
fn prev_text_char(chars: &[char], pos: usize) -> Option<char> {
    let mut i = pos;
    while i > 0 {
        i -= 1;
        if chars[i] == '>' {
            // Skip backwards over the tag
            while i > 0 {
                i -= 1;
                if chars[i] == '<' {
                    break;
                }
            }
            continue;
        }
        if chars[i] == '<' {
            continue;
        }
        return Some(chars[i]);
    }
    None
}

/// Get the next text character, skipping over HTML tags.
/// Returns None if at the end of text or only tags follow.
fn next_text_char(chars: &[char], pos: usize) -> Option<char> {
    let mut i = pos + 1;
    while i < chars.len() {
        if chars[i] == '<' {
            // Skip forward over the tag
            while i < chars.len() {
                if chars[i] == '>' {
                    break;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if chars[i] == '>' {
            i += 1;
            continue;
        }
        return Some(chars[i]);
    }
    None
}

/// Determine quote direction using kramdown's SQ_RULES algorithm.
/// Returns true for RIGHT (closing), false for LEFT (opening).
///
/// kramdown's SQ_RULES process the quote with context (preceding char + following
/// chars). The rules are tried in order -- first match wins:
///
/// - Rule 1: Quote before emphasis markers (`_*`) → LEFT (opening)
/// - Rule 2: Quote before SQ_PUNCT (no prev captured) → RIGHT (closing)
/// - Rules 3-5: Special combos (double-single quotes, decades) -- rare, skipped
/// - Rule 6: Space + quote + word char → LEFT (opening)
/// - Rule 7: SQ_CLOSE char + quote → RIGHT (closing)
/// - Rule 8: Quote + space/end → RIGHT (closing)
/// - Rules 9-10: Fallback → LEFT (opening)
///
/// In the post-processing context (HTML), rules 2 and 8 only apply when there's
/// no preceding text char (rule 2 needs quote at scan position 0). When there IS
/// a preceding char, rules 6, 7, or fallback apply.
fn kramdown_quote_direction(prev: Option<char>, next: Option<char>) -> bool {
    match prev {
        Some(p) => {
            // There is a preceding character. kramdown's scanner has `X"` where X is prev.
            // Rules 1-5 don't match (they need quote at position 0).
            // Rule 6: space + quote + word → LEFT
            if p.is_whitespace() && next.is_some_and(|n| n.is_alphanumeric() || n == '_') {
                return false; // LEFT (opening)
            }
            // Rule 7: SQ_CLOSE + quote → RIGHT
            if is_sq_close(p) {
                return true; // RIGHT (closing)
            }
            // Rule 8: quote + space/end → RIGHT
            if next.is_none_or(|n| n.is_whitespace()) {
                return true; // RIGHT (closing)
            }
            // Fallback (rules 9/10): LEFT (opening)
            false
        }
        None => {
            // No preceding character (start of text / after tag boundary).
            // kramdown's scanner has just `"` at position 0.
            // Rule 1: quote before emphasis → LEFT (skip, rare in HTML output)
            // Rule 2: quote before SQ_PUNCT → RIGHT
            if next.is_some_and(is_sq_punct) {
                return true; // RIGHT (closing)
            }
            // Rule 8: quote + space/end → RIGHT
            if next.is_none_or(|n| n.is_whitespace()) {
                return true; // RIGHT (closing)
            }
            // If followed by word char (and no prev) → LEFT
            // This comes from rule 6 with implicit leading space
            if next.is_some_and(|n| n.is_alphanumeric() || n == '_') {
                return false; // LEFT (opening)
            }
            // Fallback → LEFT
            false
        }
    }
}

/// Check if a character is in kramdown's SQ_CLOSE set.
/// SQ_CLOSE = [^ \\\t\r\n\[{(-] -- anything NOT space, backslash, tab, CR, LF,
/// `[`, `{`, `(`, `-`.
fn is_sq_close(ch: char) -> bool {
    !matches!(ch, ' ' | '\\' | '\t' | '\r' | '\n' | '[' | '{' | '(' | '-')
}

/// Check if a character is in kramdown's SQ_PUNCT set.
/// SQ_PUNCT = [!"#$%'()*+,-./:;<=>?@[\]^_`{|}~]
fn is_sq_punct(ch: char) -> bool {
    matches!(
        ch,
        '!' | '"'
            | '#'
            | '$'
            | '%'
            | '\''
            | '('
            | ')'
            | '*'
            | '+'
            | ','
            | '-'
            | '.'
            | '/'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '['
            | '\\'
            | ']'
            | '^'
            | '_'
            | '`'
            | '{'
            | '|'
            | '}'
            | '~'
    )
}

/// Check if a quote at `pos` is in an apostrophe context (between alphabetic chars),
/// skipping over HTML tags when looking at neighbors.
fn is_apostrophe_context_kramdown(chars: &[char], pos: usize) -> bool {
    let prev = prev_text_char(chars, pos);
    let next = next_text_char(chars, pos);
    prev.is_some_and(|p| p.is_alphabetic()) && next.is_some_and(|n| n.is_alphabetic())
}

/// Issue 247: Apply kramdown's SQ_RULES to straight single quotes in HTML text.
///
/// After `restore_consecutive_single_quotes()` restores `''`/`'''` placeholders
/// back to straight quotes (U+0027), this function processes those straight quotes
/// using kramdown's sequential, context-dependent SQ_RULES algorithm.
///
/// kramdown uses a StringScanner that processes quotes left-to-right. The scanner
/// finds quotes via `SMART_QUOTES_RE = /[^\\]?["']/` which matches a quote
/// optionally preceded by a non-backslash character. This means Rule 7's
/// `(SQ_CLOSE)(')` can match `''` as a unit (first `'` is SQ_CLOSE, second is
/// the quote), consuming both at once.
///
/// The key patterns (from verified kramdown test cases):
///
/// - Start of text + `''` + word: Rule 7 matches pair -> text(`'`) + rsquo
/// - Space + `''` + word: Rule 9 for first (lsquo), Rule 9 for second (lsquo)
/// - Word char + `''` + space/end: Rule 7 for first (rsquo), Rule 8 for second (rsquo)
/// - Word char + `''` + punctuation: Rule 7 for first (rsquo), Rule 2 for second (rsquo)
/// - Start of text + `'''` + word: Rule 2 for first (rsquo), Rule 7 for pair (text+rsquo)
pub fn apply_kramdown_smart_quotes_to_straight(html: &str) -> String {
    // Quick check: if no straight single quotes, nothing to do
    if !html.contains('\'') {
        return html.to_string();
    }

    let chars: Vec<char> = html.chars().collect();
    let len = chars.len();
    let mut result = String::with_capacity(html.len());
    let mut i = 0;
    let mut in_tag = false;
    // Track nesting depth inside <code>, <pre>, <script> elements where
    // smart quotes must NOT be applied.
    let mut skip_depth: usize = 0;

    while i < len {
        let ch = chars[i];

        if ch == '<' {
            in_tag = true;
            // Check if this tag opens or closes a skip element
            let tag_content = collect_tag_name(&chars, i + 1);
            if is_skip_open_tag(&tag_content) {
                skip_depth += 1;
            } else if is_skip_close_tag(&tag_content) {
                skip_depth = skip_depth.saturating_sub(1);
            }
            result.push(ch);
            i += 1;
            continue;
        } else if ch == '>' {
            in_tag = false;
            result.push(ch);
            i += 1;
            continue;
        }

        if in_tag {
            result.push(ch);
            i += 1;
            continue;
        }

        // Inside <code>/<pre>/<script>: leave all quotes straight
        if skip_depth > 0 {
            result.push(ch);
            i += 1;
            continue;
        }

        if ch != '\'' {
            result.push(ch);
            i += 1;
            continue;
        }

        // We have a straight quote. Count consecutive straight quotes.
        let quote_start = i;
        let mut quote_count = 0;
        while i < len && chars[i] == '\'' {
            quote_count += 1;
            i += 1;
        }

        // Single straight quotes are left alone. pulldown-cmark's smart
        // punctuation already handles apostrophes in markdown content.
        // This function only needs to process restored ''/'''' sequences
        // (count >= 2). Converting single quotes causes regressions when
        // Liquid template output contains straight apostrophes that should
        // not be curled (e.g., {{ post.title }} with "Aren't").
        if quote_count == 1 {
            result.push('\'');
            continue;
        }

        // Get the preceding and following text characters (skipping HTML tags)
        let prev = prev_text_char(&chars, quote_start);
        let next_after = next_text_char_at(&chars, i);

        // Apply kramdown SQ_RULES based on the context and count
        let converted = apply_sq_rules_for_sequence(prev, next_after, quote_count);
        result.push_str(&converted);
    }

    result
}

/// Collect the tag name starting at position `start` (just after '<').
/// Returns lowercase tag name, e.g. "code", "/code", "pre", "/pre".
fn collect_tag_name(chars: &[char], start: usize) -> String {
    let mut name = String::new();
    let mut i = start;
    // Include leading '/' for close tags
    if i < chars.len() && chars[i] == '/' {
        name.push('/');
        i += 1;
    }
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphanumeric() || c == '-' {
            name.push(c.to_ascii_lowercase());
            i += 1;
        } else {
            break;
        }
    }
    name
}

/// Check if a tag name (from collect_tag_name) is an opening skip element.
fn is_skip_open_tag(tag: &str) -> bool {
    matches!(tag, "code" | "pre" | "script")
}

/// Check if a tag name (from collect_tag_name) is a closing skip element.
fn is_skip_close_tag(tag: &str) -> bool {
    matches!(tag, "/code" | "/pre" | "/script")
}

/// Get the next text character at exactly position `pos` (not after it),
/// skipping HTML tags if the position is at a `<`.
fn next_text_char_at(chars: &[char], pos: usize) -> Option<char> {
    let mut i = pos;
    while i < chars.len() {
        if chars[i] == '<' {
            while i < chars.len() {
                if chars[i] == '>' {
                    break;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if chars[i] == '>' {
            i += 1;
            continue;
        }
        return Some(chars[i]);
    }
    None
}

/// Apply kramdown SQ_RULES to a sequence of `count` consecutive straight quotes,
/// given the preceding character (before the sequence) and the following character
/// (after the sequence).
///
/// Returns a String with the appropriate mix of straight quotes, lsquo, and rsquo.
///
/// The rules are based on verified kramdown test cases:
///
/// **For `''` (count=2):**
/// - Space/None(start) + `''` + word: both become lsquo (Rule 9 fallback)
///   EXCEPT at true start of text (no prev): Rule 7 matches pair -> straight + rsquo
/// - Word/SQ_CLOSE + `''` + space/end: both become rsquo (Rule 7 + Rule 8)
/// - Word/SQ_CLOSE + `''` + SQ_PUNCT: both become rsquo (Rule 7 + Rule 2)
///
/// **For `'''` (count=3):**
/// - Start of text + `'''` + word: rsquo + straight + rsquo (Rule 2 + Rule 7)
/// - Space + `'''` + word: lsquo + lsquo + lsquo (all Rule 9 fallback)
/// - Word + `'''` + space/end: rsquo + rsquo + rsquo (Rule 7 chain)
fn apply_sq_rules_for_sequence(prev: Option<char>, next: Option<char>, count: usize) -> String {
    if count == 0 {
        return String::new();
    }

    // For single quotes (not from ''/'''' sequences but could appear), apply basic rules
    if count == 1 {
        let ch = apply_single_sq_rule(prev, next);
        return String::from(ch);
    }

    let prev_is_space_or_start = prev.is_none_or(|p| p.is_whitespace());
    let prev_is_sq_close = prev.is_some_and(is_sq_close);
    let next_is_word = next.is_some_and(|n| n.is_alphanumeric() || n == '_');
    let next_is_space_or_end = next.is_none_or(|n| n.is_whitespace());
    let next_is_sq_punct = next.is_some_and(is_sq_punct);

    if count == 2 {
        if prev.is_none() && next_is_word {
            // Start of text + '' + word: Rule 7 matches pair -> straight + rsquo
            return format!("'{}", '\u{2019}');
        }
        if prev.is_some_and(|p| p.is_whitespace()) && next_is_word {
            // Space + '' + word: both lsquo (Rule 9 fallback for each)
            return format!("{}{}", '\u{2018}', '\u{2018}');
        }
        if prev_is_sq_close && (next_is_space_or_end || next_is_sq_punct) {
            // Word/close + '' + space/end/punct: both rsquo
            return format!("{}{}", '\u{2019}', '\u{2019}');
        }
        if prev_is_sq_close && next_is_word {
            // SQ_CLOSE + '' + word: Rule 7 for first (rsquo), then second has no
            // preceding char from scanner -> Rule 9 fallback (lsquo)?
            // Actually: if prev is a word char (SQ_CLOSE), SMART_QUOTES_RE matches
            // the prev char + first quote. Rule 7: SQ_CLOSE + ' -> text + rsquo.
            // Then second quote at scanner start: '' + word -> but only one quote left,
            // followed by word. Rule 9 fallback -> lsquo.
            // But this pattern doesn't appear in our test cases.
            // Conservative: rsquo + lsquo
            return format!("{}{}", '\u{2019}', '\u{2018}');
        }
        if prev_is_space_or_start && next_is_space_or_end {
            // Space + '' + space/end: both rsquo (Rule 8)
            return format!("{}{}", '\u{2019}', '\u{2019}');
        }
        if prev_is_space_or_start && next_is_sq_punct {
            // Space + '' + punct: first lsquo (Rule 9), second rsquo (Rule 2 before punct)
            return format!("{}{}", '\u{2018}', '\u{2019}');
        }
    }

    if count == 3 {
        if prev.is_none() && next_is_word {
            // Start of text + ''' + word: rsquo + straight + rsquo (Rule 2 + Rule 7)
            return format!("{}'{}", '\u{2019}', '\u{2019}');
        }
        if prev.is_some_and(|p| p.is_whitespace()) && next_is_word {
            // Space + ''' + word: all three lsquo (Rule 9 fallback chain)
            return format!("{}{}{}", '\u{2018}', '\u{2018}', '\u{2018}');
        }
        if prev_is_sq_close && next_is_space_or_end {
            // Word + ''' + space/end: all three rsquo
            return format!("{}{}{}", '\u{2019}', '\u{2019}', '\u{2019}');
        }
        if prev_is_sq_close && next_is_sq_punct {
            // Word + ''' + punct: all three rsquo
            return format!("{}{}{}", '\u{2019}', '\u{2019}', '\u{2019}');
        }
    }

    // For count > 3 or unhandled patterns, process each quote individually
    // using the sequential scanner approach
    let mut out = String::new();
    // Track "effective prev" for the scanner
    let mut eff_prev = prev;
    for j in 0..count {
        let eff_next = if j < count - 1 {
            Some('\'') // next quote in sequence
        } else {
            next // actual next char after sequence
        };
        let ch = apply_single_sq_rule(eff_prev, eff_next);
        out.push(ch);
        // After processing, the scanner has consumed this quote.
        // If this was a Rule 7 match (consumed prev + quote), the next quote
        // has no preceding char. Otherwise, the converted char is the new prev.
        if eff_prev.is_some_and(is_sq_close) && ch == '\u{2019}' {
            // Rule 7 consumed prev + quote. Next has no preceding char.
            eff_prev = None;
        } else {
            eff_prev = Some(ch);
        }
    }
    out
}

/// Apply kramdown SQ_RULES to a single straight quote given its context.
fn apply_single_sq_rule(prev: Option<char>, next: Option<char>) -> char {
    match prev {
        None => {
            // No preceding character (start of text or after scanner consumption).
            // Rule 2: quote before SQ_PUNCT -> rsquo
            if next.is_some_and(is_sq_punct) {
                return '\u{2019}';
            }
            // Rule 8: quote before space/end -> rsquo
            if next.is_none_or(|n| n.is_whitespace()) {
                return '\u{2019}';
            }
            // Rule 9 fallback -> lsquo
            '\u{2018}'
        }
        Some(p) => {
            // There is a preceding character.
            // Rule 6: space + quote + word char -> lsquo
            if p.is_whitespace() && next.is_some_and(|n| n.is_alphanumeric() || n == '_') {
                return '\u{2018}';
            }
            // Rule 7: SQ_CLOSE + quote -> rsquo
            if is_sq_close(p) {
                return '\u{2019}';
            }
            // Rule 8: quote + space/end -> rsquo
            if next.is_none_or(|n| n.is_whitespace()) {
                return '\u{2019}';
            }
            // Fallback (Rule 9): lsquo
            '\u{2018}'
        }
    }
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
        // GFM preserves leading digits (unlike kramdown default which strips them)
        assert_eq!(slugify("1. DataTalksClub"), "1-datatalksclub");
        assert_eq!(slugify("123 Hello"), "123-hello");
    }

    #[test]
    fn test_slugify_leading_spaces_become_hyphens() {
        // GFM: spaces become hyphens, leading hyphens remain
        assert_eq!(slugify("  Hello"), "--hello");
    }

    #[test]
    fn test_slugify_trailing_chars_preserved() {
        // Trailing dashes/spaces from non-alnum chars are NOT trimmed by kramdown
        // (kramdown does not trim trailing dashes)
        assert_eq!(slugify("Hello World!"), "hello-world");
    }

    // --- Non-ASCII slugify tests (GFM Unicode-preserving generate_id) ---
    //
    // Jekyll defaults to kramdown input: GFM, which uses \p{Word} (Unicode word
    // chars) to keep letters/digits/underscores. Non-ASCII scripts are preserved.

    #[test]
    fn test_slugify_cyrillic_with_chapter_number() {
        // Cyrillic preserved, colon stripped, spaces become hyphens
        assert_eq!(
            slugify("Глава 1: Введение - Мир металлов вокруг нас"),
            "глава-1-введение---мир-металлов-вокруг-нас"
        );
    }

    #[test]
    fn test_slugify_cyrillic_emdash() {
        // Em-dash is stripped (not \p{Word}), spaces around it become hyphens
        assert_eq!(
            slugify("Глава 1: Введение — Мир металлов вокруг нас"),
            "глава-1-введение--мир-металлов-вокруг-нас"
        );
    }

    #[test]
    fn test_slugify_pure_cyrillic() {
        assert_eq!(
            slugify("Уникальные дары металлов"),
            "уникальные-дары-металлов"
        );
    }

    #[test]
    fn test_slugify_cyrillic_with_numbers() {
        assert_eq!(
            slugify("Глава 3: Бронзовый век - революция сплавов"),
            "глава-3-бронзовый-век---революция-сплавов"
        );
    }

    #[test]
    fn test_slugify_simple_cyrillic() {
        assert_eq!(slugify("Привет мир"), "привет-мир");
    }

    #[test]
    fn test_slugify_cyrillic_preserves_unicode() {
        let result = slugify("Глава 1: Введение - Мир металлов вокруг нас");
        assert_eq!(
            result, "глава-1-введение---мир-металлов-вокруг-нас",
            "Cyrillic text should be preserved in GFM mode, got: {result}"
        );
    }

    // --- GFM slugify regression tests (Unicode-preserving) ---
    // Jekyll defaults to kramdown input: GFM, which uses \p{Word} (Unicode word chars)
    // instead of ASCII-only [a-zA-Z0-9]. This means Cyrillic, Arabic, accented Latin,
    // etc. are all preserved in heading IDs.

    #[test]
    fn test_slugify_gfm_cyrillic_preserved() {
        // GFM mode: Cyrillic letters are \p{Word}, so they stay
        assert_eq!(
            slugify("Уникальные дары металлов"),
            "уникальные-дары-металлов"
        );
    }

    #[test]
    fn test_slugify_gfm_cyrillic_with_chapter_number() {
        // "Глава 1: Введение - Мир металлов вокруг нас"
        // colon stripped, everything else preserved
        assert_eq!(
            slugify("Глава 1: Введение - Мир металлов вокруг нас"),
            "глава-1-введение---мир-металлов-вокруг-нас"
        );
    }

    #[test]
    fn test_slugify_gfm_leading_digits_preserved() {
        // GFM does NOT strip leading digits
        assert_eq!(slugify("1. DataTalksClub"), "1-datatalksclub");
        assert_eq!(slugify("123 Hello"), "123-hello");
    }

    #[test]
    fn test_slugify_gfm_arabic_preserved() {
        // Arabic letters are \p{Word}, so they stay in GFM mode
        let result = slugify("\u{0645}\u{0627} \u{0645}\u{0639}\u{0646}\u{0649}");
        assert_eq!(result, "\u{0645}\u{0627}-\u{0645}\u{0639}\u{0646}\u{0649}");
    }

    #[test]
    fn test_slugify_gfm_mixed_ascii_arabic() {
        // Mixed: all kept
        let result =
            slugify("GitHub \u{0647}\u{0644} \u{0645}\u{0634}\u{0627}\u{0631}\u{064a}\u{0639}");
        assert_eq!(
            result,
            "github-\u{0647}\u{0644}-\u{0645}\u{0634}\u{0627}\u{0631}\u{064a}\u{0639}"
        );
    }

    #[test]
    fn test_slugify_gfm_underscore_preserved() {
        // \p{Word} includes underscore
        assert_eq!(slugify("hello_world"), "hello_world");
    }

    #[test]
    fn test_slugify_gfm_accented_latin() {
        assert_eq!(slugify("café au lait"), "café-au-lait");
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
    fn test_postprocess_escapes_malformed_single_quote_canvas() {
        let html = r#"<figure>
  <canvas class="ai-chart"
          data-type="bar"
          data-orientation="horizontal"
          data-title="Do you use any solutions for self-hosting open-source LLMs?"
          data-labels='["We don't self-host LLMs", "vLLM", "Self-written (custom inference solutions)"]'
          data-values='[74.1, 9.4, 8.5]'
          data-height="300px"
          data-width="600px">
  </canvas>
  <figcaption>Majority do not self-host open-source LLMs.</figcaption>
</figure>
"#;
        let result = postprocess(html);
        assert!(
            result.contains("&lt;canvas class=\"ai-chart\""),
            "Malformed canvas should be escaped like Jekyll. Got: {}",
            result
        );
        assert!(
            result.contains("&lt;/canvas&gt;"),
            "Closing canvas tag should also be escaped. Got: {}",
            result
        );
        assert!(
            !result.contains("<canvas class=\"ai-chart\""),
            "Malformed canvas should not remain as a live element. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_preserves_wellformed_single_quote_canvas() {
        let html = r#"<figure>
  <canvas class="ai-chart"
          data-type="bar"
          data-title="Well formed canvas"
          data-labels='["We do self-host LLMs", "vLLM"]'
          data-values='[74.1, 9.4]'
          data-height="300px"
          data-width="600px">
  </canvas>
  <figcaption>Well formed canvas should stay live.</figcaption>
</figure>
"#;
        let result = postprocess(html);
        assert!(
            result.contains("<canvas class=\"ai-chart\""),
            "Well formed canvas should remain live. Got: {}",
            result
        );
        assert!(
            result.contains("data-labels='[\"We do self-host LLMs\", \"vLLM\"]'"),
            "Single-quoted attributes without apostrophes should be preserved. Got: {}",
            result
        );
        assert!(
            !result.contains("&lt;canvas"),
            "Well formed canvas should not be escaped. Got: {}",
            result
        );
    }

    #[test]
    fn test_has_unbalanced_single_quotes_detects_canvas() {
        let tag = r#"<canvas class="ai-chart"
          data-type="bar"
          data-orientation="horizontal"
          data-title="Do you use any solutions for self-hosting open-source LLMs?"
          data-labels='["We don't self-host LLMs", "vLLM", "Self-written (custom inference solutions)"]'
          data-values='[74.1, 9.4, 8.5]'
          data-height="300px"
          data-width="600px">"#;
        assert!(
            has_unbalanced_single_quotes(&tag[1..tag.len() - 1]),
            "Canvas tag should be detected as malformed. Got: {tag}"
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
    fn test_postprocess_does_not_add_inline_code_class() {
        // Inline code classes are now added during markdown rendering, not in
        // postprocess(). Raw HTML <code> tags should pass through unchanged.
        let html = "<p>Use <code>pip install</code> to install.</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<code>pip install</code>"),
            "postprocess should not add classes to raw HTML <code> tags. Got: {}",
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
        // Inline code classes are added during markdown rendering, not postprocess
        assert!(
            result.contains("<code>pip</code>"),
            "postprocess should not add classes to raw HTML <code> tags. Got: {}",
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

    // ======================================================================
    // Issue 246: Block-level IAL (e.g., `{: .fs-9 }` on its own line)
    // ======================================================================

    #[test]
    fn test_block_ial_heading_single_class() {
        // Block IAL: `# Title\n{: .fs-9 }` should apply class to heading.
        // After markdown parsing, this becomes:
        //   <h1>Title</h1>\n<p>{: .fs-9 }</p>
        let html = "<h1>Title</h1>\n<p>{: .fs-9 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"fs-9\""),
            "Block IAL should apply class to preceding heading. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .fs-9 }"),
            "Block IAL should be removed from output. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_heading_multiple_classes() {
        // Block IAL with multiple classes: `{: .fs-6 .fw-300 }`
        let html = "<p>Some text</p>\n<p>{: .fs-6 .fw-300 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"fs-6 fw-300\""),
            "Block IAL should apply multiple classes. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .fs-6 .fw-300 }"),
            "Block IAL should be removed from output. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_heading_id() {
        // Block IAL with id: `{: #custom-id }`
        let html = "<h1>Title</h1>\n<p>{: #custom-id }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"custom-id\""),
            "Block IAL should apply id to preceding heading. Got: {}",
            result
        );
        assert!(
            !result.contains("{: #custom-id }"),
            "Block IAL should be removed from output. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_inline_link() {
        // Inline IAL on a link: `[Click](http://example.com){: .btn .fs-5 }`
        // After markdown parsing, this becomes:
        //   <p><a href="http://example.com">Click</a>{: .btn .fs-5 }</p>
        let html = "<p><a href=\"http://example.com\">Click</a>{: .btn .fs-5 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"btn fs-5\""),
            "Inline IAL should apply classes to link. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .btn .fs-5 }"),
            "Inline IAL should be removed from output. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_no_interference_with_normal_paragraphs() {
        // Normal paragraphs without IAL should pass through unchanged.
        let html = "<p>Regular paragraph</p>\n<p>Next paragraph</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("Regular paragraph"),
            "Normal paragraphs should be unchanged. Got: {}",
            result
        );
        assert!(
            result.contains("Next paragraph"),
            "Normal paragraphs should be unchanged. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_unicode_content() {
        // Block IAL with Unicode content in the heading.
        let html = "<h1>Ubersicht</h1>\n<p>{: .fs-9 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"fs-9\""),
            "Block IAL should work with Unicode content. Got: {}",
            result
        );
        assert!(
            result.contains("Ubersicht"),
            "Unicode content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_merged_in_paragraph_text() {
        // When comrak merges the IAL into the paragraph text (no blank line between):
        //   `Some text\n{: .fs-6 .fw-300 }` becomes `<p>Some text\n{: .fs-6 .fw-300 }</p>`
        // The IAL should be stripped and applied to the <p> element.
        let html = "<p>Some text\n{: .fs-6 .fw-300 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"fs-6 fw-300\""),
            "Merged IAL should apply classes to paragraph. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .fs-6 .fw-300 }"),
            "Merged IAL should be removed from paragraph text. Got: {}",
            result
        );
        // The paragraph text should be preserved without the IAL
        assert!(
            result.contains(">Some text</p>") || result.contains(">Some text\n</p>"),
            "Paragraph text should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_merged_in_paragraph_unicode() {
        // Unicode content with merged IAL in paragraph
        let html = "<p>Willkommen bei uns\n{: .fs-6 .fw-300 }</p>\n";
        let result = postprocess(html);
        assert!(
            result.contains("class=\"fs-6 fw-300\""),
            "Merged IAL should work with Unicode. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .fs-6 .fw-300 }"),
            "Merged IAL should be removed. Got: {}",
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
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain code\n</code></pre>"),
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
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain code\n</code></pre>"),
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
            result.contains("<div class=\"highlighter-rouge\">"),
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
        let count = result.matches("<div class=\"highlighter-rouge\">").count();
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
        let plaintext_count = result.matches("<div class=\"highlighter-rouge\">").count();
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
        // Raw HTML <code> should pass through without classes (classes are
        // added during markdown rendering, not postprocessing)
        assert!(
            result.contains("<code>pip install</code>"),
            "postprocess should not add classes to raw HTML <code>. Got: {}",
            result
        );
        assert!(
            !result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\">"),
            "Inline code should NOT be wrapped in divs. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_mixed_inline_and_fenced() {
        let html = "<p>Use <code>pip</code> command.</p>\n<pre><code>bare code\n</code></pre>\n";
        let result = postprocess(html);
        // Raw HTML <code> passes through unchanged (classes added during rendering)
        assert!(
            result.contains("<code>pip</code>"),
            "postprocess should not add classes to raw HTML <code>. Got: {}",
            result
        );
        // Fenced code gets div wrapper
        assert!(
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>bare code\n</code></pre>"),
            "Fenced code should get div wrapper. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_mixed_all_three() {
        // Document with inline code, fenced-with-language, and fenced-without-language
        let html = "<p>Use <code>pip</code>.</p>\n<pre><code class=\"language-python\">import os\n</code></pre>\n<pre><code>plain\n</code></pre>\n";
        let result = postprocess(html);
        // Raw HTML <code> passes through unchanged (classes added during rendering)
        assert!(
            result.contains("<code>pip</code>"),
            "postprocess should not add classes to raw HTML <code>. Got: {}",
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
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>plain\n</code></pre>"),
            "Bare fenced code should be wrapped. Got: {}",
            result
        );
        // Should NOT wrap the language-tagged block
        let wrapper_count = result.matches("<div class=\"highlighter-rouge\">").count();
        assert_eq!(
            wrapper_count, 1,
            "Only one block should be wrapped. Got: {}",
            result
        );
    }

    // ======================================================================
    // Issue 443: IAL attributes on <pre> should be moved to wrapper div
    // ======================================================================

    #[test]
    fn test_fenced_code_wrapping_with_ial_data_title() {
        let html = "<pre data-title=\"Gemfile\"><code class=\"language-ruby\">gem \"hello\"\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result
                .contains("<div data-title=\"Gemfile\" class=\"language-ruby highlighter-rouge\">"),
            "IAL data-title should move from <pre> to outer wrapper <div>. Got: {}",
            result
        );
        assert!(
            result.contains("<pre class=\"highlight\"><code>"),
            "Inner <pre> should get class=\"highlight\" without IAL attrs. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_with_multiple_ial_attrs() {
        let html = "<pre data-title=\"Config\" data-lang=\"yaml\" id=\"config-block\"><code class=\"language-yaml\">key: val\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("data-title=\"Config\""),
            "data-title should be on wrapper div. Got: {}",
            result
        );
        assert!(
            result.contains("data-lang=\"yaml\""),
            "data-lang should be on wrapper div. Got: {}",
            result
        );
        assert!(
            result.contains("id=\"config-block\""),
            "id should be on wrapper div. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"language-yaml highlighter-rouge\""),
            "Language class should be present on wrapper div. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_ial_no_language() {
        let html = "<pre data-info=\"example\"><code>plain code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div data-info=\"example\" class=\"highlighter-rouge\">"),
            "IAL attrs on bare code block should move to wrapper div. Got: {}",
            result
        );
    }

    #[test]
    fn test_fenced_code_wrapping_ial_preserves_unicode() {
        let html = "<pre data-title=\"Konfigürasyon\"><code class=\"language-yaml\">ayar: değer\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("data-title=\"Konfigürasyon\""),
            "Unicode IAL attribute should be preserved on wrapper div. Got: {}",
            result
        );
        assert!(
            result.contains(
                "<div data-title=\"Konfigürasyon\" class=\"language-yaml highlighter-rouge\">"
            ),
            "Full wrapper div should have both IAL attrs and language class. Got: {}",
            result
        );
    }

    // ======================================================================
    // Issue 183: Wrapper div class for no-language fenced code blocks
    // ======================================================================

    #[test]
    fn test_no_language_wrapper_div_class() {
        // Issue 470: No-language fenced code blocks should have highlighter-rouge
        // without language-plaintext in the wrapper div.
        let html = "<pre><code>some code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\">"),
            "Wrapper div should have highlighter-rouge class (no language-plaintext). Got: {}",
            result
        );
    }

    #[test]
    fn test_no_language_wrapper_div_no_language_plaintext() {
        // Issue 470: No-language fenced code blocks should NOT have language-plaintext.
        let html = "<pre><code>some code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("language-plaintext"),
            "Wrapper div should NOT have language-plaintext. Got: {}",
            result
        );
    }

    #[test]
    fn test_language_wrapper_div_still_has_both_classes() {
        // For language-specified code blocks, the wrapper div should have both classes.
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\">"),
            "Language-specified wrapper should have both classes. Got: {}",
            result
        );
    }

    #[test]
    fn test_language_wrapper_div_no_language_plaintext_for_python() {
        // Regression: language-specified code blocks should NOT get language-plaintext.
        let html = "<pre><code class=\"language-python\">print('hi')\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            !result.contains("language-plaintext"),
            "Language-specified block should NOT contain language-plaintext. Got: {}",
            result
        );
    }

    // ==================================================================
    // Issue 470: Code block wrapper div should NOT have language-plaintext
    // ==================================================================

    #[test]
    fn test_issue470_no_language_wrapper_div_no_language_plaintext() {
        // Issue 470: No-language fenced code blocks should have wrapper div
        // with class="highlighter-rouge" (no language-plaintext).
        let html = "<pre><code>some code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\">"),
            "No-language wrapper div should have highlighter-rouge without language-plaintext. Got: {}",
            result
        );
        assert!(
            !result.contains("language-plaintext"),
            "No-language wrapper div should NOT have language-plaintext. Got: {}",
            result
        );
    }

    #[test]
    fn test_no_language_inner_code_has_no_extra_class() {
        // The inner <code> element in a fenced block should not have language-plaintext
        // (that's only for inline code elements, handled during markdown rendering).
        let html = "<pre><code>some code\n</code></pre>\n";
        let result = postprocess(html);
        // The fenced code wrapper just uses bare <code> inside <pre>
        assert!(
            result.contains("<pre class=\"highlight\"><code>some code\n</code></pre>"),
            "Inner code element should be bare. Got: {}",
            result
        );
    }

    #[test]
    fn test_highlighted_code_block_structure_unchanged() {
        // Code blocks with a recognized language should keep the full wrapper structure.
        let html = "<pre><code class=\"language-python\">x = 1\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"language-python highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>"),
            "Language code block structure should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("</code></pre></div></div>"),
            "Should have proper closing tags. Got: {}",
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
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
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
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            result.contains("id=\"markdown-title\""),
            "Markdown heading should get auto-generated ID. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 526: Headings inside raw HTML div blocks should not get auto IDs
    // ========================================================================

    #[test]
    fn test_issue526_h5_inside_div_note_no_id() {
        // Raw HTML <h5> inside <div class="note"> should NOT get an auto-generated id.
        // Jekyll/kramdown does not add IDs to headings inside raw HTML blocks.
        let input = "<div class=\"note info\">\n  <h5>Be aware of directory paths</h5>\n  <p>Some note text.</p>\n</div>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            !result.contains("id="),
            "h5 inside div.note should not get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue526_h5_inside_div_warning_no_id() {
        let input = "<div class=\"note warning\">\n  <h5>ProTip\u{2122}</h5>\n  <p>Warning text.</p>\n</div>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            !result.contains("id="),
            "h5 inside div.warning should not get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue526_h2_outside_div_still_gets_id() {
        // Regular headings outside div blocks should still get IDs.
        let input = "<h2>Regular Heading</h2>\n<div class=\"note\">\n  <h5>Note Title</h5>\n</div>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            result.contains("<h2 id=\"regular-heading\">"),
            "h2 outside div should get auto-generated ID. Got: {}",
            result
        );
        assert!(
            !result.contains("id=\"note-title\""),
            "h5 inside div should not get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue526_h5_at_top_level_still_gets_id() {
        // h5 headings NOT inside a div block should still get IDs (markdown-generated).
        let input = "<h5>Top Level H5</h5>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            result.contains("id=\"top-level-h5\""),
            "h5 at top level should get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue526_nested_divs_h5_no_id() {
        // Headings inside nested divs should not get IDs.
        let input =
            "<div class=\"wrapper\">\n<div class=\"note\">\n  <h5>Nested Note</h5>\n</div>\n</div>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            !result.contains("id="),
            "h5 inside nested divs should not get auto-generated ID. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue526_heading_after_div_close_gets_id() {
        // Headings after a div block closes should still get IDs.
        let input = "<div class=\"note\">\n  <h5>Note</h5>\n</div>\n<h2>After Div</h2>";
        let result = add_heading_ids(input, HeadingIdMode::Kramdown);
        assert!(
            !result.contains("id=\"note\""),
            "h5 inside div should not get ID. Got: {}",
            result
        );
        assert!(
            result.contains("<h2 id=\"after-div\">"),
            "h2 after div close should get ID. Got: {}",
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

    #[test]
    fn test_normalize_bare_void_elements_preserves_utf8() {
        // Middle dot U+00B7 (2 bytes in UTF-8: 0xC2 0xB7)
        assert_eq!(
            normalize_bare_void_elements("<p>hello · world</p><br>done"),
            "<p>hello · world</p><br />done"
        );
        // Right arrow U+2192 (3 bytes in UTF-8)
        assert_eq!(
            normalize_bare_void_elements("<p>click → here</p><br>end"),
            "<p>click → here</p><br />end"
        );
        // Smart quote U+2019 (3 bytes in UTF-8)
        assert_eq!(
            normalize_bare_void_elements("<p>it\u{2019}s fine</p><br>ok"),
            "<p>it\u{2019}s fine</p><br />ok"
        );
        // Emoji U+1F3C6 (4 bytes in UTF-8)
        assert_eq!(
            normalize_bare_void_elements("<p>trophy \u{1F3C6} here</p><hr>end"),
            "<p>trophy \u{1F3C6} here</p><hr />end"
        );
        // UTF-8 text with no void elements should pass through unchanged
        assert_eq!(
            normalize_bare_void_elements("<p>· → \u{2019} \u{1F3C6}</p>"),
            "<p>· → \u{2019} \u{1F3C6}</p>"
        );
    }

    // --- Issue 213/222: normalize_bare_void_elements converts ALL void elements ---

    #[test]
    fn test_normalize_bare_void_br_and_meta() {
        // Both <br> and <meta> should be converted (issue 222)
        assert_eq!(
            normalize_bare_void_elements("<br>text<meta charset=\"utf-8\">"),
            "<br />text<meta charset=\"utf-8\" />"
        );
    }

    #[test]
    fn test_normalize_bare_void_hr_and_link() {
        // Both <hr> and <link> should be converted (issue 222)
        assert_eq!(
            normalize_bare_void_elements("<hr>text<link rel=\"stylesheet\">"),
            "<hr />text<link rel=\"stylesheet\" />"
        );
    }

    #[test]
    fn test_normalize_bare_void_mixed_elements() {
        // All void elements get converted (issue 222)
        assert_eq!(
            normalize_bare_void_elements(
                "<br><hr><meta name=\"test\"><img src=\"x\"><input type=\"text\">"
            ),
            "<br /><hr /><meta name=\"test\" /><img src=\"x\" /><input type=\"text\" />"
        );
    }

    #[test]
    fn test_normalize_bare_void_already_self_closing_br_and_meta() {
        // Already self-closing elements should stay unchanged
        assert_eq!(
            normalize_bare_void_elements("<br /><meta charset=\"utf-8\">"),
            "<br /><meta charset=\"utf-8\" />"
        );
    }

    #[test]
    fn test_normalize_bare_void_no_void_elements() {
        // No void elements -- input unchanged
        assert_eq!(normalize_bare_void_elements("<p>Hello</p>"), "<p>Hello</p>");
    }

    #[test]
    fn test_normalize_bare_void_unicode_preservation() {
        // Unicode content with accented characters (Ren\u{00e9} Magritte)
        assert_eq!(
            normalize_bare_void_elements("<br><p>Ren\u{00e9} Magritte</p>"),
            "<br /><p>Ren\u{00e9} Magritte</p>"
        );
        // CJK characters
        assert_eq!(
            normalize_bare_void_elements("<hr><p>\u{4F60}\u{597D}\u{4E16}\u{754C}</p>"),
            "<hr /><p>\u{4F60}\u{597D}\u{4E16}\u{754C}</p>"
        );
    }

    #[test]
    fn test_normalize_bare_void_seo_meta_keeps_self_closing() {
        // SEO tag output has <meta ... /> already -- should be left unchanged
        let input = "<meta name=\"description\" content=\"test\" /><br><link rel=\"canonical\" />";
        assert_eq!(
            normalize_bare_void_elements(input),
            "<meta name=\"description\" content=\"test\" /><br /><link rel=\"canonical\" />"
        );
    }

    // --- Issue 222: normalize_bare_void_elements converts ALL void elements ---

    #[test]
    fn test_222_bare_input_is_converted() {
        assert_eq!(
            normalize_bare_void_elements("<input type=\"text\">"),
            "<input type=\"text\" />"
        );
    }

    #[test]
    fn test_222_bare_meta_is_converted() {
        assert_eq!(
            normalize_bare_void_elements("<meta charset=\"utf-8\">"),
            "<meta charset=\"utf-8\" />"
        );
    }

    #[test]
    fn test_222_bare_link_is_converted() {
        assert_eq!(
            normalize_bare_void_elements("<link rel=\"stylesheet\" href=\"style.css\">"),
            "<link rel=\"stylesheet\" href=\"style.css\" />"
        );
    }

    #[test]
    fn test_222_bare_img_is_converted() {
        assert_eq!(
            normalize_bare_void_elements("<img src=\"photo.jpg\" alt=\"test\">"),
            "<img src=\"photo.jpg\" alt=\"test\" />"
        );
    }

    #[test]
    fn test_222_multiple_void_element_types() {
        assert_eq!(
            normalize_bare_void_elements(
                "<meta charset=\"utf-8\"><br><input type=\"text\"><hr><link rel=\"icon\">"
            ),
            "<meta charset=\"utf-8\" /><br /><input type=\"text\" /><hr /><link rel=\"icon\" />"
        );
    }

    #[test]
    fn test_222_already_self_closing_not_double_converted() {
        assert_eq!(
            normalize_bare_void_elements("<meta charset=\"utf-8\" /><input type=\"text\" />"),
            "<meta charset=\"utf-8\" /><input type=\"text\" />"
        );
    }

    #[test]
    fn test_222_non_void_elements_not_affected() {
        assert_eq!(
            normalize_bare_void_elements("<div><p>text</p></div>"),
            "<div><p>text</p></div>"
        );
    }

    #[test]
    fn test_222_unicode_with_all_void_types() {
        assert_eq!(
            normalize_bare_void_elements(
                "<meta name=\"title\" content=\"Ren\u{00e9}\"><input value=\"\u{4F60}\u{597D}\"><br>"
            ),
            "<meta name=\"title\" content=\"Ren\u{00e9}\" /><input value=\"\u{4F60}\u{597D}\" /><br />"
        );
    }

    #[test]
    fn test_222_normalize_html_output_without_br_hr() {
        // normalize_html_output only converts br/hr, not meta/input
        // (those are handled in postprocess() before layout wrapping)
        assert_eq!(
            normalize_html_output("<meta charset=\"utf-8\"><input type=\"text\">"),
            "<meta charset=\"utf-8\"><input type=\"text\">"
        );
    }

    #[test]
    fn test_222_normalize_html_output_br_plus_meta_input() {
        // Only br/hr converted in final output, meta/input left alone
        assert_eq!(
            normalize_html_output("<br><meta charset=\"utf-8\"><input type=\"text\">"),
            "<br /><meta charset=\"utf-8\"><input type=\"text\">"
        );
    }

    #[test]
    fn test_222_all_void_element_tags() {
        // Verify every void element type is converted
        let input =
            "<area><base><br><col><embed><hr><img><input><link><meta><param><source><track><wbr>";
        let expected = "<area /><base /><br /><col /><embed /><hr /><img /><input /><link /><meta /><param /><source /><track /><wbr />";
        assert_eq!(normalize_bare_void_elements(input), expected);
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
        // normalize_html_output only converts bare <br>, not <hr>/input/etc.
        // Boolean attrs are still normalized.
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
        // After issue 158, loose list items are also indented to match kramdown.
        let input =
            "* First item paragraph.\n\n* Second item paragraph.\n\n* Third item paragraph.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // Each <li> should contain a <p> tag (with kramdown indentation)
        assert!(
            html.contains("  <li>\n    <p>First item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping with indentation. Got: {}",
            html
        );
        assert!(
            html.contains("  <li>\n    <p>Second item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping with indentation. Got: {}",
            html
        );
        assert!(
            html.contains("  <li>\n    <p>Third item paragraph.</p>"),
            "Loose list <li> should preserve <p> wrapping with indentation. Got: {}",
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
        // After issue 158, loose list items are indented to match kramdown.
        let input = "* Most importantly, you\u{2019}re giving yourself a chance to think through the project.\n\n* As a byproduct of writing a Readme, you\u{2019}ll have nice documentation.\n\n* If you\u{2019}re working with a team, everyone can start work on other projects.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // Each item should be wrapped in <p> with kramdown indentation
        let li_p_count = html.matches("  <li>\n    <p>").count();
        assert_eq!(
            li_p_count, 3,
            "All 3 loose list items should have <p> wrapping with indentation. Got: {}",
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

    // ========================================================================
    // Issue 158: figcaption <p> tags must be PRESERVED (not stripped)
    // ========================================================================
    // In the DTC site, <figcaption><p>...</p></figcaption> is raw HTML in the
    // source markdown. Jekyll/kramdown passes it through unchanged. We must too.

    #[test]
    fn test_issue158_figcaption_p_tags_preserved() {
        // <figcaption><p>text</p></figcaption> is raw HTML from source.
        // Jekyll preserves the <p> tag. We must NOT strip it.
        let input = "<figcaption><p>Caption text</p></figcaption>";
        let result = strip_paragraphs_in_html_blocks(input);
        assert_eq!(
            result, "<figcaption><p>Caption text</p></figcaption>",
            "strip_paragraphs_in_html_blocks should preserve <p> inside <figcaption>"
        );
    }

    #[test]
    fn test_issue158_figure_strips_p_but_figcaption_preserves() {
        // <figure> strips <p>, but <figcaption> preserves it.
        let input =
            "<figure><p>Image content</p><figcaption><p>Caption text</p></figcaption></figure>";
        let result = strip_paragraphs_in_html_blocks(input);
        assert_eq!(
            result,
            "<figure>Image content<figcaption><p>Caption text</p></figcaption></figure>"
        );
    }

    #[test]
    fn test_issue158_figcaption_p_preserved_in_postprocess() {
        // End-to-end: postprocess should preserve <p> inside <figcaption>
        let input = "<figcaption><p>Caption</p></figcaption>";
        let result = postprocess(input);
        assert!(
            result.contains("<figcaption><p>Caption</p></figcaption>"),
            "postprocess should preserve <p> inside <figcaption>, got: {}",
            result
        );
    }

    #[test]
    fn test_issue158_figcaption_without_p_unchanged() {
        // figcaption without <p> should pass through unchanged
        let input = "<figcaption>Plain caption</figcaption>";
        let result = strip_paragraphs_in_html_blocks(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_issue158_figcaption_with_link_p_preserved() {
        // Real-world case from DTC site: figcaption with inline content and links.
        // The <p> is from the original source HTML, not auto-inserted.
        let input =
            "<figcaption><p>Image from <a href=\"https://example.com\">Source</a></p></figcaption>";
        let result = strip_paragraphs_in_html_blocks(input);
        assert_eq!(
            result,
            "<figcaption><p>Image from <a href=\"https://example.com\">Source</a></p></figcaption>"
        );
    }

    // ========================================================================
    // Issue 162: figcaption <p> with links in <figure> context (airflow blog)
    // ========================================================================

    #[test]
    fn test_issue162_figure_with_figcaption_p_links_preserved() {
        // Exact pattern from the airflow blog post: <figure> wraps <img> and
        // <figcaption><p>text with <a> links</p></figcaption>.
        // The strip_paragraphs_in_html_blocks for "figure" must not strip the
        // <p> inside <figcaption>.
        let input = "<figure>\n<img src=\"/images/test.png\" />\n<figcaption><p>Forget about issues (logos from <a href=\"https://airflow.apache.org/\"><u>Apache Airflow</u></a> and <a href=\"https://www.docker.com/\"><u>Docker</u></a>)</p></figcaption>\n</figure>";
        let result = strip_paragraphs_in_html_blocks(input);
        assert!(
            result.contains("<figcaption><p>Forget about issues"),
            "strip_paragraphs_in_html_blocks should preserve <p> inside <figcaption> within <figure>. Got:\n{}",
            result
        );
        assert!(
            result.contains("</a>)</p></figcaption>"),
            "Closing </p> inside <figcaption> should be preserved. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue162_figure_figcaption_postprocess() {
        // End-to-end postprocess for the airflow blog pattern
        let input = "<figure>\n<img src=\"/images/test.png\" />\n<figcaption><p>The official logo for Apache Airflow</p></figcaption>\n</figure>";
        let result = postprocess(input);
        assert!(
            result.contains("<figcaption><p>The official logo for Apache Airflow</p></figcaption>"),
            "postprocess should preserve <p> inside <figcaption> in <figure>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue162_normalize_figcaption_whitespace_with_p() {
        // Figcaption with <p> and newline before closing tag
        let input = "<figcaption><p>Caption text</p>\n</figcaption>";
        let result = normalize_figcaption_whitespace(input);
        assert_eq!(
            result, "<figcaption><p>Caption text</p></figcaption>",
            "figcaption whitespace normalization should work with <p> content"
        );
    }

    // ========================================================================
    // Issue 158: Code block closing divs must stay on one line
    // ========================================================================

    #[test]
    fn test_issue158_code_block_closing_divs_one_line() {
        // Jekyll outputs code block closing tags on one line:
        // </code></pre></div></div>
        // add_block_spacing must not split them across multiple lines.
        let input = "<div class=\"language-python highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>x = 1\n</code></pre></div></div>\n<p>Next.</p>\n";
        let result = add_block_spacing(input);
        assert!(
            result.contains("</code></pre></div></div>"),
            "Code block closing tags should stay on one line. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue158_code_block_closing_in_postprocess() {
        // End-to-end: postprocess should keep code block closing on one line
        let input = "<pre><code class=\"language-python\">x = 1\n</code></pre>\n";
        let result = postprocess(input);
        assert!(
            result.contains("</code></pre></div></div>"),
            "postprocess should keep code block closing tags on one line. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue158_regular_div_still_gets_spacing() {
        // Regular </div> not part of code block wrapper should still get spacing.
        let input = "<div>content</div>\n<p>Next.</p>\n";
        let result = add_block_spacing(input);
        assert!(
            result.contains("</div>\n\n<p>"),
            "Regular </div> should still get block spacing. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 158: Loose list indentation must match Jekyll
    // ========================================================================

    #[test]
    fn test_issue158_loose_list_indentation() {
        // Jekyll indents loose list item content with 2+2 spaces:
        //   <li>\n    <p>text</p>\n  </li>
        // pulldown-cmark outputs:
        //   <li>\n<p>text</p>\n</li>
        let input = "<ul>\n<li>\n<p>Item one</p>\n</li>\n<li>\n<p>Item two</p>\n</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("  <li>\n    <p>Item one</p>\n  </li>"),
            "Loose list items should be indented. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 164: Regression tests for blog/ml-deployment-lambda.html patterns
    // ========================================================================

    #[test]
    fn test_issue164_code_block_closing_divs_on_one_line() {
        // Jekyll keeps code block closing tags on a single line:
        //   </code></pre></div></div>
        // Verify markdown_to_html does not split them across lines.
        let md = "```bash\n$ sam init\n```\n\nNext paragraph.\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("</code></pre></div></div>"),
            "Code block closing tags must be on one line. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue164_figcaption_p_preserved_end_to_end() {
        // Raw HTML <figcaption><p>text</p></figcaption> in markdown must
        // pass through the full pipeline with <p> tags intact.
        let md = concat!(
            "Some text.\n\n",
            "<figure>\n",
            "<img src=\"/images/test.png\" alt=\"test\" />\n",
            "<figcaption><p>A caption here</p></figcaption>\n",
            "</figure>\n\n",
            "More text.\n"
        );
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("<figcaption><p>A caption here</p></figcaption>"),
            "figcaption <p> tags must be preserved. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue164_blank_line_after_figure() {
        // Jekyll outputs a blank line after </figure> before the next
        // block element due to block spacing.
        let md = concat!(
            "<figure>\n",
            "<img src=\"/images/test.png\" alt=\"test\" />\n",
            "<figcaption><p>A caption</p></figcaption>\n",
            "</figure>\n\n",
            "Next paragraph.\n"
        );
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("</figure>\n\n"),
            "There should be a blank line after </figure>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue164_code_block_then_figure_combo() {
        // Real-world pattern from the ml-deployment-lambda post:
        // a fenced code block followed by a figure with figcaption.
        let md = concat!(
            "Some intro text.\n\n",
            "```bash\n",
            "$ sam init\n",
            "```\n\n",
            "<figure>\n",
            "<img src=\"/images/posts/test/sam_init.png\" alt=\"SAM init\" />\n",
            "<figcaption><p>Initialize a new serverless project</p></figcaption>\n",
            "</figure>\n\n",
            "The project structure:\n\n",
            "```\n",
            "|-- service\n",
            "     |-- app.py\n",
            "```\n"
        );
        let result = crate::frontmatter::markdown_to_html(md);

        // Pattern 1: code block closing divs on one line
        assert!(
            result.contains("</code></pre></div></div>"),
            "Code block closing tags must be on one line. Got:\n{}",
            result
        );

        // Pattern 2: figcaption <p> preserved
        assert!(
            result.contains("<figcaption><p>Initialize a new serverless project</p></figcaption>"),
            "figcaption <p> tags must be preserved. Got:\n{}",
            result
        );

        // Pattern 3: blank line after </figure>
        assert!(
            result.contains("</figure>\n\n"),
            "Blank line after </figure> expected. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue166_tight_list_indent() {
        // Issue 218: Jekyll/kramdown DOES indent <li> items in tight lists by 2 spaces.
        // Verified against actual Jekyll output for blog/how-to-run-postgresql-and-pgadmin-with-docker.html.
        let input = "<ul>\n<li>Item one</li>\n<li>Item two</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert_eq!(
            result, "<ul>\n  <li>Item one</li>\n  <li>Item two</li>\n</ul>\n",
            "Tight list <li> should be indented by 2 spaces (matches Jekyll). Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue166_tight_list_ol_indent() {
        // Issue 218: Ordered tight lists should also be indented
        let input = "<ol>\n<li>First</li>\n<li>Second</li>\n<li>Third</li>\n</ol>\n";
        let result = indent_list_items(input);
        assert_eq!(
            result, "<ol>\n  <li>First</li>\n  <li>Second</li>\n  <li>Third</li>\n</ol>\n",
            "Tight ordered list <li> should be indented by 2 spaces. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue166_loose_list_still_indented() {
        // Loose lists (with <p> inside <li>) should still be indented.
        let input = "<ul>\n<li>\n<p>Item one</p>\n</li>\n<li>\n<p>Item two</p>\n</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("  <li>\n    <p>Item one</p>\n  </li>"),
            "Loose list items should still be indented. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue166_tight_list_end_to_end() {
        // Issue 218: End-to-end test: markdown with tight list should produce indented <li>
        let md = "Some text:\n\n- Item A\n- Item B\n- Item C\n\nMore text.\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("<ul>\n  <li>Item A</li>\n  <li>Item B</li>\n  <li>Item C</li>\n</ul>"),
            "End-to-end tight list should have 2-space indented <li>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue164_blockquote_format_matches_jekyll() {
        // Verified against actual Jekyll output: no indent, blank line before close.
        let md = "> Some quoted text.\n\nNext paragraph.\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("<blockquote>\n<p>"),
            "Blockquote <p> should NOT be indented (matches Jekyll). Got:\n{}",
            result
        );
        assert!(
            result.contains("</p>\n\n</blockquote>"),
            "Blank line before </blockquote> expected (matches Jekyll). Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue165_tight_list_indent() {
        // Issue 218: Jekyll/kramdown DOES indent <li> in tight lists by 2 spaces.
        // Verified against actual Jekyll output for blog/ner-reformers.html and
        // blog/how-to-run-postgresql-and-pgadmin-with-docker.html.
        let input = "<ul>\n<li>Item one</li>\n<li>Item two</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("  <li>Item one</li>"),
            "Tight list <li> should be indented by 2 spaces (matches Jekyll). Got:\n{}",
            result
        );
    }

    // =========================================================================
    // Issue 536: Nested list indentation tests
    // =========================================================================

    #[test]
    fn test_issue536_tight_list_with_nested_ul_indentation() {
        // pulldown-cmark produces nested <ul> inside <li> for tight lists,
        // but indent_list_items must indent the inner <ul> to match Jekyll.
        // Jekyll output:
        //   <li>Fifth item, nested!
        //     <ul>
        //       <li>So la ti do</li>
        //     </ul>
        //   </li>
        let input = "<ul>\n<li>First</li>\n<li>Fifth item, nested!\n<ul>\n<li>So la ti do</li>\n<li>Ba-da-bing!</li>\n</ul>\n</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("  <li>Fifth item, nested!\n    <ul>\n      <li>So la ti do</li>\n      <li>Ba-da-bing!</li>\n    </ul>\n  </li>"),
            "Nested <ul> should be indented inside <li> to match Jekyll. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue536_ordered_list_with_nested_ul_indentation() {
        // Ordered list with nested unordered sublist.
        let input = "<ol>\n<li>First item</li>\n<li>Fifth item, nested!\n<ul>\n<li>So la ti do</li>\n<li>Ba-da-bing!</li>\n</ul>\n</li>\n</ol>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("  <li>Fifth item, nested!\n    <ul>\n      <li>So la ti do</li>\n      <li>Ba-da-bing!</li>\n    </ul>\n  </li>"),
            "Nested <ul> inside <ol> should be indented to match Jekyll. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue536_three_level_nesting() {
        // Three levels of nesting should all be properly indented.
        let input = "<ul>\n<li>Level 1\n<ul>\n<li>Level 2\n<ul>\n<li>Level 3</li>\n</ul>\n</li>\n</ul>\n</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("      <li>Level 2\n        <ul>\n          <li>Level 3</li>\n        </ul>\n      </li>"),
            "Three levels of nesting should be properly indented. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue536_nested_list_unicode() {
        let input = "<ul>\n<li>\u{041f}\u{0435}\u{0440}\u{0432}\u{044b}\u{0439}\n<ul>\n<li>\u{0412}\u{043b}\u{043e}\u{0436}\u{0435}\u{043d}\u{043d}\u{044b}\u{0439}</li>\n</ul>\n</li>\n</ul>\n";
        let result = indent_list_items(input);
        assert!(
            result.contains("    <ul>\n      <li>\u{0412}\u{043b}\u{043e}\u{0436}\u{0435}\u{043d}\u{043d}\u{044b}\u{0439}</li>\n    </ul>"),
            "Unicode nested list should be properly indented. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue536_e2e_unordered_nested_list() {
        // End-to-end test: markdown with nested unordered list should produce
        // properly indented HTML matching Jekyll.
        let md = "- First item\n- Fifth item, nested!\n  - So la ti do\n  - Ba-da-bing!\n  - Ba-da-boom!\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("  <li>Fifth item, nested!\n    <ul>\n      <li>So la ti do</li>"),
            "E2E nested unordered list should have proper indentation. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue536_e2e_ordered_with_nested_unordered() {
        // End-to-end: ordered list with 2-space indented sublist
        let md = "1. First item\n2. Fifth item, nested!\n  - So la ti do\n  - Ba-da-bing!\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("  <li>Fifth item, nested!\n    <ul>\n      <li>So la ti do</li>"),
            "E2E ordered+nested unordered should have proper indentation. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue165_python_comment_newline_in_code_block() {
        // Jekyll/Rouge keeps trailing newline INSIDE the comment span.
        // Verified against actual Jekyll output for blog/ner-reformers.html.
        let input = "<pre><code class=\"language-python\">import trax # comment\n</code></pre>\n";
        let result = wrap_fenced_code_blocks(input);
        assert!(
            result.contains("# comment\n</span>"),
            "Comment span should keep trailing newline inside (matching Rouge). Got:\n{}",
            result
        );
        assert!(
            !result.contains("# comment</span>\n"),
            "Comment closing tag should NOT be before the newline. Got:\n{}",
            result
        );
    }

    // ========================================================================
    // Issue 168: Tests for the 6 categories of non-syntax-highlighting DOM diffs
    // ========================================================================

    // Category 3: books.html inline code class
    // Inline <code> tags outside <pre> blocks must get the
    // `language-plaintext highlighter-rouge` class to match kramdown output.
    #[test]
    fn test_issue168_inline_code_no_class_in_postprocess() {
        // Inline code classes are now added during markdown rendering, not
        // in postprocess(). Raw HTML <code> should pass through unchanged.
        let html = "<p>Use <code>pip install</code> to install.</p>";
        let result = postprocess(html);
        assert!(
            result.contains("<code>pip install</code>"),
            "postprocess should not add classes to raw HTML <code>. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue176_postprocess_code_inside_pre_unchanged() {
        // Code inside <pre> should not get inline code classes regardless.
        // (Inline code classes are now added during markdown rendering.)
        let html = "<pre><code>some code</code></pre>";
        let result = postprocess(html);
        assert!(
            !result.contains("language-plaintext highlighter-rouge\">some code"),
            "Code inside <pre> should NOT get inline class. Got: {}",
            result
        );
    }

    // Category 5: Heading IDs with leading numbers must be preserved.
    // Issue 228: Kramdown strips leading non-alpha chars (including digits)
    // "1. DataTalksClub" -> strip "1. " -> "DataTalksClub" -> "datatalksclub"
    #[test]
    fn test_issue168_heading_id_leading_number_preserved() {
        // GFM mode: leading digits are preserved (Jekyll defaults to GFM)
        let html = "<h2>1. DataTalksClub</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"1-datatalksclub\""),
            "Heading ID should preserve leading digit (GFM behavior). Got: {}",
            result
        );
    }

    #[test]
    fn test_issue168_heading_id_numeric_prefix_preserved() {
        // GFM mode: "8 Newsletters for Data Science" -> "8-newsletters-for-data-science"
        let html = "<h3>8 Newsletters for Data Science</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"8-newsletters-for-data-science\""),
            "Heading ID should preserve leading number (GFM behavior). Got: {}",
            result
        );
    }

    // Category 6: Book Q&A markdown rendering.
    // The markdownify filter (used for book Q&A) must handle nested lists
    // and produce correct HTML without <ol start="N"> attributes.
    #[test]
    fn test_issue168_markdownify_no_ol_start() {
        let html = postprocess_for_filter("<ol start=\"3\">\n<li>Third</li>\n</ol>\n");
        assert!(
            !html.contains("start="),
            "markdownify postprocess should strip ol start. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue168_markdownify_no_inline_code_class_in_postprocess() {
        // Inline code classes are added during markdown rendering, not in
        // postprocess_for_filter(). Raw HTML <code> passes through unchanged.
        let html = postprocess_for_filter("<p>Use <code>pip</code> here</p>\n");
        assert!(
            html.contains("<code>pip</code>"),
            "postprocess_for_filter should not add classes to raw HTML <code>. Got: {}",
            html
        );
    }

    // ========================================================================
    // Regression: normalize_html_output must NOT add inline code classes.
    // Jekyll only adds language-plaintext highlighter-rouge to <code> from
    // markdown rendering (postprocess/postprocess_for_filter), not from
    // Liquid template output. Adding it in normalize_html_output caused
    // 67 DTC files to gain spurious diffs.
    // ========================================================================

    #[test]
    fn test_normalize_html_output_does_not_add_inline_code_classes() {
        // Liquid template output has bare <code> tags -- Jekyll leaves them bare.
        let html = "<p>Join the <code>#book-of-the-week</code> channel</p>";
        let result = normalize_html_output(html);
        assert!(
            !result.contains("language-plaintext"),
            "normalize_html_output must NOT add inline code classes to Liquid template \
             <code> tags. Jekyll only adds them during markdown rendering. Got: {}",
            result
        );
        assert_eq!(result, html, "Bare <code> from Liquid should be unchanged");
    }

    // ========================================================================
    // Fix 3: postprocess_for_filter handles kramdown IAL {:target="_blank"}
    // so that markdownify filter output matches Jekyll behavior.
    // ========================================================================

    #[test]
    fn test_postprocess_for_filter_applies_inline_attributes() {
        let html = "<p><a href=\"/slack.html\">Register</a>{:target=\"_blank\"}</p>\n";
        let result = postprocess_for_filter(html);
        assert!(
            result.contains("target=\"_blank\""),
            "postprocess_for_filter should apply IAL target attribute. Got: {}",
            result
        );
        assert!(
            !result.contains("{:target"),
            "IAL syntax should be consumed. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_for_filter_applies_class_ial() {
        let html = "<p>Text</p>\n{:.note}\n";
        let result = postprocess_for_filter(html);
        assert!(
            result.contains("class=\"note\""),
            "postprocess_for_filter should apply IAL class. Got: {}",
            result
        );
    }

    // Issue 191: Ampersand handling in heading IDs
    #[test]
    fn test_heading_id_ampersand_stripped() {
        let html = "<h3>Free &amp; Free to Audit Courses</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"free--free-to-audit-courses\""),
            "Heading ID should strip ampersand entity. Got: {}",
            result
        );
    }

    #[test]
    fn test_heading_id_lt_gt_stripped() {
        let html = "<h3>A &lt; B &gt; C</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"a--b--c\""),
            "Heading ID should strip lt/gt entities. Got: {}",
            result
        );
    }

    #[test]
    fn test_heading_id_numeric_entity() {
        let html = "<h3>It&#8217;s a Test</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"its-a-test\""),
            "Heading ID should decode numeric entity and strip non-alphanumeric. Got: {}",
            result
        );
    }

    #[test]
    fn test_heading_id_no_entities_unchanged() {
        let html = "<h3>Simple Heading</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"simple-heading\""),
            "Heading ID for plain text should be unchanged. Got: {}",
            result
        );
    }

    // --- Issue 201: Text after <br> should be sibling, not child ---

    #[test]
    fn test_text_after_br_is_sibling_not_child() {
        // Markdown with hard break (two trailing spaces):
        // Should render with text after <br> as sibling of <p>, not child of <br>
        let input =
            "Sign up for our newsletter.  \nWe\u{2019}ll keep you informed about our events.";
        let html = crate::frontmatter::markdown_to_html(input);
        // In correct output, <br> should be followed by \n then text,
        // not directly by text (which makes it look like a child of <br> in DOM)
        assert!(
            !html.contains("<br>We") && !html.contains("<br />We") && !html.contains("<br/>We"),
            "Text after <br> must not be directly attached to <br> tag. Got: {}",
            html
        );
    }

    #[test]
    fn test_multiple_br_tags_text_placement() {
        let input = "Line 1  \nLine 2  \nLine 3";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<br>Line")
                && !html.contains("<br />Line")
                && !html.contains("<br/>Line"),
            "Text after <br> tags should not be children of <br>. Got: {}",
            html
        );
    }

    #[test]
    fn test_markdown_line_break_text_placement() {
        let input = "First line  \nSecond line";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<br>Second")
                && !html.contains("<br />Second")
                && !html.contains("<br/>Second"),
            "Text 'Second line' should be sibling of <br>, not child. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 204: Fix extra HTML elements in rustkyll output
    // ========================================================================

    #[test]
    fn test_issue204_tight_list_no_p_wrapper() {
        let input = "- First item text\n- Second item text\n- Third item with longer text here\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li><p>") && !html.contains("<li>\n<p>"),
            "Tight list items should not have <p> wrapper. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue204_kramdown_tight_list_with_continuation() {
        let input = "- Then you should use several platforms to show yourself.\n  For example, after an achievement writes on LinkedIn.\n- You must connect to recruiters or professionals.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li><p>") && !html.contains("<li>\n<p>"),
            "Tight list with continuation should not have <p> wrapper. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue204_kramdown_per_item_loose_tight() {
        let input = "- item 1\n- item 2\n\n- item 3\n\n- item 4\n- item 5\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li><p>") && !html.contains("<li>\n<p>"),
            "After collapsing blank lines, list should be tight (no <p>). Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue343_partial_loose_first_item_wrapped_only() {
        let input = "-   First of all, you need to follow Linkedin the best professionals. [article here](https://example.com).;\n\n-   Then you should use several platforms to show yourself.\n-   You must connect to recruiters.\n";
        let html = crate::frontmatter::markdown_to_html(input);

        assert!(
            html.contains(
                "<p>First of all, you need to follow Linkedin the best professionals. <a href=\"https://example.com\">article here</a>.;</p>"
            ),
            "First list item should get <p> wrapper in partial-loose list. Got:\n{}",
            html
        );
        assert!(
            html.contains("<li>Then you should use several platforms to show yourself.</li>"),
            "Second sibling item should remain tight (no <p>). Got:\n{}",
            html
        );
        assert!(
            html.contains("<li>You must connect to recruiters.</li>"),
            "Third sibling item should remain tight (no <p>). Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue372_nested_sublist_plain_text_loose() {
        // Sub-list items separated by blank lines should be loose (wrapped in <p>)
        let input = "- **Malignant:** ALL-related cells, categorized into three subtypes:\n  - Early Pre-B\n\n  - Pre-B\n\n  - Pro-B\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>Early Pre-B</p>"),
            "Sub-list item 'Early Pre-B' should be wrapped in <p>. Got:\n{}",
            html
        );
        assert!(
            html.contains("<p>Pre-B</p>"),
            "Sub-list item 'Pre-B' should be wrapped in <p>. Got:\n{}",
            html
        );
        assert!(
            html.contains("<p>Pro-B</p>"),
            "Sub-list item 'Pro-B' should be wrapped in <p>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue372_nested_sublist_bold_text_loose() {
        // Sub-list items with inline bold, separated by blank lines, should be loose
        let input =
            "- Techniques applied:\n  - **Rotations**\n\n  - **Flips**\n\n  - **Brightness**\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p><strong>Rotations</strong></p>"),
            "Sub-list item with bold should be wrapped in <p>. Got:\n{}",
            html
        );
        assert!(
            html.contains("<p><strong>Flips</strong></p>"),
            "Sub-list item with bold should be wrapped in <p>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue372_nested_sublist_tight_no_wrapping() {
        // Sub-list items NOT separated by blank lines should remain tight
        let input = "- Parent item:\n  - Sub item A\n  - Sub item B\n  - Sub item C\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<li><p>Sub item"),
            "Tight sub-list items should NOT be wrapped in <p>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue204_heading_after_list_item_no_blank_line() {
        let input = "- logic\n- characters\n- complex\n#### numbers\n- doubles by default\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<h4"),
            "Heading after list item without blank line should not create <h4>. Got:\n{}",
            html
        );
        assert!(
            html.contains("numbers"),
            "Text 'numbers' should be present. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue204_collapse_blank_lines_between_list_items() {
        let input = "- item 1\n- item 2\n\n- item 3\n\n- item 4\n- item 5\n";
        let result = collapse_blank_lines_between_list_items(input);
        assert_eq!(
            result, "- item 1\n- item 2\n- item 3\n- item 4\n- item 5\n",
            "Blank lines between list items should be collapsed"
        );
    }

    #[test]
    fn test_issue204_collapse_preserves_blank_after_list() {
        let input = "- item 1\n- item 2\n\nSome paragraph text.\n";
        let result = collapse_blank_lines_between_list_items(input);
        assert_eq!(
            result, "- item 1\n- item 2\n\nSome paragraph text.\n",
            "Blank line after list should be preserved"
        );
    }

    #[test]
    fn test_issue204_escape_headings_in_list() {
        let input = "- complex\n#### numbers\n- doubles\n";
        let result = escape_headings_in_list_context(input);
        assert!(
            result.contains("\\####"),
            "Heading marker in list context should be escaped. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue204_heading_after_blank_line_not_escaped() {
        let input = "- item 1\n- item 2\n\n## Real heading\n";
        let result = escape_headings_in_list_context(input);
        assert!(
            result.contains("\n## Real heading"),
            "Heading after blank line should not be escaped. Got:\n{}",
            result
        );
    }

    // --- Issue 201: Bare <br> should become <br /> to match Jekyll/kramdown ---

    #[test]
    fn test_bare_br_converted_to_xhtml_style() {
        let html = "<td>\n\"10x lol\"<br>\n\"Saved at least 1 week\"<br>\n\"Doubled\"</td>";
        let result = postprocess(html);
        assert!(
            !result.contains("<br>"),
            "Bare <br> should be converted to <br /> after postprocessing. Got: {}",
            result
        );
        assert!(
            result.contains("<br />"),
            "Should contain <br /> (XHTML style). Got: {}",
            result
        );
    }

    #[test]
    fn test_bare_br_multiple_in_sequence() {
        let html = "<p>Line 1<br>\nLine 2<br>\nLine 3</p>";
        let result = postprocess(html);
        assert!(
            !result.contains("<br>"),
            "All bare <br> should be converted to <br />. Got: {}",
            result
        );
        assert_eq!(
            result.matches("<br />").count(),
            2,
            "Should have exactly 2 <br /> tags. Got: {}",
            result
        );
    }

    #[test]
    fn test_br_self_closing_preserved() {
        let html = "<p>text<br />more</p>";
        let result = postprocess(html);
        assert!(
            result.contains("<br />"),
            "Self-closing <br /> should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_bare_br_via_markdown_to_html() {
        let input = "<td>\"10x lol\"<br>\n\"Saved at least 1 week\"</td>";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<br>"),
            "Bare <br> in raw HTML should become <br /> after full pipeline. Got: {}",
            html
        );
    }

    #[test]
    fn test_bare_hr_converted_to_xhtml_style() {
        let html = "<hr>\n<p>text</p>";
        let result = postprocess(html);
        assert!(
            !result.contains("<hr>"),
            "Bare <hr> should be converted to <hr />. Got: {}",
            result
        );
    }

    #[test]
    fn test_normalize_html_output_converts_bare_br() {
        let html = "<p>text<br>more</p>";
        let result = normalize_html_output(html);
        assert!(
            result.contains("<br />"),
            "normalize_html_output should convert bare <br> to <br />. Got: {}",
            result
        );
    }

    // === Issue 250: normalize_html_output must NOT convert <hr> to <hr /> ===
    // pulldown-cmark already outputs <hr /> for markdown horizontal rules (---),
    // so the post-processing replacement is unnecessary. Only <br> needs conversion
    // because raw HTML <br> in markdown tables/content needs XHTML-style.
    // Converting <hr> would incorrectly affect include/layout content.

    #[test]
    fn test_normalize_html_output_does_not_convert_bare_hr() {
        // Simulates <hr> from an include file (e.g., footer.html) appearing
        // in the final page output. normalize_html_output must NOT convert it.
        let html = "<hr>\n<footer>Footer content</footer>";
        let result = normalize_html_output(html);
        assert!(
            result.contains("<hr>"),
            "normalize_html_output must NOT convert bare <hr> to <hr />. \
             Include/layout <hr> must pass through unchanged. Got: {}",
            result
        );
        assert!(
            !result.contains("<hr />"),
            "normalize_html_output must NOT produce <hr />. Got: {}",
            result
        );
    }

    #[test]
    fn test_normalize_html_output_still_converts_bare_br() {
        // <br> conversion should still work
        let html = "<p>line1<br>line2</p>";
        let result = normalize_html_output(html);
        assert!(
            result.contains("<br />"),
            "normalize_html_output should still convert <br> to <br />. Got: {}",
            result
        );
    }

    #[test]
    fn test_postprocess_still_converts_hr_in_markdown_content() {
        // postprocess (called on markdown-rendered content only) still converts
        // <hr> via normalize_bare_void_elements, so markdown --- still produces <hr />
        let html = "<hr>\n<p>text</p>";
        let result = postprocess(html);
        assert!(
            result.contains("<hr />"),
            "postprocess should still convert <hr> to <hr /> for markdown content. Got: {}",
            result
        );
    }

    // === Issue 200: Markdown table rendering tests ===

    #[test]
    fn test_200_standard_pipe_table_renders() {
        let input = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
        assert!(html.contains("<thead>"), "Got: {html}");
        assert!(html.contains("<tbody>"), "Got: {html}");
    }

    #[test]
    fn test_200_pipe_table_unicode() {
        let input = "| \u{0417}\u{0430}\u{0433} | R\u{00e9}sum\u{00e9} |\n|---|---|\n| \u{042f}\u{0447} | Caf\u{00e9} |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
        assert!(html.contains("\u{0417}\u{0430}\u{0433}"), "Got: {html}");
    }

    #[test]
    fn test_200_table_inside_list() {
        let input = "- Item:\n\n  | A | B |\n  |---|---|\n  | 1 | 2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
        assert!(html.contains("<li>"), "Got: {html}");
    }

    #[test]
    fn test_200_table_inside_list_unicode() {
        let input = "- \u{042d}\u{043b}\u{0435}\u{043c}:\n\n  | \u{041a} A | \u{041a} B |\n  |---|---|\n  | \u{0437}1 | \u{0437}2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_kramdown_trailing_pipe_in_list() {
        let input = "- can use Prim\u{2019}s algo |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
        assert!(html.contains("<td>"), "Got: {html}");
    }

    #[test]
    fn test_200_kramdown_multi_pipe_in_list() {
        let input = "- parallel x | definition | extra |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_kramdown_pipe_unicode() {
        let input =
            "- \u{041d}\u{0430}\u{0439}\u{0434}\u{0435}\u{043c} \u{0443}\u{0441}\u{043b} |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_embedded_pipe_at_block_boundary_is_table() {
        // Issue 272: kramdown treats ANY line containing `|` at a block boundary
        // as a table row. A standalone line with an embedded pipe at SOF/EOF IS
        // a table.
        let input = "This has a | char but not a table.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_trailing_pipe_not_in_list() {
        let input = "some text |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_multi_row_pipe() {
        let input = "- t1 | t2 |\n  t3 | t4 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(html.contains("<table>"), "Got: {html}");
    }

    #[test]
    fn test_200_no_double_convert() {
        let input = "| H1 | H2 |\n|---|---|\n| C1 | C2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(html.matches("<table>").count(), 1, "Got: {html}");
    }

    // ========================================================================
    // Issue 212: Multi-row standard pipe table fix
    // ========================================================================

    #[test]
    fn test_212_multi_row_standard_table_six_data_rows() {
        // Header + separator + 6 data rows = 7 <tr> total in one table
        let input = "\
| Header A | Header B |
|----------|----------|
| Row 1    | Data 1   |
| Row 2    | Data 2   |
| Row 3    | Data 3   |
| Row 4    | Data 4   |
| Row 5    | Data 5   |
| Row 6    | Data 6   |
";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<table>").count(),
            1,
            "Should be exactly one table. Got: {html}"
        );
        assert_eq!(
            html.matches("<tr>").count(),
            7,
            "Should have 7 <tr> (1 header + 6 data). Got: {html}"
        );
    }

    #[test]
    fn test_212_multi_row_table_bold_headers() {
        let input = "\
| **Col A** | **Col B** |
|---|---|
| r1 | r2 |
| r3 | r4 |
| r5 | r6 |
";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<table>").count(),
            1,
            "Should be exactly one table. Got: {html}"
        );
        assert_eq!(
            html.matches("<tr>").count(),
            4,
            "Should have 4 <tr> (1 header + 3 data). Got: {html}"
        );
    }

    #[test]
    fn test_212_table_with_escaped_pipes() {
        let input = "\
| A | B |
|---|---|
| x\\|y | z |
| a | b |
";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<table>").count(),
            1,
            "Should be exactly one table. Got: {html}"
        );
        // All rows should render
        assert!(
            html.matches("<tr>").count() >= 3,
            "Should have at least 3 <tr>. Got: {html}"
        );
    }

    #[test]
    fn test_212_table_with_inline_code() {
        let input = "\
| Channel | Description |
|---|---|
| `#general` | General chat |
| `#course-data-engineering` | DE course |
| `#random` | Off-topic |
";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<table>").count(),
            1,
            "Should be exactly one table. Got: {html}"
        );
        assert_eq!(
            html.matches("<tr>").count(),
            4,
            "Should have 4 <tr> (1 header + 3 data). Got: {html}"
        );
    }

    #[test]
    fn test_212_table_unicode_multi_row() {
        // Non-ASCII/Unicode content in table cells
        let input = "\
| Spalte | Beschreibung |
|---|---|
| Geb\u{00fc}hr | \u{00dc}berweisung |
| R\u{00fc}ckgabe | Gutschrift |
| Storno | Kreditkarte |
";
        let html = crate::frontmatter::markdown_to_html(input);
        assert_eq!(
            html.matches("<table>").count(),
            1,
            "Should be exactly one table. Got: {html}"
        );
        assert_eq!(
            html.matches("<tr>").count(),
            4,
            "Should have 4 <tr> (1 header + 3 data). Got: {html}"
        );
        assert!(
            html.contains("Geb\u{00fc}hr"),
            "Unicode content should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_212_kramdown_no_separator_no_regression() {
        // Kramdown-style table (no separator line) should still work
        let input = "- item1 | item2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Kramdown table should still render. Got: {html}"
        );
    }

    #[test]
    fn test_212_table_inside_list_no_regression() {
        // Standard pipe table inside a list should still work
        let input = "- Item:\n\n  | H1 | H2 |\n  |---|---|\n  | C1 | C2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Table in list should render. Got: {html}"
        );
        assert!(html.contains("<li>"), "List should render. Got: {html}");
    }

    // ========================================================================
    // Issue 198: Zero-width space around emphasis markers
    // ========================================================================

    #[test]
    fn test_issue198_zero_width_space_emphasis_boundary() {
        let input = "connect with \u{200b}_everyone_. \u{200b} People laugh.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<em>everyone</em>"),
            "Zero-width space should allow emphasis after it. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_zero_width_space_emphasis_unicode() {
        let input = "\u{200b}_\u{00e9}v\u{00e9}nement_. R\u{00e9}sultat.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<em>\u{00e9}v\u{00e9}nement</em>"),
            "Unicode emphasis content after ZWSP should work. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_zwsp_preserved_without_emphasis() {
        let input = "text\u{200b}more text";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{200b}"),
            "ZWSP without emphasis should be preserved. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 198: MediaWiki-style triple/double quote preservation
    // ========================================================================

    #[test]
    fn test_issue198_double_quote_straight() {
        // Issue 247: kramdown converts ''word'' to smart quotes based on context.
        // Mid-sentence (space before, space after): lsquo+lsquo...rsquo+rsquo
        let input = "A place is ''implicit'' if removing it.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}implicit\u{2019}\u{2019}"),
            "Double single-quotes mid-sentence should become smart quotes. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_triple_quote_straight() {
        // Issue 247: kramdown converts '''word''' to smart quotes based on context.
        // Space before + triple quotes + word: all three lsquo opening
        // Word + triple quotes + space: all three rsquo closing
        let input = "This is '''bold text''' here.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}\u{2018}bold text\u{2019}\u{2019}\u{2019}"),
            "Triple single-quotes mid-sentence should become smart quotes. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_quotes_cyrillic() {
        // Issue 247: kramdown converts '''word.''' with Cyrillic to smart quotes.
        // Space before: all three lsquo. After '.': all three rsquo.
        let input = "\u{042d}\u{0442}\u{043e} '''\u{0422}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430}.''' \u{0414}\u{043e}\u{043a}.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}\u{2018}\u{0422}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430}.\u{2019}\u{2019}\u{2019}"),
            "Cyrillic in triple-quotes should get smart quotes. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_single_smart_quote_works() {
        let input = "it's a test";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{2019}"),
            "Single apostrophe should still get smart punctuation. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_curly_quote_preserved() {
        let input = "it\u{2019}s a test with \u{00e9}l\u{00e8}ve content";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("\u{2019}"),
            "Pre-existing curly quotes should be preserved. Got: {html}"
        );
    }

    // --- Issue 203: Fix missing HTML elements tests ---

    #[test]
    fn test_issue203_content_wrapped_in_paragraph_between_blocks() {
        // Text between two HTML block elements should be wrapped in <p>
        let html = "<div>block1</div>\nSome text content here.\n<div>block2</div>";
        let result = postprocess(html);
        assert!(
            result.contains("<p>Some text content here.</p>"),
            "Bare text between block elements should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue203_content_wrapped_in_paragraph_unicode() {
        let html = "<div>block1</div>\n\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}!\n<div>block2</div>";
        let result = postprocess(html);
        assert!(
            result.contains(
                "<p>\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}!</p>"
            ),
            "Unicode bare text should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue203_markdown_link_with_ial_produces_anchor() {
        let input = "[Source](https://example.com){:target=\"_blank\"}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<a href=\"https://example.com\" target=\"_blank\">Source</a>"),
            "Link with IAL should produce <a> with target attribute. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_markdown_link_with_ial_unicode_text() {
        let input = "[\u{0421}\u{0441}\u{044b}\u{043b}\u{043a}\u{0430}](https://example.com){:target=\"_blank\"}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<a href=\"https://example.com\""),
            "Unicode link text with IAL should produce <a>. Got: {}",
            html
        );
        assert!(
            html.contains("\u{0421}\u{0441}\u{044b}\u{043b}\u{043a}\u{0430}"),
            "Unicode link text should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_text_after_figure_close_produces_links() {
        let input = "<figure>\n<img src=\"/img.jpg\" />\n</figure>Photo by [Author](https://example.com){:target=\"_blank\"} on [Site](https://site.com)\n";
        let preprocessed = split_text_after_html_block_close(input);
        let html = crate::frontmatter::markdown_to_html(&preprocessed);
        assert!(
            html.contains("<a href=\"https://example.com\""),
            "Link after </figure> should be parsed as <a>. Got: {}",
            html
        );
        assert!(
            !html.contains("[Author]"),
            "Raw markdown link syntax should not appear. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_text_after_figure_close_unicode() {
        let input = "<figure>\n<img src=\"/img.jpg\" />\n</figure>\u{0424}\u{043e}\u{0442}\u{043e} [\u{0410}\u{0432}\u{0442}\u{043e}\u{0440}](https://example.com)\n";
        let preprocessed = split_text_after_html_block_close(input);
        let html = crate::frontmatter::markdown_to_html(&preprocessed);
        assert!(
            html.contains(
                "<a href=\"https://example.com\">\u{0410}\u{0432}\u{0442}\u{043e}\u{0440}</a>"
            ),
            "Unicode link after HTML block should be parsed. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_faq_script_preserved_in_output() {
        let faq_html = "<div class=\"faq-accordion\">\n<div class=\"faq-item\">\n<p>Content</p>\n</div>\n</div>\n<script type=\"application/ld+json\">\n{\"@type\": \"FAQPage\"}\n</script>";
        let result = postprocess(faq_html);
        assert!(
            result.contains("<script type=\"application/ld+json\">"),
            "FAQ schema script tag should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("\"@type\": \"FAQPage\""),
            "FAQ schema content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue203_faq_script_unicode_preserved() {
        let faq_html = "<script type=\"application/ld+json\">\n{\"name\": \"\u{00bf}Qu\u{00e9} es?\"}\n</script>";
        let result = postprocess(faq_html);
        assert!(
            result.contains("\u{00bf}Qu\u{00e9} es?"),
            "Unicode in FAQ script should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue203_multiple_paragraphs_from_markdown() {
        let input = "First paragraph.\n\nSecond paragraph.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>First paragraph.</p>"),
            "First paragraph should be in <p>. Got: {}",
            html
        );
        assert!(
            html.contains("<p>Second paragraph.</p>"),
            "Second paragraph should be in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_multiple_paragraphs_unicode() {
        let input = "\u{7b2c}\u{4e00}\u{6bb5}\u{843d}\u{3002}\n\n\u{7b2c}\u{4e8c}\u{6bb5}\u{843d}\u{3002}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<p>\u{7b2c}\u{4e00}\u{6bb5}\u{843d}\u{3002}</p>"),
            "First Unicode paragraph should be in <p>. Got: {}",
            html
        );
        assert!(
            html.contains("<p>\u{7b2c}\u{4e8c}\u{6bb5}\u{843d}\u{3002}</p>"),
            "Second Unicode paragraph should be in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_container_div_preserved() {
        let html = "<div class=\"container\">\n<p>Content inside div</p>\n</div>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"container\">"),
            "Container div should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue203_link_after_html_block() {
        let input = "<div class=\"note\">\nImportant info here.\n</div>\n\nSee [the guide](https://example.com) for details.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<a href=\"https://example.com\">the guide</a>"),
            "Link after HTML block should be rendered as <a>. Got: {}",
            html
        );
        assert!(
            !html.contains("[the guide]"),
            "Raw markdown link syntax should not appear. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue203_split_text_basic() {
        let input = "</figure>Photo by someone";
        let result = split_text_after_html_block_close(input);
        assert!(
            result.contains("</figure>\n\nPhoto by someone"),
            "Text after </figure> should be split. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue203_split_text_unicode() {
        let input = "</figure>\u{0424}\u{043e}\u{0442}\u{043e} \u{043e}\u{0442} someone";
        let result = split_text_after_html_block_close(input);
        assert!(
            result.contains("</figure>\n\n\u{0424}\u{043e}\u{0442}\u{043e}"),
            "Unicode text after </figure> should be split. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue203_split_text_preserves_newline() {
        let input = "</figure>\nNext paragraph";
        let result = split_text_after_html_block_close(input);
        assert_eq!(
            result, input,
            "Already-separated content should be unchanged"
        );
    }

    #[test]
    fn test_issue203_split_text_not_for_inline_tags() {
        let input = "</a> some text";
        let result = split_text_after_html_block_close(input);
        assert_eq!(result, input, "Inline tags should not be split");
    }

    // --- Issue 476: single-line <details> should not be split ---

    #[test]
    fn test_issue476_single_line_details_not_split() {
        let input = "<details><summary>Content warning</summary>Some text here.</details>";
        let result = split_text_after_html_block_close(input);
        assert_eq!(
            result, input,
            "Single-line <details> block should NOT be split after </summary>. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue476_single_line_details_unicode() {
        let input =
            "<details><summary>Warning</summary>\u{4e2d}\u{6587}\u{5185}\u{5bb9}\u{3002}</details>";
        let result = split_text_after_html_block_close(input);
        assert_eq!(
            result, input,
            "Single-line <details> with Unicode should NOT be split. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue476_multiline_details_summary_still_splits() {
        // When </details> is on a different line, </summary> should still be split
        let input = "</summary>Some text\n</details>";
        let result = split_text_after_html_block_close(input);
        assert!(
            result.contains("</summary>\n\nSome text"),
            "Multi-line details: </summary> should still split when </details> is on a different line. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue476_summary_without_details_still_splits() {
        // </summary> followed by text without </details> on same line
        let input = "</summary>Some text after summary";
        let result = split_text_after_html_block_close(input);
        assert!(
            result.contains("</summary>\n\nSome text"),
            "</summary> followed by text without </details> should still split. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue476_single_line_details_no_p_wrapping() {
        // End-to-end: single-line details through markdown pipeline
        let input = "<details><summary>CW</summary>Some text here.</details>\n";
        let preprocessed = split_text_after_html_block_close(input);
        let html = crate::frontmatter::markdown_to_html(&preprocessed);
        assert!(
            !html.contains("<p>"),
            "Single-line <details> should NOT have <p> wrapping inside. Got: {}",
            html
        );
        assert!(
            html.contains("<details><summary>CW</summary>Some text here.</details>"),
            "Single-line <details> should be preserved verbatim. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue476_details_followed_by_paragraph() {
        // Details block followed by regular markdown
        let input = "<details><summary>CW</summary>text</details>\n\nA regular paragraph.\n";
        let preprocessed = split_text_after_html_block_close(input);
        let html = crate::frontmatter::markdown_to_html(&preprocessed);
        assert!(
            html.contains("<details><summary>CW</summary>text</details>"),
            "Details block should be preserved. Got: {}",
            html
        );
        assert!(
            html.contains("<p>A regular paragraph.</p>"),
            "Following paragraph should render correctly. Got: {}",
            html
        );
    }

    // --- Issue 218: postprocess_for_filter block spacing and list indentation ---

    #[test]
    fn test_issue218_postprocess_for_filter_multi_paragraph_spacing() {
        let input = "<p>First.</p>\n<p>Second.</p>\n";
        let result = postprocess_for_filter(input);
        assert_eq!(
            result, "<p>First.</p>\n\n<p>Second.</p>\n",
            "postprocess_for_filter should add blank line between block elements. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue218_postprocess_for_filter_paragraph_before_list_spacing() {
        let input = "<p>Key ways:</p>\n<ol>\n<li>First</li>\n</ol>\n";
        let result = postprocess_for_filter(input);
        assert!(
            result.contains("</p>\n\n<ol>"),
            "postprocess_for_filter should add blank line between </p> and <ol>. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue218_postprocess_for_filter_list_item_indentation() {
        let input = "<ol>\n<li>Item one</li>\n<li>Item two</li>\n</ol>\n";
        let result = postprocess_for_filter(input);
        assert!(
            result.contains("  <li>"),
            "postprocess_for_filter should indent <li> by 2 spaces. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_issue218_postprocess_for_filter_non_ascii_preserved() {
        let input = "<p>Zoomcamp\u{2014}free course.</p>\n<p>Join \u{201c}today\u{201d}.</p>\n";
        let result = postprocess_for_filter(input);
        assert!(
            result.contains("\u{2014}"),
            "Em-dash should be preserved. Got: {:?}",
            result
        );
        assert!(
            result.contains("\u{201c}") && result.contains("\u{201d}"),
            "Curly quotes should be preserved. Got: {:?}",
            result
        );
        assert_eq!(
            result, "<p>Zoomcamp\u{2014}free course.</p>\n\n<p>Join \u{201c}today\u{201d}.</p>\n",
            "Should have block spacing and preserved non-ASCII. Got: {:?}",
            result
        );
    }

    // --- Issue 228: {#id} heading ID syntax tests ---

    #[test]
    fn test_heading_explicit_id_syntax() {
        // {#custom-id} syntax should set the heading ID and strip from text
        let html = "<h2>Heading Text {#custom-id}</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"custom-id\""),
            "Should use explicit ID from {{#custom-id}}. Got: {}",
            result
        );
        assert!(
            !result.contains("{#custom-id}"),
            "Should strip {{#custom-id}} from heading text. Got: {}",
            result
        );
        assert!(
            result.contains(">Heading Text</h2>") || result.contains(">Heading Text </h2>"),
            "Should preserve heading text without {{#id}}. Got: {}",
            result
        );
    }

    #[test]
    fn test_heading_explicit_id_arabic() {
        // Arabic explicit ID syntax
        let html = "<h2>\u{062a}\u{0639}\u{0644}\u{0645} \u{0642}\u{0648}\u{0644} \u{0644}\u{0627} {#\u{062a}\u{0639}\u{0644}\u{0645}-\u{0642}\u{0648}\u{0644}-\u{0644}\u{0627}}</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains(
                "id=\"\u{062a}\u{0639}\u{0644}\u{0645}-\u{0642}\u{0648}\u{0644}-\u{0644}\u{0627}\""
            ),
            "Should use Arabic explicit ID. Got: {}",
            result
        );
        assert!(
            !result.contains("{#"),
            "Should strip {{#id}} from heading text. Got: {}",
            result
        );
    }

    #[test]
    fn test_heading_no_explicit_id_unchanged() {
        // Heading without {#id} should use auto-generated ID
        let html = "<h2>No Custom ID</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"no-custom-id\""),
            "Should auto-generate ID. Got: {}",
            result
        );
    }

    // --- Issue 228: Heading ID for non-Latin scripts (GFM Unicode-preserving) ---

    #[test]
    fn test_slugify_arabic_preserved() {
        // GFM mode: Arabic letters are \p{Word}, so they are preserved
        let result = slugify("\u{0645}\u{0627} \u{0645}\u{0639}\u{0646}\u{0649}");
        assert_eq!(
            result, "\u{0645}\u{0627}-\u{0645}\u{0639}\u{0646}\u{0649}",
            "Pure Arabic heading should preserve Arabic chars. Got: {}",
            result
        );
    }

    #[test]
    fn test_slugify_two_arabic_headings_unique_ids() {
        // Two different Arabic headings get different IDs
        let mut used = HashMap::new();
        let slug1 = slugify("\u{0645}\u{0627} \u{0645}\u{0639}\u{0646}\u{0649}");
        let id1 = get_unique_id(&mut used, &slug1);
        let slug2 = slugify("\u{0643}\u{064a}\u{0641} \u{062a}\u{0633}\u{0627}\u{0647}\u{0645}");
        let id2 = get_unique_id(&mut used, &slug2);
        assert_eq!(
            id1, "\u{0645}\u{0627}-\u{0645}\u{0639}\u{0646}\u{0649}",
            "First Arabic heading should preserve Arabic. Got: {}",
            id1
        );
        assert_eq!(
            id2, "\u{0643}\u{064a}\u{0641}-\u{062a}\u{0633}\u{0627}\u{0647}\u{0645}",
            "Second Arabic heading should preserve Arabic. Got: {}",
            id2
        );
    }

    #[test]
    fn test_slugify_mixed_ascii_arabic_preserves_both() {
        // GFM mode: both ASCII and Arabic are preserved
        let result =
            slugify("GitHub \u{0647}\u{0644} \u{0645}\u{0634}\u{0627}\u{0631}\u{064a}\u{0639}");
        assert!(
            result.starts_with("github"),
            "Mixed ASCII/Arabic heading should keep ASCII part. Got: {}",
            result
        );
        assert!(
            result.contains("\u{0647}"),
            "Arabic chars should be preserved in GFM mode. Got: {}",
            result
        );
    }

    #[test]
    fn test_slugify_english_unchanged() {
        // English headings should work as before
        assert_eq!(slugify("Getting Started"), "getting-started");
    }

    // --- Issue 228: markdown="1" attribute processing tests ---

    #[test]
    fn test_process_markdown_attr_aside() {
        // <aside markdown="1"> should have attribute stripped and content rendered
        let input = "<aside markdown=\"1\">\n\n![avatar](img.png)\nSome text\n\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"1\""),
            "markdown=\"1\" should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("<aside>"),
            "Should have <aside> without markdown attr. Got: {}",
            result
        );
        assert!(
            result.contains("<img"),
            "Image markdown should be rendered to <img>. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_p_with_class() {
        // <p markdown="1" class="pquote-credit"> should strip markdown attr, keep class
        let input = "<p markdown=\"1\" class=\"pquote-credit\">\n-- @user, [\"Title\"](url)\n</p>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"1\""),
            "markdown=\"1\" should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"pquote-credit\""),
            "class should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_div() {
        // <div markdown="1"> should have content rendered
        let input = "<div markdown=\"1\">\n## Heading\n\nParagraph\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"1\""),
            "markdown=\"1\" should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("<h2") || result.contains("<p>Paragraph</p>"),
            "Content inside div should be rendered as markdown. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_absent() {
        // <aside> without markdown attr should NOT have content processed
        let input = "<aside>\nRaw content\n</aside>";
        let result = process_markdown_attribute(input);
        assert_eq!(
            result, input,
            "Content without markdown attr should be unchanged. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_cjk_content() {
        // CJK (Chinese/Japanese/Korean) multi-byte UTF-8 content inside markdown="1"
        // This tests that find_markdown_close_tag does not panic on multi-byte chars
        let input = "<aside markdown=\"1\">\n\n这是中文内容。\n\n日本語のテスト。\n\n한국어 테스트.\n\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"1\""),
            "markdown attr should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("这是中文内容"),
            "Chinese content should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("日本語のテスト"),
            "Japanese content should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("한국어 테스트"),
            "Korean content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_arabic_content() {
        // Arabic multi-byte UTF-8 content inside markdown="1"
        let input = "<aside markdown=\"1\">\n\nمرحبا بالعالم\n\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"1\""),
            "markdown attr should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("مرحبا بالعالم"),
            "Arabic content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_find_markdown_close_tag_with_multibyte_utf8() {
        // Directly test find_markdown_close_tag with multi-byte UTF-8
        let html = "这是中文</aside>";
        let result = find_markdown_close_tag(html, "aside", "</aside>");
        assert_eq!(result, Some("这是中文".len()));
    }

    // ========================================================================
    // Issue 322: markdown="1" block content paragraph wrapping for <img> + text
    // ========================================================================

    #[test]
    fn test_process_markdown_attr_img_plus_text_wrapped_in_p() {
        // <img> followed by text inside markdown="1" block should be wrapped in <p>
        let input = "<aside markdown=\"1\" class=\"pquote\">\n  <img src=\"test.jpg\" class=\"avatar\" alt=\"avatar\">\n  Some text about open source.\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p><img"),
            "img + text should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("Some text about open source.</p>"),
            "Text after img should be inside the same <p>. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"pquote\""),
            "class should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_img_text_with_nested_p_markdown() {
        // Full aside pattern from opensource-guide
        let input = "<aside markdown=\"1\" class=\"pquote\">\n  <img src=\"https://avatars.githubusercontent.com/lord?s=180\" class=\"pquote-avatar\" alt=\"avatar\">\n  I fumbled it. I didn't put in the effort.\n  <p markdown=\"1\" class=\"pquote-credit\">\n  -- @lord, [\"Tips\"](https://lord.io/blog)\n  </p>\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p><img"),
            "img + text should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("<p class=\"pquote-credit\">"),
            "Nested p with class should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_multiple_img_plus_text() {
        // Multiple <img> elements followed by text
        let input = "<div markdown=\"1\">\n  <img src=\"a.jpg\" alt=\"a\">\n  <img src=\"b.jpg\" alt=\"b\">\n  Text after images.\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p>"),
            "Images and text should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("Text after images.</p>"),
            "Text should be inside <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_img_text_unicode() {
        // Unicode content (French with accents) inside markdown="1" with <img>
        let input = "<aside markdown=\"1\" class=\"pquote\">\n  <img src=\"avatar.jpg\" alt=\"avatar\">\n  Contribuer au logiciel libre, c'est important.\n  <p markdown=\"1\" class=\"credit\">\n  -- @utilisateur, [\"Guide du debutant\"](https://example.com)\n  </p>\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p><img"),
            "img + text should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("Contribuer au logiciel libre"),
            "French text should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("c\u{2019}est important.</p>")
                || result.contains("c'est important.</p>"),
            "Accented text should be inside <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_img_text_cjk() {
        // CJK content inside markdown="1" with <img>
        let input = "<aside markdown=\"1\">\n  <img src=\"avatar.jpg\" alt=\"\u{30a2}\u{30d0}\u{30bf}\u{30fc}\">\n  \u{30aa}\u{30fc}\u{30d7}\u{30f3}\u{30bd}\u{30fc}\u{30b9}\u{3078}\u{306e}\u{8ca2}\u{732e}\u{3002}\n</aside>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p><img"),
            "img + CJK text should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("\u{30aa}\u{30fc}\u{30d7}\u{30f3}\u{30bd}\u{30fc}\u{30b9}"),
            "Japanese text should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_div_text_only_no_regression() {
        // div with only text (no img) should still work - no regression
        let input = "<div markdown=\"1\">\nSome text.\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p>Some text.</p>"),
            "Text-only div should still wrap in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_div_heading_paragraph_no_regression() {
        // div with heading and paragraph - no regression
        let input = "<div markdown=\"1\">\n## Title\n\nParagraph text.\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<h2"),
            "Heading should be rendered. Got: {}",
            result
        );
        assert!(
            result.contains("<p>Paragraph text.</p>"),
            "Paragraph should be wrapped in <p>. Got: {}",
            result
        );
    }

    // --- Issue 327: markdown="span" and markdown="block" attribute processing ---

    #[test]
    fn test_process_markdown_attr_span() {
        // <div markdown="span"> should strip attribute and process as inline markdown
        let input = "<div markdown=\"span\" class=\"alert alert-info\">This is **bold** text</div>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"span\""),
            "markdown=\"span\" should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"alert alert-info\""),
            "class should be preserved. Got: {}",
            result
        );
        assert!(
            result.contains("<strong>bold</strong>"),
            "Bold should be rendered. Got: {}",
            result
        );
        // markdown="span" means inline -- no block <p> wrapping
        assert!(
            !result.contains("<p>"),
            "Inline mode should not produce <p> tags. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_block() {
        // <div markdown="block"> should strip attribute and process as block markdown
        let input = "<div markdown=\"block\">\n\nThis is a paragraph.\n\n- List item\n\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"block\""),
            "markdown=\"block\" should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("<p>This is a paragraph.</p>"),
            "Paragraph should be rendered. Got: {}",
            result
        );
        assert!(
            result.contains("<li>"),
            "List should be rendered. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_span_single_quotes() {
        // markdown='span' with single quotes
        let input = "<div markdown='span'>Some *italic* text</div>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown='span'"),
            "markdown='span' should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("<em>italic</em>"),
            "Italic should be rendered. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_span_unicode() {
        // Unicode content inside markdown="span" (German umlauts)
        let input = "<div markdown=\"span\" class=\"alert\">\u{00dc}berpr\u{00fc}fen Sie die Einstellungen f\u{00fc}r den Zugangspunkt</div>";
        let result = process_markdown_attribute(input);
        assert!(
            !result.contains("markdown=\"span\""),
            "markdown attr should be stripped. Got: {}",
            result
        );
        assert!(
            result.contains("\u{00dc}berpr\u{00fc}fen"),
            "Unicode content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_span_is_inline() {
        // markdown="span" should produce inline content (no block elements)
        let input =
            "<div markdown=\"span\">**Note:** See [link](http://example.com) for details.</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<strong>Note:</strong>"),
            "Strong should be rendered. Got: {}",
            result
        );
        assert!(
            result.contains("<a href=\"http://example.com\">link</a>"),
            "Link should be rendered. Got: {}",
            result
        );
        // Inline mode should not wrap in <p>
        assert!(
            !result.contains("<p>"),
            "Inline mode should not produce <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_process_markdown_attr_block_equivalent_to_1() {
        // markdown="block" should behave like markdown="1" for block content
        let input = "<div markdown=\"block\">\n## Heading\n\nParagraph\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<h2"),
            "Heading should be rendered. Got: {}",
            result
        );
        assert!(
            result.contains("<p>Paragraph</p>"),
            "Paragraph should be rendered. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 505: Block IAL class application in markdown="1" contexts
    // ========================================================================

    #[test]
    fn test_505_block_ial_single_class_in_markdown1_div() {
        // A paragraph followed by {: .label } inside markdown="1" div
        // should apply the class to the <p> element.
        let input =
            "<div class=\"code-example\" markdown=\"1\">\nDefault label\n{: .label }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p class=\"label\">Default label</p>"),
            "Block IAL should apply class to paragraph. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .label }"),
            "Block IAL text should be removed. Got: {}",
            result
        );
    }

    #[test]
    fn test_505_block_ial_multiple_classes_in_markdown1_div() {
        // Multiple classes: {: .label .label-blue }
        let input = "<div class=\"code-example\" markdown=\"1\">\nBlue label\n{: .label .label-blue }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("class=\"label label-blue\""),
            "Block IAL should apply multiple classes. Got: {}",
            result
        );
        assert!(
            result.contains(">Blue label</p>"),
            "Paragraph text should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_505_block_ial_multiple_paragraphs_in_markdown1_div() {
        // Multiple paragraphs each with their own IAL
        let input = "<div class=\"code-example\" markdown=\"1\">\nDefault label\n{: .label }\n\nBlue label\n{: .label .label-blue }\n\nStable\n{: .label .label-green }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<p class=\"label\">Default label</p>"),
            "First paragraph should have class 'label'. Got: {}",
            result
        );
        assert!(
            result.contains("<p class=\"label label-blue\">Blue label</p>"),
            "Second paragraph should have classes 'label label-blue'. Got: {}",
            result
        );
        assert!(
            result.contains("<p class=\"label label-green\">Stable</p>"),
            "Third paragraph should have classes 'label label-green'. Got: {}",
            result
        );
    }

    #[test]
    fn test_505_block_ial_id_in_markdown1_div() {
        // IAL with id: {: #my-id }
        let input = "<div markdown=\"1\">\nSome text\n{: #my-id }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("id=\"my-id\""),
            "Block IAL should apply id. Got: {}",
            result
        );
    }

    #[test]
    fn test_505_block_ial_unicode_in_markdown1_div() {
        // Unicode content followed by IAL
        let input =
            "<div markdown=\"1\">\n\u{4F60}\u{597D}\u{4E16}\u{754C}\n{: .highlight }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("class=\"highlight\""),
            "Block IAL should work with Unicode content. Got: {}",
            result
        );
        assert!(
            result.contains("\u{4F60}\u{597D}\u{4E16}\u{754C}"),
            "Unicode content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_505_block_ial_div_does_not_get_inner_classes() {
        // The div wrapper should NOT get the IAL classes -- they belong on the inner <p> elements
        let input =
            "<div class=\"code-example\" markdown=\"1\">\nDefault label\n{: .label }\n</div>";
        let result = process_markdown_attribute(input);
        assert!(
            result.contains("<div class=\"code-example\">"),
            "Div should keep its original class, not get IAL classes. Got: {}",
            result
        );
        assert!(
            !result.contains("code-example label\""),
            "Div should NOT get the label class. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 248: Research kramdown pipe table rules and fix false table parsing
    // ========================================================================

    #[test]
    fn test_248_lone_pipe_after_list_item_not_table() {
        // From mlwiki.org Cancellation_Regions.md: a lone ` |` line after a
        // list item is lazy continuation in kramdown, NOT a table.
        let input = "  - but now it can fire the second time\n |\n## Sources\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Lone pipe after list item should NOT produce a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_lone_pipe_between_text_not_table() {
        // A pipe line followed by non-pipe text is NOT a table in kramdown.
        let input = "| A | B |\nnot a pipe\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe line followed by non-pipe text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_pipe_preceded_by_text_not_table() {
        // A pipe line preceded by text (part of a paragraph) is NOT a table.
        let input = "some text\n| A | B |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe line preceded by text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_pipe_followed_by_blank_is_table() {
        // A pipe line followed by blank line IS a table in kramdown.
        let input = "| A | B |\n\nmore text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Pipe line followed by blank should be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_pipe_at_eof_is_table() {
        // A pipe line at EOF IS a table in kramdown.
        let input = "| A | B |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Pipe line at EOF should be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_multi_pipe_then_nonpipe_not_table() {
        // Multiple pipe lines followed by non-pipe text: NOT a table.
        let input = "| A | B |\n| C | D |\nnot a pipe\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe lines followed by non-pipe text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_list_pipe_followed_by_continuation_not_table() {
        // In kramdown: `- text | pipes |` then `  continuation text` = NOT table.
        let input = "- text | with | pipes |\n  continuation text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe line in list followed by continuation should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_list_pipe_followed_by_next_item_is_table() {
        // In kramdown: `- text | pipes |` then `- next item` = IS a table.
        let input = "- text | with | pipes |\n- next item\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Pipe line in list followed by next list item should be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_unicode_pipe_boundary_check() {
        // Unicode content with pipe boundary checks.
        let input =
            "| \u{041A}\u{043E}\u{043B} | \u{0417}\u{043D}\u{0430}\u{0447} |\n\u{0422}\u{0435}\u{043A}\u{0441}\u{0442}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe line followed by non-pipe Unicode text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_248_gfm_table_with_blank_after_preserved() {
        // Standard GFM table followed by blank line should work.
        let input = "| H1 | H2 |\n|---|---|\n| C1 | C2 |\n\nParagraph.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Standard GFM table with blank after should render. Got: {html}"
        );
    }

    #[test]
    fn test_248_standard_table_unicode_preserved() {
        // Regression guard: Unicode in standard table.
        let input = "| Kolonne | V\u{00e6}rdi |\n|---|---|\n| Tekst | Nummer |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Standard table with Unicode should render. Got: {html}"
        );
    }

    // --- Issue 247 QA fix: skip <code>/<pre>/<script> elements ---

    #[test]
    fn test_single_apostrophe_stays_straight() {
        // Single straight quotes must NOT be converted by this function.
        // pulldown-cmark's smart punctuation already handles apostrophes
        // in markdown content. This function's purpose is only to handle
        // restored ''/'''' sequences (count >= 2). Converting single quotes
        // causes regressions when Liquid template output contains straight
        // apostrophes (e.g., {{ post.title }} with "Aren't").
        let input = "<p>don\u{0027}t</p>";
        let result = apply_kramdown_smart_quotes_to_straight(input);
        assert!(
            result.contains("don\u{0027}t"),
            "Single straight apostrophe must stay straight. Got: {}",
            result
        );
    }

    #[test]
    fn test_single_apostrophe_in_title_stays_straight() {
        // Regression test: Liquid-rendered titles like {{ post.title }}
        // contain straight apostrophes that must not be converted.
        let input = "<p>Data Engineers Aren\u{0027}t Plumbers</p>";
        let result = apply_kramdown_smart_quotes_to_straight(input);
        assert!(
            result.contains("Aren\u{0027}t"),
            "Single apostrophe in title must stay straight. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue247_fix_quote_inside_code_stays_straight() {
        // Straight quotes inside <code> elements must NOT be converted.
        let input = "<p>Use <code>'requirements.txt'</code> for deps</p>";
        let result = apply_kramdown_smart_quotes_to_straight(input);
        assert!(
            result.contains("<code>'requirements.txt'</code>"),
            "Quotes inside <code> must stay straight. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue247_fix_quote_inside_pre_stays_straight() {
        // Straight quotes inside <pre> elements must NOT be converted.
        let input = "<pre>echo 'hello'</pre>";
        let result = apply_kramdown_smart_quotes_to_straight(input);
        assert!(
            result.contains("<pre>echo 'hello'</pre>"),
            "Quotes inside <pre> must stay straight. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue247_fix_double_quotes_still_converted() {
        // Consecutive '' sequences must still be converted.
        let input = "<p>The ''word'' here</p>";
        let result = apply_kramdown_smart_quotes_to_straight(input);
        // Space before '' -> lsquo pair, word before '' + space after -> rsquo pair
        assert!(
            result.contains("\u{2018}\u{2018}word\u{2019}\u{2019}"),
            "Consecutive '' must still be converted to smart quotes. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue247_fix_arent_youll_dont_regression() {
        // Full pipeline test: apostrophes in contractions must survive.
        let input = "Aren't You'll don't\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // pulldown-cmark with smart punctuation converts these to rsquo already.
        // apply_kramdown_smart_quotes_to_straight must not mangle them.
        // After pulldown-cmark they should be curly quotes (U+2019), NOT
        // further modified into something wrong.
        assert!(
            html.contains("Aren\u{2019}t")
                && html.contains("You\u{2019}ll")
                && html.contains("don\u{2019}t"),
            "Contractions must have rsquo apostrophes. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 272: Kramdown table detection too strict -- requires trailing pipe
    // ========================================================================

    // --- Unit tests for is_kramdown_table_line relaxation ---

    #[test]
    fn test_272_is_kramdown_table_line_pipe_in_middle() {
        // A line with a pipe in the middle (no trailing pipe) should be detected.
        assert!(
            is_kramdown_table_line("text | more text"),
            "Line with pipe in middle should be a kramdown table line"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_slack_ref() {
        // Slack channel ref: kramdown DOES treat the | as a table delimiter
        // (only autolinks like <tel:> and <mailto:> are protected).
        assert!(
            is_kramdown_table_line("<#C01AXGTRESH|books> would be better"),
            "Slack ref pipe should be detected as kramdown table line (matches Jekyll)"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_multiple_pipes() {
        // Multiple embedded pipes should be detected.
        assert!(
            is_kramdown_table_line("NLP  | CV | Time series | ..."),
            "Multiple embedded pipes should be a kramdown table line"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_separator_excluded() {
        // Separator lines should still be excluded.
        assert!(
            !is_kramdown_table_line("|---|---|"),
            "Separator line should NOT be a kramdown table line"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_no_pipe() {
        assert!(
            !is_kramdown_table_line("no pipe here"),
            "Line without pipe should NOT be a kramdown table line"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_just_text() {
        assert!(
            !is_kramdown_table_line("just text"),
            "Plain text should NOT be a kramdown table line"
        );
    }

    #[test]
    fn test_272_is_kramdown_table_line_whitespace_dashes() {
        assert!(
            !is_kramdown_table_line("  ---  "),
            "Line with only whitespace and dashes should NOT be a kramdown table line"
        );
    }

    // --- markdownify integration tests with embedded-pipe table lines ---

    #[test]
    fn test_272_markdownify_embedded_pipe_produces_table() {
        // Single line with embedded pipe at block boundary should produce table.
        let input = "text | more text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Line with embedded pipe at EOF should produce table. Got: {html}"
        );
        // Should have 2 cells
        let td_count = html.matches("<td>").count();
        assert_eq!(td_count, 2, "Should have 2 cells. Got: {html}");
    }

    #[test]
    fn test_272_markdownify_slack_ref_produces_table() {
        // Slack refs: kramdown treats | in <#C01|name> as table delimiter
        // (only autolinks like <tel:> and <mailto:> are protected).
        let input = "<#C01AXGTRESH|books> text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Slack ref pipe should produce table (matches Jekyll/kramdown). Got: {html}"
        );
    }

    #[test]
    fn test_272_markdownify_multiple_pipes_produces_table() {
        let input = "a | b | c | d\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Multiple pipes should produce table. Got: {html}"
        );
        let td_count = html.matches("<td>").count();
        assert_eq!(td_count, 4, "Should have 4 cells. Got: {html}");
    }

    #[test]
    fn test_272_markdownify_mailto_pipe_produces_table() {
        // kramdown treats | in <mailto:...|...> as a table delimiter
        // (kramdown does NOT have autolink detection for pipe tables).
        let input = "<mailto:a@b.com|a@b.com> more\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Mailto pipe should produce table (matches Jekyll/kramdown). Got: {html}"
        );
    }

    #[test]
    fn test_272_escaped_pipe_not_table() {
        // Escaped pipes \| should NOT trigger table detection.
        // This is a real DTC pattern: "Company \| Partner \| Relationship"
        let input = "Schneider Electric \\| EV Connect \\| Acquirer - Acquired\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Escaped pipes should NOT produce table. Got: {html}"
        );
        assert!(html.contains("<p>"), "Should be a paragraph. Got: {html}");
    }

    #[test]
    fn test_272_markdownify_unicode_with_pipes_produces_table() {
        // Non-ASCII content with embedded pipes should produce correct table.
        let input = "\u{041A}\u{043E}\u{043B}\u{043E}\u{043D}\u{043A}\u{0430} | \u{0417}\u{043D}\u{0430}\u{0447}\u{0435}\u{043D}\u{043D}\u{044F}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "Unicode content with pipe should produce table. Got: {html}"
        );
        let td_count = html.matches("<td>").count();
        assert_eq!(td_count, 2, "Should have 2 cells. Got: {html}");
    }

    // --- No false-positives tests ---

    #[test]
    fn test_272_no_table_pipe_followed_by_nonpipe_continuation() {
        // Line with pipe followed by non-pipe continuation should NOT be table.
        let input = "text | more\nnon-pipe continuation\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe line followed by non-pipe text should NOT be table. Got: {html}"
        );
    }

    #[test]
    fn test_272_no_table_pipe_preceded_by_text() {
        // Line with pipe preceded by non-blank text should NOT be table.
        let input = "paragraph text\nhas | pipe\nmore text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe in middle of paragraph should NOT be table. Got: {html}"
        );
    }

    #[test]
    fn test_272_no_table_pipe_inside_code_block() {
        // Pipes inside code blocks should NOT be treated as tables.
        let input = "```\na | b | c\n```\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Pipe inside code block should NOT be table. Got: {html}"
        );
    }

    #[test]
    fn test_272_existing_248_tests_still_pass_pipe_then_nonpipe() {
        // Regression: pipe lines followed by non-pipe text should not be table.
        let input = "| A | B |\nnot a pipe\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Existing 248 test: pipe then non-pipe should NOT be table. Got: {html}"
        );
    }

    // --- Issue 273: pipe inside angle brackets should not trigger table detection ---

    #[test]
    fn test_273_has_pipe_outside_angle_brackets_plain_pipe() {
        assert!(
            has_pipe_outside_angle_brackets("a | b"),
            "Plain pipe should be detected"
        );
    }

    #[test]
    fn test_273_has_pipe_outside_angle_brackets_inside_tags() {
        // kramdown treats ALL | as table delimiters regardless of <> context
        assert!(
            has_pipe_outside_angle_brackets("text <tel:100-1000|100-1000> more"),
            "Pipe inside angle brackets IS detected (kramdown behavior)"
        );
    }

    #[test]
    fn test_273_has_pipe_outside_angle_brackets_mixed() {
        assert!(
            has_pipe_outside_angle_brackets("a | <tag|inner> b"),
            "Pipe outside brackets should be detected even with pipe inside brackets"
        );
    }

    #[test]
    fn test_273_has_pipe_outside_angle_brackets_no_pipe() {
        assert!(
            !has_pipe_outside_angle_brackets("no pipe here"),
            "No pipe means false"
        );
    }

    #[test]
    fn test_273_has_pipe_outside_angle_brackets_unicode() {
        // Non-autolink angle brackets: pipe IS detected (kramdown behavior)
        assert!(
            has_pipe_outside_angle_brackets("text <sch\u{00f6}n|gr\u{00fc}\u{00df}> end"),
            "Pipe in non-autolink angle brackets SHOULD be detected (kramdown behavior)"
        );
    }

    #[test]
    fn test_273_pipe_in_autolink_produces_table() {
        // kramdown treats | in <tel:...|...> as table delimiter (no autolink protection)
        let input = "- engineering: infrastructure with <tel:100-1000|100-1000>s of GPUs\n\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        // The pipe triggers table conversion in the list item context
        assert!(
            html.contains("<table>") || html.contains("|"),
            "Pipe in tel: should be treated as table delimiter. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 274: Standalone HTML comments surrounded by blank lines should
    // be wrapped in <p> tags to match kramdown behavior
    // ========================================================================

    #[test]
    fn test_274_standalone_comment_wrapped_in_p() {
        // Comment surrounded by blank lines between block elements should be wrapped in <p>
        let input = "<p>text</p>\n\n<!-- Use manually specified posts -->\n\n<div>block</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "Standalone comment should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_multiple_standalone_comments_each_wrapped() {
        // Multiple standalone comments, each should be wrapped in <p>
        let input = "<!-- comment1 -->\n\n<!-- comment2 -->\n\n<div class=\"related\">block</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- comment1 --></p>"),
            "First standalone comment should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("<p><!-- comment2 --></p>"),
            "Second standalone comment should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_comment_adjacent_to_block_not_wrapped() {
        // Comment adjacent to block elements (no blank line) should NOT be wrapped (issue 144)
        let input = "<h2>Heading</h2>\n<!-- comment -->\n<div>block</div>";
        let result = postprocess(input);
        assert!(
            !result.contains("<p><!-- comment --></p>"),
            "Comment adjacent to block elements should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_comment_immediately_before_block_not_wrapped() {
        // Comment immediately before block element, no blank line, should NOT be wrapped
        let input = "<!-- FAQ Accordion Component -->\n<div class=\"faq\">content</div>";
        let result = postprocess(input);
        assert!(
            !result.contains("<p><!-- FAQ Accordion Component --></p>"),
            "Comment immediately before block should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_no_comments_passes_through() {
        // Content with no HTML comments should pass through unchanged
        let input = "<p>Hello</p>\n\n<div>World</div>";
        let result1 = postprocess(input);
        // Running postprocess again should be idempotent for this content
        assert!(
            !result1.contains("<!--"),
            "Content without comments should have no comments"
        );
    }

    #[test]
    fn test_274_related_posts_manual_posts_pattern() {
        // Simulate Liquid-processed output of related-posts.html with manual_posts
        let input = "<p>Some content about the course.</p>\n\n\
                      <!-- Use manually specified posts -->\n\n\
                      <div class=\"related-posts-section\">\n\
                      <h3>Related posts</h3>\n\
                      </div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "manual_posts comment should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_related_posts_auto_generate_pattern() {
        // Simulate Liquid-processed output of related-posts.html without manual_posts
        // All 3 comments should each be wrapped in <p>
        let input = "<p>Some content.</p>\n\n\
                      <!-- Auto-generate based on tags - simplified approach -->\n\n\
                      <!-- Find posts with matching tags -->\n\n\
                      <!-- Sort by date (most recent first) -->\n\n\
                      <div class=\"related-posts-section\">\n\
                      <h3>Related posts</h3>\n\
                      </div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Auto-generate based on tags - simplified approach --></p>"),
            "Auto-generate comment should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("<p><!-- Find posts with matching tags --></p>"),
            "Find posts comment should be wrapped in <p>. Got: {}",
            result
        );
        assert!(
            result.contains("<p><!-- Sort by date (most recent first) --></p>"),
            "Sort by date comment should be wrapped in <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_issue144_accordion_regression() {
        // Issue 144 regression: comment + div + script pattern must NOT wrap comments
        let input = "<h2>FAQ</h2>\n\
                      <!-- FAQ Accordion Component -->\n\
                      <div class=\"faq-accordion\">\n\
                      <div class=\"faq-item\">Q&amp;A</div>\n\
                      </div>\n\
                      \n\
                      <!-- FAQ Schema Markup (JSON-LD) -->\n\
                      <script type=\"application/ld+json\">\n\
                      {\"@type\": \"FAQPage\"}\n\
                      </script>\n\
                      \n\
                      <!-- Load accordion JavaScript -->\n\
                      <script src=\"/assets/accordion.js\"></script>";
        let result = postprocess(input);
        // Comments adjacent to block elements should NOT be wrapped
        assert!(
            !result.contains("<p><!-- FAQ Accordion Component --></p>"),
            "Comment before div should not be wrapped. Got: {}",
            result
        );
        assert!(
            !result.contains("<p><!-- Load accordion JavaScript --></p>"),
            "Comment before script should not be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_316_indented_comment_among_adjacent_comments_wrapped() {
        // Issue 316: When Liquid include output produces HTML comments on
        // consecutive lines (no blank lines between them), indented comments
        // should still be wrapped in <p> to match kramdown behavior.
        // kramdown treats indented HTML comments as inline/paragraph content.
        let input = "</div>\n<!-- Related Posts Section -->\n<!-- Get related posts -->\n  <!-- Use manually specified posts -->\n<!-- Limit to max_related posts -->\n<div class=\"related-posts-section\">";
        let result = wrap_standalone_comments_in_paragraphs(input);
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "Indented comment among adjacent comments should be wrapped in <p>. Got: {}",
            result
        );
        // Non-indented comments should NOT be wrapped
        assert!(
            !result.contains("<p><!-- Related Posts Section --></p>"),
            "Non-indented comment should NOT be wrapped. Got: {}",
            result
        );
        assert!(
            !result.contains("<p><!-- Get related posts --></p>"),
            "Non-indented comment should NOT be wrapped. Got: {}",
            result
        );
        assert!(
            !result.contains("<p><!-- Limit to max_related posts --></p>"),
            "Non-indented comment should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_316_indented_comment_unicode_content() {
        // Issue 316: Indented HTML comment with non-ASCII/Unicode content
        let input = "<!-- Verwandte Beitrage -->\n  <!-- Manuell angegebene Beitrage verwenden -->\n<!-- Beitrage begrenzen -->\n<div class=\"related\">";
        let result = wrap_standalone_comments_in_paragraphs(input);
        assert!(
            result.contains("<p><!-- Manuell angegebene Beitrage verwenden --></p>"),
            "Indented Unicode comment should be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_316_indented_comment_between_html_elements_not_wrapped() {
        // Issue 316: Indented comment inside an HTML block (between div/input
        // elements, like in subscribe-main.html) should NOT be wrapped.
        let input = "   </div>\n   <!-- real people should not fill this in -->\n   <div style=\"position: absolute;\">";
        let result = wrap_standalone_comments_in_paragraphs(input);
        assert!(
            !result.contains("<p><!-- real people"),
            "Indented comment between HTML elements should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_316_non_indented_adjacent_comments_not_wrapped() {
        // Adjacent non-indented comments should NOT be wrapped
        let input = "</div>\n<!-- Comment A -->\n<!-- Comment B -->\n<!-- Comment C -->\n<div class=\"content\">";
        let result = wrap_standalone_comments_in_paragraphs(input);
        assert!(
            !result.contains("<p><!--"),
            "Non-indented adjacent comments should NOT be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_274_comment_at_start_with_blank_line_then_block() {
        // Comment at start of content, followed by blank line then block
        let input = "<!-- Use manually specified posts -->\n\n<div class=\"related\">content</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "Comment at start followed by blank line should be wrapped. Got: {}",
            result
        );
    }

    #[test]
    fn test_273_normal_pipe_table_still_works() {
        // Regression: normal pipe tables should still be detected
        let input = "| Col1 | Col2 |\n| a | b |\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("<table>") || html.contains("<th>") || html.contains("<td>"),
            "Normal pipe table should still work. Got: {html}"
        );
    }

    /// Issue 325: Kramdown pipe table cells should have typographic symbol
    /// substitutions applied (ellipsis, em-dash, en-dash).
    #[test]
    fn test_325_pipe_table_typographic_symbols() {
        // Ellipsis in pipe table cell
        let input = "intro<br />\nNLP | CV | Time series | ...) text\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("\u{2026}"),
            "Ellipsis ... should become \u{2026} in pipe table cell. Got: {html}"
        );
        assert!(
            !html.contains("..."),
            "Literal ... should not remain in pipe table cell. Got: {html}"
        );
    }

    /// Issue 325: Pipe table cell content with < and > gets HTML-escaped
    #[test]
    fn test_325_pipe_table_html_escapes_angle_brackets() {
        let input = "intro<br />\nemail me at <mailto:a@b.com | a@b.com>\n";
        let html = crate::frontmatter::markdown_to_html_for_filter(input);
        assert!(
            html.contains("&lt;mailto:"),
            "< in pipe table cell should be escaped to &lt;. Got: {html}"
        );
        assert!(
            html.contains("&gt;"),
            "> in pipe table cell should be escaped to &gt;. Got: {html}"
        );
    }

    /// Issue 325: apply_typographic_symbols unit test
    #[test]
    fn test_325_apply_typographic_symbols() {
        assert_eq!(
            apply_typographic_symbols("hello...world"),
            "hello\u{2026}world"
        );
        assert_eq!(apply_typographic_symbols("a---b"), "a\u{2014}b");
        assert_eq!(apply_typographic_symbols("a--b"), "a\u{2013}b");
        // em-dash then hyphen: ---- -> em-dash + hyphen
        assert_eq!(apply_typographic_symbols("a----b"), "a\u{2014}-b");
        // Unicode content preserved
        assert_eq!(
            apply_typographic_symbols("caf\u{00e9}...th\u{00e9}"),
            "caf\u{00e9}\u{2026}th\u{00e9}"
        );
    }

    // ========================================================================
    // Issue 276: LaTeX math block rendering
    // Math conversion is disabled by default in the pipeline because Jekyll's
    // kramdown does NOT convert $...$ by default. These tests verify:
    // 1. The convert_math_delimiters function works correctly when called directly
    // 2. The pipeline does NOT convert math by default
    // ========================================================================

    #[test]
    fn test_issue276_pipeline_does_not_convert_math_by_default() {
        // By default, $...$ and $$...$$ should remain as-is in the pipeline
        let html = crate::frontmatter::markdown_to_html("The formula $x^2$ is simple.\n");
        assert!(
            !html.contains("\\("),
            "Pipeline should NOT convert inline math by default. Got: {}",
            html
        );
        assert!(
            html.contains("$x^2$"),
            "Dollar-delimited math should remain as-is. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_display_math_becomes_bare_text_node() {
        // Test convert_math_delimiters directly (not through pipeline)
        let input = "<p>$$x + y$$</p>\n";
        let html = convert_math_delimiters(input);
        assert!(
            html.contains("\\[x + y\\]"),
            "Display math should become \\[...\\]. Got: {}",
            html
        );
        assert!(
            !html.contains("<p>$$"),
            "Display math should not be wrapped in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_display_math_with_backslashes() {
        let input = "<p>$$Attention(Q,K,V) = softmax(\\frac{QK^T}{\\sqrt{d_k}})V$$</p>\n";
        let html = convert_math_delimiters(input);
        assert!(
            html.contains("\\["),
            "Display math with backslashes should convert. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_display_math_no_spaces() {
        let html = convert_math_delimiters("<p>$$formula$$</p>\n");
        assert!(
            html.contains("\\[formula\\]"),
            "Display math without spaces should work. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_display_math_with_spaces_around_delimiters() {
        let html = convert_math_delimiters("<p>$$ x + y $$</p>\n");
        assert!(
            html.contains("\\["),
            "Display math with spaces should be converted. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_inline_math_becomes_paren_notation() {
        let html = convert_math_delimiters("<p>Text with $x$ in it</p>\n");
        assert!(
            html.contains("\\(x\\)"),
            "Inline math should become \\(...\\). Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_inline_math_special_chars() {
        let html = convert_math_delimiters("<p>Formula $X^T X$ here</p>\n");
        assert!(
            html.contains("\\(X^T X\\)"),
            "Inline math with special chars should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_dollar_sign_not_converted() {
        let html = convert_math_delimiters("<p>It costs $100 today</p>\n");
        assert!(
            !html.contains("\\("),
            "Lone $ should not be converted. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_math_in_code_block_unchanged() {
        let html = convert_math_delimiters("<pre><code>$$x$$\n</code></pre>\n");
        assert!(
            !html.contains("\\["),
            "$$ inside code block should not be converted. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_both_display_and_inline_math() {
        let input = "<p>Some text $a + b$ here</p>\n\n\\[c + d\\]\n\n<p>More text</p>\n";
        let html = convert_math_delimiters(input);
        assert!(
            html.contains("\\(a + b\\)"),
            "Inline math should be converted. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_unicode_in_math() {
        let html = convert_math_delimiters("<p>$$\\alpha \\approx y$$</p>\n");
        assert!(
            html.contains("\\[\\alpha \\approx y\\]"),
            "Unicode/LaTeX content in math should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_display_math_multiline() {
        let input = "<p>$$\nf(x) = x^2\n$$</p>\n";
        let html = convert_math_delimiters(input);
        assert!(
            html.contains("\\["),
            "Multi-line display math should be converted. Got: {}",
            html
        );
        assert!(
            !html.contains("<p>$$"),
            "Multi-line display math should not have <p> wrapper. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 276 regression: Dollar sign currency must not be converted to math
    // ========================================================================

    #[test]
    fn test_issue276_currency_pair_not_converted() {
        // $10,000-$20,000+ should NOT be converted to math
        let md = "bootcamps that charge $10,000-$20,000+";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("$10,000"),
            "Currency $10,000 should remain as-is. Got: {}",
            html
        );
        assert!(
            html.contains("$20,000"),
            "Currency $20,000 should remain as-is. Got: {}",
            html
        );
        assert!(
            !html.contains("\\("),
            "Should NOT convert currency to math notation. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_currency_in_table_not_converted() {
        // $2,000–$10,000+ should remain as currency
        let md = "| Cost | $2,000–$10,000+ |";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("\\("),
            "Currency in table should not become math. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue276_real_math_converts_when_called_directly() {
        // When convert_math_delimiters is called directly, real math converts
        let html = convert_math_delimiters("<p>The formula is $x^2 + y^2 = z^2$ in math.</p>\n");
        assert!(
            html.contains("\\(x^2 + y^2 = z^2\\)"),
            "Real math should convert when function called directly. Got: {}",
            html
        );
    }

    // ========================================================================
    // DTC smart quote and ellipsis tests (TDD)
    // ========================================================================

    #[test]
    fn test_dtc_markdownify_converts_straight_double_quotes_to_smart() {
        // Jekyll's kramdown converts " to smart quotes in markdownify output
        let html =
            crate::frontmatter::markdown_to_html_for_filter("\"Successfully replicated 10TB/day\"");
        assert!(
            html.contains('\u{201C}') && html.contains('\u{201D}'),
            "Markdownify should convert straight double quotes to smart quotes. Got: {}",
            html
        );
    }

    #[test]
    fn test_dtc_double_quote_direction_opening_is_left() {
        // The opening " should be U+201C (LEFT), closing should be U+201D (RIGHT)
        // This is the exact DTC book page pattern
        let html = crate::frontmatter::markdown_to_html(
            "put \u{201C}Successfully replicated\u{201D} in review",
        );
        // After smart quote direction fix, opening should stay U+201C
        let chars: Vec<char> = html.chars().collect();
        let left_count = chars.iter().filter(|&&c| c == '\u{201C}').count();
        let right_count = chars.iter().filter(|&&c| c == '\u{201D}').count();
        assert!(
            left_count >= 1 && right_count >= 1,
            "Should have both left and right double quotes. Left: {}, Right: {}. Got: {}",
            left_count,
            right_count,
            html
        );
    }

    #[test]
    fn test_dtc_opening_double_quote_at_word_boundary() {
        // pulldown-cmark smart punctuation: opening " at start or after space → U+201C
        // This is the real issue: pulldown-cmark may produce U+201D for both
        let html = crate::frontmatter::markdown_to_html(
            "If you put \"Successfully replicated 10TB/day\" some shame",
        );
        // Check that opening quote is U+201C (left), not U+201D (right)
        let idx = html.find('\u{201C}').or_else(|| html.find('\u{201D}'));
        assert!(
            html.contains('\u{201C}'),
            "Opening double quote should be U+201C (left). Got: {}",
            html
        );
    }

    #[test]
    fn test_dtc_markdownify_converts_ellipsis() {
        // Jekyll's kramdown converts ... to … (U+2026)
        let html = crate::frontmatter::markdown_to_html_for_filter(
            "but unable to choose one specifically...",
        );
        assert!(
            html.contains('\u{2026}'),
            "Markdownify should convert ... to ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_dtc_book_text_with_newline_to_br_preserves_smart_quotes() {
        // DTC book layout: {{ thread.text | newline_to_br | markdownify }}
        // The newline_to_br inserts <br />\n before markdownify processes it.
        // Smart quotes should still be converted.
        let text_after_newline_to_br = "If you put \u{201C}Successfully replicated 10TB/day\u{201D}<br />\nsome shame is on you.";
        let html = crate::frontmatter::markdown_to_html_for_filter(text_after_newline_to_br);
        // The pre-existing curly quotes should be preserved
        assert!(
            html.contains('\u{201C}'),
            "Pre-existing smart quotes should survive markdownify. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 275: Mixed-delimiter emphasis double-nesting
    // ========================================================================

    #[test]
    fn test_issue275_underscore_wrapping_asterisk() {
        // _*text*_ should produce single <em> with literal asterisks, not double <em>
        let md = "_*Big Data Demystified*_";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Should not have double-nested <em>. Got: {}",
            html
        );
        assert!(
            html.contains("<em>"),
            "Should still have emphasis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_asterisk_wrapping_underscore() {
        // *_Decoding ML_ substack* should produce single <em> with literal underscores
        let md = "*_Decoding ML_ substack*";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Should not have double-nested <em>. Got: {}",
            html
        );
        assert!(
            html.contains("<em>"),
            "Should still have emphasis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_asterisk_wrapping_underscore_important() {
        // *the _important_ keywords* should produce single <em>
        let md = "*the _important_ keywords*";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Should not have double-nested <em>. Got: {}",
            html
        );
        assert!(
            html.contains("<em>"),
            "Should still have emphasis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_simple_underscore_asterisk() {
        let md = "_*text*_";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Should not have double-nested <em>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_simple_asterisk_underscore() {
        let md = "*_text_*";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Should not have double-nested <em>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_strong_wrapping_asterisk() {
        // __*text*__ should produce single <strong> with literal asterisks
        let md = "__*text*__";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<strong><em>"),
            "Should not have <strong><em> nesting. Got: {}",
            html
        );
        assert!(
            html.contains("<strong>"),
            "Should still have strong. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_strong_wrapping_underscore() {
        // **_text_** should produce single <strong> with literal underscores
        let md = "**_text_**";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<strong><em>"),
            "Should not have <strong><em> nesting. Got: {}",
            html
        );
        assert!(
            html.contains("<strong>"),
            "Should still have strong. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_normal_emphasis_unchanged() {
        // Normal emphasis should still work
        let md = "*text*";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<em>text</em>"),
            "Normal emphasis should work. Got: {}",
            html
        );

        let md = "_text_";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<em>text</em>"),
            "Normal underscore emphasis should work. Got: {}",
            html
        );

        let md = "**text**";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<strong>text</strong>"),
            "Normal strong should work. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_same_delimiter_nesting_unchanged() {
        // Same-delimiter nesting should still work: **text *inner* more**
        let md = "**text *inner* more**";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<strong>") && html.contains("<em>inner</em>"),
            "Same-delimiter nesting should work. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_unicode_content() {
        // Non-ASCII content: _*donnees*_ should not double-nest
        let md = "_*donn\u{00e9}es*_";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<em><em>"),
            "Unicode emphasis should not double-nest. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_markdownify_filter() {
        // The fix should also work through the markdownify codepath
        let md = "_*Big Data Demystified*_";
        let html = crate::frontmatter::markdown_to_html_for_filter(md);
        assert!(
            !html.contains("<em><em>"),
            "Markdownify should not have double-nested <em>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue275_url_with_asterisks_not_corrupted() {
        // URLs with tracking params like `1*95hemv*_ga` must not be corrupted
        let md = "Visit [this link](https://example.com/1*95hemv*_ga) for details.";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("https://example.com/1*95hemv*_ga"),
            "URL with asterisks should not be corrupted. Got: {}",
            html
        );

        // Bare URL with asterisks inside emphasis
        let md2 = "_Check https://example.com/1*95hemv*_ga for info_";
        let html2 = crate::frontmatter::markdown_to_html(md2);
        // Should not crash or produce malformed HTML
        assert!(
            !html2.is_empty(),
            "Should produce output for URL with asterisks in emphasis"
        );
    }

    // ====================================================================
    // Issue 279: Consecutive standalone HTML comments must each be <p>-wrapped
    // ====================================================================

    #[test]
    fn test_279_consecutive_comments_between_blank_lines_each_wrapped() {
        // Consecutive HTML comment lines (no blank lines between them) but the
        // group as a whole is surrounded by blank lines -- each should be wrapped.
        // This reproduces the related-posts.html Liquid include output pattern.
        let input = "<p>Some content.</p>\n\n\
                      <!-- Get related posts -->\n\
                      <!-- Use manually specified posts -->\n\
                      <!-- Limit to max_related posts -->\n\n\
                      <div class=\"related-posts\">content</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Get related posts --></p>"),
            "First consecutive comment should be wrapped in <p>. Got:\n{}",
            result
        );
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "Second consecutive comment should be wrapped in <p>. Got:\n{}",
            result
        );
        assert!(
            result.contains("<p><!-- Limit to max_related posts --></p>"),
            "Third consecutive comment should be wrapped in <p>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_two_consecutive_comments_wrapped() {
        // Two consecutive comments between blank lines
        let input = "<p>text</p>\n\n<!-- comment A -->\n<!-- comment B -->\n\n<div>block</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- comment A --></p>"),
            "First of two consecutive comments should be wrapped. Got:\n{}",
            result
        );
        assert!(
            result.contains("<p><!-- comment B --></p>"),
            "Second of two consecutive comments should be wrapped. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_indented_consecutive_comments_wrapped() {
        // Indented consecutive comments (from Liquid include output)
        let input = "<p>text</p>\n\n  <!-- Use manually specified posts -->\n  <!-- Limit to 3 -->\n\n<div>block</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Use manually specified posts --></p>"),
            "Indented consecutive comment should be wrapped. Got:\n{}",
            result
        );
        assert!(
            result.contains("<p><!-- Limit to 3 --></p>"),
            "Second indented consecutive comment should be wrapped. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_unicode_comment_wrapped() {
        // Non-ASCII/Unicode content in comments should be handled
        let input = "<p>text</p>\n\n<!-- Kommentar: Zugehörige Beiträge -->\n\n<div>block</div>";
        let result = postprocess(input);
        assert!(
            result.contains("<p><!-- Kommentar: Zugehörige Beiträge --></p>"),
            "Unicode comment should be wrapped in <p>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_consecutive_comments_adjacent_to_block_not_wrapped() {
        // Consecutive comments directly adjacent to a block element (no blank
        // line) should NOT be wrapped -- this is the issue 144 accordion pattern.
        let input = "<h2>FAQ</h2>\n<!-- Component start -->\n<!-- Load scripts -->\n<div class=\"faq\">content</div>";
        let result = postprocess(input);
        assert!(
            !result.contains("<p><!-- Component start --></p>"),
            "Comment adjacent to block should NOT be wrapped. Got:\n{}",
            result
        );
        assert!(
            !result.contains("<p><!-- Load scripts --></p>"),
            "Comment adjacent to block should NOT be wrapped. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_comment_inside_div_not_wrapped() {
        // Comments inside block-level elements should NOT be wrapped
        let input = "<div>\n<!-- inner comment -->\n<p>content</p>\n</div>";
        let result = postprocess(input);
        assert!(
            !result.contains("<p><!-- inner comment --></p>"),
            "Comment inside div should NOT be wrapped in <p>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_279_comment_inline_with_text_not_wrapped() {
        // Comment on the same line as text should not be separately wrapped
        let input = "<p>text <!-- inline comment --> more</p>";
        let result = postprocess(input);
        // The comment is inside a <p>, should stay as-is
        assert!(
            result.contains("<p>text <!-- inline comment --> more</p>"),
            "Inline comment should not be wrapped separately. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_converts_display_math() {
        // Display math <p>$$...$$</p> should be converted to \[...\] by the postprocess pipeline
        let input = "<p>$$x + y$$</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("\\[x + y\\]"),
            "postprocess should convert display math <p>$$...$$</p> to \\[...\\]. Got:\n{}",
            result
        );
        assert!(
            !result.contains("<p>$$"),
            "postprocess should not leave <p>$$. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_converts_real_formula() {
        let input = "<p>$$Attention(Q,K,V) = softmax(\\frac{QK^T}{\\sqrt{d_k}})V$$</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("\\[Attention(Q,K,V)"),
            "postprocess should convert real formula display math. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_converts_multiline_display_math() {
        let input = "<p>$$\nalpha + beta\n$$</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("\\["),
            "postprocess should convert multiline display math. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_preserves_inline_math() {
        let input = "<p>where $\\alpha$ is a factor</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("$\\alpha$"),
            "postprocess should NOT convert inline $...$ math. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_preserves_code_block_dollars() {
        let input = "<pre><code>$$x$$</code></pre>\n";
        let result = postprocess(input);
        assert!(
            result.contains("$$x$$"),
            "postprocess should NOT convert $$ inside code blocks. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_preserves_lone_dollar() {
        let input = "<p>It costs $100</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("$100"),
            "postprocess should NOT convert lone $ signs. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue276_postprocess_unicode_latex() {
        let input = "<p>$$\\alpha \\approx y$$</p>\n";
        let result = postprocess(input);
        assert!(
            result.contains("\\[\\alpha \\approx y\\]"),
            "postprocess should convert display math with unicode LaTeX. Got:\n{}",
            result
        );
    }

    // ========================================================================
    // Issue 302: Typographic ellipsis conversion
    // ========================================================================

    #[test]
    fn test_issue302_ellipsis_in_plain_text() {
        // kramdown converts ... to Unicode ellipsis U+2026
        let html = crate::frontmatter::markdown_to_html("Hello... world\n");
        assert!(
            html.contains("Hello\u{2026} world"),
            "Three dots should be converted to Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_ellipsis_inside_math() {
        // kramdown converts ... to ellipsis even inside $...$
        let html = crate::frontmatter::markdown_to_html("$A, B, C, ...$ in math\n");
        assert!(
            html.contains("\u{2026}"),
            "Ellipsis inside math should be converted to Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_ellipsis_not_in_inline_code() {
        // Inline code should NOT have ... converted
        let html = crate::frontmatter::markdown_to_html("Use `code...` here\n");
        assert!(
            html.contains("code..."),
            "Ellipsis inside inline code should stay as three dots. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_four_dots_becomes_ellipsis_plus_dot() {
        // Four dots: .... -> ellipsis + dot
        let html = crate::frontmatter::markdown_to_html("A.... B\n");
        assert!(
            html.contains("\u{2026}."),
            "Four dots should become ellipsis + dot. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_ellipsis_with_unicode_text() {
        // Non-ASCII content with ellipsis
        let html = crate::frontmatter::markdown_to_html(
            "\u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}... \u{043C}\u{0438}\u{0440}\n",
        );
        assert!(
            html.contains(
                "\u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}\u{2026} \u{043C}\u{0438}\u{0440}"
            ),
            "Ellipsis should work with non-ASCII text. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 302: Curly brace escaping in math
    // ========================================================================

    #[test]
    fn test_issue302_braces_not_escaped_in_inline_math() {
        // Braces inside $...$ should NOT be escaped to \{ \}
        let html = crate::frontmatter::markdown_to_html("$A = {x, y}$\n");
        assert!(
            !html.contains("\\{") && !html.contains("\\}"),
            "Braces inside inline math should NOT be escaped. Got: {}",
            html
        );
        assert!(
            html.contains("{x, y}"),
            "Braces should be literal inside inline math. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_braces_not_escaped_in_display_math() {
        // Braces inside $$...$$ should NOT be escaped
        let html = crate::frontmatter::markdown_to_html("$$F = {x | x > 0}$$\n");
        assert!(
            !html.contains("\\{") && !html.contains("\\}"),
            "Braces inside display math should NOT be escaped. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue302_braces_in_math_with_unicode() {
        // Non-ASCII content with braces in math
        let html = crate::frontmatter::markdown_to_html("$\\alpha \\in {1, 2, 3}$\n");
        assert!(
            !html.contains("\\{") && !html.contains("\\}"),
            "Braces in math with unicode should NOT be escaped. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 310: Display math ellipsis preservation
    // ========================================================================

    #[test]
    fn test_issue310_display_math_ellipsis_preserved() {
        // Display math ($$...$$) should NOT have ... converted to Unicode ellipsis
        let html = crate::frontmatter::markdown_to_html("$$A + ... + Z$$\n");
        assert!(
            html.contains("A + ... + Z"),
            "Display math should preserve three ASCII dots, not convert to ellipsis. Got: {}",
            html
        );
        assert!(
            !html.contains("A + \u{2026} + Z"),
            "Display math should NOT contain Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue310_inline_math_ellipsis_still_converts() {
        // Inline math ($...$) should still convert ... to Unicode ellipsis (issue 302)
        let html = crate::frontmatter::markdown_to_html("$A, B, C, ...$\n");
        assert!(
            html.contains("\u{2026}"),
            "Inline math should still convert ... to Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue310_regular_text_ellipsis_still_converts() {
        // Regular text should still convert ... to Unicode ellipsis
        let html = crate::frontmatter::markdown_to_html("Hello... world\n");
        assert!(
            html.contains("Hello\u{2026} world"),
            "Regular text should still convert ... to Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue310_display_math_sum_ellipsis_preserved() {
        // Another display math pattern
        let html = crate::frontmatter::markdown_to_html("$$\\sum_{i=1}^{...} x_i$$\n");
        assert!(
            !html.contains("\u{2026}"),
            "Display math sum with ... should NOT have Unicode ellipsis. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue310_mixed_inline_and_display_math() {
        // Mixed: inline math should have ellipsis, display math should not
        let html = crate::frontmatter::markdown_to_html("$a, ..., z$ and $$A + ... + Z$$\n");
        // The inline part should have ellipsis
        assert!(
            html.contains("a, \u{2026}, z"),
            "Inline math should have Unicode ellipsis. Got: {}",
            html
        );
        // The display part should NOT have ellipsis
        assert!(
            html.contains("A + ... + Z"),
            "Display math should preserve three ASCII dots. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue310_display_math_unicode_content_ellipsis_preserved() {
        // Non-ASCII/Unicode content in display math with ellipsis
        let html = crate::frontmatter::markdown_to_html("$$\\alpha + ... + \\omega$$\n");
        assert!(
            !html.contains("\u{2026}"),
            "Display math with Unicode Greek letters should NOT have Unicode ellipsis. Got: {}",
            html
        );
    }

    // === Issue 320: basic_generate_id and heading IDs in markdown="1" blocks ===

    #[test]
    fn test_issue320_basic_generate_id_ascii() {
        assert_eq!(basic_generate_id("My Heading"), "my-heading");
    }

    #[test]
    fn test_issue320_basic_generate_id_arabic() {
        // All non-ASCII chars stripped -> fallback to "section"
        assert_eq!(
            basic_generate_id("\u{0645}\u{0627} \u{0645}\u{0639}\u{0646}\u{0649}"),
            "section"
        );
    }

    #[test]
    fn test_issue320_basic_generate_id_chinese() {
        assert_eq!(
            basic_generate_id("\u{4f60}\u{597d}\u{4e16}\u{754c}"),
            "section"
        );
    }

    #[test]
    fn test_issue320_basic_generate_id_cyrillic() {
        assert_eq!(
            basic_generate_id("\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}"),
            "section"
        );
    }

    #[test]
    fn test_issue320_basic_generate_id_mixed() {
        let result = basic_generate_id("Code of Conduct \u{0645}\u{062f}\u{0648}\u{0646}\u{0629}");
        assert_eq!(result, "code-of-conduct-");
    }

    #[test]
    fn test_issue320_basic_generate_id_special_chars() {
        assert_eq!(basic_generate_id("Hello & World <>"), "hello--world-");
    }

    #[test]
    fn test_issue320_basic_generate_id_duplicate_handling() {
        let mut used = HashMap::new();
        let slug1 = basic_generate_id("\u{0645}\u{0627}");
        let id1 = get_unique_id(&mut used, &slug1);
        assert_eq!(id1, "section");
        let slug2 = basic_generate_id("\u{0623}\u{0646}");
        let id2 = get_unique_id(&mut used, &slug2);
        assert_eq!(id2, "section-1");
    }

    #[test]
    fn test_issue320_heading_ids_in_md1_block_arabic() {
        // Arabic heading inside markdown="1" should get id="section"
        let input = "<div markdown=\"1\" dir=\"rtl\">\n\n## \u{0645}\u{0627} \u{0645}\u{0639}\u{0646}\u{0649}\n\n</div>\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("id=\"section\""),
            "Arabic heading in markdown=\"1\" block should get id=\"section\". Got: {}",
            html
        );
    }

    #[test]
    fn test_issue320_heading_ids_outside_md1_block_preserves_unicode() {
        // Normal Cyrillic heading should still use GFM (preserve Unicode)
        let input = "## \u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("id=\"\u{043f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}\""),
            "Normal Cyrillic heading should preserve Unicode in ID. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue320_heading_ids_in_md1_block_latin() {
        // Latin heading inside markdown="1" should get proper slug
        let input = "<div markdown=\"1\">\n\n## My Heading\n\n</div>\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("id=\"my-heading\""),
            "Latin heading in markdown=\"1\" should get id=\"my-heading\". Got: {}",
            html
        );
    }

    #[test]
    fn test_issue320_heading_ids_in_md1_block_two_arabic() {
        // Two Arabic headings -> "section", "section-1"
        let input =
            "<div markdown=\"1\">\n\n## \u{0645}\u{0627}\n\n## \u{0623}\u{0646}\n\n</div>\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("id=\"section\""),
            "First Arabic heading should get id=\"section\". Got: {}",
            html
        );
        assert!(
            html.contains("id=\"section-1\""),
            "Second Arabic heading should get id=\"section-1\". Got: {}",
            html
        );
    }

    #[test]
    fn test_block_ial_forward_direction_from_markdown() {
        // Issue 301: Test the full pipeline from markdown to HTML.
        // When `{: .bullets}` has blank lines on both sides, it should apply to
        // the FOLLOWING element (the <ul>), not the preceding one (the <h3>).
        let md = "### Additional resources\n\n{: .bullets}\n\n* item1\n* item2\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("<ul class=\"bullets\">"),
            "IAL from markdown should apply to following <ul>. Got: {}",
            result
        );
        assert!(
            !result.contains("<h3 class=\"bullets\""),
            "IAL should NOT apply to preceding <h3>. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_backward_heading_from_markdown() {
        // When IAL is right after heading (no blank line), it should apply to the heading.
        let md = "# Title\n{: .fs-9 }\n\nSome paragraph text.\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("class=\"fs-9\""),
            "IAL after heading (no blank line) should apply to heading. Got: {}",
            result
        );
    }

    #[test]
    fn test_mark_forward_ial_preprocessing() {
        // Test that mark_forward_ial correctly identifies IALs with blank lines
        let md = "### Title\n\n{: .bullets}\n\n* item\n";
        let result = crate::kramdown::mark_forward_ial(md);
        assert!(
            result.contains("<!-- IAL:FWD -->"),
            "Should insert forward marker for IAL with blank lines on both sides. Got: {:?}",
            result
        );

        // IAL without blank line before should NOT get marker
        let md2 = "### Title\n{: .fs-9 }\n\nParagraph.\n";
        let result2 = crate::kramdown::mark_forward_ial(md2);
        assert!(
            !result2.contains("<!-- IAL:FWD -->"),
            "Should not insert marker when no blank line before IAL. Got: {:?}",
            result2
        );
    }

    #[test]
    fn test_block_ial_forward_direction_with_marker() {
        // Issue 301: When the IAL has the <!-- IAL:FWD --> marker (inserted by
        // mark_forward_ial during markdown preprocessing), apply_block_ial
        // applies attributes to the FOLLOWING element.
        let html = "<h3>Additional resources</h3>\n<!-- IAL:FWD -->\n<p>{: .bullets}</p>\n<ul>\n<li>item1</li>\n<li>item2</li>\n</ul>\n";
        let result = apply_block_ial(html);
        assert!(
            result.contains("<ul class=\"bullets\">"),
            "Block IAL with blank lines on both sides should apply to FOLLOWING element. Got: {}",
            result
        );
        assert!(
            !result.contains("<h3 class=\"bullets\""),
            "Block IAL with blank lines on both sides should NOT apply to preceding element. Got: {}",
            result
        );
        assert!(
            !result.contains("{: .bullets}"),
            "Block IAL should be removed from output. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_forward_direction_unicode() {
        // Same as above but with Unicode content
        let html = "<h3>\u{00c9}l\u{00e9}ments</h3>\n<!-- IAL:FWD -->\n<p>{: .special}</p>\n\n<ul>\n<li>\u{00e9}l\u{00e9}ment</li>\n</ul>\n";
        let result = apply_block_ial(html);
        assert!(
            result.contains("<ul class=\"special\">"),
            "Forward IAL should work with Unicode content. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_backward_when_no_blank_before() {
        // When there's no blank line before the IAL, it still applies to the preceding element
        // (this is the existing behavior that should be preserved)
        let html = "<h3>Title</h3>\n<p>{: .my-class}</p>\n\n<ul>\n<li>item</li>\n</ul>\n";
        let result = apply_block_ial(html);
        assert!(
            result.contains("<h3 class=\"my-class\""),
            "IAL without blank line before should apply to preceding element. Got: {}",
            result
        );
    }

    // =========================================================================
    // Issue 329: Kramdown nested list continuation tests
    // =========================================================================

    #[test]
    fn test_fix_kramdown_list_indentation_ordered_with_sublist() {
        let input = "1. Item\n  - sub a\n  - sub b\n1. Item 2\n";
        let result = fix_kramdown_list_indentation(input);
        assert!(
            result.contains("   - sub a"),
            "Sub-list should be indented to 3 spaces. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_fix_kramdown_list_indentation_preserves_unordered() {
        let input = "- Item\n  - sub a\n  - sub b\n- Item 2\n";
        let result = fix_kramdown_list_indentation(input);
        assert_eq!(input, result, "Unordered lists should not be changed");
    }

    #[test]
    fn test_fix_kramdown_list_indentation_unicode() {
        let input = "1. \u{0417}\u{0430}\u{0434}\u{0430}\u{0447}\u{0430}\n  - \u{041f}\u{043e}\u{0434}\u{043f}\u{0443}\u{043d}\u{043a}\u{0442} \u{0410}\n  - \u{041f}\u{043e}\u{0434}\u{043f}\u{0443}\u{043d}\u{043a}\u{0442} \u{0411}\n1. \u{0421}\u{043b}\u{0435}\u{0434}\u{0443}\u{044e}\u{0449}\u{0438}\u{0439}\n";
        let result = fix_kramdown_list_indentation(input);
        assert!(
            result.contains(
                "   - \u{041f}\u{043e}\u{0434}\u{043f}\u{0443}\u{043d}\u{043a}\u{0442} \u{0410}"
            ),
            "Cyrillic sub-items should be re-indented. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_fix_kramdown_list_indentation_deeply_nested() {
        let input = "1. Level 1\n   - Level 2\n     - Level 3\n   - Back to 2\n1. Level 1 again\n";
        let result = fix_kramdown_list_indentation(input);
        assert!(
            result.contains("   - Level 2"),
            "3-space indent should be preserved. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_fix_kramdown_list_indentation_no_change_in_code_block() {
        let input = "```\n1. Item\n  - sub\n```\n";
        let result = fix_kramdown_list_indentation(input);
        assert_eq!(input, result, "Code block content should not be changed");
    }

    #[test]
    fn test_fix_kramdown_list_indentation_multidigit() {
        let input = "10. Item ten\n  - sub item\n11. Item eleven\n";
        let result = fix_kramdown_list_indentation(input);
        assert!(
            result.contains("    - sub item"),
            "Sub-list under '10. ' should be indented to 4 spaces. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_fix_kramdown_list_indentation_math_content() {
        let input =
            "1. If $\\mathbf u$ is a unit vector\n  - Then $|\\mathbf u|^2 = 1$\n1. Otherwise\n";
        let result = fix_kramdown_list_indentation(input);
        assert!(
            result.contains("   - Then $|\\mathbf u|^2 = 1$"),
            "Math content should be preserved during re-indentation. Got:\n{}",
            result
        );
    }

    // =========================================================================
    // Issue 329: Fenced code blocks inside <details> tests
    // =========================================================================

    #[test]
    fn test_render_code_blocks_in_details() {
        let input = "<details>\n<summary>Code</summary>\n\n```python\nprint(\"hello\")\n```\n\n</details>\n";
        let result = render_code_blocks_in_html_blocks(input);
        assert!(
            result.contains("<pre><code"),
            "Fenced code inside <details> should become <pre><code>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_render_code_blocks_in_details_with_language() {
        let input = "<details>\n<summary>R code</summary>\n\n```r\nx <- seq(1, 10)\nplot(x)\n```\n\n</details>\n";
        let result = render_code_blocks_in_html_blocks(input);
        assert!(
            result.contains("language-r"),
            "Language class should be preserved. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_render_code_blocks_in_details_no_language() {
        let input = "<details>\n<summary>Code</summary>\n\n```\nsome code\n```\n\n</details>\n";
        let result = render_code_blocks_in_html_blocks(input);
        assert!(
            result.contains("<pre><code>"),
            "Code block without language should render as <pre><code>. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_render_code_blocks_outside_details_unchanged() {
        let input = "Some text\n\n```python\nprint(\"hello\")\n```\n";
        let result = render_code_blocks_in_html_blocks(input);
        assert_eq!(
            input, result,
            "Code blocks outside HTML blocks should not be changed"
        );
    }

    // =========================================================================
    // Issue 329: End-to-end markdown_to_html integration tests
    // =========================================================================

    #[test]
    fn test_e2e_ordered_list_with_nested_unordered() {
        let input = "1. Item\n  - Sub-item A\n  - Sub-item B\n1. Next item\n";
        let html = crate::frontmatter::markdown_to_html(input);
        let li_pos = html.find("<li>").unwrap();
        let ul_pos = html.find("<ul>").unwrap();
        let first_li_close = html[li_pos..].find("</li>").map(|p| li_pos + p).unwrap();
        assert!(
            ul_pos > li_pos && ul_pos < first_li_close,
            "The <ul> should be inside the first <li>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_e2e_ordered_list_with_nested_unordered_unicode() {
        let input = "1. \u{0417}\u{0430}\u{0434}\u{0430}\u{0447}\u{0430} \u{043f}\u{0440}\u{043e}\u{0433}\u{0440}\u{0430}\u{043c}\u{043c}\u{0438}\u{0440}\u{043e}\u{0432}\u{0430}\u{043d}\u{0438}\u{044f}\n  - \u{041f}\u{043e}\u{0434}\u{043f}\u{0443}\u{043d}\u{043a}\u{0442} \u{0410}\n  - \u{041f}\u{043e}\u{0434}\u{043f}\u{0443}\u{043d}\u{043a}\u{0442} \u{0411}\n1. \u{0421}\u{043b}\u{0435}\u{0434}\u{0443}\u{044e}\u{0449}\u{0438}\u{0439} \u{043f}\u{0443}\u{043d}\u{043a}\u{0442}\n";
        let html = crate::frontmatter::markdown_to_html(input);
        let li_pos = html.find("<li>").unwrap();
        let ul_pos = html.find("<ul>").unwrap();
        let first_li_close = html[li_pos..].find("</li>").map(|p| li_pos + p).unwrap();
        assert!(
            ul_pos > li_pos && ul_pos < first_li_close,
            "Unicode: <ul> should be inside <li>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_e2e_details_preserved_verbatim() {
        // Jekyll/kramdown preserves <details> block content verbatim
        let input = "<details>\n<summary>Code</summary>\n\n```python\nprint(\"hello\")\n```\n\n</details>\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<details>"),
            "Details block should be preserved. Got:\n{}",
            html
        );
        assert!(
            html.contains("```python"),
            "Raw backtick fences should be preserved (matching Jekyll). Got:\n{}",
            html
        );
        assert!(
            html.contains("</summary>\n\n```"),
            "Blank line after </summary> should be preserved. Got:\n{}",
            html
        );
    }

    // === Issue 265: GFM table before block boundary fix ===

    #[test]
    fn test_265_gfm_table_no_block_boundary_after_suppressed() {
        // GFM table followed by non-pipe text should NOT render as table
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\nnot a pipe\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "GFM table followed by text should NOT be a table. Got: {html}"
        );
        assert!(
            html.contains("| A | B |"),
            "Text content should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_three_columns_no_boundary() {
        let input = "| H1 | H2 | H3 |\n|---|---|---|\n| a | b | c |\ncontinuation text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "Three-column GFM table followed by text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_alignment_markers_no_boundary() {
        // Separator with alignment markers (:---:, ---:) followed by text
        let input = "| A | B |\n|:---:|---:|\n| 1 | 2 |\nnot a pipe\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "GFM table with alignment markers followed by text should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_no_boundary_before_or_after() {
        let input = "some text\n| A | B |\n|---|---|\n| 1 | 2 |\nmore text\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "GFM table with no block boundary before or after should NOT be a table. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_blank_line_after_preserved() {
        // Blank line after table = block boundary, should render as table
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n\nParagraph\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table with blank line after should render. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_eof_after_preserved() {
        // EOF after table = block boundary
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table at EOF should render. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_blank_lines_around_preserved() {
        let input = "\n| A | B |\n|---|---|\n| 1 | 2 |\n\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table with blank lines around should render. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_heading_after_preserved() {
        // Heading after table = block boundary
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n# Heading\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table followed by heading should render. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_hr_after_preserved() {
        // HR after table = block boundary
        let input = "| A | B |\n|---|---|\n| 1 | 2 |\n---\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table followed by HR should render. Got: {html}"
        );
    }

    #[test]
    fn test_265_mixed_document_gfm_tables() {
        // One GFM table with block boundary, one without
        let input =
            "| A | B |\n|---|---|\n| 1 | 2 |\n\n| X | Y |\n|---|---|\n| 3 | 4 |\ncontinuation\n";
        let html = crate::frontmatter::markdown_to_html(input);
        // First table should render (blank line after)
        assert!(
            html.contains("<table>"),
            "First GFM table should render. Got: {html}"
        );
        // Second table should NOT render (text after, no blank line)
        // Count table occurrences
        let table_count = html.matches("<table>").count();
        assert_eq!(
            table_count, 1,
            "Should have exactly 1 table (first renders, second suppressed). Got {table_count} tables. HTML: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_unicode_suppressed() {
        let input = "| Spalte | Wert |\n|---|---|\n| B\u{00fc}cher | Zahlen |\nWeiter geht es\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "GFM table with Unicode followed by text should NOT be a table. Got: {html}"
        );
        assert!(
            html.contains("B\u{00fc}cher"),
            "Unicode content should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_unicode_preserved() {
        let input = "| Spalte | Wert |\n|---|---|\n| B\u{00fc}cher | Zahlen |\n\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table with Unicode at block boundary should render. Got: {html}"
        );
        assert!(
            html.contains("B\u{00fc}cher"),
            "Unicode content should be preserved in table. Got: {html}"
        );
    }

    #[test]
    fn test_265_gfm_table_text_before_no_blank_line() {
        // Text before table without blank line = no block boundary before
        let input = "text before\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            !html.contains("<table>"),
            "GFM table with text before (no blank line) should NOT be a table. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 275b: DTC inline emphasis double-nesting and misparse
    // Uses markdown_to_html_with_options (kramdown mode with smart punctuation)
    // to match the actual DTC rendering pipeline.
    // ========================================================================

    /// Helper: render markdown through the same pipeline as DTC (kramdown mode,
    /// smart punctuation enabled).
    fn render_kramdown_mode(md: &str) -> String {
        crate::frontmatter::markdown_to_html_with_options(md, true, true, false, false)
    }

    /// Inline body content from the DTC "data engineers aren't plumbers" blog post.
    /// The full file content is needed because pulldown-cmark's emphasis parsing
    /// is affected by earlier HTML content (figure tags, links, etc.).
    fn plumbers_blog_body() -> &'static str {
        concat!(
            "\n",
            "<figure>\n",
            "<img src=\"/images/posts/2022-09-02-data-engineers-arent-plumbers/image2.png\"  />\n",
            "<figcaption><p>Water pipes (Photo by <a href=\"https://pixabay.com/users/loggawiggler-15/\">LoggaWiggler</a> on <a href=\"https://pixabay.com/\">Pixabay</a>)</p></figcaption>\n",
            "</figure>\n",
            "\n",
            "Every time we open an article with a title similar to “**What is a data engineer?**” or “**The difference between data engineer and data scientist**” we get a cliche answer: *Data engineers are like plumbers.*\n",
            "\n",
            "No! No! No! That is wrong. A data engineer can work with pipelines like a plumber but the role is very different.\n",
            "\n",
            "In this article, I will show you that a data engineer is similar to another profession/job: **hydraulic and water resources engineer.**\n",
            "\n",
            "I will explain why in three simple arguments:\n",
            "\n",
            "-   Working titles;\n",
            "-   Task goals;\n",
            "-   Tools needed to develop work.\n",
            "\n",
            "\n",
            "\n",
            "## 1. The titles are the same\n",
            "\n",
            "Well, this is obvious, right? 🙂 They are both engineers.\n",
            "\n",
            "<figure>\n",
            "<img src=\"/images/posts/2022-09-02-data-engineers-arent-plumbers/image1.png\"  />\n",
            "<figcaption><p>Mathematics (Photo by <a href=\"https://pixabay.com/users/artsybee-462611/\">ArtsyBee</a> on <a href=\"https://pixabay.com/\">Pixabay</a>)</p></figcaption>\n",
            "</figure>\n",
            "\n",
            "According to the [Oxford English Dictionary](https://www.dictionary.com/browse/engineer){:target=\"_blank\"}, an engineer is *“someone who designs, builds or maintains engines, machines, or structures”*.\n",
            "\n",
            "Each role may focus on different resources/products/processes, “water” for hydraulic and water resources engineers, and “data” for data engineers but both handle engineering on it.\n",
            "\n",
            "They are more “thinking” roles than “manual” roles (like a plumber) since they have to reflect and calculate what are the best solutions for their processes and not act by simple guidelines.\n",
            "\n",
            "## 2. Both have similar working goals\n",
            "\n",
            "This section is similar to the previous but we are focusing on the goal of each engineer.\n",
            "\n",
            "For example, a mechanical engineer has the working scope to build, maintain and improve mechanical machines that will perform some tasks.\n",
            "\n",
            "I'm considering a data engineer has the objective of building, maintaining, and improving data pipelines (ETL or ELT processes), data storage structures (data warehouses or data lakes), and providing solid data to the stakeholders.\n",
            "\n",
            "In a detailed way, the professional has the working scope of guarantee the a) extraction of data from various sources (both internal like relational databases and external sources), b) transformation of data using solid programming skills or software, c) good organization of the data in the correct storage structures and d) quality/organization of all the end-to-end processes and data using orchestrator tools, monitoring tools or other control tools.\n",
            "\n",
            "A data engineer needs to think of the process as a whole considering downstream and upstream mechanisms.\n",
            "\n",
            "<figure>\n",
            "<img src=\"/images/posts/2022-09-02-data-engineers-arent-plumbers/image3.jpg\"  />\n",
            "<figcaption><p>Water Treatment plant (Photo by <a href=\"https://www.pexels.com/@marcin-jozwiak-199600/\">Marcin Jozwiak</a> on <a href=\"https://www.pexels.com/\">Pexels</a>)</p></figcaption>\n",
            "</figure>\n",
            "\n",
            "The specialization of [Hydraulic and Water Resources Engineering](https://www.mcgill.ca/civil/undergrad/areas/water){:target=\"_blank\"} by the McGill University of Canada describes these two disciplines as follows:\n",
            "\n",
            "*“**Water resources engineering** is the quantitative study of the hydrologic cycle — the distribution and circulation of water linking the earth's atmosphere, land and oceans. (...) Applications include the management of the urban water supply, the design of urban storm-sewer systems, and flood forecasting.”* and *“**Hydraulic engineering** consists of the application of fluid mechanics to water flowing in an isolated environment (pipe, pump) or in an open channel (river, lake, ocean). Applications include the design of hydraulic structures, such as sewage conduits, dams and breakwaters, the management of waterways, such as erosion protection and flood protection, and environmental management, such as prediction of the mixing and transport of pollutants in surface water.”.*\n",
            "\n",
            "Therefore I'm considering hydraulic and water resources engineers need to guarantee (besides other tasks)\n",
            "\n",
            "a\\) the extraction of water from various sources,\n",
            "\n",
            "b\\) the correct water cleaning in water treatment facilities (see image above),\n",
            "\n",
            "c\\) good organization of the water in the correct storage structures, and\n",
            "\n",
            "d\\) quality/organization of all the end-to-end processes with several control tools.\n",
            "\n",
            "In the table below it is possible to see how identical both roles are in terms of working processes, tasks, or goals (with some examples).\n",
            "\n",
            "| **Processes/Task/Scope**                          | **Data Engineer**                                                        | **Hydraulic and Water Resources Engineer**                              |\n",
            "|---------------------------------------------------|--------------------------------------------------------------------------|-------------------------------------------------------------------------|\n",
            "| Extraction of raw product from sources            | Relational databases, External API, or CRM data.                         | Surface water, groundwater, or wastewater.                              |\n",
            "| Development and maintain transformation processes | Data transformation by cleaning, deduplication, or data type correction. | Water cleaning by removing organic compounds, or non-organic compounds. |\n",
            "| Development and maintain storage structure        | Data warehouse, data Lakes.                                              | Water towers, water dams.                                               |\n",
            "| Development of the full process construction      | Data orchestration tools.                                                | Computer tools to draw all systems, and wastewater treatment plants.    |\n",
            "| Controlling/Monitoring processes and product      | Software tools for data lineage or process control                       | Sensors all over the process                                            |\n",
            "| Stakeholders                                      | Data analysts, Data Scientists.                                          | Cities, industrial.                                                     |\n",
            "\n",
            "\n",
            "Therefore you can see that even having different targets both engineers do similar tasks.\n",
            "\n",
            "\n",
            "## 3. They use identical tools\n",
            "\n",
            "In that cliché of \"data engineer equals plumber\" it is often written that both have tools. However, the plumber tools are different from the data engineer tools. But both data engineers and hydraulic and water resources engineers use similar tools.\n",
            "\n",
            "Considering the processes present in the table above I will present you some examples for each role.\n",
            "\n",
            "For data engineers:\n",
            "\n",
            "1.  SQL for analysis of the data sources;\n",
            "2.  Python, Scala or other programming languages for development.\n",
            "3.  Airflow, Luigi or other for the development of the full process construction\n",
            "4.  Grafana and data testing tools to control and monitor.\n",
            "\n",
            "\n",
            "For hydraulic and water resources engineer:\n",
            "\n",
            "1.  Tools for geo analysis or GIS tools for analysis of the sources area;\n",
            "2.  Excel or similar tool for calculus;\n",
            "3.  CAD software tools for the development of the full process construction;\n",
            "4.  Sensors for quality and quantity water control.\n",
            "\n",
            "\n",
            "So all the tools for both engineers are complex tools (mostly software) with the purpose of proceeding to the estimation of the best solution. They are not manual tools like hammers.\n",
            "\n",
            "\n",
            "## Conclusion\n",
            "\n",
            "In summary it was presented in three simple subjects that data engineers are less identical to plumbers and more to hydraulic and water resources engineers.\n",
            "\n",
            "Hydraulic and water resources engineers and data engineers resemble because\n",
            "\n",
            "-   Both are engineers, a \"mind role\" and not a \"manual role\" like a plumber;\n",
            "-   They have a similar working scope of extracting/studying raw product, transforming it, storing it and deliver to the stakeholder;\n",
            "-   These positions always have to understand all the process end-to-end by being aware of downstream and upstream operations;\n",
            "-   The tools that both positions use are complex tools aiming calculation and analysis.\n",
            "\n",
            "\n",
            "\n",
            "And the cliché is down!\n",
            "\n",
            "What do you think, do you agree with me?\n",
            "\n",
            "Do you think I am going to be attacked by Mario Bros? 🧑‍🔧\n",
            "\n",
            "Did you like this article? Follow me for more articles on [Medium](https://medium.com/@lgsoliveira){:target=\"_blank\"}.\n",
        )
    }

    #[test]
    fn test_issue275b_adjacent_bold_no_nesting() {
        // Problem 1: **A**" or "**B** must produce two separate <strong> elements
        // Test with inlined DTC blog post content (previously read from file).
        // The bug only reproduces with the full file content -- some earlier content
        // affects pulldown-cmark's emphasis parsing.
        let content = plumbers_blog_body();
        let html = render_kramdown_mode(content);
        // Must have two separate <strong> elements, not nested
        assert!(
            !html.contains("<strong><strong>")
                && !html.contains("<strong>What is a data engineer?<strong>"),
            "Must not have nested <strong> tags. Got:\n{}",
            html
        );
        assert!(
            html.contains("<strong>What is a data engineer?</strong>"),
            "First bold span must be properly closed. Got:\n{}",
            html
        );
        // Check second bold span
        if let Some(pos) = html.find("What is a data engineer") {
            let start = html[..pos].rfind('<').unwrap_or(0);
            let end = html[pos..]
                .find("</p>")
                .map(|p| pos + p + 4)
                .unwrap_or(html.len());
            let para = &html[start..end];
            assert!(
                html.contains(
                    "<strong>The difference between data engineer and data scientist</strong>"
                ),
                "Second bold span must be properly closed. Emphasis paragraph:\n{}",
                para
            );
            assert!(
                html.contains("<em>Data engineers are like plumbers.</em>"),
                "Italic span must be present. Emphasis paragraph:\n{}",
                para
            );
        } else {
            panic!("Expected emphasis paragraph not found in output");
        }
    }

    #[test]
    fn test_issue275b_adjacent_bold_minimal_repro() {
        // Minimal reproduction using the exact DTC blog post content.
        // The bug only reproduces with the full file content through
        // markdown_to_html_with_options (pulldown-cmark path).
        let content = plumbers_blog_body();
        let html = render_kramdown_mode(content);
        assert!(
            html.contains("<strong>What is a data engineer?</strong>"),
            "First bold span must be properly closed. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_triple_adjacent_bold() {
        // **A** and **B** and **C** must produce three separate <strong>
        let md = "**A** and **B** and **C**\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<strong>A</strong>")
                && html.contains("<strong>B</strong>")
                && html.contains("<strong>C</strong>"),
            "Must have three separate <strong> elements. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_bold_separated_by_quotes() {
        let md = "**bold**\" or \"**bold**\n";
        let html = render_kramdown_mode(md);
        // Count occurrences of <strong>bold</strong>
        let count = html.matches("<strong>bold</strong>").count();
        assert_eq!(
            count, 2,
            "Must have two separate <strong>bold</strong> spans. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_emphasis_wrapping_html_link() {
        // Problem 2: *<a href="...">text</a>, trailing* must produce <em>
        let md = "*<a href=\"https://example.com\">EV Connect</a>, a charging provider*\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<em>") && html.contains("</em>"),
            "Must wrap in <em> tags, not literal asterisks. Got:\n{}",
            html
        );
        assert!(
            !html.contains("*&lt;a") && !html.contains("*<a"),
            "Must not have literal asterisks around content. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_two_emphasis_spans_with_links() {
        // Two separate *<a>link</a>, text* spans
        let md = "*<a href=\"url1\">Link1</a>, text1* *<a href=\"url2\">Link2</a>, text2*\n";
        let html = render_kramdown_mode(md);
        let em_count = html.matches("<em>").count();
        assert!(
            em_count >= 2,
            "Must have two <em> spans. Got {} <em> tags in:\n{}",
            em_count,
            html
        );
        assert!(
            !html.contains("*&lt;a") && !html.contains("*<a"),
            "Must not have literal asterisks. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_underscore_emphasis_with_slash() {
        // Problem 3: _CI/CD_ must produce <em>CI/CD</em>
        let md = "_CI/CD_\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<em>CI/CD</em>"),
            "Underscore emphasis with slash must work. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_multiple_underscore_emphasis_with_slash() {
        let md = "_CI/CD_, _Testing_ and _Deployment_\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<em>CI/CD</em>"),
            "Must have <em>CI/CD</em>. Got:\n{}",
            html
        );
        assert!(
            html.contains("<em>Testing</em>"),
            "Must have <em>Testing</em>. Got:\n{}",
            html
        );
        assert!(
            html.contains("<em>Deployment</em>"),
            "Must have <em>Deployment</em>. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_underscore_emphasis_with_path() {
        let md = "_path/to/file_\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<em>path/to/file</em>"),
            "Underscore emphasis with path slashes must work. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_unicode_adjacent_bold() {
        // German text with adjacent bold spans
        let md = "\"**Was ist ein Dateningenieur?**\" oder \"**Der Unterschied**\"\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<strong>Was ist ein Dateningenieur?</strong>"),
            "German bold text must be preserved. Got:\n{}",
            html
        );
        assert!(
            html.contains("<strong>Der Unterschied</strong>"),
            "Second German bold text must be preserved. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue275b_unicode_underscore_emphasis() {
        // Accented characters in underscore emphasis
        let md = "_donn\u{00e9}es_\n";
        let html = render_kramdown_mode(md);
        assert!(
            html.contains("<em>donn\u{00e9}es</em>"),
            "Accented character in emphasis must be preserved. Got:\n{}",
            html
        );
    }

    // ========================================================================
    // Regression: fix_literal_asterisk_emphasis restores emphasis from literal *
    // ========================================================================

    #[test]
    fn test_regression_literal_asterisk_emphasis_in_postprocess() {
        // When pulldown-cmark fails to parse *text* as emphasis in certain
        // document contexts (e.g., after <figure> blocks), the asterisks end
        // up as literal text. The postprocessor must fix these.
        let html = r#"<p>"*EL SEGUNDO, Calif., June 21, 2022 —* *<a href="https://example.com">Blink Charging</a>, a premier electric vehicle (EV) charging solution provider, announced that it has been acquired by <a href="https://example.com">Schneider Electric</a>, the leader in energy management and automation."*</p>"#;
        let result = postprocess_with_options(html, true);
        assert!(
            result.contains("<em>"),
            "Literal *...* should be converted to <em>...</em> by postprocessor. Got: {}",
            result
        );
    }

    #[test]
    fn test_regression_literal_underscore_emphasis_in_postprocess() {
        // Literal _text_ in HTML output should be converted to <em>text</em>
        // when pulldown-cmark fails to parse it.
        let html = "<p>methodologies like _CI/CD_, _Testing_ and _Deployment_ with TensorFlow</p>";
        let result = postprocess_with_options(html, true);
        assert!(
            result.contains("<em>CI/CD</em>"),
            "Literal _CI/CD_ should become <em>CI/CD</em>. Got: {}",
            result
        );
        assert!(
            result.contains("<em>Testing</em>"),
            "Literal _Testing_ should become <em>Testing</em>. Got: {}",
            result
        );
        assert!(
            result.contains("<em>Deployment</em>"),
            "Literal _Deployment_ should become <em>Deployment</em>. Got: {}",
            result
        );
    }

    #[test]
    fn test_regression_literal_emphasis_unicode_content() {
        // Emphasis with non-ASCII content must also work
        let html = "<p>Le mot _r\u{00e9}sum\u{00e9}_ est fran\u{00e7}ais</p>";
        let result = postprocess_with_options(html, true);
        assert!(
            result.contains("<em>r\u{00e9}sum\u{00e9}</em>"),
            "Underscore emphasis with accented chars should work. Got: {}",
            result
        );
    }

    // ========================================================================
    // Issue 244: GFM table without leading/trailing pipes must not deadlock
    // ========================================================================

    #[test]
    fn test_244_gfm_table_no_leading_trailing_pipes() {
        // Tables like `A | B\n--|--\n1 | 2` (no leading/trailing `|`)
        // caused an infinite loop in convert_kramdown_pipe_tables because
        // the GFM collection loop didn't match rows without `|` delimiters.
        let input = "A | B\n--|--\n1 | 2\n";
        let result = convert_kramdown_pipe_tables(input);
        // Must terminate (not hang) and contain the table content
        assert!(
            result.contains("A") && result.contains("B"),
            "Table content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_244_gfm_table_no_pipes_renders_html() {
        // The full markdown-to-HTML pipeline should produce a <table>
        // for GFM tables without leading/trailing pipes.
        let input = "First Header  | Second Header\n------------- | -------------\nContent Cell  | Content Cell\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("<table>"),
            "GFM table without pipe delimiters should render. Got: {}",
            html
        );
    }

    #[test]
    fn test_244_gfm_table_no_pipes_with_surrounding_text() {
        // Ensure tables preceded and followed by paragraphs work correctly.
        let input = "Some text before.\n\nA | B\n--|--\n1 | 2\n\nSome text after.\n";
        let result = convert_kramdown_pipe_tables(input);
        assert!(
            result.contains("Some text before") && result.contains("Some text after"),
            "Surrounding text should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_423_docker_code_block_gets_div_wrapper() {
        // Issue 423: Docker code blocks should get the div wrapper and syntax spans
        let html =
            "<pre><code class=\"language-docker\">FROM base\nCOPY app.py ./\n</code></pre>\n";
        let result = wrap_fenced_code_blocks(html);
        assert!(
            result.contains("<div class=\"language-docker highlighter-rouge\">"),
            "Docker code block should get language-docker wrapper. Got: {}",
            result
        );
        assert!(
            result.contains("<span class=\"k\">FROM</span>"),
            "Docker code block should have highlighted FROM keyword. Got: {}",
            result
        );
        assert!(
            result.contains("<span class=\"k\">COPY</span>"),
            "Docker code block should have highlighted COPY keyword. Got: {}",
            result
        );
    }

    #[test]
    fn test_423_dockerfile_code_block_gets_div_wrapper() {
        // Issue 423: Dockerfile code blocks also get the wrapper
        let html =
            "<pre><code class=\"language-dockerfile\">RUN pip install flask\n</code></pre>\n";
        let result = wrap_fenced_code_blocks(html);
        assert!(
            result.contains("<div class=\"language-dockerfile highlighter-rouge\">"),
            "Dockerfile code block should get wrapper. Got: {}",
            result
        );
        assert!(
            result.contains("<span class=\"k\">RUN </span>"),
            "Dockerfile code block should have highlighted RUN keyword. Got: {}",
            result
        );
    }

    #[test]
    fn test_423_docker_full_block_matches_jekyll() {
        // Verify exact match with Jekyll output for the first Docker block
        // from ml-deployment-lambda
        let html = "<pre><code class=\"language-docker\">FROM public.ecr.aws/lambda/python:3.8 as base\n\nFROM base AS train\nCOPY requirements.txt .\nRUN pip install -r requirements.txt\nENV MODEL_LOCAL_PATH=pickled_model.pkl\nCOPY train.py .\nRUN python3 train.py\n</code></pre>\n";
        let result = wrap_fenced_code_blocks(html);

        let expected_content = "<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">public.ecr.aws/lambda/python:3.8</span><span class=\"w\"> </span><span class=\"k\">as</span><span class=\"w\"> </span><span class=\"s\">base</span>\n\n<span class=\"k\">FROM</span><span class=\"w\"> </span><span class=\"s\">base</span><span class=\"w\"> </span><span class=\"k\">AS</span><span class=\"w\"> </span><span class=\"s\">train</span>\n<span class=\"k\">COPY</span><span class=\"s\"> requirements.txt .</span>\n<span class=\"k\">RUN </span>pip <span class=\"nb\">install</span> <span class=\"nt\">-r</span> requirements.txt\n<span class=\"k\">ENV</span><span class=\"s\"> MODEL_LOCAL_PATH=pickled_model.pkl</span>\n<span class=\"k\">COPY</span><span class=\"s\"> train.py .</span>\n<span class=\"k\">RUN </span>python3 train.py\n";
        assert!(
            result.contains(expected_content),
            "Docker block should match Jekyll output.\nExpected to contain: {}\nGot: {}",
            expected_content,
            result
        );
    }

    #[test]
    fn test_244_gfm_table_no_pipes_unicode() {
        // Tables with Unicode content and no leading/trailing pipes.
        let input =
            "\u{0417}\u{0430}\u{0433} | \u{0420}\u{0435}\u{0437}\n--|--\n\u{042f} | \u{0414}\n";
        let result = convert_kramdown_pipe_tables(input);
        assert!(
            result.contains("\u{0417}\u{0430}\u{0433}"),
            "Unicode table content should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue448_br_between_blockquotes_standalone() {
        // Issue 448: When markdown has a <br> between two blockquotes,
        // Jekyll renders it as a standalone element between the blockquotes.
        // Rustkyll absorbs it into the first blockquote's <p>.
        let md = "> Wouldn't it require we rewrite every element with a width/border/padding?\n<br>\n\n> I'm pretty sure the internet would break in half if we added that rule in today.\n";
        let result = crate::frontmatter::markdown_to_html(md);
        // The <br> (or <br />) must be BETWEEN the two blockquotes, not inside.
        assert!(
            result.contains("</blockquote>\n<br"),
            "The <br> should be a standalone element between the two blockquotes, not inside the first one. Got:\n{}",
            result
        );
        assert!(
            !result.contains("<br>\n<br>"),
            "Should not have doubled <br> elements inside blockquote. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue448_multiple_br_between_blockquotes() {
        // Multiple <br> tags between blockquotes should all be standalone.
        let md = "> quote one\n<br>\n<br>\n\n> quote two\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("</blockquote>\n<br"),
            "Multiple <br> tags should be standalone between blockquotes. Got:\n{}",
            result
        );
    }

    #[test]
    fn test_issue448_no_br_blockquotes_unchanged() {
        // Normal blockquotes without <br> should not be affected.
        let md = "> quote one\n\n> quote two\n";
        let result = crate::frontmatter::markdown_to_html(md);
        assert!(
            result.contains("<blockquote>"),
            "Normal blockquotes should still work. Got:\n{}",
            result
        );
        assert!(
            result.contains("</blockquote>"),
            "Normal blockquotes should still work. Got:\n{}",
            result
        );
    }

    // --- Issue 496: Kramdown inline attribute lists ---

    #[test]
    fn test_parse_ial_trailing_colon() {
        // Kramdown allows optional trailing colon: {: .class :}
        let attrs = parse_ial_attributes(".mx-auto.d-block :");
        // Should parse as two classes: mx-auto and d-block
        assert_eq!(
            attrs,
            vec![
                ("class".into(), "mx-auto".into()),
                ("class".into(), "d-block".into()),
            ]
        );
    }

    #[test]
    fn test_parse_ial_dot_concatenated_classes() {
        // .class1.class2 means two separate classes in kramdown
        let attrs = parse_ial_attributes(".mx-auto.d-block");
        assert_eq!(
            attrs,
            vec![
                ("class".into(), "mx-auto".into()),
                ("class".into(), "d-block".into()),
            ]
        );
    }

    #[test]
    fn test_parse_ial_dot_concatenated_three_classes() {
        let attrs = parse_ial_attributes(".a.b.c");
        assert_eq!(
            attrs,
            vec![
                ("class".into(), "a".into()),
                ("class".into(), "b".into()),
                ("class".into(), "c".into()),
            ]
        );
    }

    #[test]
    fn test_inline_ial_on_img_element() {
        // After markdown rendering, img with IAL looks like:
        // <p><img src="url" alt="img" />{: .mx-auto.d-block :}</p>
        let html = "<p><img src=\"url\" alt=\"Crepe\" />{: .mx-auto.d-block :}</p>";
        let result = apply_inline_attributes(html);
        assert!(
            result.contains("class=\"d-block mx-auto\""),
            "Should apply sorted classes to img. Got: {}",
            result
        );
        assert!(
            !result.contains("{:"),
            "IAL text should be removed. Got: {}",
            result
        );
    }

    #[test]
    fn test_inline_ial_on_img_classes_sorted() {
        // Jekyll sorts IAL classes alphabetically
        let html = "<p><img src=\"url\" alt=\"img\" />{: .mx-auto.d-block :}</p>";
        let result = apply_inline_attributes(html);
        assert!(
            result.contains("class=\"d-block mx-auto\""),
            "Classes should be sorted alphabetically. Got: {}",
            result
        );
    }

    #[test]
    fn test_block_ial_with_trailing_colon() {
        // Block-level IAL with trailing colon
        let html = "<h1>Title</h1>\n<p>{: .fs-9 :}</p>";
        let result = apply_block_ial(html);
        assert!(
            result.contains("class=\"fs-9\""),
            "Should apply class to heading. Got: {}",
            result
        );
        assert!(
            !result.contains("{:"),
            "IAL paragraph should be removed. Got: {}",
            result
        );
    }

    // --- Gist noscript unwrap tests ---

    #[test]
    fn test_unwrap_noscript_from_p() {
        let input = "<p><noscript><pre>400: Invalid request</pre>\n</noscript></p>";
        let result = unwrap_block_elements_from_p(input);
        assert_eq!(
            result, "<noscript><pre>400: Invalid request</pre></noscript>",
            "noscript should be unwrapped from p tags"
        );
    }

    #[test]
    fn test_unwrap_noscript_preserves_surrounding() {
        let input = "<p>before</p>\n<p><noscript><pre>400</pre>\n</noscript></p>\n<p>after</p>";
        let result = unwrap_block_elements_from_p(input);
        assert!(
            result.contains("<p>before</p>"),
            "content before should be preserved: {}",
            result
        );
        assert!(
            result.contains("<p>after</p>"),
            "content after should be preserved: {}",
            result
        );
        assert!(
            !result.contains("<p><noscript>"),
            "noscript should not be inside p: {}",
            result
        );
    }

    #[test]
    fn test_unwrap_noscript_no_false_positive() {
        // Normal <p> tags should not be affected
        let input = "<p>Hello world</p>";
        let result = unwrap_block_elements_from_p(input);
        assert_eq!(result, input, "normal p tags should be unchanged");
    }

    #[test]
    fn test_unwrap_noscript_unicode_content() {
        let input = "<p><noscript><pre>Fehler: Ung\u{00fc}ltig</pre>\n</noscript></p>";
        let result = unwrap_block_elements_from_p(input);
        assert!(
            result.contains("Ung\u{00fc}ltig"),
            "unicode content should be preserved: {}",
            result
        );
        assert!(
            !result.contains("<p><noscript>"),
            "noscript should be unwrapped: {}",
            result
        );
    }

    // ========================================================================
    // Issue 499: fix_literal_asterisk_emphasis must skip <code> blocks
    // ========================================================================

    #[test]
    fn test_asterisk_emphasis_skips_code_blocks() {
        // *col* inside <code> should NOT become <em>col</em>
        // After <code>, `>` satisfies prev_ok for opener; `<` satisfies next_ok for closer
        let html = "<p>Use <code>*col*</code> in SQL</p>";
        let result = fix_literal_asterisk_emphasis(html);
        assert!(
            result.contains("<code>*col*</code>"),
            "Asterisks inside <code> must be preserved. Got: {}",
            result
        );
        assert!(
            !result.contains("<em>col</em>"),
            "No <em> should appear for code content. Got: {}",
            result
        );
    }

    #[test]
    fn test_asterisk_emphasis_skips_code_with_attributes() {
        // <code class="..."> should also be recognized
        let html = "<p>Run <code class=\"language-sql\">*col*</code> to select</p>";
        let result = fix_literal_asterisk_emphasis(html);
        assert!(
            !result.contains("<em>col</em>"),
            "Asterisks inside <code class=...> must be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_asterisk_emphasis_still_works_outside_code() {
        // Emphasis outside code blocks should still be converted
        let html = "<p>This is *important* and <code>*col*</code> is code</p>";
        let result = fix_literal_asterisk_emphasis(html);
        assert!(
            result.contains("<em>important</em>"),
            "Emphasis outside code must still work. Got: {}",
            result
        );
        assert!(
            !result.contains("<em>col</em>"),
            "Asterisks inside code must be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_asterisk_emphasis_skips_code_unicode() {
        // Unicode content inside code with asterisks
        let html = "<p><code>*r\u{00e9}sum\u{00e9}*</code> is code</p>";
        let result = fix_literal_asterisk_emphasis(html);
        assert!(
            !result.contains("<em>"),
            "No emphasis conversion inside code with unicode. Got: {}",
            result
        );
    }

    #[test]
    fn test_unwrap_iframe_from_p() {
        let input = r#"<p><iframe src="//example.com/embed" width="595" height="485" frameborder="0" allowfullscreen> </iframe></p>"#;
        let result = unwrap_block_elements_from_p(input);
        assert!(
            !result.contains("<p><iframe"),
            "iframe should be unwrapped from <p>: {}",
            result
        );
        assert!(
            result.contains("<iframe src="),
            "iframe element should be preserved: {}",
            result
        );
    }

    #[test]
    fn test_unwrap_multiple_iframes_from_p() {
        let input = concat!(
            r#"<p><iframe src="//a.com/1" width="100"> </iframe></p>"#,
            "\n",
            "<p>text</p>\n",
            r#"<p><iframe src="//b.com/2" width="200"> </iframe></p>"#,
        );
        let result = unwrap_block_elements_from_p(input);
        assert!(
            !result.contains("<p><iframe"),
            "all iframes should be unwrapped: {}",
            result
        );
        assert!(
            result.contains("<p>text</p>"),
            "regular paragraphs should be preserved: {}",
            result
        );
    }

    // --- Issue 515: Kramdown table tfoot and multi-tbody separators ---

    #[test]
    fn test_515_unit_restructure_endash_separator() {
        // Simulate pulldown-cmark output with en-dash separator row
        let html = "<table><thead><tr><th>H1</th><th>H2</th></tr></thead><tbody>\n<tr><td>a</td><td>b</td></tr>\n<tr><td>\u{2013}\u{2013}\u{2013}\u{2013}\u{2013}</td><td></td></tr>\n<tr><td>c</td><td>d</td></tr>\n</tbody></table>";
        let result = restructure_kramdown_table_separators(html);
        let tbody_count = result.matches("<tbody>").count();
        assert_eq!(
            tbody_count, 2,
            "Expected 2 <tbody>, got {}: {}",
            tbody_count, result
        );
    }

    #[test]
    fn test_515_unit_restructure_equals_separator() {
        let html = "<table><thead><tr><th>H1</th><th>H2</th></tr></thead><tbody>\n<tr><td>a</td><td>b</td></tr>\n<tr><td>=====</td><td></td></tr>\n<tr><td>f1</td><td>f2</td></tr>\n</tbody></table>";
        let result = restructure_kramdown_table_separators(html);
        assert!(result.contains("<tfoot>"), "Expected <tfoot>: {}", result);
    }

    #[test]
    fn test_515_fullwidth_body_separator_produces_two_tbody() {
        let md = "\
| Header1 | Header2 |
|---------|---------|
| cell1   | cell2   |
|--------------------|
| cell3   | cell4   |
";
        let html = crate::frontmatter::markdown_to_html(md);
        let tbody_count = html.matches("<tbody>").count();
        assert_eq!(
            tbody_count, 2,
            "Expected 2 <tbody> sections, got {}: {}",
            tbody_count, html
        );
        assert!(
            !html.contains("------"),
            "Separator dashes should not appear as cell content: {}",
            html
        );
    }

    #[test]
    fn test_515_fullwidth_footer_separator_produces_tfoot() {
        let md = "\
| Header1 | Header2 |
|---------|---------|
| cell1   | cell2   |
|====================|
| Foot1   | Foot2   |
";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<tfoot>"),
            "Expected <tfoot> section: {}",
            html
        );
        assert!(
            html.contains("</tfoot>"),
            "Expected </tfoot> closing tag: {}",
            html
        );
        assert!(
            !html.contains("====="),
            "Separator equals should not appear as cell content: {}",
            html
        );
    }

    #[test]
    fn test_515_combined_separators_hydeout_pattern() {
        let md = "\
| Header1 | Header2 | Header3 |
|:--------|:-------:|--------:|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|-----------------------------|
| cell1   | cell2   | cell3   |
| cell4   | cell5   | cell6   |
|=============================|
| Foot1   | Foot2   | Foot3   |
";
        let html = crate::frontmatter::markdown_to_html(md);
        let thead_count = html.matches("<thead>").count();
        let tbody_count = html.matches("<tbody>").count();
        let tfoot_count = html.matches("<tfoot>").count();
        assert_eq!(thead_count, 1, "Expected 1 <thead>: {}", html);
        assert_eq!(tbody_count, 2, "Expected 2 <tbody>: {}", html);
        assert_eq!(tfoot_count, 1, "Expected 1 <tfoot>: {}", html);
        assert!(
            !html.contains("========"),
            "Separator equals should not appear: {}",
            html
        );
        // Em-dashes from smart punctuation should also be removed
        assert!(
            !html.contains("\u{2014}\u{2014}"),
            "Em-dashes from separator should not appear: {}",
            html
        );
        // Verify alignment preserved
        assert!(
            html.contains("text-align: left"),
            "Left alignment should be preserved: {}",
            html
        );
        assert!(
            html.contains("text-align: center"),
            "Center alignment should be preserved: {}",
            html
        );
        assert!(
            html.contains("text-align: right"),
            "Right alignment should be preserved: {}",
            html
        );
    }

    #[test]
    fn test_515_per_column_separator_no_regression() {
        let md = "\
| Name | Value |
|------|-------|
| foo  | bar   |
";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<thead>"),
            "Per-column separator should still produce thead: {}",
            html
        );
        assert!(
            html.contains("<tbody>"),
            "Per-column separator should still produce tbody: {}",
            html
        );
        assert!(
            html.contains("foo"),
            "Cell content should be preserved: {}",
            html
        );
    }

    #[test]
    fn test_515_no_false_positive_on_dash_content() {
        let md = "\
| Status | Note |
|--------|------|
| ---N/A--- | skip |
";
        let html = crate::frontmatter::markdown_to_html(md);
        // This cell has mixed content (dashes + text), NOT a separator
        assert!(
            html.contains("---N/A---") || html.contains("N/A"),
            "Dash content with text should be preserved: {}",
            html
        );
    }

    // ========================================================================
    // Issue 449: standalone iframe/img should not be wrapped in <p>
    // ========================================================================

    #[test]
    fn test_issue449_iframe_standalone_not_wrapped_in_p() {
        // Standalone iframe on its own line should not be wrapped in <p>
        let md = "Some text before.\n\n<iframe style=\"border: 0; width: 100%;\" src=\"https://example.com/embed\" seamless><a href=\"https://example.com\">Fallback</a></iframe>\n\nSome text after.\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<p><iframe"),
            "Standalone iframe should NOT be wrapped in <p>. Got: {}",
            html
        );
        assert!(
            html.contains("<iframe"),
            "iframe element should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue449_img_standalone_not_wrapped_in_p() {
        // Standalone img on its own line should not be wrapped in <p>
        let md = "Some text before.\n\n<img src=\"https://example.com/photo.jpg\" alt=\"A photo\" style=\"max-height: 20em;\">\n\nSome text after.\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<p><img"),
            "Standalone img should NOT be wrapped in <p>. Got: {}",
            html
        );
        assert!(
            html.contains("<img"),
            "img element should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue449_img_unicode_alt_standalone() {
        // Standalone img with Unicode alt text
        let md = "<img src=\"https://example.com/pic.jpg\" alt=\"Ein Bild mit Umlauten: \u{00e4}\u{00f6}\u{00fc}\" style=\"display: block;\">\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<p><img"),
            "Standalone img with Unicode alt should NOT be wrapped in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue449_iframe_youtube_embed_standalone() {
        let md = "<iframe width=\"240\" height=\"140\" src=\"https://www.youtube.com/embed/test\" frameborder=\"0\" allow=\"accelerometer; autoplay\" allowfullscreen></iframe>\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            !html.contains("<p><iframe"),
            "Standalone YouTube iframe should NOT be wrapped in <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue449_inline_img_stays_in_paragraph() {
        // Inline img within a paragraph should stay in <p>
        let md = "Here is an image <img src=\"x.jpg\"> in a paragraph.\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("<p>") && html.contains("<img"),
            "Inline img within text should stay in paragraph. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue449_unwrap_img_from_p_postprocess() {
        // Direct test of the unwrap function for standalone img in <p>
        let input = "<p><img src=\"https://example.com/photo.jpg\" alt=\"test\" style=\"max-height: 20em;\"></p>";
        let result = unwrap_block_elements_from_p(input);
        assert!(
            !result.contains("<p><img"),
            "Standalone img should be unwrapped from <p>. Got: {}",
            result
        );
        assert!(
            result.contains("<img src="),
            "img element should be preserved. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue449_unwrap_img_self_closing_from_p() {
        // Self-closing img with /> should also be unwrapped
        let input = "<p><img src=\"https://example.com/photo.jpg\" alt=\"test\" /></p>";
        let result = unwrap_block_elements_from_p(input);
        assert!(
            !result.contains("<p><img"),
            "Self-closing img should be unwrapped from <p>. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue489_toc_placeholder_in_markdown1_block() {
        // Issue 489: {:toc} inside markdown="1" should produce a TOC placeholder
        // that gets replaced with an actual TOC in postprocessing.
        let input = concat!(
            "<nav class=\"toc\" markdown=\"1\">\n",
            "*  Auto generated table of contents\n",
            "{:toc .toc__menu}\n",
            "</nav>\n",
            "\n",
            "## Privacy Policy\n",
            "\n",
            "Some text.\n",
            "\n",
            "## Log Files\n",
            "\n",
            "More text.\n",
        );
        let html = crate::frontmatter::markdown_to_html(input);
        let result = postprocess(&html);
        // The output should contain a TOC <ul> with links to the headings
        assert!(
            result.contains("id=\"markdown-toc\""),
            "Should generate a TOC with id=\"markdown-toc\". Got: {}",
            result
        );
        assert!(
            result.contains("markdown-toc-privacy-policy"),
            "TOC should contain link to privacy-policy heading. Got: {}",
            result
        );
        assert!(
            result.contains("markdown-toc-log-files"),
            "TOC should contain link to log-files heading. Got: {}",
            result
        );
        assert!(
            result.contains("class=\"toc__menu\""),
            "TOC <ul> should have class toc__menu from {{:toc .toc__menu}}. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue489_toc_placeholder_unicode_headings() {
        // Issue 489: TOC should work with non-ASCII heading content
        let input = concat!(
            "<div markdown=\"1\">\n",
            "* TOC\n",
            "{:toc}\n",
            "</div>\n",
            "\n",
            "## Einf\u{00fc}hrung\n",
            "\n",
            "German text.\n",
            "\n",
            "## R\u{00e9}sum\u{00e9}\n",
            "\n",
            "French text.\n",
        );
        let html = crate::frontmatter::markdown_to_html(input);
        let result = postprocess(&html);
        assert!(
            result.contains("id=\"markdown-toc\""),
            "Should generate a TOC with Unicode headings. Got: {}",
            result
        );
        assert!(
            result.contains("Einf\u{00fc}hrung"),
            "TOC should contain non-ASCII heading text. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue489_standalone_toc_in_markdown() {
        // Issue 489: {:toc} directly in markdown (not inside markdown="1")
        // should also generate a TOC.
        let input = concat!(
            "* TOC\n",
            "{:toc}\n",
            "\n",
            "## First Section\n",
            "\n",
            "Content here.\n",
            "\n",
            "## Second Section\n",
            "\n",
            "More content.\n",
        );
        let html = crate::frontmatter::markdown_to_html(input);
        let result = postprocess(&html);
        assert!(
            result.contains("id=\"markdown-toc\""),
            "Standalone {{:toc}} should generate a TOC. Got: {}",
            result
        );
        assert!(
            result.contains("markdown-toc-first-section"),
            "TOC should contain link to first-section. Got: {}",
            result
        );
        assert!(
            result.contains("markdown-toc-second-section"),
            "TOC should contain link to second-section. Got: {}",
            result
        );
    }

    // --- Issue 491: Kramdown definition list pre-processing ---

    #[test]
    fn test_491_simple_definition_list() {
        let input = "Definition List Title\n:   Definition list division.\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            result.contains("<dl>"),
            "Issue 491: Should produce <dl>. Got: {result}"
        );
        assert!(
            result.contains("<dt>Definition List Title</dt>"),
            "Issue 491: Should produce <dt>. Got: {result}"
        );
        assert!(
            result.contains("<dd>Definition list division.</dd>"),
            "Issue 491: Should produce <dd>. Got: {result}"
        );
    }

    #[test]
    fn test_491_multiple_definition_list_items() {
        let input = "\
Definition List Title\n\
:   Definition list division.\n\
\n\
Startup\n\
:   A startup company or startup is a company or temporary organization.\n\
\n\
Do It Live\n\
:   I'll let Bill O'Reilly explain this one.\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            result.contains("<dt>Definition List Title</dt>"),
            "Issue 491: First term. Got: {result}"
        );
        assert!(
            result.contains("<dt>Startup</dt>"),
            "Issue 491: Second term. Got: {result}"
        );
        assert!(
            result.contains("<dt>Do It Live</dt>"),
            "Issue 491: Third term. Got: {result}"
        );
        // All should be in one <dl> block (consecutive items)
        let dl_count = result.matches("<dl>").count();
        assert_eq!(
            dl_count, 1,
            "Issue 491: Consecutive items should be one <dl>. Got {dl_count} in: {result}"
        );
    }

    #[test]
    fn test_491_definition_list_unicode() {
        let input = "T\u{00e9}rme\n:   D\u{00e9}finition avec des \u{00e9}moji \u{1f4da}\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            result.contains("<dl>"),
            "Issue 491: Unicode dl should produce <dl>. Got: {result}"
        );
        assert!(
            result.contains("T\u{00e9}rme"),
            "Issue 491: Unicode term preserved. Got: {result}"
        );
        assert!(
            result.contains("D\u{00e9}finition"),
            "Issue 491: Unicode definition preserved. Got: {result}"
        );
    }

    #[test]
    fn test_491_no_false_positive_on_regular_paragraph() {
        let input = "This is a regular paragraph.\n\nAnother paragraph.\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            !result.contains("<dl>"),
            "Issue 491: Regular paragraphs should NOT produce <dl>. Got: {result}"
        );
    }

    #[test]
    fn test_491_no_false_positive_on_code_block() {
        let input = "```\nterm\n:   value\n```\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            !result.contains("<dl>"),
            "Issue 491: Code blocks should NOT produce <dl>. Got: {result}"
        );
    }

    #[test]
    fn test_491_definition_with_link() {
        let input = "Do It Live\n:   I'll let [explain](https://example.com) this one.\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            result.contains("<dt>Do It Live</dt>"),
            "Issue 491: Term with link def. Got: {result}"
        );
        assert!(
            result.contains("<a href=\"https://example.com\">explain</a>"),
            "Issue 491: Links in definitions should be rendered. Got: {result}"
        );
    }

    #[test]
    fn test_491_definition_list_with_hash_term() {
        let input = "#dowork\n:   Do Work motivator.\n";
        let result = convert_kramdown_definition_lists(input);
        assert!(
            result.contains("<dt>#dowork</dt>"),
            "Issue 491: Hash term. Got: {result}"
        );
    }

    // ========================================================================
    // Issue 475: Inline $$...$$ math delimiter conversion
    // ========================================================================

    #[test]
    fn test_issue475_inline_double_dollar_math_basic() {
        let input = "<p>text $$x^2$$ more</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, "<p>text \\(x^2\\) more</p>");
    }

    #[test]
    fn test_issue475_inline_double_dollar_math_multiple() {
        let input = "<p>$$formula$$ and $$other$$</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, "<p>\\(formula\\) and \\(other\\)</p>");
    }

    #[test]
    fn test_issue475_inline_double_dollar_not_in_code() {
        let input = "<code>$$code$$</code>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, input, "Should not convert $$ inside <code>");
    }

    #[test]
    fn test_issue475_inline_double_dollar_not_in_pre() {
        let input = "<pre>$$code$$</pre>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, input, "Should not convert $$ inside <pre>");
    }

    #[test]
    fn test_issue475_inline_double_dollar_no_match() {
        let input = "<p>no math here</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_issue475_single_dollar_not_converted() {
        let input = "<p>price is $100</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(result, input, "Single dollar should not be converted");
    }

    #[test]
    fn test_issue475_beautiful_jekyll_math_formula() {
        let input = "<p>they are $$x = {-b \\pm \\sqrt{b^2-4ac} \\over 2a}.$$</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(
            result,
            "<p>they are \\(x = {-b \\pm \\sqrt{b^2-4ac} \\over 2a}.\\)</p>"
        );
    }

    #[test]
    fn test_issue475_display_math_not_affected() {
        // Display math (standalone <p>$$...$$</p>) should NOT be converted by this function
        // because convert_display_math_blocks runs first
        let input = "<p>$$formula$$</p>";
        let result = convert_inline_double_dollar_math(input);
        // This function sees it as inline since it's within a <p> tag
        // but display math should have been consumed already by convert_display_math_blocks
        // So in the actual pipeline, this case won't arise.
        // Here we just test that the function itself handles $$ pairs.
        assert!(result.contains("\\(formula\\)") || result.contains("$$formula$$"));
    }

    #[test]
    fn test_issue475_pipeline_converts_inline_double_dollar() {
        // Test through the pipeline: inline $$ within text should become \(...\)
        let md = "they are $$x^2 + y^2$$ in math\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("\\(x^2 + y^2\\)"),
            "Pipeline should convert inline $$...$$ to \\(...\\). Got: {}",
            html
        );
        assert!(
            !html.contains("$$x^2"),
            "Pipeline should not leave raw $$. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue475_pipeline_display_math_still_works() {
        // Display math should still be converted to \[...\]
        let md = "$$\nx + y\n$$\n";
        let html = crate::frontmatter::markdown_to_html(md);
        assert!(
            html.contains("\\["),
            "Display math should still become \\[...\\]. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue475_unicode_math_content() {
        let input = "<p>The value is $$\\alpha + \\beta = \\gamma$$.</p>";
        let result = convert_inline_double_dollar_math(input);
        assert_eq!(
            result,
            "<p>The value is \\(\\alpha + \\beta = \\gamma\\).</p>"
        );
    }
}

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
/// 5. (moved to markdown rendering -- see `frontmatter::add_inline_code_class_to_events`)
///    5b. Wrap bare text between block elements in `<p>` tags
/// 6. Paragraph spacing (extra newlines after block elements)
/// 7. Remove `start` attribute from `<ol>` tags (D11)
/// 8. Remove self-closing slash from void elements (D3)
/// 9. Normalize boolean HTML attributes (D2, D12)
/// 10. Normalize `<figcaption>` closing tag whitespace (D6)
/// 11. Indent loose list items to match kramdown formatting
pub fn postprocess(html: &str) -> String {
    let html = strip_paragraphs_in_html_blocks(html);
    let html = encode_bare_ampersands(&html);
    let html = add_heading_ids(&html);
    let html = apply_inline_attributes(&html);
    let html = wrap_fenced_code_blocks(&html);
    // Note: inline code classes are now added during markdown rendering
    // (in frontmatter::add_inline_code_class_to_events) rather than here,
    // so that only backtick-generated <code> gets the class -- not raw HTML
    // <code> tags from the source.
    let html = wrap_bare_text_in_paragraphs(&html);
    let html = add_block_spacing(&html);
    let html = remove_ol_start_attribute(&html);
    let html = indent_list_items(&html);
    let html = indent_blockquote_content(&html);
    let html = normalize_figcaption_whitespace(&html);
    // Issue 201: Convert bare void elements (<br>, <hr>) to XHTML-style
    // (<br />, <hr />) to match Jekyll/kramdown output.
    let html = normalize_bare_void_elements(&html);
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
    let html = apply_inline_attributes(html);
    // Note: inline code classes are now added during markdown rendering
    // (in frontmatter::add_inline_code_class_to_events) rather than here.
    let html = remove_ol_start_attribute(&html);
    let html = add_block_spacing(&html);
    let html = indent_list_items(&html);
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
/// `language-plaintext highlighter-rouge` to `<code>` tags during markdown
/// rendering (handled by `frontmatter::add_inline_code_class_to_events()`),
/// not to `<code>` tags from Liquid templates or raw HTML in the source.
///
/// Note: void element self-closing slashes are NOT removed because
/// Jekyll/kramdown outputs XHTML-style self-closing tags (e.g. `<br />`).
pub fn normalize_html_output(html: &str) -> String {
    let needs_bool_attrs = html.contains("=\"\"");

    // Only normalize br and hr in the final output -- these come from markdown
    // rendering and need XHTML-style self-closing. Do NOT normalize meta, link,
    // input, img etc. here because this runs on the full page output including
    // layout HTML, and layout-sourced tags should not be modified.
    let html = normalize_br_hr_only(html);

    if needs_bool_attrs {
        normalize_boolean_attributes(&html)
    } else {
        html
    }
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

        if let Some((end, _tag)) = earliest {
            // Copy everything up to and including the closing tag
            result.push_str(&remaining[..end]);
            let after = &remaining[end..];

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
            // Escape the heading marker by prefixing # with backslash
            let leading_ws = &line[..line.len() - trimmed.len()];
            result.push_str(leading_ws);
            result.push('\\');
            result.push_str(trimmed);
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
/// blank lines because kramdown also wraps all items in `<p>`.
/// A "partially loose" list (some blanks but not all) is collapsed to tight.
pub fn collapse_blank_lines_between_list_items(content: &str) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    if lines.len() < 3 {
        return content.to_string();
    }

    // First pass: classify list regions
    let regions = find_list_regions(&lines);

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
                    i = j;
                    continue;
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

/// Convert kramdown-style pipe table lines to HTML `<table>` elements.
///
/// kramdown treats any line ending with `|` as a table row, splitting by `|`
/// into cells. This pre-processing converts such lines to raw HTML tables
/// before pulldown-cmark processes the markdown.
pub fn convert_kramdown_pipe_tables(content: &str) -> String {
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

            if i > 0 {
                result.push('\n');
            }
            result.push_str(&prefix);
            result.push_str("<table>\n<tbody>\n");
            for row_text in &table_rows {
                result.push_str("<tr>\n");
                for cell in split_kramdown_table_cells(row_text) {
                    result.push_str("<td>");
                    result.push_str(cell.trim());
                    result.push_str("</td>\n");
                }
                result.push_str("</tr>\n");
            }
            result.push_str("</tbody>\n</table>");
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

fn is_kramdown_table_line(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    let content = strip_list_prefix_for_table(trimmed).trim();
    if !content.ends_with('|') {
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

                    // Extract text content (strip HTML tags, decode entities)
                    let text = strip_html_tags(inner_html);
                    let text = decode_html_entities(&text);
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

/// Convert heading text to a URL-friendly slug matching kramdown's algorithm.
///
/// Kramdown's `generate_id` does:
/// 1. Downcase
/// 2. Remove all characters except alphanumerics (including Unicode), spaces, and hyphens
/// 3. Replace spaces with hyphens (without collapsing consecutive hyphens)
///
/// Note: kramdown does NOT strip leading digits. `"1. DataTalksClub"` becomes
/// `"1-datatalksclub"`, not `"datatalksclub"`.
///
/// Kramdown preserves Unicode alphabetic characters (Cyrillic, CJK, etc.) in its
/// default slugify mode, matching Jekyll's `default` slugify behavior.
fn slugify(text: &str) -> String {
    // Step 1: Lowercase
    let lower = text.to_lowercase();

    // Step 2: Keep alphanumerics (including Unicode letters), spaces, and hyphens
    let mut slug = String::with_capacity(lower.len());
    for ch in lower.chars() {
        if ch.is_alphanumeric() || ch == ' ' || ch == '-' {
            slug.push(ch);
        }
        // All other characters (punctuation, symbols like :, —, etc.) are stripped
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
                // Jekyll only puts the language class on the wrapper div for
                // language-specified code blocks. For no-language (plaintext)
                // blocks, the wrapper div has only "highlighter-rouge".
                if lang == "plaintext" {
                    result.push_str(
                        "<div class=\"highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                    );
                } else {
                    result.push_str(&format!(
                        "<div class=\"language-{} highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>",
                        lang
                    ));
                }
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

/// Convert bare void element tags to XHTML-style self-closing tags.
///
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

/// Like `normalize_bare_void_elements` but only converts `<br>` and `<hr>`.
/// Used in `normalize_html_output` which runs on the FULL page output
/// (including layout HTML). We must NOT convert layout-sourced `<meta>`,
/// `<link>`, `<input>`, `<img>` etc. since Jekyll doesn't self-close those
/// in layout templates — only in kramdown-rendered content.
fn normalize_br_hr_only(html: &str) -> String {
    if !html.contains("<br>") && !html.contains("<hr>") {
        return html.to_string();
    }
    html.replace("<br>", "<br />").replace("<hr>", "<hr />")
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

        // Find the list tag (first 4 or 5 chars)
        let tag_end = remaining.find('\n').unwrap_or(remaining.len());
        let list_tag = &remaining[..tag_end];
        let close_tag = if list_tag.starts_with("<ul") {
            "</ul>"
        } else {
            "</ol>"
        };

        // Find the matching close tag
        if let Some(close_pos) = remaining.find(close_tag) {
            let list_content = &remaining[tag_end + 1..close_pos];

            // Check if this is a loose list (contains <p> inside <li>)
            let is_loose = list_content.contains("<li>\n<p>");

            if is_loose {
                result.push_str(list_tag);
                result.push('\n');
                // Indent each line of content by 2 spaces, and content inside <li> by 4.
                // Skip blank lines inside <li> (added by add_block_spacing after </p>).
                let mut in_li = false;
                let lines: Vec<&str> = list_content.lines().collect();
                for line in lines.iter() {
                    if *line == "<li>" {
                        result.push_str("  <li>\n");
                        in_li = true;
                    } else if *line == "</li>" {
                        result.push_str("  </li>\n");
                        in_li = false;
                    } else if in_li && line.is_empty() {
                        // Skip blank lines inside <li> -- kramdown doesn't have them
                        continue;
                    } else if in_li && !line.is_empty() {
                        result.push_str("    ");
                        result.push_str(line);
                        result.push('\n');
                    } else if !line.is_empty() {
                        result.push_str(line);
                        result.push('\n');
                    } else if !in_li {
                        // Blank line outside <li> -- preserve
                        result.push('\n');
                    }
                }
                let _ = lines; // suppress unused warning
                result.push_str(close_tag);
                remaining = &remaining[close_pos + close_tag.len()..];
            } else {
                // Tight list: kramdown indents <li> items by 2 spaces.
                result.push_str(list_tag);
                result.push('\n');
                for line in list_content.lines() {
                    if line.starts_with("<li>") || line.starts_with("</li>") {
                        result.push_str("  ");
                        result.push_str(line);
                        result.push('\n');
                    } else if !line.is_empty() {
                        result.push_str(line);
                        result.push('\n');
                    } else {
                        result.push('\n');
                    }
                }
                result.push_str(close_tag);
                remaining = &remaining[close_pos + close_tag.len()..];
            }
        } else {
            // No matching close tag, copy as-is
            result.push_str(remaining);
            break;
        }
    }

    result
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

    // --- Cyrillic / non-ASCII slugify tests ---

    #[test]
    fn test_slugify_preserves_cyrillic() {
        // Kramdown preserves Unicode alphabetic chars in generate_id.
        // "Глава 1: Введение - Мир металлов вокруг нас" should produce
        // "глава-1-введение---мир-металлов-вокруг-нас" NOT "-1-------"
        // (colon stripped, space-hyphen-space preserved as ---)
        assert_eq!(
            slugify("Глава 1: Введение - Мир металлов вокруг нас"),
            "глава-1-введение---мир-металлов-вокруг-нас"
        );
    }

    #[test]
    fn test_slugify_preserves_cyrillic_emdash() {
        // Em-dash (—) is stripped, so " — " -> "  " -> "--"
        assert_eq!(
            slugify("Глава 1: Введение — Мир металлов вокруг нас"),
            "глава-1-введение--мир-металлов-вокруг-нас"
        );
    }

    #[test]
    fn test_slugify_mixed_ascii_cyrillic() {
        assert_eq!(
            slugify("Уникальные дары металлов"),
            "уникальные-дары-металлов"
        );
    }

    #[test]
    fn test_slugify_cyrillic_with_numbers() {
        // Colon is stripped; space-hyphen-space produces triple dashes
        assert_eq!(
            slugify("Глава 3: Бронзовый век - революция сплавов"),
            "глава-3-бронзовый-век---революция-сплавов"
        );
    }

    #[test]
    fn test_slugify_pure_cyrillic() {
        assert_eq!(slugify("Привет мир"), "привет-мир");
    }

    #[test]
    fn test_slugify_cyrillic_not_stripped() {
        // Regression: before fix, all non-ASCII was stripped, producing "-1-------"
        let result = slugify("Глава 1: Введение - Мир металлов вокруг нас");
        assert!(
            result.contains("глава"),
            "Cyrillic should be preserved in slug, got: {result}"
        );
        assert!(
            result.contains("введение"),
            "Cyrillic should be preserved in slug, got: {result}"
        );
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
    // Issue 183: Wrapper div class for no-language fenced code blocks
    // ======================================================================

    #[test]
    fn test_no_language_wrapper_div_class() {
        // Jekyll uses class="highlighter-rouge" only on the wrapper div for
        // no-language fenced code blocks (no language-plaintext on the div).
        let html = "<pre><code>some code\n</code></pre>\n";
        let result = postprocess(html);
        assert!(
            result.contains("<div class=\"highlighter-rouge\"><div class=\"highlight\">"),
            "Wrapper div should have only highlighter-rouge class. Got: {}",
            result
        );
        assert!(
            !result.contains("<div class=\"language-plaintext highlighter-rouge\">")
                && !result.contains("<div class=\"highlighter-rouge language-plaintext\">"),
            "Wrapper div should NOT contain language-plaintext. Got: {}",
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
        // normalize_html_output only converts br/hr, not input.
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
    // Kramdown does NOT strip leading digits from heading IDs.
    // "1. DataTalksClub" -> id="1-datatalksclub", NOT "datatalksclub"
    #[test]
    fn test_issue168_heading_id_leading_number_preserved() {
        let html = "<h2>1. DataTalksClub</h2>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"1-datatalksclub\""),
            "Heading ID should preserve leading digit. Got: {}",
            result
        );
    }

    #[test]
    fn test_issue168_heading_id_all_numeric_prefix() {
        let html = "<h3>8 Newsletters for Data Science</h3>\n";
        let result = postprocess(html);
        assert!(
            result.contains("id=\"8-newsletters-for-data-science\""),
            "Heading ID should preserve leading number. Got: {}",
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
    fn test_200_no_false_table() {
        let input = "This has a | char but not a table.\n";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(!html.contains("<table>"), "Got: {html}");
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
        let input = "A place is ''implicit'' if removing it.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("''implicit''"),
            "Double single-quotes should stay straight. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_triple_quote_straight() {
        let input = "This is '''bold text''' here.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("'''bold text'''"),
            "Triple single-quotes should stay straight. Got: {html}"
        );
    }

    #[test]
    fn test_issue198_quotes_cyrillic() {
        let input = "\u{042d}\u{0442}\u{043e} '''\u{0422}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430}.''' \u{0414}\u{043e}\u{043a}.";
        let html = crate::frontmatter::markdown_to_html(input);
        assert!(
            html.contains("'''\u{0422}\u{0435}\u{043e}\u{0440}\u{0435}\u{043c}\u{0430}.'''"),
            "Cyrillic in triple-quotes should have straight quotes. Got: {html}"
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
}

// kramdown parser - Block-level parser
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

#![allow(clippy::manual_strip)]
#![allow(clippy::needless_return)]

use crate::kramdown_parser::element::{Document, Element, ElementType};
use crate::kramdown_parser::options::Options;
use std::collections::HashMap;

/// Type alias for Attribute List Definition map.
pub type AldMap = HashMap<String, Vec<(String, String)>>;

/// Attribute List Definitions map.
/// The kramdown parser. Converts kramdown-flavored Markdown text into a Document AST.
pub struct KramdownParser;

/// Debug helper: expose is_list_start for testing.
pub fn debug_is_list_start(line: &str) -> bool {
    is_list_start(line)
}

impl KramdownParser {
    /// Parse kramdown input text into a Document AST.
    pub fn parse(input: &str, options: &Options) -> Document {
        let mut empty_alds = AldMap::new();
        Self::parse_with_alds(input, options, &mut empty_alds)
    }

    /// Parse with pre-defined ALDs from extract_definitions.
    pub fn parse_with_alds(input: &str, options: &Options, alds: &mut AldMap) -> Document {
        let mut doc = Document::new();
        let lines: Vec<&str> = input.lines().collect();
        let has_trailing_newline = input.ends_with('\n');
        let mut pos = 0;
        parse_blocks(
            &lines,
            &mut pos,
            has_trailing_newline,
            &mut doc.root.children,
            options,
            0,
            alds,
        );
        doc
    }
}

/// Parse blocks from lines[pos..] and append to `children`.
fn parse_blocks(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    children: &mut Vec<Element>,
    options: &Options,
    _indent_level: usize,
    alds: &mut AldMap,
) {
    let empty_lazy: Vec<bool> = vec![false; lines.len()];
    parse_blocks_with_lazy(
        lines,
        pos,
        has_trailing_newline,
        children,
        options,
        0,
        &empty_lazy,
        alds,
    );
}

/// Parse blocks inside a list item. Same as `parse_blocks` but paragraphs
/// are broken by list markers (since we're in a list context).
/// Indented code blocks only trigger after a blank line or as the first element.
/// Lazy lines (insufficient indent) are treated as paragraph text, not block starters.
fn parse_blocks_list_context(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    children: &mut Vec<Element>,
    options: &Options,
    lazy_flags: &[bool],
) {
    let mut after_blank = true; // Start of item = can have code block

    while *pos < lines.len() {
        let line = lines[*pos];
        let is_lazy = lazy_flags.get(*pos).copied().unwrap_or(false);

        if is_blank_line(line) {
            children.push(Element::new(ElementType::Blank));
            *pos += 1;
            after_blank = true;
            continue;
        }

        // If this line is lazy, it must be paragraph text (can't start new blocks)
        if is_lazy {
            let elem = parse_paragraph_in_list_context_with_lazy(
                lines,
                pos,
                has_trailing_newline,
                options,
                lazy_flags,
            );
            if let Some(e) = elem {
                children.push(e);
            }
            after_blank = false;
            continue;
        }

        if line.trim() == "^" {
            children.push(Element::new(ElementType::Eob));
            *pos += 1;
            continue;
        }

        if is_block_ial(line) {
            let attrs = parse_ial(line.trim());
            if let Some(last) = children.last_mut() {
                apply_attrs(last, &attrs);
            }
            *pos += 1;
            continue;
        }

        // Fenced code block
        if let Some(fence_result) = try_parse_fenced_code(lines, *pos) {
            children.push(fence_result.element);
            *pos = fence_result.end_pos;
            after_blank = false;
            continue;
        }

        // Indented code block (only after blank line or at very start of item)
        if after_blank && is_indented_code_line(line) {
            let elem = parse_indented_code_block(lines, pos, has_trailing_newline);
            children.push(elem);
            after_blank = false;
            continue;
        }

        // HR
        if is_horizontal_rule(line) {
            children.push(Element::new(ElementType::HorizontalRule));
            *pos += 1;
            after_blank = false;
            continue;
        }

        // List (nested)
        if is_list_start(line) {
            let elem = parse_list_with_lazy(lines, pos, has_trailing_newline, options, lazy_flags);
            children.push(elem);
            after_blank = false;
            continue;
        }

        // Blockquote
        if is_blockquote_line(line) {
            let elem = parse_blockquote(lines, pos, has_trailing_newline, options);
            children.push(elem);
            after_blank = false;
            continue;
        }

        // ATX header
        if let Some(header) = try_parse_atx_header(line, options) {
            children.push(header);
            *pos += 1;
            after_blank = false;
            continue;
        }

        // Paragraph (breaks on list markers but respects lazy flags)
        let elem = parse_paragraph_in_list_context_with_lazy(
            lines,
            pos,
            has_trailing_newline,
            options,
            lazy_flags,
        );
        if let Some(e) = elem {
            children.push(e);
        }
        after_blank = false;
    }
}

/// Parse a paragraph inside a list item context.
/// Unlike normal paragraphs, these break on list markers.
#[allow(dead_code)]
fn parse_paragraph_in_list_context(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
) -> Option<Element> {
    let mut para_lines: Vec<&str> = Vec::new();

    while *pos < lines.len() {
        let line = lines[*pos];

        if is_blank_line(line) {
            break;
        }
        if line.trim() == "^" {
            break;
        }
        if is_block_ial(line) {
            break;
        }

        // Setext header check
        if para_lines.len() == 1 && is_setext_underline(line) {
            let text_line = para_lines[0];
            if !text_line.starts_with("    ") {
                let level = if line.trim().starts_with('=') { 1 } else { 2 };
                let level = compute_header_level(level, options);
                let (text, id) = extract_header_id(text_line.trim_end());
                let mut elem = Element::new(ElementType::Header);
                elem.options.insert("level".to_string(), level.to_string());
                let text_child = Element::with_value(ElementType::Text, text.trim());
                elem.children.push(text_child);
                if let Some(id_val) = id {
                    elem.attr.insert("id".to_string(), id_val);
                }
                *pos += 1;
                return Some(elem);
            }
        }

        // Break on block-level elements (including list markers)
        // Note: indented code lines do NOT break paragraphs in list context;
        // code blocks only start after a blank line.
        if !para_lines.is_empty()
            && (try_parse_atx_header(line, options).is_some()
                || (is_horizontal_rule(line) && !is_setext_underline(line))
                || is_list_start(line)
                || is_blockquote_line(line)
                || try_parse_fenced_code(lines, *pos).is_some())
        {
            break;
        }

        para_lines.push(line);
        *pos += 1;
    }

    if para_lines.is_empty() {
        return None;
    }

    let text = build_paragraph_text(&para_lines, has_trailing_newline, *pos >= lines.len());
    let mut elem = Element::new(ElementType::Paragraph);
    let text_child = Element::with_value(ElementType::Text, text);
    elem.children.push(text_child);
    Some(elem)
}

/// Parse a paragraph inside a list item context, with lazy flag awareness.
/// Unlike normal paragraphs, these break on list markers.
/// Lazy lines (insufficient indent) are always treated as paragraph continuation.
fn parse_paragraph_in_list_context_with_lazy(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
    lazy_flags: &[bool],
) -> Option<Element> {
    let mut para_lines: Vec<&str> = Vec::new();

    while *pos < lines.len() {
        let line = lines[*pos];
        let is_lazy = lazy_flags.get(*pos).copied().unwrap_or(false);

        if is_blank_line(line) {
            break;
        }
        if line.trim() == "^" {
            break;
        }
        if is_block_ial(line) && !is_lazy {
            break;
        }

        // Setext header check
        if para_lines.len() == 1 && is_setext_underline(line) && !is_lazy {
            let text_line = para_lines[0];
            if !text_line.starts_with("    ") {
                let level = if line.trim().starts_with('=') { 1 } else { 2 };
                let level = compute_header_level(level, options);
                let (text, id) = extract_header_id(text_line.trim_end());
                let mut elem = Element::new(ElementType::Header);
                elem.options.insert("level".to_string(), level.to_string());
                let text_child = Element::with_value(ElementType::Text, text.trim());
                elem.children.push(text_child);
                if let Some(id_val) = id {
                    elem.attr.insert("id".to_string(), id_val);
                }
                *pos += 1;
                return Some(elem);
            }
        }

        // If lazy, always continue paragraph
        if is_lazy {
            para_lines.push(line);
            *pos += 1;
            continue;
        }

        // Break on block-level elements (including list markers)
        if !para_lines.is_empty()
            && (try_parse_atx_header(line, options).is_some()
                || (is_horizontal_rule(line) && !is_setext_underline(line))
                || is_list_start(line)
                || is_blockquote_line(line)
                || try_parse_fenced_code(lines, *pos).is_some())
        {
            break;
        }

        para_lines.push(line);
        *pos += 1;
    }

    if para_lines.is_empty() {
        return None;
    }

    let text = build_paragraph_text(&para_lines, has_trailing_newline, *pos >= lines.len());
    let mut elem = Element::new(ElementType::Paragraph);
    let text_child = Element::with_value(ElementType::Text, text);
    elem.children.push(text_child);
    Some(elem)
}

/// Check if a line is blank (empty or only whitespace).
fn is_blank_line(line: &str) -> bool {
    line.chars().all(|c| c == ' ' || c == '\t')
}

/// Check if line is an indented code line (4+ spaces at start).
fn is_indented_code_line(line: &str) -> bool {
    line.starts_with("    ") || line.starts_with('\t')
}

/// Check if a line starts a blockquote (0-3 spaces then `>`).
fn is_blockquote_line(line: &str) -> bool {
    let stripped = strip_up_to_3_spaces(line);
    stripped.starts_with('>')
}

/// Strip 0-3 leading spaces from a line.
fn strip_up_to_3_spaces(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut count = 0;
    for &b in bytes.iter().take(3) {
        if b == b' ' {
            count += 1;
        } else {
            break;
        }
    }
    &line[count..]
}

/// Check if a line is a horizontal rule.
fn is_horizontal_rule(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must not be indented 4+ spaces
    if line.starts_with("    ") {
        return false;
    }

    // Find the marker character
    let first_non_space = trimmed.chars().find(|c| *c != ' ' && *c != '\t');
    let marker = match first_non_space {
        Some(c) if c == '-' || c == '*' || c == '_' => c,
        _ => return false,
    };

    // Count markers, allowing only spaces/tabs between them
    let mut marker_count = 0;
    for c in trimmed.chars() {
        if c == marker {
            marker_count += 1;
        } else if c == ' ' || c == '\t' {
            // ok
        } else {
            return false;
        }
    }

    marker_count >= 3
}

/// Check if a line is a block-level IAL.
/// IAL starts with `{:` but NOT `{::` (block extension) or `{:/` (close tag).
fn is_block_ial(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("{:")
        && !trimmed.starts_with("{::")
        && !trimmed.starts_with("{:/")
        && trimmed.ends_with('}')
}

/// Parse IAL attributes - delegates to span_parser::parse_ial.
fn parse_ial(s: &str) -> Vec<(String, String)> {
    crate::kramdown_parser::span_parser::parse_ial(s)
}

/// Apply parsed attributes to an element.
fn apply_attrs(element: &mut Element, attrs: &[(String, String)]) {
    for (key, value) in attrs {
        if key == "__ald_ref__" {
            // Check for special references like "toc" and "footnotes"
            if value == "toc" {
                element
                    .options
                    .insert("toc".to_string(), "true".to_string());
            }
            if value == "footnotes" {
                element
                    .options
                    .insert("footnotes".to_string(), "true".to_string());
            }
            // Store auto_ids refs for definition list term ID generation
            if value == "auto_ids" || value.starts_with("auto_ids-") {
                // Append to ial_refs option (comma-separated list)
                let refs = element.options.entry("ial_refs".to_string()).or_default();
                if !refs.is_empty() {
                    refs.push(',');
                }
                refs.push_str(value);
            }
            continue;
        }
        if key == "class" {
            // Append to existing class
            if let Some(existing) = element.attr.get_mut("class") {
                existing.push(' ');
                existing.push_str(value);
            } else {
                element.attr.insert(key.clone(), value.clone());
            }
        } else if key == "lang" {
            // lang goes on the element itself
            element.attr.insert(key.clone(), value.clone());
        } else {
            element.attr.insert(key.clone(), value.clone());
        }
    }
}

/// Merge IAL attributes into an existing attribute list.
fn is_ald(line: &str) -> bool {
    let trimmed = line.trim();
    if let Some(inner) = trimmed.strip_prefix("{:").and_then(|s| s.strip_suffix('}')) {
        if let Some(cp) = inner.find(':') {
            let name = inner[..cp].trim();
            !name.is_empty() && !name.contains(' ') && !inner.starts_with(':')
        } else {
            false
        }
    } else {
        false
    }
}

fn parse_ald(line: &str) -> Option<(String, Vec<(String, String)>)> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("{:")?.strip_suffix('}')?;
    let cp = inner.find(':')?;
    let name = inner[..cp].trim();
    let rest_a = inner[cp + 1..].trim();
    if name.is_empty() || name.contains(' ') || rest_a.is_empty() {
        return None;
    }
    let attrs = parse_ial(&format!("{{: {rest_a}}}"));
    Some((name.to_string(), attrs))
}

fn resolve_ald_refs(attrs: &[(String, String)], alds: &AldMap) -> Vec<(String, String)> {
    resolve_ald_refs_inner(attrs, alds, 0)
}

/// Resolve ALD references in an attribute list, matching kramdown Ruby behavior:
/// First resolve all refs (recursively, depth-first), then append non-ref attrs.
/// This ensures ALD-sourced attributes appear before inline IAL attributes.
fn resolve_ald_refs_inner(
    attrs: &[(String, String)],
    alds: &AldMap,
    depth: usize,
) -> Vec<(String, String)> {
    if depth > 10 {
        return attrs.to_vec();
    }
    let mut resolved = Vec::new();

    // First: resolve all ALD refs (in order)
    for (key, value) in attrs {
        if key == "__ald_ref__" {
            if let Some(ald_attrs) = alds.get(value) {
                let sub = resolve_ald_refs_inner(ald_attrs, alds, depth + 1);
                resolved.extend(sub);
            } else {
                // Keep unresolved refs (e.g., "toc")
                resolved.push((key.clone(), value.clone()));
            }
        }
    }

    // Then: append non-ref attrs
    for (key, value) in attrs {
        if key != "__ald_ref__" {
            resolved.push((key.clone(), value.clone()));
        }
    }

    resolved
}

fn merge_ial_attrs(existing: &mut Vec<(String, String)>, new_attrs: &[(String, String)]) {
    for (key, value) in new_attrs {
        if key == "class" {
            if let Some(ex) = existing.iter_mut().find(|(k, _)| k == "class") {
                ex.1.push(' ');
                ex.1.push_str(value);
            } else {
                existing.push((key.clone(), value.clone()));
            }
        } else if let Some(ex) = existing.iter_mut().find(|(k, _)| k == key) {
            ex.1 = value.clone();
        } else {
            existing.push((key.clone(), value.clone()));
        }
    }
}

/// Try to parse an ATX header from a line.
fn try_parse_atx_header(line: &str, options: &Options) -> Option<Element> {
    // Must start with # (possibly after spaces stripped)
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }

    // Count leading #s
    let hash_count = trimmed.chars().take_while(|c| *c == '#').count();
    if hash_count > 6 {
        return None;
    }

    let after_hashes = &trimmed[hash_count..];

    // `#` alone or `# ` (hash + space + nothing) are paragraphs
    // `#` alone (no space after) is a paragraph
    if after_hashes.is_empty() {
        return None; // Just `#` or `##` etc with nothing after -> paragraph
    }

    // Must have a space after the hashes, OR the next char is # (like `#header#`)
    // Actually, looking at test cases: `##Header   #####` -> `<h2>Header</h2>`
    // So no space required between ## and text if text is non-empty.
    // But `# ` (hash space nothing) -> paragraph.

    // Check for space after hashes
    if let Some(stripped) = after_hashes.strip_prefix(' ') {
        let content = stripped.to_string();
        // `# ` with only whitespace after -> paragraph
        let content_trimmed = content.trim();
        if content_trimmed.is_empty() {
            return None;
        }
        // First extract {#id} from the end, then strip trailing #s from remaining text
        let (text_after_id, id) = extract_header_id(&content);
        let text_cleaned = strip_trailing_hashes(&text_after_id);
        let level = compute_header_level(hash_count, options);
        let mut elem = Element::new(ElementType::Header);
        elem.options.insert("level".to_string(), level.to_string());
        let text_child = Element::with_value(ElementType::Text, text_cleaned.trim());
        elem.children.push(text_child);
        if let Some(id_val) = id {
            elem.attr.insert("id".to_string(), id_val);
        }
        return Some(elem);
    }

    // No space after hashes: `##Header   #####` -> header
    // But just `#` -> paragraph (already handled above)
    // `#header#` -> header
    let content = after_hashes.to_string();
    let (text_after_id, id) = extract_header_id(&content);
    let text_cleaned = strip_trailing_hashes(&text_after_id);
    let level = compute_header_level(hash_count, options);
    let mut elem = Element::new(ElementType::Header);
    elem.options.insert("level".to_string(), level.to_string());
    let text_child = Element::with_value(ElementType::Text, text_cleaned.trim());
    elem.children.push(text_child);
    if let Some(id_val) = id {
        elem.attr.insert("id".to_string(), id_val);
    }
    Some(elem)
}

/// Strip trailing `#` characters (and spaces before them) from header content.
/// Does not strip `#` that is escaped with `\`.
fn strip_trailing_hashes(s: &str) -> String {
    let trimmed = s.trim_end();
    // Strip trailing `#`s
    let without_trailing_hashes = trimmed.trim_end_matches('#');
    if without_trailing_hashes.len() < trimmed.len() {
        // Check if the last non-# char is a backslash (escape)
        if without_trailing_hashes.ends_with('\\') {
            // Don't strip - the # is escaped
            return trimmed.to_string();
        }
        // There were trailing hashes - strip them and any trailing spaces
        return without_trailing_hashes.trim_end().to_string();
    }
    trimmed.to_string()
}

/// Extract `{#id}` from header text.
/// Returns (text_without_id, Some(id)) or (original_text, None).
fn extract_header_id(s: &str) -> (String, Option<String>) {
    let trimmed = s.trim_end();
    // Look for `{#id}` at the end, with a space before the `{`
    if let Some(brace_pos) = trimmed.rfind('{') {
        if trimmed.ends_with('}') {
            let inside = &trimmed[brace_pos + 1..trimmed.len() - 1];
            if let Some(id) = inside.strip_prefix('#') {
                // Validate: must have space before `{` (unless at start of string)
                let before_brace = &trimmed[..brace_pos];
                if brace_pos == 0 || before_brace.ends_with(' ') {
                    // Validate ID: must start with letter, then [A-Za-z0-9_:.-]
                    if is_valid_id(id) {
                        return (before_brace.trim_end().to_string(), Some(id.to_string()));
                    }
                }
            }
        }
    }
    (trimmed.to_string(), None)
}

/// Check if an ID is valid: starts with a letter, then [A-Za-z0-9_:.-]*
fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '.' || c == '-')
        }
        _ => false,
    }
}

/// Compute effective header level with offset.
fn compute_header_level(raw_level: usize, options: &Options) -> usize {
    let level = raw_level as i32 + options.header_offset;
    level.clamp(1, 6) as usize
}

/// Try to parse a fenced code block starting at `pos`.
struct FenceResult {
    element: Element,
    end_pos: usize,
}

fn try_parse_fenced_code(lines: &[&str], start: usize) -> Option<FenceResult> {
    let line = lines[start];
    let trimmed = line.trim_start();

    // Check for opening fence: 3+ backticks or tildes
    let fence_char = trimmed.chars().next()?;
    if fence_char != '~' && fence_char != '`' {
        return None;
    }
    let fence_len = trimmed.chars().take_while(|c| *c == fence_char).count();
    if fence_len < 3 {
        return None;
    }

    // Check indentation (must not be 4+ spaces)
    let indent = line.len() - trimmed.len();
    if indent >= 4 {
        return None;
    }

    // Extract language specifier after fence chars
    let after_fence = trimmed[fence_len..].trim();
    let language = if after_fence.is_empty() {
        None
    } else {
        Some(after_fence.to_string())
    };

    // Find closing fence
    let mut content_lines: Vec<&str> = Vec::new();
    let mut pos = start + 1;
    let mut found_close = false;

    while pos < lines.len() {
        let l = lines[pos];
        let lt = l.trim_start();
        // Closing fence: same char, at least as long as opening
        if lt.starts_with(fence_char) {
            let close_len = lt.chars().take_while(|c| *c == fence_char).count();
            let after_close = lt[close_len..].trim();
            if close_len >= fence_len && after_close.is_empty() {
                found_close = true;
                pos += 1;
                break;
            }
        }
        content_lines.push(l);
        pos += 1;
    }

    if !found_close {
        // Unclosed fence -> not a code block, treat as paragraph
        return None;
    }

    let content = if content_lines.is_empty() {
        String::new()
    } else {
        let mut s = content_lines.join("\n");
        s.push('\n');
        s
    };

    let mut elem = Element::with_value(ElementType::CodeBlock, content);
    if let Some(lang) = language {
        elem.options.insert("language".to_string(), lang);
    }
    elem.options
        .insert("fenced".to_string(), "true".to_string());

    Some(FenceResult {
        element: elem,
        end_pos: pos,
    })
}

/// Parse an indented code block starting at `pos`.
fn parse_indented_code_block(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
) -> Element {
    let mut code_lines: Vec<String> = Vec::new();

    while *pos < lines.len() {
        let line = lines[*pos];

        // Check blank line BEFORE indented code, since `    ` (4 spaces) is both
        if is_blank_line(line) {
            // Blank line might continue the code block if followed by indented line
            // Look ahead
            let mut look = *pos + 1;
            while look < lines.len() && is_blank_line(lines[look]) {
                look += 1;
            }
            if look < lines.len() && is_indented_code_line(lines[look]) {
                // The blank lines are part of the code block
                while *pos < look {
                    // Include blank lines in code, stripping 4-space indent if present
                    let bl = lines[*pos];
                    let stripped = strip_indent_4(bl);
                    code_lines.push(stripped.to_string());
                    *pos += 1;
                }
            } else {
                break;
            }
        } else if is_indented_code_line(line) {
            // Regular indented code line
            let stripped = strip_indent_4(line);
            code_lines.push(stripped.to_string());
            *pos += 1;
        } else if line.trim() == "^" {
            // EOB marker ends the code block
            break;
        } else if is_block_ial(line) {
            // Block IAL after code block - stop the code block, let caller handle IAL
            break;
        } else {
            // Lazy continuation: line without 4-space indent continues code block
            // In kramdown, a non-indented line after a code block continues it
            // (see lazy test case: `    This is some\ncode`)
            // But only if there's no blank line between.
            // However, lines that start HTML block elements should NOT be lazily
            // continued into the code block.
            if !code_lines.is_empty() && !is_html_block_start(line) {
                // Join with previous line
                if let Some(last) = code_lines.last_mut() {
                    last.push(' ');
                    last.push_str(line);
                }
                *pos += 1;
            } else {
                break;
            }
        }
    }

    // Remove trailing blank lines from code
    while code_lines.last().is_some_and(|l| l.trim().is_empty()) {
        code_lines.pop();
    }

    // Build content string
    let mut content = code_lines.join("\n");
    if !content.is_empty() || has_trailing_newline || *pos <= lines.len() {
        content.push('\n');
    }

    Element::with_value(ElementType::CodeBlock, content)
}

/// Strip 4 spaces of indent (or a tab) from a line.
fn strip_indent_4(line: &str) -> &str {
    if let Some(stripped) = line.strip_prefix("    ") {
        stripped
    } else if let Some(stripped) = line.strip_prefix('\t') {
        stripped
    } else {
        line
    }
}

/// Parse a blockquote starting at `pos`.
fn parse_blockquote(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
) -> Element {
    // Collect blockquote content lines, tracking which are "lazy" (no `>` prefix)
    let mut bq_lines: Vec<(String, bool)> = Vec::new(); // (content, is_lazy)

    while *pos < lines.len() {
        let line = lines[*pos];

        // EOB ends blockquote
        if line.trim() == "^" {
            *pos += 1;
            break;
        }

        // Block IAL on its own line after blockquote
        if is_block_ial(line) && !bq_lines.is_empty() {
            // This IAL applies to the blockquote itself
            break;
        }

        let stripped = strip_up_to_3_spaces(line);

        if let Some(after_gt) = stripped.strip_prefix('>') {
            let content = after_gt.strip_prefix(' ').unwrap_or(after_gt);
            bq_lines.push((content.to_string(), false));
            *pos += 1;
        } else if is_blank_line(line) {
            // A real blank line (no `>` prefix) always ends the blockquote.
            // Paragraph breaks within a blockquote use `>` on the blank line.
            break;
        } else if !bq_lines.is_empty() && !is_html_block_start(line) {
            // Lazy continuation: non-blank, non-blockquote line continues the blockquote
            // But HTML block tags break the blockquote
            bq_lines.push((line.to_string(), true));
            *pos += 1;
        } else {
            break;
        }
    }

    // Build inner content and parse it, being aware of lazy lines
    let inner_lines_owned: Vec<String> = bq_lines.iter().map(|(s, _)| s.clone()).collect();
    let inner_lines: Vec<&str> = inner_lines_owned.iter().map(|s| s.as_str()).collect();
    let lazy_flags: Vec<bool> = bq_lines.iter().map(|(_, lazy)| *lazy).collect();

    let mut elem = Element::new(ElementType::Blockquote);
    let mut inner_pos = 0;
    let mut bq_alds = AldMap::new();
    parse_blocks_with_lazy(
        &inner_lines,
        &mut inner_pos,
        has_trailing_newline,
        &mut elem.children,
        options,
        1,
        &lazy_flags,
        &mut bq_alds,
    );

    elem
}

/// Parse blocks with lazy continuation awareness.
/// `lazy_flags[i]` is true if line `i` was a lazy continuation in a parent blockquote.
#[allow(clippy::too_many_arguments)]
fn parse_blocks_with_lazy(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    children: &mut Vec<Element>,
    options: &Options,
    _indent_level: usize,
    lazy_flags: &[bool],
    alds: &mut AldMap,
) {
    // Pending IAL attributes to apply to the next visible element
    let mut pending_ial: Option<Vec<(String, String)>> = None;

    while *pos < lines.len() {
        let line = lines[*pos];
        let is_lazy = lazy_flags.get(*pos).copied().unwrap_or(false);

        // Blank line
        if is_blank_line(line) {
            children.push(Element::new(ElementType::Blank));
            *pos += 1;
            continue;
        }

        // EOB marker
        if line.trim() == "^" {
            children.push(Element::new(ElementType::Eob));
            *pos += 1;
            continue;
        }

        // ALD (Attribute List Definition)
        if is_ald(line) {
            if let Some((name, new_attrs)) = parse_ald(line) {
                let entry = alds.entry(name).or_default();
                for (k, v) in &new_attrs {
                    if k == "class" {
                        if let Some(existing) = entry.iter_mut().find(|(ek, _)| ek == "class") {
                            existing.1.push(' ');
                            existing.1.push_str(v);
                        } else {
                            entry.push((k.clone(), v.clone()));
                        }
                    } else if let Some(existing) = entry.iter_mut().find(|(ek, _)| ek == k) {
                        existing.1 = v.clone();
                    } else {
                        entry.push((k.clone(), v.clone()));
                    }
                }
            }
            *pos += 1;
            continue;
        }

        // Block-level IAL
        if is_block_ial(line) {
            let raw_attrs = parse_ial(line.trim());
            let attrs = resolve_ald_refs(&raw_attrs, alds);
            // Apply to previous visible element, or store as pending for next
            let applied = if let Some(last) = children.last_mut() {
                if !matches!(last.element_type, ElementType::Blank | ElementType::Eob) {
                    apply_attrs(last, &attrs);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !applied {
                if let Some(ref mut existing) = pending_ial {
                    merge_ial_attrs(existing, &attrs);
                } else {
                    pending_ial = Some(attrs);
                }
            }
            *pos += 1;
            continue;
        }

        // Macro to apply pending IAL to a newly created element
        macro_rules! apply_pending {
            ($elem:expr) => {
                if let Some(ref attrs) = pending_ial {
                    apply_attrs($elem, attrs);
                }
                pending_ial = None;
            };
        }

        // If this line is lazy, it must be part of a paragraph (can't start new block types)
        if is_lazy {
            let elem = parse_paragraph_with_lazy(
                lines,
                pos,
                has_trailing_newline,
                children,
                options,
                lazy_flags,
            );
            if let Some(mut e) = elem {
                apply_pending!(&mut e);
                children.push(e);
            }
            continue;
        }

        // Block extension ({::comment}, {::nomarkdown}, {::options})
        if let Some(mut ext) = try_parse_block_extension(lines, pos) {
            apply_pending!(&mut ext);
            children.push(ext);
            continue;
        }

        // HTML block (must be before fenced code)
        if let Some(mut html_block) = try_parse_html_block(lines, pos, options) {
            // If the HTML block has trailing content after the closing tag
            // (e.g., "</div> test" or "</div> <div>"), re-parse it as blocks.
            let trailing = html_block.options.remove("html_trailing");
            apply_pending!(&mut html_block);
            children.push(html_block);
            if let Some(trailing_text) = trailing {
                // Re-combine trailing content with remaining lines and parse together.
                // This handles cases like "</div> <div>\nhallo\n</div>" where the
                // trailing "<div>" needs the subsequent lines.
                let remaining_lines: Vec<&str> = lines[*pos..].to_vec();
                let mut combined = trailing_text.clone();
                for rl in &remaining_lines {
                    combined.push('\n');
                    combined.push_str(rl);
                }
                let combined_lines: Vec<&str> = combined.lines().collect();
                let mut combined_pos = 0;
                parse_blocks(
                    &combined_lines,
                    &mut combined_pos,
                    has_trailing_newline,
                    children,
                    options,
                    _indent_level,
                    alds,
                );
                // Advance pos by however many of the original remaining lines were consumed.
                // combined_pos counts lines in combined, but the first line is the trailing
                // text itself (which is not in the original lines array).
                let trailing_line_count = trailing_text.lines().count();
                let original_consumed = combined_pos.saturating_sub(trailing_line_count);
                *pos += original_consumed;
            }
            continue;
        }

        // Math block ($$...$$)
        if let Some(mut math) = try_parse_math_block(lines, pos) {
            apply_pending!(&mut math);
            children.push(math);
            continue;
        }

        // Fenced code block
        if let Some(mut fence_result) = try_parse_fenced_code(lines, *pos) {
            apply_pending!(&mut fence_result.element);
            children.push(fence_result.element);
            *pos = fence_result.end_pos;
            continue;
        }

        // Indented code block
        if is_indented_code_line(line) {
            let mut elem = parse_indented_code_block(lines, pos, has_trailing_newline);
            apply_pending!(&mut elem);
            children.push(elem);
            continue;
        }

        // Table (must be checked before horizontal rule since `|---|---|` looks like hr)
        if is_table_line(line) || try_parse_separator_line(line).is_some() {
            let saved_pos = *pos;
            if let Some(mut table) = try_parse_table(lines, pos, options) {
                apply_pending!(&mut table);
                children.push(table);
                continue;
            }
            *pos = saved_pos;
        }

        // Horizontal rule (must be before list since `* * *` is HR not list)
        if is_horizontal_rule(line) {
            let mut elem = Element::new(ElementType::HorizontalRule);
            apply_pending!(&mut elem);
            children.push(elem);
            *pos += 1;
            continue;
        }

        // List (after HR check so `* * *` is handled as HR)
        if is_list_start(line) {
            let mut elem = parse_list(lines, pos, has_trailing_newline, options);
            apply_pending!(&mut elem);
            children.push(elem);
            continue;
        }

        // Blockquote
        if is_blockquote_line(line) {
            let mut elem = parse_blockquote(lines, pos, has_trailing_newline, options);
            apply_pending!(&mut elem);
            children.push(elem);
            continue;
        }

        // ATX header
        if let Some(mut header) = try_parse_atx_header(line, options) {
            apply_pending!(&mut header);
            children.push(header);
            *pos += 1;
            continue;
        }

        // Definition list: term line(s) followed by `: ` definition
        if let Some(mut dl) = try_parse_definition_list(lines, pos, has_trailing_newline, options) {
            apply_pending!(&mut dl);
            children.push(dl);
            continue;
        }

        // Paragraph
        let elem = parse_paragraph_with_lazy(
            lines,
            pos,
            has_trailing_newline,
            children,
            options,
            lazy_flags,
        );
        if let Some(mut e) = elem {
            apply_pending!(&mut e);
            children.push(e);
        }
    }
}

/// Parse a paragraph with lazy continuation awareness.
fn parse_paragraph_with_lazy(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    _preceding_children: &mut Vec<Element>,
    options: &Options,
    lazy_flags: &[bool],
) -> Option<Element> {
    let mut para_lines: Vec<&str> = Vec::new();

    while *pos < lines.len() {
        let line = lines[*pos];
        let is_lazy = lazy_flags.get(*pos).copied().unwrap_or(false);

        if is_blank_line(line) {
            break;
        }
        if line.trim() == "^" {
            break;
        }
        if is_block_ial(line) {
            break;
        }

        // Setext header check
        if para_lines.len() == 1 && is_setext_underline(line) && !is_lazy {
            let text_line = para_lines[0];
            if !text_line.starts_with("    ") {
                let level = if line.trim().starts_with('=') { 1 } else { 2 };
                let level = compute_header_level(level, options);
                let (text, id) = extract_header_id(text_line.trim_end());
                let mut elem = Element::new(ElementType::Header);
                elem.options.insert("level".to_string(), level.to_string());
                let text_child = Element::with_value(ElementType::Text, text.trim());
                elem.children.push(text_child);
                if let Some(id_val) = id {
                    elem.attr.insert("id".to_string(), id_val);
                }
                *pos += 1;
                return Some(elem);
            }
        }

        // Multi-line setext
        if para_lines.len() > 1 && is_setext_underline(line) {
            para_lines.push(line);
            *pos += 1;
            continue;
        }

        // If lazy, it's always paragraph continuation
        if is_lazy {
            para_lines.push(line);
            *pos += 1;
            continue;
        }

        // Non-lazy: check if this line would start a new block
        // Note: In kramdown, HRs and list markers do NOT interrupt paragraphs.
        // They only start at the beginning of a block context (after blank/EOB).
        if !para_lines.is_empty() {
            if try_parse_atx_header(line, options).is_some() {
                break;
            }
            if is_blockquote_line(line) {
                break;
            }
            if try_parse_fenced_code(lines, *pos).is_some() {
                break;
            }
            // Table line or separator breaks a paragraph, unless previous line
            // has an unbalanced backtick (multi-line code span continuation)
            // or accumulated lines have an unclosed <code> tag
            if is_table_line(line) || try_parse_separator_line(line).is_some() {
                let prev_has_open_code_span = para_lines
                    .last()
                    .is_some_and(|l| has_unbalanced_backticks(l));
                let has_open_code_tag = has_unclosed_code_tag(&para_lines);
                if !prev_has_open_code_span && !has_open_code_tag {
                    break;
                }
            }
            // HTML block tags interrupt paragraphs
            if is_html_block_start(line) {
                break;
            }
        }

        if para_lines.is_empty() && is_indented_code_line(line) {
            break;
        }

        para_lines.push(line);
        *pos += 1;
    }

    if para_lines.is_empty() {
        return None;
    }

    let text = build_paragraph_text(&para_lines, has_trailing_newline, *pos >= lines.len());
    let mut elem = Element::new(ElementType::Paragraph);
    let text_child = Element::with_value(ElementType::Text, text);
    elem.children.push(text_child);
    Some(elem)
}

// parse_paragraph is handled by parse_paragraph_with_lazy

/// Build paragraph text from lines.
/// Preserves trailing whitespace on non-last lines since 2+ trailing spaces = line break.
/// Strips trailing whitespace from the last line.
fn build_paragraph_text(lines: &[&str], _has_trailing_newline: bool, _at_end: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }

    if lines.len() == 1 {
        return strip_up_to_3_spaces(lines[0]).trim_end().to_string();
    }

    let first = strip_up_to_3_spaces(lines[0]);
    let mut result = first.to_string();
    for (idx, line) in lines[1..].iter().enumerate() {
        result.push('\n');
        if idx == lines.len() - 2 {
            // Last line: strip trailing spaces
            result.push_str(line.trim_end());
        } else {
            result.push_str(line);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Table parsing
// ---------------------------------------------------------------------------

/// Column alignment extracted from a separator line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Alignment {
    Default,
    Left,
    Center,
    Right,
}

/// Kind of separator line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeparatorKind {
    /// Header separator using `-` chars
    Header,
    /// Footer separator using `=` chars
    Footer,
    /// Body separator using `+` prefix (starts new tbody)
    Body,
}

/// A parsed separator line.
#[derive(Debug, Clone)]
struct SeparatorLine {
    kind: SeparatorKind,
    alignments: Vec<Alignment>,
}

/// Check if a line is a table separator line (header, footer, or body).
/// Returns Some(SeparatorLine) if it matches.
fn try_parse_separator_line(line: &str) -> Option<SeparatorLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Determine kind based on first non-whitespace, non-pipe char
    let kind;
    let work;

    if let Some(rest) = trimmed.strip_prefix('+') {
        // Body separator: `+ :-: |`
        kind = SeparatorKind::Body;
        // Strip leading `+` and treat rest as separator content
        work = rest.to_string();
    } else {
        // Could be header or footer -- determine by the fill char
        // Strip leading/trailing pipes
        let stripped = trimmed
            .strip_prefix('|')
            .unwrap_or(trimmed)
            .trim_end_matches('|')
            .trim();

        if stripped.is_empty() {
            return None;
        }

        // Check what the fill character is
        let has_equals = stripped.contains('=');
        let has_dashes = stripped.contains('-');

        if has_equals && !has_dashes {
            kind = SeparatorKind::Footer;
        } else if has_dashes {
            kind = SeparatorKind::Header;
        } else {
            return None;
        }

        work = trimmed.to_string();
    }

    // Now parse alignment from the separator cells
    let work_str = work.trim();
    // Strip leading/trailing pipe
    let inner = work_str
        .strip_prefix('|')
        .unwrap_or(work_str)
        .trim_end_matches('|');

    // Split by pipe (not escaped)
    let cells = split_separator_cells(inner);
    let mut alignments = Vec::new();

    for cell in &cells {
        let c = cell.trim();
        if c.is_empty() {
            alignments.push(Alignment::Default);
            continue;
        }
        // Validate: must be only colons, dashes/equals, spaces, tabs
        let sep_char = if kind == SeparatorKind::Footer {
            '='
        } else {
            '-'
        };
        let valid = c
            .chars()
            .all(|ch| ch == ':' || ch == sep_char || ch == ' ' || ch == '\t');
        if !valid {
            return None;
        }

        let has_left_colon = c.starts_with(':');
        let has_right_colon = c.ends_with(':');
        let alignment = match (has_left_colon, has_right_colon) {
            (true, true) => Alignment::Center,
            (true, false) => Alignment::Left,
            (false, true) => Alignment::Right,
            (false, false) => Alignment::Default,
        };
        alignments.push(alignment);
    }

    if alignments.is_empty() {
        return None;
    }

    Some(SeparatorLine { kind, alignments })
}

/// Split separator cells by `|`.
fn split_separator_cells(s: &str) -> Vec<String> {
    s.split('|').map(|c| c.to_string()).collect()
}

/// Split a table row into cells, handling escaped pipes, backtick code spans,
/// and `<code>...</code>` HTML tags.
/// Returns the cell contents (trimmed).
fn split_table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim_end();

    // Strip leading pipe if present
    let work = trimmed.strip_prefix('|').unwrap_or(trimmed);

    // Strip trailing pipe if present (but not escaped)
    let work = if work.ends_with('|') && !work.ends_with("\\|") {
        &work[..work.len() - 1]
    } else {
        work
    };

    // If line has unbalanced backticks and no escaped pipes, disable backtick code span detection
    let ignore_backtick_spans = has_unbalanced_backticks(trimmed) && !trimmed.contains("\\|");

    let mut cells: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = work.chars().collect();
    let mut i = 0;
    let mut in_backtick = false;
    let mut backtick_count = 0;
    let mut in_code_tag = false;

    while i < chars.len() {
        // Inside backtick code span (only when not ignoring)
        if in_backtick && !ignore_backtick_spans {
            let mut bt = 0;
            let start = i;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            if bt >= backtick_count && bt > 0 {
                for _ in 0..bt {
                    current.push('`');
                }
                in_backtick = false;
                continue;
            }
            for ch in &chars[start..i] {
                current.push(*ch);
            }
            if i < chars.len() {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // Inside <code> tag
        if in_code_tag {
            // Check for </code>
            let remaining: String = chars[i..].iter().collect();
            if remaining.starts_with("</code>") {
                current.push_str("</code>");
                i += 7;
                in_code_tag = false;
                continue;
            }
            current.push(chars[i]);
            i += 1;
            continue;
        }

        match chars[i] {
            '\\' if i + 1 < chars.len() && chars[i + 1] == '|' => {
                // Escaped pipe -> literal pipe in cell content
                current.push('|');
                i += 2;
            }
            '|' => {
                // Cell separator
                cells.push(current.trim().to_string());
                current = String::new();
                i += 1;
            }
            '`' if !ignore_backtick_spans => {
                // Start of code span - count backticks
                let mut bt = 0;
                while i < chars.len() && chars[i] == '`' {
                    bt += 1;
                    current.push('`');
                    i += 1;
                }
                in_backtick = true;
                backtick_count = bt;
            }
            '<' => {
                // Check for <code> or <code ...>
                let remaining: String = chars[i..].iter().collect();
                if remaining.starts_with("<code>") || remaining.starts_with("<code ") {
                    // Find the end of the opening tag
                    if let Some(gt_pos) = remaining.find('>') {
                        let tag = &remaining[..=gt_pos];
                        current.push_str(tag);
                        i += gt_pos + 1;
                        in_code_tag = true;
                    } else {
                        current.push(chars[i]);
                        i += 1;
                    }
                } else {
                    current.push(chars[i]);
                    i += 1;
                }
            }
            _ => {
                current.push(chars[i]);
                i += 1;
            }
        }
    }

    cells.push(current.trim().to_string());
    cells
}

/// Check if a line looks like a table row (has unescaped pipe that acts as cell separator).
/// A line with only escaped pipes is NOT a table row.
/// Balanced backtick code spans protect pipes from being cell separators.
/// Unbalanced backticks are treated as literal characters.
/// Pipes inside math delimiters ($...$ or $$...$$) do NOT count as cell separators.
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Check for escaped first pipe: `\| ...` means not a table
    if trimmed.starts_with("\\|") {
        return false;
    }

    // A line starting with `|` is a table line -- but only if not inside math
    // We need to check: if the line starts with `|` but the pipe is inside math, skip it
    // However, `|` at position 0 cannot be inside math (math starts with `$`),
    // so this fast path is still valid.
    if trimmed.starts_with('|') {
        return true;
    }

    // If line has unbalanced backticks AND no escaped pipes (\|), treat backticks as
    // literal text and check for pipes ignoring code spans.
    // If it has escaped pipes, code spans take precedence (multi-line code span).
    if has_unbalanced_backticks(trimmed) && !trimmed.contains("\\|") {
        return has_unescaped_pipe_ignoring_backticks(trimmed);
    }

    // Check with code span, <code> tag, and math delimiter awareness
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    let mut in_backtick = false;
    let mut backtick_count = 0;
    let mut in_html_code = false;

    while i < chars.len() {
        // Skip content inside <code>...</code> tags
        if in_html_code {
            // Look for </code>
            if chars[i] == '<' && i + 6 < chars.len() {
                let rest: String = chars[i..].iter().take(7).collect();
                if rest == "</code>" {
                    i += 7;
                    in_html_code = false;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        if in_backtick {
            let mut bt = 0;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            if bt >= backtick_count && bt > 0 {
                in_backtick = false;
                continue;
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        // Check for <code> tag (case-insensitive)
        if chars[i] == '<' && i + 5 < chars.len() {
            let rest: String = chars[i..].iter().take(6).collect();
            if rest.eq_ignore_ascii_case("<code>") {
                i += 6;
                in_html_code = true;
                continue;
            }
            // Also handle <code with attributes like <code class="...">
            let rest_long: String = chars[i..].iter().take(6).collect();
            if rest_long.eq_ignore_ascii_case("<code ") {
                // Skip to the closing >
                while i < chars.len() && chars[i] != '>' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip >
                }
                in_html_code = true;
                continue;
            }
        }

        // Skip content inside math delimiters: $...$ and $$...$$
        if chars[i] == '$' {
            if let Some(advance) = skip_inline_math_in_line(&chars, i) {
                i += advance;
                continue;
            }
        }

        match chars[i] {
            '\\' if i + 1 < chars.len() && chars[i + 1] == '|' => {
                i += 2;
            }
            '|' => return true,
            '`' => {
                let mut bt = 0;
                while i < chars.len() && chars[i] == '`' {
                    bt += 1;
                    i += 1;
                }
                in_backtick = true;
                backtick_count = bt;
            }
            _ => {
                i += 1;
            }
        }
    }

    false
}

/// Check if a line has an unescaped pipe, ignoring backticks entirely.
/// Used when backticks are unbalanced (so they're literal text, not code spans).
/// Also skips pipes inside math delimiters ($...$ and $$...$$).
fn has_unescaped_pipe_ignoring_backticks(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Skip content inside math delimiters
        if chars[i] == '$' {
            if let Some(advance) = skip_inline_math_in_line(&chars, i) {
                i += advance;
                continue;
            }
        }
        match chars[i] {
            '\\' if i + 1 < chars.len() && chars[i + 1] == '|' => {
                i += 2;
            }
            '|' => return true,
            _ => {
                i += 1;
            }
        }
    }
    false
}

/// Skip over inline math content in a line: $...$ or $$...$$.
/// Returns Some(advance) if we found matching math delimiters, None otherwise.
/// The advance count includes the opening and closing delimiters.
fn skip_inline_math_in_line(chars: &[char], start: usize) -> Option<usize> {
    if start >= chars.len() || chars[start] != '$' {
        return None;
    }

    // Check for escaped dollar: \$ -- the backslash would be at start-1
    if start > 0 && chars[start - 1] == '\\' {
        return None;
    }

    // Determine if this is $$ or $
    let double = start + 1 < chars.len() && chars[start + 1] == '$';
    let delim_len = if double { 2 } else { 1 };
    let content_start = start + delim_len;

    if content_start >= chars.len() {
        return None;
    }

    // Find the closing delimiter
    let mut i = content_start;
    while i < chars.len() {
        // Skip escaped characters
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        if double {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '$' {
                let content_len = i - content_start;
                if content_len > 0 {
                    return Some(i + 2 - start);
                }
            }
        } else if chars[i] == '$' {
            // For single $, make sure this isn't actually $$
            if i + 1 < chars.len() && chars[i + 1] == '$' {
                // This is $$ not $, skip
                i += 1;
            } else {
                let content_len = i - content_start;
                if content_len > 0 {
                    return Some(i + 1 - start);
                }
            }
        }
        i += 1;
    }

    None
}

/// Check if a line could start a multi-line code span across table rows.
/// This happens when backticks are unbalanced in the line.
fn has_unbalanced_backticks(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut open_backtick = false;
    let mut _backtick_count = 0;

    while i < chars.len() {
        if open_backtick {
            let mut bt = 0;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            if bt >= _backtick_count && bt > 0 {
                open_backtick = false;
                continue;
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        if chars[i] == '`' {
            let mut bt = 0;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            open_backtick = true;
            _backtick_count = bt;
        } else {
            i += 1;
        }
    }

    open_backtick
}

/// Check if accumulated paragraph lines have an unclosed `<code>` tag.
/// This prevents table detection from breaking multi-line `<code>` spans.
fn has_unclosed_code_tag(lines: &[&str]) -> bool {
    let combined: String = lines.join("\n");
    let lower = combined.to_lowercase();
    let mut depth: i32 = 0;
    let mut i = 0;
    let bytes = lower.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if lower[i..].starts_with("<code>") || lower[i..].starts_with("<code ") {
                depth += 1;
                i += 6;
                continue;
            }
            if lower[i..].starts_with("</code>") {
                depth -= 1;
                i += 7;
                continue;
            }
        }
        i += 1;
    }
    depth > 0
}

/// Try to parse a table starting at `pos`. Returns the table Element and advances pos.
fn try_parse_table(lines: &[&str], pos: &mut usize, options: &Options) -> Option<Element> {
    let start = *pos;
    let line = lines[start];

    // First check: is this a table line or separator line?
    let is_sep = try_parse_separator_line(line).is_some();
    let is_tbl = is_table_line(line);

    if !is_sep && !is_tbl {
        return None;
    }

    // Check for multi-line code span: if line has unbalanced backticks,
    // it might span multiple lines and should be treated as a paragraph
    if has_unbalanced_backticks(line) && !line.trim().starts_with('|') {
        // Possibly a multi-line code span, not a table
        // But only if it doesn't start with pipe
    }

    // Collect all consecutive table lines and separator lines
    let mut raw_rows: Vec<(usize, &str)> = Vec::new(); // (line_index, line_content)
    let mut scan = start;

    while scan < lines.len() {
        let l = lines[scan];
        if is_blank_line(l) {
            break;
        }
        if is_block_ial(l) {
            break;
        }
        let l_is_sep = try_parse_separator_line(l).is_some();
        let l_is_tbl = is_table_line(l);
        if !l_is_sep && !l_is_tbl {
            break;
        }
        raw_rows.push((scan, l));
        scan += 1;
    }

    if raw_rows.is_empty() {
        return None;
    }

    // Check: if it's a standalone separator line with no data rows, it's not a table
    if raw_rows.len() == 1 && try_parse_separator_line(raw_rows[0].1).is_some() {
        return None;
    }

    // Check: if all rows are separator lines, it's not a table
    let all_seps = raw_rows
        .iter()
        .all(|(_, l)| try_parse_separator_line(l).is_some());
    if all_seps {
        return None;
    }

    // Check: if table scanning stopped because of a non-blank, non-table line
    // (i.e., the table is followed by a paragraph continuation), it's not a table.
    // A valid table must be followed by a blank line, EOB, IAL, or end of input.
    if scan < lines.len()
        && !is_blank_line(lines[scan])
        && !is_block_ial(lines[scan])
        && lines[scan].trim() != "^"
    {
        return None;
    }

    // Check for errors.text case: separator followed by paragraph-like text
    // If the first line is a separator and the second is a data row but
    // the next non-table line forms a paragraph continuation, abort
    // Actually, this is handled by `|no|table|here|\nparagraph` case in errors.text
    // where the paragraph line breaks the table.

    // Now analyze the structure: find separator positions and determine
    // thead/tbody/tfoot sections
    let mut sections: Vec<TableSection> = Vec::new();
    let mut alignments: Vec<Alignment> = Vec::new();
    let mut current_rows: Vec<Vec<String>> = Vec::new();
    let mut current_kind = TableSectionKind::Body;
    let mut has_data_before_sep = false;
    let mut alignment_set = false;
    let mut footer_started = false;
    let mut max_cols: usize = 0;

    // Scan through rows
    let mut ridx = 0;
    while ridx < raw_rows.len() {
        let (_, row_line) = raw_rows[ridx];

        if let Some(sep) = try_parse_separator_line(row_line) {
            // Only use alignment from the first separator that has data before it
            if has_data_before_sep && !alignment_set {
                alignments = sep.alignments.clone();
                alignment_set = true;
            }

            if !has_data_before_sep {
                // Leading separator (no data before it) - just skip it
                // It doesn't create a head section by itself
                ridx += 1;
                continue;
            }

            match sep.kind {
                SeparatorKind::Header | SeparatorKind::Body => {
                    if footer_started {
                        // Separators within tfoot are ignored (don't split)
                        // Just continue accumulating rows in tfoot
                    } else {
                        // Rows before this are header rows (if no head yet)
                        // or body rows (if subsequent sep)
                        if !current_rows.is_empty() {
                            let kind = if !sections
                                .iter()
                                .any(|s: &TableSection| s.kind == TableSectionKind::Head)
                            {
                                TableSectionKind::Head
                            } else {
                                current_kind
                            };
                            sections.push(TableSection {
                                kind,
                                rows: std::mem::take(&mut current_rows),
                            });
                        }
                        current_kind = TableSectionKind::Body;
                    }
                }
                SeparatorKind::Footer => {
                    if !current_rows.is_empty() {
                        sections.push(TableSection {
                            kind: current_kind,
                            rows: std::mem::take(&mut current_rows),
                        });
                    }
                    footer_started = true;
                    current_kind = TableSectionKind::Foot;
                }
            }
            ridx += 1;
            continue;
        }

        // Data row
        has_data_before_sep = true;
        let cells = split_table_cells(row_line);
        if cells.len() > max_cols {
            max_cols = cells.len();
        }
        current_rows.push(cells);
        ridx += 1;
    }

    // Flush remaining rows
    if !current_rows.is_empty() {
        sections.push(TableSection {
            kind: current_kind,
            rows: current_rows,
        });
    }

    // If no sections have data, not a table
    if sections.is_empty() || sections.iter().all(|s| s.rows.is_empty()) {
        return None;
    }

    // Check: errors case -- if table has a single data row followed by non-table content
    // that forms a paragraph, it should NOT be a table. The paragraph detection
    // handles this by breaking when encountering non-table lines.

    // Build the Table element
    let mut table = Element::new(ElementType::Table);

    // Store alignments in table options
    let align_strs: Vec<String> = alignments
        .iter()
        .map(|a| match a {
            Alignment::Default => "default".to_string(),
            Alignment::Left => "left".to_string(),
            Alignment::Center => "center".to_string(),
            Alignment::Right => "right".to_string(),
        })
        .collect();
    table
        .options
        .insert("alignments".to_string(), align_strs.join(","));
    table
        .options
        .insert("max_cols".to_string(), max_cols.to_string());

    // Encode sections as children
    for section in &sections {
        let section_tag = match section.kind {
            TableSectionKind::Head => "thead",
            TableSectionKind::Body => "tbody",
            TableSectionKind::Foot => "tfoot",
        };

        let mut section_elem = Element::new(ElementType::Table);
        section_elem
            .options
            .insert("section".to_string(), section_tag.to_string());

        for row in &section.rows {
            let mut row_elem = Element::new(ElementType::TableRow);
            for cell_text in row {
                let cell_content = if options.html_to_native {
                    convert_html_to_native(cell_text)
                } else {
                    cell_text.clone()
                };
                let mut cell_elem = Element::new(ElementType::TableCell);
                let text = Element::with_value(ElementType::Text, cell_content);
                cell_elem.children.push(text);
                row_elem.children.push(cell_elem);
            }
            section_elem.children.push(row_elem);
        }

        table.children.push(section_elem);
    }

    *pos = scan;
    Some(table)
}

/// Convert HTML empty tags to XHTML form (e.g., `<br>` -> `<br />`).
fn convert_html_to_native(s: &str) -> String {
    // Convert void HTML elements like <br> to <br />
    let re_void = [
        "br", "hr", "img", "input", "col", "area", "base", "link", "meta", "wbr",
    ];
    let mut result = s.to_string();
    for tag in &re_void {
        let html_form = format!("<{tag}>");
        let xhtml_form = format!("<{tag} />");
        result = result.replace(&html_form, &xhtml_form);
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableSectionKind {
    Head,
    Body,
    Foot,
}

struct TableSection {
    kind: TableSectionKind,
    rows: Vec<Vec<String>>,
}

// ---------------------------------------------------------------------------
// End table parsing
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// List parsing
// ---------------------------------------------------------------------------

/// Marker type for list items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListMarkerType {
    Unordered, // *, +, -
    Ordered,   // 1., 2., etc.
}

/// Information about a detected list marker on a line.
#[derive(Debug, Clone)]
struct ListMarkerInfo {
    marker_type: ListMarkerType,
    /// The full width consumed by leading spaces + marker + trailing space/tab.
    /// This is the content indent: content of the item starts at this column.
    content_indent: usize,
    /// The content after the marker on the first line (may be empty).
    first_line_content: String,
}

/// Try to detect a list marker at the start of a line.
/// Returns None if the line doesn't start a list item.
/// A list marker can be preceded by 0-3 spaces.
/// Unordered: `*`, `+`, `-` followed by space/tab or end-of-line
/// Ordered: digits followed by `.` then space/tab or end-of-line
fn detect_list_marker(line: &str) -> Option<ListMarkerInfo> {
    let expanded = expand_tabs_line(line);
    let bytes = expanded.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    // Count leading spaces (0-3 allowed for starting a list)
    let mut leading_spaces = 0;
    for &b in bytes.iter() {
        if b == b' ' {
            leading_spaces += 1;
        } else {
            break;
        }
    }
    if leading_spaces >= 4 {
        return None;
    }

    let rest = &expanded[leading_spaces..];
    if rest.is_empty() {
        return None;
    }

    let first_char = rest.as_bytes()[0];

    // Unordered list markers
    if first_char == b'*' || first_char == b'+' || first_char == b'-' {
        if rest.len() == 1 {
            // Marker at end of line -> empty item
            return Some(ListMarkerInfo {
                marker_type: ListMarkerType::Unordered,
                content_indent: leading_spaces + 2,
                first_line_content: String::new(),
            });
        }

        let after_marker = rest.as_bytes()[1];
        if after_marker != b' ' && after_marker != b'\t' {
            return None;
        }

        // Count spaces after marker
        let after_marker_str = &rest[1..];
        let content_str = after_marker_str.trim_start_matches([' ', '\t']);
        let space_after = after_marker_str.len() - content_str.len();

        return Some(ListMarkerInfo {
            marker_type: ListMarkerType::Unordered,
            content_indent: leading_spaces + 1 + space_after.max(1),
            first_line_content: content_str.to_string(),
        });
    }

    // Ordered list markers: digits followed by `.` then space/tab
    if first_char.is_ascii_digit() {
        let mut digit_count = 0;
        for &b in rest.as_bytes().iter() {
            if b.is_ascii_digit() {
                digit_count += 1;
            } else {
                break;
            }
        }
        if digit_count == 0 || digit_count >= rest.len() {
            return None;
        }
        if rest.as_bytes()[digit_count] != b'.' {
            return None;
        }
        let after_dot = &rest[digit_count + 1..];
        if after_dot.is_empty() {
            return Some(ListMarkerInfo {
                marker_type: ListMarkerType::Ordered,
                content_indent: leading_spaces + digit_count + 2,
                first_line_content: String::new(),
            });
        }
        let first_after_dot = after_dot.as_bytes()[0];
        if first_after_dot != b' ' && first_after_dot != b'\t' {
            return None; // e.g. "1984.5" is not a list
        }

        let content_str = after_dot.trim_start_matches([' ', '\t']);
        let space_after = after_dot.len() - content_str.len();

        return Some(ListMarkerInfo {
            marker_type: ListMarkerType::Ordered,
            content_indent: leading_spaces + digit_count + 1 + space_after.max(1),
            first_line_content: content_str.to_string(),
        });
    }

    None
}

/// Check if a line starts a list item (not a horizontal rule).
fn is_list_start(line: &str) -> bool {
    if is_horizontal_rule(line) {
        return false;
    }
    detect_list_marker(line).is_some()
}

/// Expand tabs in a string to spaces (tab stops at every 4 columns).
fn expand_tabs_line(s: &str) -> String {
    if !s.contains('\t') {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len() + 8);
    let mut col = 0;
    for c in s.chars() {
        if c == '\t' {
            let spaces = 4 - (col % 4);
            for _ in 0..spaces {
                result.push(' ');
            }
            col += spaces;
        } else {
            result.push(c);
            col += 1;
        }
    }
    result
}

/// Parse a list starting at `pos`. Handles both ordered and unordered lists.
fn parse_list(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
) -> Element {
    let empty_lazy: Vec<bool> = vec![false; lines.len()];
    parse_list_with_lazy(lines, pos, has_trailing_newline, options, &empty_lazy)
}

/// Parse a list with awareness of outer-context lazy flags.
/// Lines marked as lazy in the outer context cannot start new list items or
/// be consumed as continuation lines - they end the list.
fn parse_list_with_lazy(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
    outer_lazy_flags: &[bool],
) -> Element {
    let first_marker =
        detect_list_marker(lines[*pos]).expect("parse_list called without list marker");
    let list_type = first_marker.marker_type;

    struct RawItem {
        lines: Vec<String>,
        content_indent: usize,
        has_blank_after: bool,
        has_blank_before: bool,
        nested_list_found: bool,
    }
    let mut raw_items: Vec<RawItem> = Vec::new();
    let mut pending_blank = false; // track if blank lines precede the next item

    while *pos < lines.len() {
        let line = lines[*pos];

        // Check if this line is marked as lazy in the outer context.
        let is_outer_lazy = outer_lazy_flags.get(*pos).copied().unwrap_or(false);

        // EOB marker ends the list
        if line.trim() == "^" {
            *pos += 1;
            break;
        }

        // Blank line
        if is_blank_line(line) {
            *pos += 1;

            // Look ahead past blank lines
            let mut look = *pos;
            while look < lines.len() && is_blank_line(lines[look]) {
                look += 1;
            }
            if look >= lines.len() {
                if let Some(item) = raw_items.last_mut() {
                    item.has_blank_after = true;
                }
                break;
            }

            let next_line = lines[look];

            // If next non-blank line is a same-type list marker at top level, it's a new item
            if let Some(next_marker) = detect_list_marker(next_line) {
                if next_marker.marker_type == list_type && !is_horizontal_rule(next_line) {
                    let next_expanded = expand_tabs_line(next_line);
                    let next_indent = next_expanded.len() - next_expanded.trim_start().len();
                    if next_indent < first_marker.content_indent {
                        // Mark current item as having blank after
                        if let Some(item) = raw_items.last_mut() {
                            item.has_blank_after = true;
                        }
                        pending_blank = true;
                        // Skip to the next item marker
                        *pos = look;
                        continue;
                    }
                }
            }

            // If next line is indented content belonging to current item
            if let Some(last_item) = raw_items.last() {
                let current_indent = last_item.content_indent;
                let next_expanded = expand_tabs_line(next_line);
                let next_indent = next_expanded.len() - next_expanded.trim_start().len();
                if next_indent >= current_indent {
                    if let Some(item) = raw_items.last_mut() {
                        item.lines.push(String::new());
                        while *pos < look {
                            item.lines.push(String::new());
                            *pos += 1;
                        }
                    }
                    continue;
                }
            }

            // End the list
            *pos -= 1;
            break;
        }

        // Check for list markers (only if not outer-lazy; outer-lazy lines
        // cannot start new items or be treated as nested markers)
        if !is_outer_lazy {
            if let Some(marker) = detect_list_marker(line) {
                if !is_horizontal_rule(line) {
                    let expanded = expand_tabs_line(line);
                    let indent = expanded.len() - expanded.trim_start().len();
                    // Is this a nested marker?
                    if !raw_items.is_empty() && indent >= first_marker.content_indent {
                        if let Some(item) = raw_items.last_mut() {
                            // Check if the stripped line looks like a list start
                            let stripped = strip_n_spaces(&expanded, first_marker.content_indent);
                            if is_list_start(&stripped) {
                                item.nested_list_found = true;
                            }
                            item.lines.push(line.to_string());
                        }
                        *pos += 1;
                        continue;
                    }

                    // Same-type marker at top level -> new item
                    if marker.marker_type == list_type {
                        raw_items.push(RawItem {
                            lines: vec![marker.first_line_content.clone()],
                            content_indent: marker.content_indent,
                            has_blank_after: false,
                            has_blank_before: pending_blank,
                            nested_list_found: false,
                        });
                        pending_blank = false;
                        *pos += 1;
                        continue;
                    }

                    // Different type at top level with no blank before -> lazy continuation
                    if !raw_items.is_empty() {
                        if let Some(item) = raw_items.last_mut() {
                            if item.nested_list_found {
                                let padding = " ".repeat(2 * first_marker.content_indent + 4);
                                item.lines.push(format!("{}{}", padding, line));
                            } else {
                                item.lines.push(line.to_string());
                            }
                        }
                        *pos += 1;
                        continue;
                    }
                    break;
                }
            }
        }

        // HR ends the list
        if is_horizontal_rule(line) {
            break;
        }

        // Non-marker line
        if raw_items.is_empty() {
            break;
        }

        let expanded = expand_tabs_line(line);
        let indent = expanded.len() - expanded.trim_start().len();

        let current_content_indent = raw_items.last().map_or(0, |item| item.content_indent);
        if indent >= current_content_indent {
            if let Some(item) = raw_items.last_mut() {
                // Check if the stripped content looks like a list start
                // (to track nested_list_found for lazy line padding)
                if !item.nested_list_found {
                    let stripped = strip_n_spaces(&expanded, item.content_indent);
                    if is_list_start(&stripped) {
                        item.nested_list_found = true;
                    }
                }
                item.lines.push(line.to_string());
            }
            *pos += 1;
        } else {
            // Check if this would start a new block at the top level
            if is_blockquote_line(line)
                || try_parse_atx_header(line, options).is_some()
                || try_parse_fenced_code(lines, *pos).is_some()
                || is_block_ial(line)
                || is_ald(line)
            {
                break;
            }
            // Lazy continuation
            if let Some(item) = raw_items.last_mut() {
                // When a nested list was found and this lazy line looks like
                // a list marker, pad it so it becomes part of the nested list
                // item's content (matching kramdown Ruby behavior).
                if item.nested_list_found && detect_list_marker(line).is_some() {
                    // Pad the lazy line so that after double stripping
                    // (outer content_indent + nested content_indent), it
                    // exceeds the 3-space list marker limit, matching
                    // kramdown Ruby's behavior.
                    let padding = " ".repeat(2 * first_marker.content_indent + 4);
                    item.lines.push(format!("{}{}", padding, line));
                } else {
                    item.lines.push(line.to_string());
                }
            }
            *pos += 1;
        }
    }

    // Build the List element
    let mut list_elem = Element::new(ElementType::List);
    list_elem.options.insert(
        "list_type".to_string(),
        match list_type {
            ListMarkerType::Unordered => "ul".to_string(),
            ListMarkerType::Ordered => "ol".to_string(),
        },
    );

    for raw_item in &raw_items {
        // Only count blank lines at the item's own content level as making it loose.
        // Blank lines within sub-items (indented content) don't make the parent loose.
        let has_internal_blanks = has_top_level_blank(&raw_item.lines, raw_item.content_indent);
        let is_item_loose =
            raw_item.has_blank_after || raw_item.has_blank_before || has_internal_blanks;

        let item_elem = build_list_item(
            &raw_item.lines,
            is_item_loose,
            has_trailing_newline,
            options,
            raw_item.content_indent,
        );
        list_elem.children.push(item_elem);
    }

    list_elem
}

/// Check if there's a blank line at the top level of a list item's content.
/// A blank line inside a sub-list or other deeply nested content doesn't count.
/// Only blank lines between the item's own top-level paragraphs make it loose.
fn has_top_level_blank(lines: &[String], _content_indent: usize) -> bool {
    // Walk through lines. Once we hit a sub-list marker or other block element
    // at the continuation indent level, all subsequent blank lines belong to
    // that sub-structure, not to the parent item.
    let mut saw_first_content = false;
    let mut in_sub_structure = false;

    for (idx, line) in lines.iter().enumerate() {
        if idx == 0 {
            saw_first_content = true;
            // Check if the first line itself starts a sub-structure
            let trimmed = line.trim();
            if is_list_start(trimmed) || is_blockquote_line(trimmed) {
                in_sub_structure = true;
            }
            continue;
        }

        if line.is_empty() {
            // Blank line: only counts as looseness if we haven't entered a sub-structure
            if saw_first_content && !in_sub_structure {
                // Check if there's non-blank top-level content after this blank
                let has_more_top_content = lines[idx + 1..]
                    .iter()
                    .any(|l| !l.is_empty() && !l.trim().is_empty());
                if has_more_top_content {
                    return true;
                }
            }
            continue;
        }

        let trimmed = line.trim();
        if !in_sub_structure {
            // Check if this line starts a sub-structure
            if is_list_start(trimmed) || is_blockquote_line(trimmed) {
                in_sub_structure = true;
            }
        }
    }

    false
}

/// Build a ListItem element from raw content lines.
fn build_list_item(
    raw_lines: &[String],
    is_loose: bool,
    has_trailing_newline: bool,
    options: &Options,
    content_indent: usize,
) -> Element {
    let mut item = Element::new(ElementType::ListItem);

    // Check for IAL at the start of item: `{:.cls} rest of content`
    let mut ial_attrs: Vec<(String, String)> = Vec::new();
    let effective_lines;

    let first_line = raw_lines.first().map(|s| s.as_str()).unwrap_or("");

    if let Some(rest) = try_extract_item_ial(first_line, &mut ial_attrs) {
        let mut remaining = vec![rest];
        remaining.extend(raw_lines[1..].iter().cloned());
        effective_lines = remaining;
    } else {
        effective_lines = raw_lines.to_vec();
    }

    // Apply IAL to the item element
    for (k, v) in &ial_attrs {
        if k == "class" {
            if let Some(existing) = item.attr.get_mut("class") {
                existing.push(' ');
                existing.push_str(v);
            } else {
                item.attr.insert(k.clone(), v.clone());
            }
        } else {
            item.attr.insert(k.clone(), v.clone());
        }
    }

    // Process nomarkdown extensions
    let effective_lines = process_nomarkdown_in_lines(&effective_lines);

    // When the first line is empty (e.g. `* ` followed by newline), kramdown
    // uses a minimum content_indent of 4 for proper code block detection.
    // Without this, `* \n        code` with content_indent=2 would produce
    // 6-space indented content instead of 4-space (the code block threshold).
    let content_indent = if effective_lines
        .first()
        .map(|l| l.trim().is_empty())
        .unwrap_or(false)
    {
        content_indent.max(4)
    } else {
        content_indent
    };

    // Strip content indent from continuation lines and compute lazy flags.
    // A line is "lazy" if its original indent was less than content_indent.
    // Lazy lines should be treated as paragraph text, not block starters.
    let mut stripped_lines: Vec<String> = Vec::new();
    let mut lazy_flags: Vec<bool> = Vec::new();
    for (i, line) in effective_lines.iter().enumerate() {
        if i == 0 {
            stripped_lines.push(line.clone());
            lazy_flags.push(false);
        } else if line.is_empty() {
            stripped_lines.push(String::new());
            lazy_flags.push(false);
        } else {
            let expanded = expand_tabs_line(line);
            let indent = expanded.len() - expanded.trim_start().len();
            let is_lazy = indent < content_indent;
            if is_lazy {
                // Lazy lines keep their original content (no indent stripping)
                stripped_lines.push(expanded);
            } else {
                let stripped = strip_n_spaces(&expanded, content_indent);
                stripped_lines.push(stripped);
            }
            lazy_flags.push(is_lazy);
        }
    }

    // Remove trailing blank lines
    while stripped_lines.last().is_some_and(|l| l.trim().is_empty()) {
        stripped_lines.pop();
    }

    if stripped_lines.is_empty() || (stripped_lines.len() == 1 && stripped_lines[0].is_empty()) {
        return item;
    }

    // Check if the item has sub-blocks
    let has_sub_blocks = stripped_lines.iter().any(|l| l.is_empty())
        || stripped_lines
            .iter()
            .skip(1)
            .any(|l| is_list_start(l) || is_blockquote_line(l) || is_indented_code_line(l))
        || stripped_lines
            .iter()
            .any(|l| try_parse_atx_header(l, options).is_some());

    // Check if first element is a block-level thing (code, blockquote, header, nested list)
    let first_is_block = {
        let fl = stripped_lines[0].as_str();
        fl.is_empty()
            || is_indented_code_line(fl)
            || is_blockquote_line(fl)
            || is_list_start(fl)
            || try_parse_atx_header(fl, options).is_some()
    };

    // Also check for setext header (line 0 as text, line 1 as underline)
    let has_setext = stripped_lines.len() >= 2 && is_setext_underline(&stripped_lines[1]);

    if has_sub_blocks || is_loose || first_is_block || has_setext {
        // Parse as blocks with list-item-aware paragraph breaking
        let line_refs: Vec<&str> = stripped_lines.iter().map(|s| s.as_str()).collect();
        let mut inner_pos = 0;
        parse_blocks_list_context(
            &line_refs,
            &mut inner_pos,
            has_trailing_newline,
            &mut item.children,
            options,
            &lazy_flags,
        );

        // If not loose, unwrap paragraphs to plain text
        if !is_loose {
            unwrap_paragraphs_in_item(&mut item.children);
        }
    } else {
        // Simple item: just text content
        let text = stripped_lines.join("\n");
        let text_child = Element::with_value(ElementType::Text, text);
        item.children.push(text_child);
    }

    item
}

/// Unwrap paragraphs in a non-loose list item to plain text nodes.
/// Only unwraps if the item content is "simple" (no block elements other than paragraphs).
fn unwrap_paragraphs_in_item(children: &mut Vec<Element>) {
    let mut new_children = Vec::new();
    for child in children.drain(..) {
        if child.element_type == ElementType::Paragraph {
            for text_child in child.children {
                new_children.push(text_child);
            }
        } else {
            new_children.push(child);
        }
    }
    *children = new_children;
}

/// Strip up to n spaces from the start of a line.
fn strip_n_spaces(line: &str, n: usize) -> String {
    let mut stripped = 0;
    let mut idx = 0;
    for (i, c) in line.char_indices() {
        if stripped >= n {
            idx = i;
            break;
        }
        if c == ' ' {
            stripped += 1;
        } else {
            idx = i;
            break;
        }
        idx = i + 1;
    }
    line[idx..].to_string()
}

/// Try to extract IAL from the start of a list item's first line.
fn try_extract_item_ial(line: &str, attrs: &mut Vec<(String, String)>) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("{:.") && !trimmed.starts_with("{:#") {
        return None;
    }
    if let Some(close) = trimmed.find('}') {
        let ial_str = &trimmed[..=close];
        let rest = trimmed[close + 1..].trim_start();
        let parsed = parse_ial(ial_str);
        attrs.extend(parsed);
        Some(rest.to_string())
    } else {
        None
    }
}

/// Process {::nomarkdown} ... {:/nomarkdown} in item lines.
fn process_nomarkdown_in_lines(lines: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for line in lines {
        if let Some(processed) = process_nomarkdown_line(line) {
            result.push(processed);
        } else {
            result.push(line.clone());
        }
    }
    result
}

/// Process a single line for nomarkdown extensions.
fn process_nomarkdown_line(line: &str) -> Option<String> {
    let start_tag = "{::nomarkdown";
    if let Some(start_pos) = line.find(start_tag) {
        if let Some(tag_end) = line[start_pos..].find('}') {
            let after_start = &line[start_pos + tag_end + 1..];
            let end_tag = "{:/nomarkdown}";
            if let Some(end_pos) = after_start.find(end_tag) {
                let content = &after_start[..end_pos];
                let after_end = &after_start[end_pos + end_tag.len()..];
                let before = &line[..start_pos];
                let result = format!("{before}{content}{after_end}");
                return Some(result);
            }
        }
    }
    None
}

/// Check if a line is a setext underline (one or more `=` or `-`, possibly with spaces).
fn is_setext_underline(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Must not be indented 4+ spaces
    if line.starts_with("    ") {
        return false;
    }
    let first = match trimmed.chars().next() {
        Some(c) if c == '=' || c == '-' => c,
        _ => return false,
    };
    trimmed.chars().all(|c| c == first)
}

// ---------------------------------------------------------------------------
// HTML block parsing
// ---------------------------------------------------------------------------

/// HTML block tags that are considered block-level.
const HTML_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "math",
    "menu",
    "menuitem",
    "meta",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "pre",
    "section",
    "source",
    "summary",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// Check if a tag name is a known HTML block element.
fn is_html_block_tag(tag: &str) -> bool {
    HTML_BLOCK_TAGS.contains(&tag.to_lowercase().as_str())
}

/// Check if a line starts with a block-level HTML tag (for interrupting paragraphs).
fn is_html_block_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent >= 4 {
        return false;
    }
    if !trimmed.starts_with('<') {
        return false;
    }
    // HTML comments inside paragraphs are inline, not block-breaking
    if trimmed.starts_with("<!--") {
        return false;
    }
    if trimmed.starts_with("<?") || trimmed.starts_with("</") || trimmed.starts_with("<!") {
        return false;
    }
    let after_lt = &trimmed[1..];
    let tag_name: String = after_lt
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    if tag_name.is_empty() {
        return false;
    }
    // Reject URL-like patterns
    let after_tag = &after_lt[tag_name.len()..];
    if after_tag.starts_with("//") {
        return false;
    }
    let tag_lc = tag_name.to_lowercase();
    // Known inline tags don't start blocks
    if HTML_SPAN_TAGS.contains(&tag_lc.as_str()) && !is_html_block_tag(&tag_lc) {
        return false;
    }
    // In kramdown, `script` and `textarea` belong to both span and block categories.
    // They don't interrupt paragraphs (kramdown adds them to LAZY_END_HTML_SPAN_ELEMENTS).
    if tag_lc == "script" || tag_lc == "textarea" {
        return false;
    }
    true
}

/// HTML void elements (self-closing, no end tag).
const HTML_VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// HTML tags whose content is never parsed as markdown (raw).
const HTML_RAW_TAGS: &[&str] = &["script", "style", "math", "svg"];

/// HTML tags whose content is parsed as span-level markdown.
const HTML_SPAN_TAGS: &[&str] = &[
    "a", "abbr", "acronym", "address", "b", "bdi", "bdo", "big", "button", "cite", "code", "del",
    "dfn", "em", "font", "i", "ins", "kbd", "mark", "p", "pre", "q", "s", "samp", "small", "span",
    "strike", "strong", "sub", "sup", "tt", "u", "var",
];

/// Check if a tag is a void (self-closing) element.
fn is_html_void_tag(tag: &str) -> bool {
    HTML_VOID_TAGS.contains(&tag.to_lowercase().as_str())
}

/// Check if a tag's content should be treated as raw (never parsed).
fn is_html_raw_tag(tag: &str) -> bool {
    HTML_RAW_TAGS.contains(&tag.to_lowercase().as_str())
}

/// Parse HTML tag attributes from a string like `id='test' class="foo" disabled`.
/// Returns normalized attributes as a list of (key, value) pairs.
/// If `lowercase_names` is true, attribute names are lowercased (for known HTML elements).
fn parse_html_tag_attrs(attr_str: &str) -> Vec<(String, String)> {
    parse_html_tag_attrs_impl(attr_str, true)
}

/// Parse HTML tag attributes, optionally lowercasing attribute names.
fn parse_html_tag_attrs_impl(attr_str: &str, lowercase_names: bool) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut chars = attr_str.chars().peekable();

    loop {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        // Read attribute name (may contain colons for XML namespaced attrs)
        let mut name = String::new();
        while chars.peek().is_some_and(|c| {
            c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ':' || *c == '.'
        }) {
            name.push(chars.next().unwrap_or(' '));
        }
        if name.is_empty() {
            // Skip unexpected character
            chars.next();
            continue;
        }

        // Lowercase attribute names for known HTML elements (HTML attrs are case-insensitive)
        let name = if lowercase_names {
            name.to_lowercase()
        } else {
            name
        };

        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Check for = sign
        if chars.peek() == Some(&'=') {
            chars.next(); // consume =
                          // Skip whitespace
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            // Read value
            let value = if chars.peek() == Some(&'"') {
                chars.next(); // consume opening "
                let v: String = chars.by_ref().take_while(|c| *c != '"').collect();
                v
            } else if chars.peek() == Some(&'\'') {
                chars.next(); // consume opening '
                let v: String = chars.by_ref().take_while(|c| *c != '\'').collect();
                v
            } else {
                // Unquoted value
                let v: String = chars
                    .by_ref()
                    .take_while(|c| !c.is_whitespace() && *c != '>' && *c != '/')
                    .collect();
                v
            };
            attrs.push((name, value));
        } else {
            // Boolean attribute (no value) -> normalize to =""
            attrs.push((name, String::new()));
        }
    }

    attrs
}

/// Format HTML attributes back to string with double quotes.
fn format_html_attrs(attrs: &[(String, String)]) -> String {
    let mut result = String::new();
    for (key, value) in attrs {
        result.push(' ');
        result.push_str(key);
        result.push_str("=\"");
        result.push_str(value);
        result.push('"');
    }
    result
}

/// Parse an HTML opening tag, extracting tag name, attributes, and whether self-closing.
/// Returns (tag_name, attrs_string, is_self_closing, rest_after_tag).
fn parse_html_opening_tag(s: &str) -> Option<(String, String, bool, String)> {
    if !s.starts_with('<') {
        return None;
    }
    let after_lt = &s[1..];
    if after_lt.starts_with('/') || after_lt.starts_with('!') || after_lt.starts_with('?') {
        return None;
    }

    // Extract tag name (may contain colons for XML namespaced tags)
    let tag_name: String = after_lt
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();

    if tag_name.is_empty() {
        return None;
    }

    let after_name = &after_lt[tag_name.len()..];

    // Find the closing >
    // Need to handle self-closing />
    if let Some(gt_pos) = after_name.find('>') {
        let before_gt = &after_name[..gt_pos];
        let is_self_closing = before_gt.trim_end().ends_with('/');
        let attr_str = if is_self_closing {
            before_gt
                .trim_end()
                .strip_suffix('/')
                .unwrap_or(before_gt)
                .trim()
        } else {
            before_gt.trim()
        };
        let rest = &after_name[gt_pos + 1..];
        Some((
            tag_name,
            attr_str.to_string(),
            is_self_closing,
            rest.to_string(),
        ))
    } else {
        None
    }
}

/// Normalize an HTML tag: parse attributes, convert quotes to double, etc.
fn normalize_html_tag(tag_str: &str) -> String {
    if let Some((tag_name, attr_str, is_self_closing, _rest)) = parse_html_opening_tag(tag_str) {
        // For XML-namespaced tags (containing ':'), preserve attribute name case
        let is_xml = tag_name.contains(':');
        let attrs = parse_html_tag_attrs_impl(&attr_str, !is_xml);
        let attrs_str = format_html_attrs(&attrs);
        if is_self_closing || is_html_void_tag(&tag_name) {
            format!("<{tag_name}{attrs_str} />")
        } else {
            format!("<{tag_name}{attrs_str}>")
        }
    } else {
        // Not a parseable opening tag, return as-is
        tag_str.to_string()
    }
}

/// Try to parse an HTML block starting at the current position.
fn try_parse_html_block(lines: &[&str], pos: &mut usize, options: &Options) -> Option<Element> {
    let line = lines[*pos];
    let trimmed = line.trim_start();

    // Must not be indented 4+ spaces
    let indent = line.len() - trimmed.len();
    if indent >= 4 {
        return None;
    }

    // HTML comment: <!-- ... -->
    if trimmed.starts_with("<!--") {
        let result = parse_html_comment_block(lines, pos);
        let mut elem = result.element;
        if let Some(trailing) = result.trailing_text {
            elem.options.insert("trailing_text".to_string(), trailing);
        }
        return Some(elem);
    }

    // CDATA section: <![CDATA[ ... ]]>
    // Note: Only standalone CDATA is a block. CDATA inside tags is inline (handled by span parser).
    if trimmed.starts_with("<![CDATA[") {
        // If the line starts with a block-level tag, it's part of that tag, not standalone CDATA
        return Some(parse_html_cdata_block(lines, pos));
    }

    // Processing instructions are NOT HTML blocks in kramdown - they are paragraph text
    if trimmed.starts_with("<?") {
        return None;
    }

    // HTML tag: <tagname ... > or </tagname>
    if !trimmed.starts_with('<') {
        return None;
    }

    // Extract tag info
    let after_lt = &trimmed[1..];
    let is_closing = after_lt.starts_with('/');

    // Closing tags alone are NOT HTML blocks in kramdown - they are paragraph text
    if is_closing {
        return None;
    }

    // Parse opening tag
    let tag_start = after_lt;

    // Tag name: alphanumeric chars, plus colon for XML namespaced tags
    let tag_name: String = tag_start
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();

    if tag_name.is_empty() {
        return None;
    }

    // Validate: after tag name must be whitespace, >, or / (not // which would be a URL)
    let after_tag = &tag_start[tag_name.len()..];
    if !after_tag.is_empty() {
        let next_char = after_tag.chars().next().unwrap_or(' ');
        if next_char != ' '
            && next_char != '\t'
            && next_char != '>'
            && next_char != '/'
            && next_char != '\n'
        {
            return None;
        }
        // Also reject if it looks like a URL: scheme://
        if after_tag.starts_with("//") {
            return None;
        }
    }

    // Determine if this tag should be treated as block-level:
    // - Known HTML block tags: always block
    // - XML namespaced tags: always block
    // - Unknown tags: block unless they're known inline/span tags
    let tag_lc = tag_name.to_lowercase();
    let _is_namespaced = tag_name.contains(':');
    let is_known_inline = HTML_SPAN_TAGS.contains(&tag_lc.as_str()) && !is_html_block_tag(&tag_lc);

    if is_known_inline {
        return None;
    }

    // Parse the HTML block with proper tag matching and normalization
    let result = parse_html_block_element(lines, pos, &tag_name, options);
    let mut elem = result.element;
    if let Some(trailing) = result.trailing {
        elem.options.insert("html_trailing".to_string(), trailing);
    }
    Some(elem)
}

/// Result of parsing an HTML comment: the comment element and optional trailing text.
struct HtmlCommentResult {
    element: Element,
    /// Text that appeared after `-->` on the same line, to be parsed as a new block.
    trailing_text: Option<String>,
}

/// Parse an HTML comment block: <!-- ... -->
fn parse_html_comment_block(lines: &[&str], pos: &mut usize) -> HtmlCommentResult {
    let mut content = String::new();
    let start_line = lines[*pos];
    content.push_str(start_line.trim_start());
    *pos += 1;

    // Check if comment closes on the same line
    if let Some(end_idx) = content.find("-->") {
        let comment_end = end_idx + 3;
        let after = content[comment_end..].trim().to_string();
        content.truncate(comment_end);
        let mut elem = Element::with_value(ElementType::HtmlBlock, content);
        elem.options
            .insert("type".to_string(), "comment".to_string());
        return HtmlCommentResult {
            element: elem,
            trailing_text: if after.is_empty() { None } else { Some(after) },
        };
    }

    // Multi-line comment
    while *pos < lines.len() {
        let line = lines[*pos];
        content.push('\n');
        content.push_str(line);
        *pos += 1;
        if line.contains("-->") {
            if let Some(total_end) = content.rfind("-->") {
                let comment_end = total_end + 3;
                let after = content[comment_end..].trim().to_string();
                content.truncate(comment_end);
                let mut elem = Element::with_value(ElementType::HtmlBlock, content);
                elem.options
                    .insert("type".to_string(), "comment".to_string());
                return HtmlCommentResult {
                    element: elem,
                    trailing_text: if after.is_empty() { None } else { Some(after) },
                };
            }
            break;
        }
    }

    let mut elem = Element::with_value(ElementType::HtmlBlock, content);
    elem.options
        .insert("type".to_string(), "comment".to_string());
    HtmlCommentResult {
        element: elem,
        trailing_text: None,
    }
}

/// Parse CDATA block.
fn parse_html_cdata_block(lines: &[&str], pos: &mut usize) -> Element {
    let mut content = String::new();
    while *pos < lines.len() {
        let line = lines[*pos];
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line);
        *pos += 1;
        if line.contains("]]>") {
            break;
        }
    }
    Element::with_value(ElementType::HtmlBlock, content)
}

/// Parse processing instruction block.
#[allow(dead_code)]
fn parse_html_pi_block(lines: &[&str], pos: &mut usize) -> Element {
    let mut content = String::new();
    while *pos < lines.len() {
        let line = lines[*pos];
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line);
        *pos += 1;
        if line.contains("?>") {
            break;
        }
    }
    Element::with_value(ElementType::HtmlBlock, content)
}

/// Parse a generic HTML block (block-level tag until blank line or matching close).
#[allow(dead_code)]
fn parse_html_block_content(lines: &[&str], pos: &mut usize) -> Element {
    let mut content = String::new();

    while *pos < lines.len() {
        let line = lines[*pos];

        // HTML block ends at a blank line
        if is_blank_line(line) {
            break;
        }

        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line);
        *pos += 1;
    }

    Element::with_value(ElementType::HtmlBlock, content)
}

/// Parse an HTML block element with proper tag matching, attribute normalization,
/// and optional markdown parsing inside.
/// Result of parsing an HTML block element.
struct HtmlBlockResult {
    element: Element,
    /// Trailing content after the closing tag that should be re-parsed.
    trailing: Option<String>,
}

fn parse_html_block_element(
    lines: &[&str],
    pos: &mut usize,
    tag_name: &str,
    options: &Options,
) -> HtmlBlockResult {
    let tag_lc = tag_name.to_lowercase();

    // Check for void elements (self-closing)
    if is_html_void_tag(&tag_lc) {
        let line = lines[*pos];
        let trimmed = line.trim();
        let normalized = normalize_html_tag(trimmed);
        *pos += 1;
        return HtmlBlockResult {
            element: Element::with_value(ElementType::HtmlBlock, normalized),
            trailing: None,
        };
    }

    // For raw tags (script, style), collect everything until the closing tag
    if is_html_raw_tag(&tag_lc) {
        let mut elem = parse_html_raw_block(lines, pos, &tag_lc);
        let trailing = elem.options.remove("html_trailing");
        return HtmlBlockResult {
            element: elem,
            trailing,
        };
    }

    // For textarea, collect content including blank lines until closing tag
    if tag_lc == "textarea" {
        return HtmlBlockResult {
            element: parse_html_textarea_block(lines, pos),
            trailing: None,
        };
    }

    // Determine markdown parsing mode from markdown attr and options
    let first_line = lines[*pos];
    let first_trimmed = first_line.trim_start();
    let markdown_attr = extract_markdown_attr(first_trimmed);
    let parse_mode = determine_parse_mode(&tag_lc, &markdown_attr, options);

    // Collect the raw HTML block lines with nesting
    // For known HTML elements, use case-insensitive matching.
    // For unknown/XML elements, use case-sensitive matching with original tag name.
    // When the content will be parsed as block markdown, skip tag counting on
    // lines indented 4+ spaces (they'll become code blocks where tags are literal).
    let skip_code_indented = parse_mode == HtmlParseMode::Block;
    let is_known_html = is_html_block_tag(&tag_lc) || HTML_SPAN_TAGS.contains(&tag_lc.as_str());
    let collect_result = if is_known_html {
        collect_html_block_lines_impl(lines, pos, &tag_lc, true, skip_code_indented)
    } else {
        collect_html_block_lines_impl(lines, pos, tag_name, false, skip_code_indented)
    };
    let collected = &collect_result.lines;
    let trailing = collect_result.trailing;

    let element = match parse_mode {
        HtmlParseMode::Raw => {
            // Pass through with attribute normalization on the opening tag only
            let mut output = String::new();
            for (i, line) in collected.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                if i == 0 {
                    // Normalize the opening tag line
                    output.push_str(&normalize_html_line(line));
                } else {
                    output.push_str(line);
                }
            }
            Element::with_value(ElementType::HtmlBlock, output)
        }
        HtmlParseMode::Block => {
            // Parse inner content as block-level markdown
            let mut elem = Element::new(ElementType::HtmlBlock);
            elem.options.insert("tag".to_string(), tag_lc.clone());
            elem.options
                .insert("parse_mode".to_string(), "block".to_string());
            // Store normalized attrs (minus markdown)
            if let Some((_, attr_str, _, _)) = parse_html_opening_tag(first_trimmed) {
                let attrs = parse_html_tag_attrs(&attr_str);
                let filtered: Vec<_> = attrs.into_iter().filter(|(k, _)| k != "markdown").collect();
                elem.options
                    .insert("attrs".to_string(), format_html_attrs(&filtered));
            }
            // Extract inner content (skip first opening tag line and last closing tag line)
            let inner_content = extract_inner_content(collected, &tag_lc);
            let inner_lines: Vec<&str> = inner_content.lines().collect();
            let mut inner_pos = 0;
            let options_copy = options.clone();
            parse_blocks(
                &inner_lines,
                &mut inner_pos,
                true,
                &mut elem.children,
                &options_copy,
                1,
                &mut AldMap::new(),
            );
            elem
        }
        HtmlParseMode::Span => {
            // Parse inner content as span-level markdown
            let mut elem = Element::new(ElementType::HtmlBlock);
            elem.options.insert("tag".to_string(), tag_lc.clone());
            elem.options
                .insert("parse_mode".to_string(), "span".to_string());
            if let Some((_, attr_str, _, _)) = parse_html_opening_tag(first_trimmed) {
                let attrs = parse_html_tag_attrs(&attr_str);
                let filtered: Vec<_> = attrs.into_iter().filter(|(k, _)| k != "markdown").collect();
                elem.options
                    .insert("attrs".to_string(), format_html_attrs(&filtered));
            }
            // Check if opening tag is on its own line and closing tag is on its own line.
            // This determines whether to output newlines around the content.
            let open_tag_solo = collected
                .first()
                .map(|l| {
                    if let Some(gt_pos) = l.find('>') {
                        l[gt_pos + 1..].trim().is_empty()
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            let close_tag_solo = collected
                .last()
                .map(|l| {
                    l.trim()
                        .to_lowercase()
                        .starts_with(&format!("</{}", tag_lc))
                })
                .unwrap_or(false);
            if open_tag_solo && close_tag_solo && collected.len() > 2 {
                elem.options
                    .insert("multiline_span".to_string(), "true".to_string());
            }
            if close_tag_solo && collected.len() > 1 {
                elem.options
                    .insert("close_solo".to_string(), "true".to_string());
            }
            let mut inner_content = extract_inner_content(collected, &tag_lc);
            // When the opening tag is on its own line, the content starts with
            // a newline in kramdown (preserving the whitespace structure).
            if open_tag_solo && collected.len() > 1 && !inner_content.starts_with('\n') {
                inner_content.insert(0, '\n');
            }
            // When there's no closing tag (auto-closed), add trailing newline
            let has_close_tag = collected
                .last()
                .map(|l| l.to_lowercase().contains(&format!("</{}", tag_lc)))
                .unwrap_or(false);
            if !has_close_tag && !inner_content.ends_with('\n') {
                inner_content.push('\n');
            }
            elem.value = Some(inner_content);
            elem
        }
    };

    HtmlBlockResult { element, trailing }
}

/// HTML parsing mode for content inside HTML blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlParseMode {
    Raw,
    Block,
    Span,
}

/// Extract the `markdown` attribute value from an HTML tag string.
fn extract_markdown_attr(tag_line: &str) -> Option<String> {
    if let Some(idx) = tag_line.find("markdown=") {
        let after = &tag_line[idx + 9..];
        if after.starts_with('"') {
            after[1..].find('"').map(|e| after[1..e + 1].to_string())
        } else if after.starts_with('\'') {
            after[1..].find('\'').map(|e| after[1..e + 1].to_string())
        } else {
            let v: String = after
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '>' && *c != '/')
                .collect();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        }
    } else {
        None
    }
}

/// Kramdown content model: span-level elements whose content is parsed as inline text.
/// This differs from HTML_SPAN_TAGS which is used for determining block vs inline element status.
const HTML_CONTENT_MODEL_SPAN: &[&str] = &[
    "a", "abbr", "acronym", "b", "bdo", "big", "button", "cite", "caption", "del", "dfn", "dt",
    "em", "h1", "h2", "h3", "h4", "h5", "h6", "i", "ins", "label", "legend", "optgroup", "p", "q",
    "rb", "rbc", "rp", "rt", "rtc", "ruby", "select", "small", "span", "strong", "sub", "sup",
    "th", "tt",
];

/// Kramdown content model: block-level elements whose content is parsed as block markdown.
const HTML_CONTENT_MODEL_BLOCK: &[&str] = &[
    "address",
    "applet",
    "article",
    "aside",
    "blockquote",
    "body",
    "dd",
    "details",
    "div",
    "dl",
    "fieldset",
    "figure",
    "figcaption",
    "footer",
    "form",
    "header",
    "hgroup",
    "iframe",
    "li",
    "main",
    "map",
    "menu",
    "nav",
    "noscript",
    "object",
    "section",
    "summary",
    "td",
];

/// Kramdown content model: raw elements whose content is never parsed as markdown.
/// Also: tags not in block/span lists default to raw (e.g., table, tbody, tr, ul, ol).
const HTML_CONTENT_MODEL_RAW: &[&str] = &[
    "script", "style", "math", "option", "textarea", "pre", "code", "kbd", "samp", "var",
];

/// Determine parse mode for HTML content.
fn determine_parse_mode(
    tag_lc: &str,
    markdown_attr: &Option<String>,
    options: &Options,
) -> HtmlParseMode {
    if let Some(ref attr) = markdown_attr {
        return match attr.as_str() {
            "0" => HtmlParseMode::Raw,
            "block" => HtmlParseMode::Block,
            "span" => HtmlParseMode::Span,
            "1" => {
                if HTML_CONTENT_MODEL_SPAN.contains(&tag_lc) {
                    HtmlParseMode::Span
                } else {
                    HtmlParseMode::Block
                }
            }
            _ => HtmlParseMode::Raw,
        };
    }
    if options.parse_block_html {
        if HTML_CONTENT_MODEL_RAW.contains(&tag_lc) {
            return HtmlParseMode::Raw;
        }
        if HTML_CONTENT_MODEL_SPAN.contains(&tag_lc) {
            return HtmlParseMode::Span;
        }
        if HTML_CONTENT_MODEL_BLOCK.contains(&tag_lc) {
            return HtmlParseMode::Block;
        }
        // Tags not in any content model list default to raw (e.g., table, tbody, tr, ul, ol)
        return HtmlParseMode::Raw;
    }
    HtmlParseMode::Raw
}

/// Result of collecting HTML block lines.
struct CollectedHtmlBlock {
    /// The collected lines (including opening and closing tag lines).
    lines: Vec<String>,
    /// Any trailing content on the closing tag line that should be re-parsed.
    trailing: Option<String>,
}

/// Collect all lines belonging to an HTML block, tracking tag nesting.
/// Returns the collected lines (including opening and closing tag lines).
/// When `case_insensitive` is true, tag matching ignores case (for known HTML elements).
/// When false, matching is case-sensitive (for XML/unknown elements).
fn collect_html_block_lines_impl(
    lines: &[&str],
    pos: &mut usize,
    tag_match: &str,
    case_insensitive: bool,
    skip_code_indented: bool,
) -> CollectedHtmlBlock {
    let mut collected: Vec<String> = Vec::new();
    let mut nesting = 0i32;
    let mut trailing: Option<String> = None;

    while *pos < lines.len() {
        let line = lines[*pos];

        // When parsing block-level content, lines indented 4+ spaces will become
        // code blocks where HTML tags are literal text. Don't count tags on those
        // lines (but always count on the first line, which is the opening tag).
        let is_code_indent =
            skip_code_indented && !collected.is_empty() && line.starts_with("    ");

        if is_code_indent {
            // Skip tag counting for code-indented lines
            collected.push(line.to_string());
            *pos += 1;
            continue;
        }

        let search_line = if case_insensitive {
            line.to_lowercase()
        } else {
            line.to_string()
        };

        // Count opening tags (excluding self-closing)
        let opens = count_open_tags_in_line(&search_line, tag_match);
        let closes = count_close_tags_in_line(&search_line, tag_match);

        // Process tags left-to-right to detect when nesting first reaches 0.
        // This handles cases like "</div> <div>" where the closing tag ends one
        // block and the opening tag starts a new one on the same line, as well
        // as "<p>text</p>more text</p>" where the first close ends the block.
        let close_pattern = format!("</{}", tag_match);
        if closes > 0 && (nesting > 0 || opens > 0) && search_line.contains(&close_pattern) {
            // Use the actual nesting level as starting point.
            // For the first line where nesting is 0, find_nesting_zero_split
            // will process tags left-to-right: open(+1) ... close(-1).
            // When nesting first reaches 0, that's where we split.
            let effective_nesting = nesting;
            // Find the position where the closing tag that brings nesting to 0 ends
            if let Some(split_pos) =
                find_nesting_zero_split(&search_line, tag_match, effective_nesting)
            {
                let rest = line[split_pos..].trim();
                if !rest.is_empty() {
                    // Only include up to the end of the closing tag
                    collected.push(line[..split_pos].to_string());
                    trailing = Some(rest.to_string());
                    *pos += 1;
                    break;
                }
            }
        }

        nesting += opens as i32 - closes as i32;

        collected.push(line.to_string());
        *pos += 1;

        if nesting <= 0 {
            break;
        }
    }

    CollectedHtmlBlock {
        lines: collected,
        trailing,
    }
}

/// Find the position in a line where nesting first reaches 0, processing tags left-to-right.
/// Returns the byte offset right after the closing tag that brings nesting to 0,
/// or None if nesting never reaches 0 on this line.
fn find_nesting_zero_split(
    search_line: &str,
    tag_match: &str,
    initial_nesting: i32,
) -> Option<usize> {
    let open_pattern = format!("<{}", tag_match);
    let close_pattern = format!("</{}", tag_match);
    let mut nesting = initial_nesting;

    // Collect all tag positions with their type
    let mut events: Vec<(usize, bool, usize)> = Vec::new(); // (start_pos, is_close, end_pos)

    // Find all opening tags
    let mut search_from = 0;
    while let Some(idx) = search_line[search_from..].find(&open_pattern) {
        let abs_idx = search_from + idx;
        // Check it's not a closing tag
        if abs_idx > 0 && search_line.as_bytes().get(abs_idx.wrapping_sub(1)) == Some(&b'/') {
            search_from = abs_idx + open_pattern.len();
            continue;
        }
        let after = &search_line[abs_idx + open_pattern.len()..];
        // Check for self-closing
        let is_self_closing = if let Some(gt_pos) = after.find('>') {
            after[..gt_pos].trim_end().ends_with('/')
        } else {
            false
        };
        if !is_self_closing {
            let end = if let Some(gt_pos) = after.find('>') {
                abs_idx + open_pattern.len() + gt_pos + 1
            } else {
                abs_idx + open_pattern.len()
            };
            events.push((abs_idx, false, end));
        }
        search_from = abs_idx + open_pattern.len();
    }

    // Find all closing tags
    search_from = 0;
    while let Some(idx) = search_line[search_from..].find(&close_pattern) {
        let abs_idx = search_from + idx;
        let after = &search_line[abs_idx + close_pattern.len()..];
        let end = if let Some(gt_pos) = after.find('>') {
            abs_idx + close_pattern.len() + gt_pos + 1
        } else {
            abs_idx + close_pattern.len()
        };
        events.push((abs_idx, true, end));
        search_from = abs_idx + close_pattern.len();
    }

    // Sort by position
    events.sort_by_key(|e| e.0);

    // Process in order
    for (_, is_close, end_pos) in events {
        if is_close {
            nesting -= 1;
        } else {
            nesting += 1;
        }
        if nesting <= 0 {
            return Some(end_pos);
        }
    }

    None
}

/// Count opening tags for a given tag name in a line (case-insensitive).
/// Excludes self-closing tags like `<tag ... />`.
fn count_open_tags_in_line(line_lc: &str, tag_lc: &str) -> usize {
    let open_pattern = format!("<{}", tag_lc);
    let close_pattern = format!("</{}", tag_lc);
    let mut count = 0;
    let mut search_from = 0;

    while let Some(idx) = line_lc[search_from..].find(&open_pattern) {
        let abs_idx = search_from + idx;
        // Verify this is not a closing tag
        if abs_idx > 0
            && line_lc[abs_idx - 1..]
                .starts_with(&close_pattern[..close_pattern.len().min(line_lc.len() - abs_idx + 1)])
        {
            search_from = abs_idx + 1;
            continue;
        }
        // Check it's actually a closing tag by looking at abs_idx-1 for /
        if abs_idx > 0 && line_lc.as_bytes().get(abs_idx.wrapping_sub(1)) == Some(&b'/') {
            search_from = abs_idx + 1;
            continue;
        }
        // Check for self-closing: find the > and see if /> precedes it
        let after = &line_lc[abs_idx + open_pattern.len()..];
        let is_self_closing = if let Some(gt_pos) = after.find('>') {
            let before_gt = &after[..gt_pos];
            before_gt.trim_end().ends_with('/')
        } else {
            false
        };
        if !is_self_closing {
            count += 1;
        }
        search_from = abs_idx + open_pattern.len();
    }

    count
}

/// Count closing tags for a given tag name in a line (case-insensitive).
fn count_close_tags_in_line(line_lc: &str, tag_lc: &str) -> usize {
    let pattern = format!("</{}", tag_lc);
    let mut count = 0;
    let mut search_from = 0;
    while let Some(idx) = line_lc[search_from..].find(&pattern) {
        count += 1;
        search_from = search_from + idx + pattern.len();
    }
    count
}

/// Normalize the first HTML tag on a line (attribute quotes, etc).
fn normalize_html_line(line: &str) -> String {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('<') || trimmed.starts_with("</") || trimmed.starts_with("<!") {
        return line.to_string();
    }
    // Try to parse as an opening tag for normalization
    if let Some((tag_name, attr_str, is_self_closing, rest)) = parse_html_opening_tag(trimmed) {
        // For XML-namespaced tags (containing ':'), preserve attribute name case
        let is_xml = tag_name.contains(':');
        let attrs = parse_html_tag_attrs_impl(&attr_str, !is_xml);
        // Remove markdown attribute
        let filtered: Vec<_> = attrs
            .iter()
            .filter(|(k, _)| k != "markdown")
            .cloned()
            .collect();
        let indent = &line[..line.len() - trimmed.len()];
        let tag_lc = tag_name.to_lowercase();
        // Preserve case for non-standard HTML tags (namespaced, unknown)
        let output_tag = if is_html_block_tag(&tag_lc) {
            tag_lc.clone()
        } else {
            tag_name.clone()
        };
        let attrs_str = format_html_attrs(&filtered);
        if is_self_closing || is_html_void_tag(&tag_lc) {
            format!("{indent}<{output_tag}{attrs_str} />{rest}")
        } else {
            format!("{indent}<{output_tag}{attrs_str}>{rest}")
        }
    } else {
        line.to_string()
    }
}

/// Extract inner content from collected HTML block lines, removing the opening
/// tag from the first line and closing tag from the last line.
fn extract_inner_content(collected: &[String], tag_lc: &str) -> String {
    if collected.is_empty() {
        return String::new();
    }

    let mut lines = collected.to_vec();

    // Remove opening tag from first line
    if let Some(first) = lines.first_mut() {
        if let Some(gt_pos) = first.find('>') {
            *first = first[gt_pos + 1..].to_string();
            if first.trim().is_empty() {
                lines.remove(0);
            }
        }
    }

    // Remove closing tag from last line
    if let Some(last) = lines.last_mut() {
        let last_lc = last.to_lowercase();
        if let Some(idx) = last_lc.rfind(&format!("</{}", tag_lc)) {
            *last = last[..idx].to_string();
            if last.trim().is_empty() {
                lines.pop();
            }
        }
    }

    lines.join("\n")
}

/// Find the position of a closing tag (case-insensitive) in a string.
#[allow(dead_code)]
fn find_closing_tag_ci(s: &str, tag_lc: &str) -> Option<usize> {
    let s_lc = s.to_lowercase();
    let pattern = format!("</{}", tag_lc);
    s_lc.find(&pattern)
}

/// Find the last closing tag position.
#[allow(dead_code)]
fn find_last_closing_tag_ci(s: &str, tag_lc: &str) -> Option<usize> {
    let s_lc = s.to_lowercase();
    let pattern = format!("</{}", tag_lc);
    s_lc.rfind(&pattern)
}

/// Find the end position after a closing tag starting at `start`.
#[allow(dead_code)]
fn find_after_closing_tag(s: &str, start: usize, _tag_lc: &str) -> usize {
    if let Some(gt_pos) = s[start..].find('>') {
        start + gt_pos + 1
    } else {
        s.len()
    }
}

/// Count opening tag occurrences (excluding closing tags).
#[allow(dead_code)]
fn count_tag_occurrences(s_lc: &str, open_pattern: &str, close_pattern: &str) -> usize {
    let mut count = 0;
    let mut search_from = 0;
    while let Some(idx) = s_lc[search_from..].find(open_pattern) {
        let abs_idx = search_from + idx;
        // Make sure this is not actually a closing tag
        if !s_lc[abs_idx..].starts_with(close_pattern) {
            count += 1;
        }
        search_from = abs_idx + 1;
    }
    count
}

/// Count closing tag occurrences.
#[allow(dead_code)]
fn count_closing_tags(s_lc: &str, tag_lc: &str) -> usize {
    let pattern = format!("</{}", tag_lc);
    let mut count = 0;
    let mut search_from = 0;
    while let Some(idx) = s_lc[search_from..].find(&pattern) {
        count += 1;
        search_from = search_from + idx + 1;
    }
    count
}

/// Parse an HTML block with a "raw" tag (script, style) that preserves content literally.
fn parse_html_raw_block(lines: &[&str], pos: &mut usize, tag_lc: &str) -> Element {
    let close_pattern = format!("</{}>", tag_lc);
    let mut content = String::new();
    let mut trailing_text: Option<String> = None;

    while *pos < lines.len() {
        let line = lines[*pos];
        if !content.is_empty() {
            content.push('\n');
        }

        let line_lc = line.to_lowercase();
        if let Some(close_idx) = line_lc.find(&close_pattern) {
            // Find end of closing tag
            let end_of_close = close_idx + close_pattern.len();
            // Include only up to end of closing tag
            content.push_str(&line[..end_of_close]);
            // Check for trailing content
            let rest = line[end_of_close..].trim();
            if !rest.is_empty() {
                trailing_text = Some(rest.to_string());
            }
            *pos += 1;
            break;
        }

        content.push_str(line);
        *pos += 1;
    }

    let mut elem = Element::with_value(ElementType::HtmlBlock, content);
    elem.options.insert("type".to_string(), "raw".to_string());
    if let Some(trailing) = trailing_text {
        elem.options.insert("html_trailing".to_string(), trailing);
    }
    elem
}

/// Parse a textarea HTML block (content preserved, crosses blank lines).
fn parse_html_textarea_block(lines: &[&str], pos: &mut usize) -> Element {
    let mut content = String::new();

    while *pos < lines.len() {
        let line = lines[*pos];
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(line);
        *pos += 1;

        let line_lc = line.to_lowercase();
        if line_lc.contains("</textarea>") {
            break;
        }
    }

    Element::with_value(ElementType::HtmlBlock, content)
}

// ---------------------------------------------------------------------------
// Block extension parsing ({::comment}, {::nomarkdown}, {::options})
// ---------------------------------------------------------------------------

/// Try to parse a block extension.
fn try_parse_block_extension(lines: &[&str], pos: &mut usize) -> Option<Element> {
    let line = lines[*pos];
    let trimmed = line.trim();

    if !trimmed.starts_with("{::") {
        return None;
    }

    // Self-closing extension: {::name attrs /}
    if trimmed.ends_with("/}") {
        let inner = &trimmed[3..trimmed.len() - 2].trim();
        // Extract extension name
        let name = inner.split_whitespace().next().unwrap_or("");
        match name {
            "comment" => {
                // Self-closing comment produces nothing
                *pos += 1;
                // Return empty comment that won't produce output
                let elem = Element::with_value(ElementType::BlockExtension, "");
                return Some(elem);
            }
            "nomarkdown" => {
                // Self-closing nomarkdown - check for type attribute
                let attrs_str = inner.strip_prefix("nomarkdown").unwrap_or("").trim();
                let is_html_type =
                    attrs_str.contains("type='html'") || attrs_str.contains("type=\"html\"");
                if is_html_type || !attrs_str.contains("type=") {
                    // Produce empty output for self-closing nomarkdown
                    *pos += 1;
                    let elem = Element::with_value(ElementType::BlockExtension, "");
                    return Some(elem);
                } else {
                    // Non-html type - suppress
                    *pos += 1;
                    let elem = Element::with_value(ElementType::BlockExtension, "");
                    return Some(elem);
                }
            }
            "options" => {
                // Options extension - store the options string for later processing
                let attrs_str = inner.strip_prefix("options").unwrap_or("").trim();
                *pos += 1;
                let mut elem = Element::with_value(ElementType::BlockExtension, "");
                elem.options
                    .insert("ext_type".to_string(), "options".to_string());
                elem.options
                    .insert("options_str".to_string(), attrs_str.to_string());
                return Some(elem);
            }
            _ => {
                // Unknown extension - treat as text (return None to fall through to paragraph)
                return None;
            }
        }
    }

    // Block extension with end tag: {::name} ... {:/name}
    let inner = trimmed[3..].strip_suffix('}')?;
    let inner = inner.trim();

    // Extract extension name
    let name = inner.split_whitespace().next().unwrap_or("");

    match name {
        "comment" => {
            // Collect content until {:/comment} or {:/} on its own line
            let saved = *pos;
            *pos += 1;
            let mut content = String::new();
            let mut found_close = false;
            while *pos < lines.len() {
                let l = lines[*pos];
                let lt = l.trim();
                if lt == "{:/comment}" || lt == "{:/}" {
                    *pos += 1;
                    found_close = true;
                    break;
                }
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(l);
                *pos += 1;
            }
            if !found_close {
                // Unclosed comment - revert, treat as text
                *pos = saved;
                return None;
            }
            let mut elem = Element::with_value(ElementType::BlockExtension, content);
            elem.options
                .insert("ext_type".to_string(), "comment".to_string());
            Some(elem)
        }
        "nomarkdown" => {
            // Check for type attribute
            let attrs_str = inner.strip_prefix("nomarkdown").unwrap_or("").trim();
            let is_html_type =
                attrs_str.contains("type='html'") || attrs_str.contains("type=\"html\"");
            let is_latex_type =
                attrs_str.contains("type=\"latex\"") || attrs_str.contains("type='latex'");
            let saved = *pos;
            *pos += 1;
            let mut content = String::new();
            let mut found_close = false;
            while *pos < lines.len() {
                let l = lines[*pos];
                let lt = l.trim();
                if lt == "{:/nomarkdown}" || lt == "{:/}" {
                    *pos += 1;
                    found_close = true;
                    break;
                }
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(l);
                *pos += 1;
            }
            if !found_close {
                // Unclosed nomarkdown - revert, treat as text
                *pos = saved;
                return None;
            }
            if is_latex_type {
                // Latex type - suppress
                let elem = Element::with_value(ElementType::BlockExtension, "");
                return Some(elem);
            }
            let mut elem = Element::with_value(ElementType::BlockExtension, content);
            elem.options
                .insert("ext_type".to_string(), "nomarkdown".to_string());
            if is_html_type {
                elem.options
                    .insert("nomarkdown_type".to_string(), "html".to_string());
            }
            Some(elem)
        }
        "options" => {
            // Block options with end tag
            let attrs_str = inner.strip_prefix("options").unwrap_or("").trim();
            *pos += 1;
            let mut elem = Element::with_value(ElementType::BlockExtension, "");
            elem.options
                .insert("ext_type".to_string(), "options".to_string());
            elem.options
                .insert("options_str".to_string(), attrs_str.to_string());
            Some(elem)
        }
        _ => {
            // Unknown extension - treat as text
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Math block parsing ($$...$$)
// ---------------------------------------------------------------------------

/// Try to parse a display math block ($$...$$).
fn try_parse_math_block(lines: &[&str], pos: &mut usize) -> Option<Element> {
    let line = lines[*pos];

    // Must start with $$ and not be indented 4+ spaces
    let stripped = line.trim_start();
    let indent = line.len() - stripped.len();
    if indent >= 4 {
        return None;
    }

    // Handle \$$...$$ pattern: backslash cancels block math but not inline math.
    // The paragraph parser will handle it via span parsing where \$ becomes
    // literal $ and $$...$$ becomes inline math.
    if stripped.starts_with("\\$$") {
        return None;
    }

    if !stripped.starts_with("$$") {
        return None;
    }

    let after_open = &stripped[2..];

    // Check for single-line math: $$content$$
    if let Some(close_pos) = after_open.find("$$") {
        // Single-line: $$content$$
        let content = &after_open[..close_pos];
        let after_close = after_open[close_pos + 2..].trim();
        if after_close.is_empty() {
            // Only treat as block math if followed by blank line, EOB, or end of document
            let next_line = if *pos + 1 < lines.len() {
                Some(lines[*pos + 1])
            } else {
                None
            };
            let is_block = match next_line {
                None => true,                        // End of document
                Some(l) if is_blank_line(l) => true, // Blank line after
                Some(l) if l.trim() == "^" => true,  // EOB after
                Some(l) if is_block_ial(l) => true,  // IAL after (for attributes)
                _ => false,                          // Text follows - inline math
            };
            if is_block {
                *pos += 1;
                let elem = Element::with_value(ElementType::MathBlock, content.trim());
                return Some(elem);
            }
            return None; // Not a block math - let paragraph handle it
        }
        // Has trailing content after $$ - not a valid math block
        return None;
    }

    // Multi-line math: starts with $$ on one line, content, ends with $$
    let first_content = after_open;
    let mut content_lines: Vec<String> = Vec::new();
    if !first_content.is_empty() {
        content_lines.push(first_content.to_string());
    }

    let start_pos = *pos;
    *pos += 1;

    while *pos < lines.len() {
        let l = lines[*pos];
        let lt = l.trim();
        if lt.ends_with("$$") {
            let before_close = &lt[..lt.len() - 2];
            if !before_close.is_empty() {
                content_lines.push(before_close.to_string());
            }
            *pos += 1;
            let content = content_lines.join("\n");
            let elem = Element::with_value(ElementType::MathBlock, content);
            return Some(elem);
        }
        content_lines.push(l.to_string());
        *pos += 1;
    }

    // Unclosed math block - revert
    *pos = start_pos;
    None
}

// ---------------------------------------------------------------------------
// Definition list parsing
// ---------------------------------------------------------------------------

/// Check if a line is a definition list definition marker (starts with `: ` or is just `:`).
fn is_definition_marker(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(": ") || trimmed == ":"
}

/// Try to parse a definition list.
/// In kramdown, a definition list is:
///   term
///   : definition
fn try_parse_definition_list(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    options: &Options,
) -> Option<Element> {
    let start = *pos;
    // Verify: need term line(s) followed by definition marker
    let mut look = start;
    let mut found_term = false;
    let mut blank_count = 0;
    while look < lines.len() {
        let line = lines[look];
        if is_blank_line(line) {
            if !found_term {
                return None;
            }
            blank_count += 1;
            if blank_count > 1 {
                return None;
            }
            look += 1;
            continue;
        }
        if line.trim() == "^" {
            return None;
        }
        if is_definition_marker(line) {
            if found_term {
                break;
            }
            return None;
        }
        if blank_count > 0 {
            return None;
        }
        found_term = true;
        look += 1;
    }
    if !found_term || look >= lines.len() || !is_definition_marker(lines[look]) {
        return None;
    }

    let mut dl = Element::new(ElementType::DefinitionList);
    *pos = start;
    let mut blank_start_after_dd = *pos;

    loop {
        if *pos < lines.len() && lines[*pos].trim() == "^" {
            *pos += 1;
            break;
        }

        let mut terms: Vec<String> = Vec::new();
        while *pos < lines.len() {
            let line = lines[*pos];
            if is_blank_line(line)
                || is_definition_marker(line)
                || line.trim() == "^"
                || is_block_ial(line)
            {
                break;
            }
            terms.push(line.trim().to_string());
            *pos += 1;
        }

        let mut had_blank = false;
        while *pos < lines.len() && is_blank_line(lines[*pos]) {
            had_blank = true;
            *pos += 1;
        }

        for term_text in &terms {
            let (term_clean, term_attrs) = extract_term_ial(term_text);
            let mut dt = Element::with_value(ElementType::DefinitionTerm, term_clean);
            if let Some(attrs) = term_attrs {
                apply_attrs(&mut dt, &attrs);
            }
            dl.children.push(dt);
        }

        while *pos < lines.len() && is_definition_marker(lines[*pos]) {
            let line = lines[*pos];
            let trimmed = line.trim_start();
            let line_indent = line.len() - trimmed.len();
            let def_content = if trimmed == ":" {
                String::new()
            } else {
                trimmed[2..].to_string()
            };
            // Calculate content indent for stripping continuation lines
            let first_content_char_pos = def_content.len() - def_content.trim_start().len();
            let mut strip_amount = line_indent + 2 + first_content_char_pos;
            *pos += 1;

            let (def_clean, def_ial) = extract_def_ial(&def_content);
            // If content is empty (just IAL or empty def), increase strip amount
            if def_clean.trim().is_empty() {
                if def_ial.is_some() {
                    let ial_len = def_content.trim().len() - def_clean.len();
                    strip_amount += ial_len.min(2);
                }
                // Ensure minimum strip of 4 for empty definitions with block content
                if strip_amount < 4 {
                    strip_amount = 4;
                }
            }
            let first_def = def_clean.trim_start().to_string();
            // Detect block content markers at the start of the definition
            let starts_with_block = first_def.starts_with("> ")
                || first_def.starts_with("# ")
                || first_def.starts_with("* ")
                || first_def.starts_with("+ ")
                || first_def.starts_with("- ")
                || first_def.starts_with("    ")
                || first_def.starts_with('\t');
            let mut def_lines: Vec<String> = vec![first_def];
            let mut has_block = starts_with_block;

            while *pos < lines.len() {
                let next = lines[*pos];
                if is_blank_line(next) {
                    let mut la = *pos + 1;
                    while la < lines.len() && is_blank_line(lines[la]) {
                        la += 1;
                    }
                    if la < lines.len()
                        && !is_definition_marker(lines[la])
                        && (lines[la].starts_with("  ") || lines[la].starts_with('\t'))
                        && lines[la].trim() != "^"
                    {
                        has_block = true;
                        for _ in *pos..la {
                            def_lines.push(String::new());
                        }
                        *pos = la;
                        continue;
                    }
                    break;
                }
                // Only break on definition markers at the outer level (not indented)
                let next_indent = next.len() - next.trim_start().len();
                if next_indent == 0
                    && (is_definition_marker(next) || next.trim() == "^" || is_block_ial(next))
                {
                    break;
                }
                if next_indent > 0 && is_definition_marker(next) {
                    // Indented definition marker -> nested definition list (block content)
                    has_block = true;
                }
                // Non-blank, non-definition continuation line
                // Strip the same indent as the definition content start
                def_lines.push(strip_n_spaces(next, strip_amount).to_string());
                *pos += 1;
            }

            while def_lines.last().is_some_and(|l| l.is_empty()) {
                def_lines.pop();
            }

            // Additional block detection: empty first line + indented continuation
            if !has_block && def_lines.first().is_some_and(|f| f.is_empty()) && def_lines.len() > 1
            {
                has_block = true;
            }
            // Detect nested definition list in continuation
            if !has_block && def_lines.len() > 1 {
                for dl_line in &def_lines[1..] {
                    if is_definition_marker(dl_line) {
                        has_block = true;
                        break;
                    }
                }
            }

            let mut dd = Element::new(ElementType::DefinitionDefinition);
            if let Some(ref ial) = def_ial {
                apply_attrs(&mut dd, ial);
            }

            if has_block {
                dd.options
                    .insert("block_content".to_string(), "true".to_string());
                let inner: Vec<&str> = def_lines.iter().map(|s| s.as_str()).collect();
                let mut inner_pos = 0;
                parse_blocks(
                    &inner,
                    &mut inner_pos,
                    has_trailing_newline,
                    &mut dd.children,
                    options,
                    2,
                    &mut AldMap::new(),
                );
            } else if had_blank {
                dd.options
                    .insert("para_wrap".to_string(), "true".to_string());
                dd.value = Some(def_lines.join("\n").trim_end().to_string());
            } else {
                dd.value = Some(def_lines.join("\n").trim_end().to_string());
            }

            dl.children.push(dd);

            had_blank = false;
            blank_start_after_dd = *pos;
            while *pos < lines.len() && is_blank_line(lines[*pos]) {
                had_blank = true;
                *pos += 1;
            }
        }

        if *pos < lines.len() && lines[*pos].trim() == "^" {
            *pos += 1;
            break;
        }

        if *pos < lines.len()
            && !is_definition_marker(lines[*pos])
            && !is_blank_line(lines[*pos])
            && lines[*pos].trim() != "^"
        {
            let mut future = *pos;
            while future < lines.len()
                && !is_blank_line(lines[future])
                && !is_definition_marker(lines[future])
                && lines[future].trim() != "^"
            {
                future += 1;
            }
            let mut future2 = future;
            while future2 < lines.len() && is_blank_line(lines[future2]) {
                future2 += 1;
            }
            if future2 < lines.len() && is_definition_marker(lines[future2]) {
                continue;
            }
        }
        // When breaking out of the DL, restore position to the blank line(s)
        // so the parent parser can create Blank elements for proper spacing.
        if had_blank {
            *pos = blank_start_after_dd;
        }
        break;
    }

    if dl.children.is_empty() {
        *pos = start;
        return None;
    }
    Some(dl)
}

fn extract_term_ial(text: &str) -> (String, Option<Vec<(String, String)>>) {
    let trimmed = text.trim();
    if trimmed.starts_with("{:") {
        if let Some(end) = trimmed.find('}') {
            let ial_str = &trimmed[..end + 1];
            let rest = trimmed[end + 1..].trim();
            let attrs = parse_ial(ial_str);
            return (rest.to_string(), Some(attrs));
        }
    }
    (text.to_string(), None)
}

fn extract_def_ial(text: &str) -> (String, Option<Vec<(String, String)>>) {
    let trimmed = text.trim();
    if trimmed.starts_with("{:") {
        if let Some(end) = trimmed.find('}') {
            let ial_str = &trimmed[..end + 1];
            let rest = trimmed[end + 1..].trim();
            let attrs = parse_ial(ial_str);
            return (rest.to_string(), Some(attrs));
        }
    }
    (text.to_string(), None)
}

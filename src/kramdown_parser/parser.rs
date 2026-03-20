// kramdown parser - Block-level parser
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use crate::kramdown_parser::element::{Document, Element, ElementType};
use crate::kramdown_parser::options::Options;

/// The kramdown parser. Converts kramdown-flavored Markdown text into a Document AST.
pub struct KramdownParser;

/// Debug helper: expose is_list_start for testing.
pub fn debug_is_list_start(line: &str) -> bool {
    is_list_start(line)
}

impl KramdownParser {
    /// Parse kramdown input text into a Document AST.
    pub fn parse(input: &str, options: &Options) -> Document {
        let mut doc = Document::new();
        let lines: Vec<&str> = input.lines().collect();
        // If input ends with newline, lines() doesn't include a trailing empty element,
        // but we need to know if the file ended with newline for correct behavior.
        let has_trailing_newline = input.ends_with('\n');
        let mut pos = 0;
        parse_blocks(
            &lines,
            &mut pos,
            has_trailing_newline,
            &mut doc.root.children,
            options,
            0,
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
        if !para_lines.is_empty() {
            if try_parse_atx_header(line, options).is_some()
                || (is_horizontal_rule(line) && !is_setext_underline(line))
                || is_list_start(line)
                || is_blockquote_line(line)
                || try_parse_fenced_code(lines, *pos).is_some()
            {
                break;
            }
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
        if !para_lines.is_empty() {
            if try_parse_atx_header(line, options).is_some()
                || (is_horizontal_rule(line) && !is_setext_underline(line))
                || is_list_start(line)
                || is_blockquote_line(line)
                || try_parse_fenced_code(lines, *pos).is_some()
            {
                break;
            }
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
fn is_block_ial(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("{:") && trimmed.ends_with('}')
}

/// Parse IAL attributes from a string like `{: #id .class key="value"}`.
fn parse_ial(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    // Strip `{:` and `}`
    let inner = s
        .trim()
        .strip_prefix('{')
        .unwrap_or(s)
        .strip_prefix(':')
        .unwrap_or(s);
    let inner = inner.strip_suffix('}').unwrap_or(inner).trim();

    let mut chars = inner.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        match chars.peek() {
            Some('#') => {
                chars.next(); // consume #
                let id: String = chars.by_ref().take_while(|c| !c.is_whitespace()).collect();
                if !id.is_empty() {
                    attrs.push(("id".to_string(), id));
                }
            }
            Some('.') => {
                chars.next(); // consume .
                let class: String = chars.by_ref().take_while(|c| !c.is_whitespace()).collect();
                if !class.is_empty() {
                    attrs.push(("class".to_string(), class));
                }
            }
            _ => {
                // key="value" or key=value
                let key: String = chars.by_ref().take_while(|c| *c != '=').collect();
                if key.is_empty() {
                    break;
                }
                // Now read value
                let value = if chars.peek() == Some(&'"') {
                    chars.next(); // consume opening quote
                    let v: String = chars.by_ref().take_while(|c| *c != '"').collect();
                    v
                } else if chars.peek() == Some(&'\'') {
                    chars.next();
                    let v: String = chars.by_ref().take_while(|c| *c != '\'').collect();
                    v
                } else {
                    let v: String = chars.by_ref().take_while(|c| !c.is_whitespace()).collect();
                    v
                };
                attrs.push((key.trim().to_string(), value));
            }
        }
    }

    attrs
}

/// Apply parsed attributes to an element.
fn apply_attrs(element: &mut Element, attrs: &[(String, String)]) {
    for (key, value) in attrs {
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
            // But only if there's no blank line between
            // Actually, check: is next line indented?
            // Looking at the lazy test: the non-indented line is part of the code
            // Let's append it with a space before it (joining with previous)
            // Actually from the test case output:
            //   Input: "    This is some\ncode"
            //   Output: "This is some code\n"
            // So the lazy continuation line is appended to the previous line with a space.
            if !code_lines.is_empty() {
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
        } else if !bq_lines.is_empty() {
            // Lazy continuation: non-blank, non-blockquote line continues the blockquote
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
    parse_blocks_with_lazy(
        &inner_lines,
        &mut inner_pos,
        has_trailing_newline,
        &mut elem.children,
        options,
        1,
        &lazy_flags,
    );

    elem
}

/// Parse blocks with lazy continuation awareness.
/// `lazy_flags[i]` is true if line `i` was a lazy continuation in a parent blockquote.
fn parse_blocks_with_lazy(
    lines: &[&str],
    pos: &mut usize,
    has_trailing_newline: bool,
    children: &mut Vec<Element>,
    options: &Options,
    _indent_level: usize,
    lazy_flags: &[bool],
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

        // Block-level IAL
        if is_block_ial(line) {
            let attrs = parse_ial(line.trim());
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
                pending_ial = Some(attrs);
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
            if is_table_line(line) || try_parse_separator_line(line).is_some() {
                let prev_has_open_code_span = para_lines
                    .last()
                    .is_some_and(|l| has_unbalanced_backticks(l));
                if !prev_has_open_code_span {
                    break;
                }
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
fn build_paragraph_text(lines: &[&str], _has_trailing_newline: bool, _at_end: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }

    // First line: strip up to 3 spaces of indent
    let first = strip_up_to_3_spaces(lines[0]).trim_end();

    if lines.len() == 1 {
        return first.to_string();
    }

    // Multi-line: join with newlines, preserving original spacing for continuation lines
    let mut result = first.to_string();
    for line in &lines[1..] {
        result.push('\n');
        result.push_str(line.trim_end());
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

    if trimmed.starts_with('+') {
        // Body separator: `+ :-: |`
        kind = SeparatorKind::Body;
        // Strip leading `+` and treat rest as separator content
        work = trimmed[1..].to_string();
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
    let work = if trimmed.starts_with('|') {
        &trimmed[1..]
    } else {
        trimmed
    };

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
            for j in start..i {
                current.push(chars[j]);
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
fn is_table_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Check for escaped first pipe: `\| ...` means not a table
    if trimmed.starts_with("\\|") {
        return false;
    }

    // A line starting with `|` is a table line
    if trimmed.starts_with('|') {
        return true;
    }

    // If line has unbalanced backticks AND no escaped pipes (\|), treat backticks as
    // literal text and check for pipes ignoring code spans.
    // If it has escaped pipes, code spans take precedence (multi-line code span).
    if has_unbalanced_backticks(trimmed) && !trimmed.contains("\\|") {
        return has_unescaped_pipe_ignoring_backticks(trimmed);
    }

    // Check with code span awareness (balanced backticks)
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    let mut in_backtick = false;
    let mut backtick_count = 0;

    while i < chars.len() {
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
fn has_unescaped_pipe_ignoring_backticks(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
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
        let content_str = after_marker_str.trim_start_matches(|c: char| c == ' ' || c == '\t');
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

        let content_str = after_dot.trim_start_matches(|c: char| c == ' ' || c == '\t');
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
            if !raw_items.is_empty() {
                let current_indent = raw_items.last().unwrap().content_indent;
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

        if indent >= raw_items.last().unwrap().content_indent {
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
        let has_internal_blanks = raw_item.lines.iter().any(|l| l.is_empty());
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
    if stripped >= n {
        line[idx..].to_string()
    } else {
        line[idx..].to_string()
    }
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

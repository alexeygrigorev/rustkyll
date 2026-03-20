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
            if let Some(last) = children.last_mut() {
                apply_attrs(last, &attrs);
            }
            *pos += 1;
            continue;
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
            if let Some(e) = elem {
                children.push(e);
            }
            continue;
        }

        // Fenced code block
        if let Some(fence_result) = try_parse_fenced_code(lines, *pos) {
            children.push(fence_result.element);
            *pos = fence_result.end_pos;
            continue;
        }

        // Indented code block
        if is_indented_code_line(line) {
            let elem = parse_indented_code_block(lines, pos, has_trailing_newline);
            children.push(elem);
            continue;
        }

        // Horizontal rule
        if is_horizontal_rule(line) {
            children.push(Element::new(ElementType::HorizontalRule));
            *pos += 1;
            continue;
        }

        // Blockquote
        if is_blockquote_line(line) {
            let elem = parse_blockquote(lines, pos, has_trailing_newline, options);
            children.push(elem);
            continue;
        }

        // ATX header
        if let Some(header) = try_parse_atx_header(line, options) {
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
        if let Some(e) = elem {
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
        if !para_lines.is_empty() {
            if try_parse_atx_header(line, options).is_some() {
                break;
            }
            if is_horizontal_rule(line) && !is_setext_underline(line) {
                break;
            }
            if is_blockquote_line(line) {
                break;
            }
            if try_parse_fenced_code(lines, *pos).is_some() {
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

// kramdown parser - Span-level (inline) parser
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.

use crate::kramdown_parser::entities;
use crate::kramdown_parser::options::{EntityOutput, Options};
use std::collections::HashMap;

/// Context for span parsing, carrying link definitions and abbreviations.
pub struct SpanContext {
    /// Link definitions: id -> (url, optional title, optional attrs)
    pub link_defs: HashMap<String, LinkDef>,
    /// Abbreviations: abbreviation -> full text
    pub abbreviations: HashMap<String, String>,
    /// Footnote definitions: name -> content
    pub footnote_defs: HashMap<String, String>,
    /// Footnote counter for numbering
    pub footnote_counter: usize,
    /// Footnote order (for rendering at end)
    pub footnote_order: Vec<String>,
    /// Footnote reference counts: name -> number of times referenced
    pub footnote_ref_counts: HashMap<String, usize>,
    /// Attribute List Definitions: name -> list of (key, value) pairs
    pub ald_defs: HashMap<String, Vec<(String, String)>>,
    /// Options
    pub options: Options,
}

/// A link definition with optional IAL attributes
pub struct LinkDef {
    pub url: String,
    pub title: Option<String>,
    pub attrs: HashMap<String, String>,
}

impl SpanContext {
    pub fn new(options: &Options) -> Self {
        let mut link_defs = HashMap::new();
        for (key, (url, title)) in &options.link_defs {
            link_defs.insert(
                key.to_lowercase(),
                LinkDef {
                    url: url.clone(),
                    title: title.clone(),
                    attrs: HashMap::new(),
                },
            );
        }
        Self {
            link_defs,
            abbreviations: HashMap::new(),
            footnote_defs: HashMap::new(),
            footnote_counter: options.footnote_nr as usize,
            footnote_order: Vec::new(),
            footnote_ref_counts: HashMap::new(),
            ald_defs: HashMap::new(),
            options: options.clone(),
        }
    }
}

/// Parse the full document text to extract link definitions, abbreviations, and footnotes.
/// Returns the text with those definitions removed.
pub fn extract_definitions(text: &str, ctx: &mut SpanContext) -> String {
    let mut output_lines: Vec<&str> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Link definition: [id]: url 'title'
        // Can have 0-3 spaces of indent
        // Note: IDs starting with ^ are footnotes, not link defs
        let indent = line.len() - trimmed.len();
        if indent <= 3 {
            if let Some(rest) = trimmed.strip_prefix('[') {
                if let Some(close_bracket) = rest.find("]:") {
                    let id = &rest[..close_bracket];
                    // Valid link def IDs: non-empty, can't contain [ or ], not footnotes (^)
                    if !id.is_empty()
                        && !id.contains('[')
                        && !id.contains(']')
                        && !id.starts_with('^')
                    {
                        let after_colon = rest[close_bracket + 2..].trim_start();
                        if let Some(link_def) =
                            parse_link_definition(id, after_colon, &lines, &mut i)
                        {
                            // Check for IAL on the next line(s)
                            let mut attrs = HashMap::new();
                            while i < lines.len() && is_block_ial(lines[i]) {
                                let ial_attrs = parse_ial(lines[i].trim());
                                for (k, v) in ial_attrs {
                                    if k == "class" {
                                        let entry = attrs.entry(k).or_insert_with(String::new);
                                        if !entry.is_empty() {
                                            entry.push(' ');
                                        }
                                        entry.push_str(&v);
                                    } else {
                                        attrs.insert(k, v);
                                    }
                                }
                                i += 1;
                            }
                            let key = id.to_lowercase();
                            let def = LinkDef {
                                url: link_def.0,
                                title: link_def.1,
                                attrs,
                            };
                            // Check for IAL before the link def
                            // (already handled by looking ahead)
                            ctx.link_defs.entry(key).or_insert(def);
                            continue;
                        }
                    }
                }
            }
        }

        // ALD (Attribute List Definition): {:name: attrs}
        if trimmed.starts_with("{:") && !trimmed.starts_with("{::") && !trimmed.starts_with("{:/") {
            // Check for ALD pattern: {:name: attrs} where name is followed by :
            if let Some(inner) = trimmed.strip_prefix("{:").and_then(|s| s.strip_suffix('}')) {
                // Find the ALD name (terminated by ':')
                if let Some(colon_pos) = inner.find(':') {
                    let name = inner[..colon_pos].trim();
                    let rest_attrs = inner[colon_pos + 1..].trim();
                    // Valid ALD name: no whitespace
                    if !name.is_empty() && !name.contains(' ') && !rest_attrs.is_empty() {
                        let attrs = parse_ial(&format!("{{: {rest_attrs}}}"));
                        // Merge with existing ALD if present
                        let entry = ctx.ald_defs.entry(name.to_string()).or_default();
                        entry.extend(attrs);
                        i += 1;
                        continue;
                    }
                }
            }
        }

        // Abbreviation definition: *[ABBR]: Full text
        if let Some(rest) = trimmed.strip_prefix("*[") {
            if let Some(close) = rest.find("]:") {
                let abbr = &rest[..close];
                let full = rest[close + 2..].trim();
                if !abbr.is_empty() {
                    ctx.abbreviations.insert(abbr.to_string(), full.to_string());
                    i += 1;
                    continue;
                }
            }
        }

        // Footnote definition: [^name]: content
        if let Some(rest) = trimmed.strip_prefix("[^") {
            if let Some(close) = rest.find("]:") {
                let name = &rest[..close];
                let after_colon = &rest[close + 2..];
                let content_start = if after_colon.starts_with(' ') {
                    &after_colon[1..] // strip single space after :
                } else {
                    after_colon
                };
                let mut content = content_start.to_string();

                // Collect continuation lines (indented by at least 4 spaces or tab)
                i += 1;
                while i < lines.len() {
                    let next = lines[i];
                    if next.trim().is_empty() {
                        // Blank line: look ahead to see if there are more indented lines
                        let mut look = i + 1;
                        while look < lines.len() && lines[look].trim().is_empty() {
                            look += 1;
                        }
                        if look < lines.len()
                            && (lines[look].starts_with("    ") || lines[look].starts_with('\t'))
                        {
                            // More content follows, include blank line(s)
                            while i < look {
                                content.push('\n');
                                i += 1;
                            }
                            continue;
                        }
                        break;
                    }
                    if next.starts_with("    ") || next.starts_with('\t') {
                        content.push('\n');
                        // Strip 4 spaces or 1 tab of indent
                        let stripped = if next.starts_with("    ") {
                            &next[4..]
                        } else if next.starts_with('\t') {
                            &next[1..]
                        } else {
                            next.trim_start()
                        };
                        content.push_str(stripped);
                        i += 1;
                    } else if next.starts_with("  ") {
                        // 2-3 space indent: also continuation
                        content.push('\n');
                        content.push_str(next.trim_start());
                        i += 1;
                    } else {
                        break;
                    }
                }

                ctx.footnote_defs
                    .insert(name.to_string(), content.trim_end().to_string());
                continue;
            }
        }

        output_lines.push(line);
        i += 1;
    }

    // Reconstruct text
    let mut result = output_lines.join("\n");
    if text.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn parse_link_definition(
    _id: &str,
    after_colon: &str,
    lines: &[&str],
    pos: &mut usize,
) -> Option<(String, Option<String>)> {
    if after_colon.is_empty() {
        *pos += 1;
        return None;
    }

    // URL can be in angle brackets
    let (url, rest) = if after_colon.starts_with('<') {
        if let Some(close) = after_colon[1..].find('>') {
            let url = &after_colon[1..=close];
            let rest = after_colon[close + 2..].trim_start();
            (url.to_string(), rest.to_string())
        } else {
            *pos += 1;
            return None;
        }
    } else {
        // URL is everything up to first whitespace or end of line
        let parts: Vec<&str> = after_colon
            .splitn(2, |c: char| c == ' ' || c == '\t')
            .collect();
        let url = parts[0].to_string();
        let rest = parts
            .get(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        (url, rest)
    };

    if url.is_empty() {
        *pos += 1;
        return None;
    }

    // Parse title from rest or continuation line
    let title = if rest.is_empty() {
        // Check next line for title
        if *pos + 1 < lines.len() {
            let next_trimmed = lines[*pos + 1].trim();
            if let Some(t) = extract_title(next_trimmed) {
                *pos += 2;
                Some(t)
            } else {
                *pos += 1;
                None
            }
        } else {
            *pos += 1;
            None
        }
    } else if let Some(t) = extract_title(&rest) {
        *pos += 1;
        Some(t)
    } else {
        *pos += 1;
        None
    };

    Some((url, title))
}

fn extract_title(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.rfind('"') {
            if end == rest.len() - 1 {
                return Some(rest[..end].to_string());
            }
        }
    }
    if let Some(rest) = s.strip_prefix('\'') {
        if let Some(end) = rest.rfind('\'') {
            if end == rest.len() - 1 {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn is_block_ial(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("{:")
        && trimmed.ends_with('}')
        && !trimmed.starts_with("{::")
        && !trimmed.starts_with("{:/")
}

/// Parse IAL attributes from a string like `{: #id .class key="value"}`.
/// Also handles `{:.cls1#id.cls2}` (no spaces between attributes).
pub fn parse_ial(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let inner = s
        .trim()
        .strip_prefix('{')
        .unwrap_or(s)
        .strip_prefix(':')
        .unwrap_or(s);
    let inner = inner.strip_suffix('}').unwrap_or(inner).trim();

    let mut chars = inner.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        match chars.peek() {
            Some('#') => {
                chars.next();
                // ID: take until whitespace, '.', '#', or '}'
                let mut id = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '.' || c == '#' || c == '}' {
                        break;
                    }
                    id.push(c);
                    chars.next();
                }
                if !id.is_empty() {
                    attrs.push(("id".to_string(), id));
                }
            }
            Some('.') => {
                chars.next();
                // Class: take until whitespace, '.', '#', or '}'
                let mut class = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '.' || c == '#' || c == '}' {
                        break;
                    }
                    class.push(c);
                    chars.next();
                }
                if !class.is_empty() {
                    attrs.push(("class".to_string(), class));
                }
            }
            _ => {
                // key="value" or key='value' or key=value or bare word (ignored)
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '=' || c.is_whitespace() || c == '.' || c == '#' || c == '}' {
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
                if key.is_empty() {
                    // Skip unknown character
                    chars.next();
                    continue;
                }
                // Check for = sign
                if chars.peek() == Some(&'=') {
                    chars.next(); // consume =
                    let value = if chars.peek() == Some(&'"') {
                        chars.next();
                        // Read until unescaped closing "
                        let mut v = String::new();
                        while let Some(c) = chars.next() {
                            if c == '\\' {
                                if let Some(&next) = chars.peek() {
                                    if next == '"' || next == '\\' {
                                        v.push(next);
                                        chars.next();
                                        continue;
                                    }
                                }
                                v.push('\\');
                            } else if c == '"' {
                                break;
                            } else {
                                v.push(c);
                            }
                        }
                        v
                    } else if chars.peek() == Some(&'\'') {
                        chars.next();
                        let mut v = String::new();
                        while let Some(c) = chars.next() {
                            if c == '\\' {
                                if let Some(&next) = chars.peek() {
                                    if next == '\'' || next == '\\' {
                                        v.push(next);
                                        chars.next();
                                        continue;
                                    }
                                }
                                v.push('\\');
                            } else if c == '\'' {
                                break;
                            } else {
                                v.push(c);
                            }
                        }
                        v
                    } else {
                        let mut v = String::new();
                        while let Some(&c) = chars.peek() {
                            if c.is_whitespace() || c == '}' {
                                break;
                            }
                            v.push(c);
                            chars.next();
                        }
                        v
                    };
                    attrs.push((key.trim().to_string(), value));
                }
                // else: bare word without =, skip/ignore (ALD reference, handled elsewhere)
            }
        }
    }
    attrs
}

/// Convert inline text to HTML, processing all span elements.
pub fn spans_to_html(text: &str, ctx: &mut SpanContext) -> String {
    // Pre-process CJK line breaks if enabled
    let processed_text = if ctx.options.remove_line_breaks_for_cjk {
        remove_cjk_line_breaks(text)
    } else {
        text.to_string()
    };

    let mut result = String::with_capacity(processed_text.len() * 2);
    let chars: Vec<char> = processed_text.chars().collect();
    parse_spans(&chars, 0, chars.len(), ctx, &mut result, false);

    // Apply abbreviations
    if !ctx.abbreviations.is_empty() {
        result = apply_abbreviations(&result, &ctx.abbreviations);
    }

    result
}

/// Remove line breaks between CJK characters.
fn remove_cjk_line_breaks(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\n' {
            // Check if preceding and following chars are CJK
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                None
            };
            if let (Some(p), Some(n)) = (prev, next) {
                if is_cjk_char(p) && is_cjk_char(n) {
                    // Remove the newline (join CJK chars directly)
                    i += 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Check if a character is a CJK character.
fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
    // CJK Unified Ideographs Extension A
    || (0x3400..=0x4DBF).contains(&cp)
    // CJK Unified Ideographs Extension B
    || (0x20000..=0x2A6DF).contains(&cp)
    // CJK Compatibility Ideographs
    || (0xF900..=0xFAFF).contains(&cp)
    // Hiragana
    || (0x3040..=0x309F).contains(&cp)
    // Katakana
    || (0x30A0..=0x30FF).contains(&cp)
    // Hangul Syllables
    || (0xAC00..=0xD7AF).contains(&cp)
    // CJK Symbols and Punctuation
    || (0x3000..=0x303F).contains(&cp)
    // Fullwidth forms
    || (0xFF00..=0xFFEF).contains(&cp)
}

fn parse_spans(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
    output: &mut String,
    in_link: bool,
) {
    let mut i = start;
    while i < end {
        // Backslash escape
        if chars[i] == '\\' && i + 1 < end {
            let next = chars[i + 1];
            // \\ followed by optional spaces and then \n => line break
            if next == '\\' {
                // Check what follows the \\
                let mut j = i + 2;
                let mut space_count = 0;
                while j < end && chars[j] == ' ' {
                    space_count += 1;
                    j += 1;
                }
                if j < end && chars[j] == '\n' {
                    if space_count > 0 {
                        // \\ + spaces + \n => literal \ then <br /> (spaces trigger the break)
                        output.push_str("\\ <br />\n");
                    } else {
                        // \\ + \n => <br /> (\\ = escaped \, then \ at end of line = line break)
                        output.push_str("<br />\n");
                    }
                    i = j + 1;
                    continue;
                } else if j >= end {
                    // \\ at end of text (no newline after) => just output backslash
                    output.push('\\');
                    i += 2;
                    continue;
                }
            }
            if is_escapable_char(next) {
                output.push_str(&escape_html_char(next));
                i += 2;
                continue;
            }
        }

        // {::comment}...{:/comment} inline
        if chars[i] == '{' && i + 2 < end && chars[i + 1] == ':' && chars[i + 2] == ':' {
            let remaining: String = chars[i..end].iter().collect();
            if remaining.starts_with("{::comment}") {
                // Find the earliest close tag: {:/comment} or {:/}
                let close_comment = remaining.find("{:/comment}");
                let close_short = remaining.find("{:/}");
                let (close_pos, close_len) = match (close_comment, close_short) {
                    (Some(a), Some(b)) => {
                        if b <= a {
                            (Some(b), "{:/}".len())
                        } else {
                            (Some(a), "{:/comment}".len())
                        }
                    }
                    (Some(a), None) => (Some(a), "{:/comment}".len()),
                    (None, Some(b)) => (Some(b), "{:/}".len()),
                    (None, None) => (None, 0),
                };
                if let Some(end_pos) = close_pos {
                    let comment_content = &remaining["{::comment}".len()..end_pos];
                    output.push_str("<!-- ");
                    output.push_str(comment_content);
                    output.push_str(" -->");
                    i += end_pos + close_len;
                    continue;
                }
            }
            if remaining.starts_with("{::nomarkdown}") {
                if let Some(end_pos) = remaining.find("{:/nomarkdown}") {
                    let content = &remaining["{::nomarkdown}".len()..end_pos];
                    output.push_str(content);
                    i += end_pos + "{:/nomarkdown}".len();
                    continue;
                } else if let Some(end_pos) = remaining.find("{:/}") {
                    let content = &remaining["{::nomarkdown}".len()..end_pos];
                    output.push_str(content);
                    i += end_pos + "{:/}".len();
                    continue;
                }
            }
            // Self-closing comment/nomarkdown: {::comment/} or {::nomarkdown/}
            if remaining.starts_with("{::comment/}") {
                i += "{::comment/}".len();
                continue;
            }
            if remaining.starts_with("{::nomarkdown/}") {
                i += "{::nomarkdown/}".len();
                continue;
            }

            if remaining.starts_with("{::options") {
                // Skip options extensions inline
                if let Some(end_pos) = remaining.find("/}") {
                    i += end_pos + 2;
                    continue;
                }
            }
        }

        // Inline math: $$...$$
        if chars[i] == '$' && i + 1 < end && chars[i + 1] == '$' {
            // Check for escaped dollar: \$ before
            if i > 0 && chars[i - 1] == '\\' {
                // The backslash was already output (oops), we need to handle this differently
                // Actually, backslash-dollar is handled above in escape section
                // If we get here, it's not escaped
            }
            // Check that the $$ is not preceded by a backslash that was already consumed
            if let Some((math_content, advance)) = try_parse_inline_math(chars, i, end) {
                if let Some(ref _engine) = ctx.options.math_engine {
                    output.push_str("\\(");
                    output.push_str(&escape_html_str(&math_content));
                    output.push_str("\\)");
                } else {
                    output.push_str("<span class=\"kdmath\">$");
                    output.push_str(&escape_html_str(&math_content));
                    output.push_str("$</span>");
                }
                i += advance;
                continue;
            }
        }

        // Backtick code span
        if chars[i] == '`' {
            if let Some((content, advance)) = try_parse_code_span(chars, i, end) {
                let highlighter_class = if ctx.options.syntax_highlighter.is_some()
                    && ctx.options.syntax_highlighter_opts.guess_lang == Some(true)
                {
                    " class=\"highlighter-rouge\""
                } else {
                    ""
                };
                output.push_str(&format!("<code{highlighter_class}>"));
                output.push_str(&escape_html_str(&content));
                output.push_str("</code>");
                // Check for IAL(s) after code span
                let mut after = i + advance;
                let mut all_ial_attrs: Vec<(String, String)> = Vec::new();
                while let Some((ial_attrs, ial_len)) = try_parse_span_ial(chars, after, end) {
                    all_ial_attrs.extend(ial_attrs);
                    after += ial_len;
                }
                if !all_ial_attrs.is_empty() {
                    // Find the last <code tag in output and merge attributes
                    if let Some(pos) = output.rfind("<code") {
                        let tag_end = output[pos..].find('>').unwrap_or(0) + pos;
                        let existing_tag = &output[pos..=tag_end];
                        let new_tag = merge_code_tag_attrs(existing_tag, &all_ial_attrs);
                        let rest = output[tag_end + 1..].to_string();
                        output.truncate(pos);
                        output.push_str(&new_tag);
                        output.push_str(&rest);
                    }
                    i = after;
                } else {
                    i += advance;
                }
                continue;
            }
            // Not a valid code span, output literal backtick
            output.push('`');
            i += 1;
            continue;
        }

        // HTML comment: <!-- ... -->
        if chars[i] == '<'
            && i + 3 < end
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            let remaining: String = chars[i..end].iter().collect();
            if let Some(close) = remaining.find("-->") {
                let comment = &remaining[..close + 3];
                output.push_str(comment);
                i += close + 3;
                continue;
            }
        }

        // Autolinks: <url> or <email>
        if chars[i] == '<' {
            if let Some((link_html, advance)) = try_parse_autolink(chars, i, end, ctx) {
                output.push_str(&link_html);
                i += advance;
                continue;
            }
        }

        // HTML span elements
        if chars[i] == '<' {
            if let Some((html, advance)) = try_parse_html_span(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // Image link: ![alt](url) or ![alt][ref]
        if chars[i] == '!' && i + 1 < end && chars[i + 1] == '[' && !in_link {
            if let Some((html, advance)) = try_parse_image(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // Link: [text](url) or [text][ref] or [ref]
        if chars[i] == '[' && !in_link {
            if let Some((html, advance)) = try_parse_link(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // Emphasis: *, **, ***, _, __, ___
        if (chars[i] == '*' || chars[i] == '_') && i < end {
            if let Some((html, advance)) = try_parse_emphasis(chars, i, end, ctx, in_link) {
                output.push_str(&html);
                let mut after = i + advance;
                // Check for IAL after emphasis
                let mut all_ial_attrs: Vec<(String, String)> = Vec::new();
                while let Some((ial_attrs, ial_len)) = try_parse_span_ial(chars, after, end) {
                    all_ial_attrs.extend(ial_attrs);
                    after += ial_len;
                }
                if !all_ial_attrs.is_empty() {
                    // Apply IAL to the em/strong tag
                    let attrs_str = format_attrs(&all_ial_attrs);
                    // Find the last <em or <strong in output and add attrs
                    if let Some(em_pos) = output.rfind("<em>") {
                        let rest = output[em_pos + 4..].to_string();
                        output.truncate(em_pos);
                        output.push_str(&format!("<em{attrs_str}>"));
                        output.push_str(&rest);
                    } else if let Some(strong_pos) = output.rfind("<strong>") {
                        let rest = output[strong_pos + 8..].to_string();
                        output.truncate(strong_pos);
                        output.push_str(&format!("<strong{attrs_str}>"));
                        output.push_str(&rest);
                    }
                    i = after;
                } else {
                    i += advance;
                }
                continue;
            }
        }

        // Smart quotes and typography
        if chars[i] == '"' || chars[i] == '\'' {
            if let Some((html, advance)) = try_parse_smart_quote(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // Guillemets: << >> (French quotes)
        if chars[i] == '<' && i + 1 < end && chars[i + 1] == '<' {
            let (html, advance) = parse_guillemet_open(chars, i, end, ctx);
            output.push_str(&html);
            i += advance;
            continue;
        }
        if chars[i] == '>' && i + 1 < end && chars[i + 1] == '>' {
            // Check if preceding char in output is a space that should become &nbsp;
            let space_before = output.ends_with(' ');
            if space_before {
                output.pop(); // remove the trailing space
            }
            let (html, advance) = parse_guillemet_close(chars, i, end, ctx);
            output.push_str(&html);
            i += advance;
            continue;
        }

        // Dashes: --- -> em-dash, -- -> en-dash
        if chars[i] == '-' {
            let dash_start = i;
            while i < end && chars[i] == '-' {
                i += 1;
            }
            let count = i - dash_start;
            let em_dashes = count / 3;
            let remaining = count % 3;
            let en_dashes = remaining / 2;
            let single = remaining % 2;
            for _ in 0..em_dashes {
                output.push_str(&entity_str("mdash", ctx));
            }
            for _ in 0..en_dashes {
                output.push_str(&entity_str("ndash", ctx));
            }
            for _ in 0..single {
                output.push('-');
            }
            continue;
        }

        // Ellipsis: ...
        if chars[i] == '.' && i + 2 < end && chars[i + 1] == '.' && chars[i + 2] == '.' {
            output.push_str(&entity_str("hellip", ctx));
            i += 3;
            continue;
        }

        // HTML entities: &name; &#num; &#xhex;
        if chars[i] == '&' {
            if let Some((html, advance)) = try_parse_entity(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
            // Bare & - escape it
            output.push_str("&amp;");
            i += 1;
            continue;
        }

        // Line break: trailing spaces before \n
        if chars[i] == ' ' {
            let space_start = i;
            while i < end && chars[i] == ' ' {
                i += 1;
            }
            let space_count = i - space_start;
            if i < end && chars[i] == '\n' && space_count >= 2 {
                output.push_str("<br />\n");
                i += 1; // skip the \n
                continue;
            }
            // Just regular spaces
            for _ in 0..space_count {
                output.push(' ');
            }
            continue;
        }

        // Footnote reference: [^name]
        if chars[i] == '[' && i + 1 < end && chars[i + 1] == '^' {
            if let Some((html, advance)) = try_parse_footnote_ref(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // < and > that aren't autolinks/HTML - escape them
        if chars[i] == '<' {
            output.push_str("&lt;");
            i += 1;
            continue;
        }
        if chars[i] == '>' {
            output.push_str("&gt;");
            i += 1;
            continue;
        }

        // Regular character
        output.push(chars[i]);
        i += 1;
    }
}

/// Check if a character can be escaped with backslash in kramdown.
fn is_escapable_char(c: char) -> bool {
    matches!(
        c,
        '\\' | '`'
            | '*'
            | '_'
            | '{'
            | '}'
            | '['
            | ']'
            | '('
            | ')'
            | '#'
            | '+'
            | '-'
            | '.'
            | '!'
            | '|'
            | '~'
            | '^'
            | '>'
            | '<'
            | '/'
            | '='
            | ':'
            | '"'
            | '\''
            | '$'
    )
}

fn escape_html_char(c: char) -> String {
    match c {
        '&' => "&amp;".to_string(),
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        _ => c.to_string(),
    }
}

fn escape_html_str(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape a URL for autolinks, preserving existing HTML entities.
/// For autolinks like `<http://...>`, entities in the URL are preserved as-is.
/// Only bare `&` (not part of an entity) is escaped.
fn escape_autolink_url(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '&' {
            // Check if this starts an HTML entity
            let mut j = i + 1;
            if j < chars.len() && chars[j] == '#' {
                // Numeric entity
                j += 1;
                if j < chars.len() && (chars[j] == 'x' || chars[j] == 'X') {
                    j += 1;
                    while j < chars.len() && chars[j].is_ascii_hexdigit() {
                        j += 1;
                    }
                } else {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                }
                if j < chars.len() && chars[j] == ';' {
                    // Valid entity - preserve it
                    let entity: String = chars[i..=j].iter().collect();
                    result.push_str(&entity);
                    i = j + 1;
                    continue;
                }
            } else {
                // Named entity
                while j < chars.len() && chars[j].is_ascii_alphanumeric() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == ';' && j > i + 1 {
                    // Valid named entity - preserve it
                    let entity: String = chars[i..=j].iter().collect();
                    result.push_str(&entity);
                    i = j + 1;
                    continue;
                }
            }
            // Bare & - escape it
            result.push_str("&amp;");
            i += 1;
        } else if chars[i] == '<' {
            result.push_str("&lt;");
            i += 1;
        } else if chars[i] == '>' {
            result.push_str("&gt;");
            i += 1;
        } else if chars[i] == '"' {
            result.push_str("&quot;");
            i += 1;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// Merge IAL attributes into an existing <code ...> tag string.
fn merge_code_tag_attrs(existing_tag: &str, new_attrs: &[(String, String)]) -> String {
    // Extract existing class from tag like <code class="highlighter-rouge">
    let mut classes = Vec::new();
    let mut id = None;
    let mut others: Vec<(String, String)> = Vec::new();

    // Parse existing attributes from tag
    if let Some(class_start) = existing_tag.find("class=\"") {
        let after = &existing_tag[class_start + 7..];
        if let Some(class_end) = after.find('"') {
            let existing_class = &after[..class_end];
            for c in existing_class.split_whitespace() {
                classes.push(c.to_string());
            }
        }
    }

    // Add new attributes
    for (k, v) in new_attrs {
        match k.as_str() {
            "class" => classes.push(v.clone()),
            "id" => id = Some(v.clone()),
            _ => others.push((k.clone(), v.clone())),
        }
    }

    let mut result = "<code".to_string();
    if !classes.is_empty() {
        result.push_str(&format!(" class=\"{}\"", classes.join(" ")));
    }
    if let Some(id_val) = id {
        result.push_str(&format!(" id=\"{id_val}\""));
    }
    for (k, v) in &others {
        result.push_str(&format!(" {k}=\"{v}\""));
    }
    result.push('>');
    result
}

/// Format attributes for HTML output
fn format_attrs(attrs: &[(String, String)]) -> String {
    let mut result = String::new();
    // Write class first, then id, then rest (kramdown order)
    let mut id = None;
    let mut classes = Vec::new();
    let mut others: Vec<(&str, &str)> = Vec::new();

    for (k, v) in attrs {
        match k.as_str() {
            "id" => id = Some(v.as_str()),
            "class" => classes.push(v.as_str()),
            _ => others.push((k.as_str(), v.as_str())),
        }
    }

    if !classes.is_empty() {
        result.push_str(&format!(" class=\"{}\"", classes.join(" ")));
    }
    if let Some(id_val) = id {
        result.push_str(&format!(" id=\"{id_val}\""));
    }
    for (k, v) in others {
        result.push_str(&format!(" {k}=\"{v}\""));
    }
    result
}

/// Output an entity based on the entity_output option and typographic_symbols overrides.
fn entity_str(name: &str, ctx: &SpanContext) -> String {
    // Check typographic_symbols override first
    if let Some(override_val) = ctx.options.typographic_symbols.get(name) {
        return escape_html_str(override_val);
    }

    // Also check with _space suffix for laquo_space, raquo_space
    match ctx.options.entity_output {
        EntityOutput::AsChar => {
            if let Some(ch) = entities::resolve_named_entity(name) {
                ch.to_string()
            } else {
                format!("&{name};")
            }
        }
        EntityOutput::Symbolic => format!("&{name};"),
        EntityOutput::Numeric => {
            if let Some(ch) = entities::resolve_named_entity(name) {
                let cp = ch.chars().next().unwrap_or_default() as u32;
                format!("&#{cp};")
            } else {
                format!("&{name};")
            }
        }
        EntityOutput::AsInput => format!("&{name};"),
    }
}

/// Try to parse a code span starting at position i (which must be a backtick).
fn try_parse_code_span(chars: &[char], start: usize, end: usize) -> Option<(String, usize)> {
    let mut bt = 0;
    let mut i = start;
    while i < end && chars[i] == '`' {
        bt += 1;
        i += 1;
    }

    if bt == 0 {
        return None;
    }

    // For single backtick: if followed by space or end, it's NOT a code span
    if bt == 1 && (i >= end || chars[i] == ' ' || chars[i] == '\n') {
        return None;
    }

    let content_start = i;

    // Find matching closing backticks
    while i < end {
        if chars[i] == '`' {
            let mut close_bt = 0;
            let close_start = i;
            while i < end && chars[i] == '`' {
                close_bt += 1;
                i += 1;
            }
            if close_bt == bt {
                let content: String = chars[content_start..close_start].iter().collect();
                // Trim single leading/trailing space if both present and content isn't all spaces
                let trimmed = if content.len() >= 2
                    && content.starts_with(' ')
                    && content.ends_with(' ')
                    && !content.trim().is_empty()
                {
                    &content[1..content.len() - 1]
                } else if bt > 1 {
                    // For multi-backtick: trim leading/trailing space
                    content.trim()
                } else {
                    &content
                };
                return Some((trimmed.to_string(), i - start));
            }
        } else {
            i += 1;
        }
    }

    None
}

/// Try to parse a span-level IAL: {: .class #id key="value"}
fn try_parse_span_ial(
    chars: &[char],
    start: usize,
    end: usize,
) -> Option<(Vec<(String, String)>, usize)> {
    if start >= end || chars[start] != '{' {
        return None;
    }
    if start + 1 >= end || chars[start + 1] != ':' {
        return None;
    }
    // Must not be {:: (extension)
    if start + 2 < end && chars[start + 2] == ':' {
        return None;
    }

    // Find closing }
    let mut i = start + 2;
    while i < end && chars[i] != '}' {
        i += 1;
    }
    if i >= end {
        return None;
    }

    let ial_str: String = chars[start..=i].iter().collect();
    let attrs = parse_ial(&ial_str);
    if attrs.is_empty() {
        return None;
    }

    Some((attrs, i + 1 - start))
}

/// Try to parse inline math: $$...$$
fn try_parse_inline_math(chars: &[char], start: usize, end: usize) -> Option<(String, usize)> {
    if start + 1 >= end || chars[start] != '$' || chars[start + 1] != '$' {
        return None;
    }

    let content_start = start + 2;
    let mut i = content_start;

    // Find closing $$
    while i + 1 < end {
        if chars[i] == '$' && chars[i + 1] == '$' {
            let content: String = chars[content_start..i].iter().collect();
            if content.is_empty() {
                return None;
            }
            return Some((content, i + 2 - start));
        }
        i += 1;
    }

    None
}

/// Try to parse an autolink: <url> or <email>
fn try_parse_autolink(
    chars: &[char],
    start: usize,
    end: usize,
    _ctx: &SpanContext,
) -> Option<(String, usize)> {
    if chars[start] != '<' {
        return None;
    }

    // Find closing >
    let mut i = start + 1;
    while i < end && chars[i] != '>' && chars[i] != '\n' {
        i += 1;
    }
    if i >= end || chars[i] != '>' {
        return None;
    }

    let content: String = chars[start + 1..i].iter().collect();

    // URL autolink: starts with a scheme like http:// https:// ftp:// mailto:
    if content.starts_with("http://")
        || content.starts_with("https://")
        || content.starts_with("ftp://")
    {
        let escaped_url = escape_autolink_url(&content);
        let display = escape_autolink_url(&content);
        return Some((
            format!("<a href=\"{escaped_url}\">{display}</a>"),
            i + 1 - start,
        ));
    }

    // mailto: autolink
    if let Some(addr) = content.strip_prefix("mailto:") {
        let escaped_url = escape_html_attr(&content);
        let display_addr = escape_html_str(addr);
        return Some((
            format!("<a href=\"{escaped_url}\">{display_addr}</a>"),
            i + 1 - start,
        ));
    }

    // Email autolink: something@something.something
    // Must not contain spaces, quotes, or '=' (which indicate HTML tag attributes)
    if content.contains('@')
        && !content.starts_with('[')
        && !content.contains(' ')
        && !content.contains('"')
        && !content.contains('\'')
        && !content.contains('=')
    {
        // Validate email-like pattern
        let parts: Vec<&str> = content.splitn(2, '@').collect();
        if parts.len() == 2
            && !parts[0].is_empty()
            && !parts[1].is_empty()
            && parts[1].contains('.')
        {
            let display = escape_html_str(&content);
            return Some((
                format!("<a href=\"mailto:{content}\">{display}</a>"),
                i + 1 - start,
            ));
        }
        // Less strict: just check for @
        if !content.contains(' ') && !content.contains('<') {
            let parts: Vec<&str> = content.splitn(2, '@').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                let display = escape_html_str(&content);
                return Some((
                    format!("<a href=\"mailto:{content}\">{display}</a>"),
                    i + 1 - start,
                ));
            }
        }
    }

    None
}

#[allow(dead_code)]
fn apply_abbreviations_inline(text: &str, abbreviations: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (abbr, full) in abbreviations {
        if result.contains(abbr.as_str()) {
            let replacement = format!("<abbr title=\"{full}\">{abbr}</abbr>");
            result = replace_whole_word(&result, abbr, &replacement);
        }
    }
    result
}

/// Try to parse an HTML span element.
fn try_parse_html_span(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
) -> Option<(String, usize)> {
    if chars[start] != '<' {
        return None;
    }

    let remaining: String = chars[start..end].iter().collect();

    // Processing instruction: <? ... ?>
    if remaining.starts_with("<?") {
        // PIs are output as escaped text in span context
        return None; // Let < be escaped
    }

    // Closing tag: </tag>
    if remaining.starts_with("</") {
        if let Some(gt) = remaining.find('>') {
            let tag_content = &remaining[2..gt];
            let tag_name = tag_content
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();
            // Check if it's a valid inline HTML tag
            if is_valid_span_tag(&tag_name) {
                return None; // Invalid closing tag without opening - let < be escaped
            }
        }
        return None;
    }

    // Opening tag: <tag ...> or self-closing <tag ... />
    if let Some(gt) = remaining.find('>') {
        let tag_str = &remaining[..=gt];
        let inner = &remaining[1..gt];

        // Extract tag name
        let tag_name_end = inner
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(inner.len());
        let tag_name = inner[..tag_name_end].to_lowercase();

        if tag_name.is_empty() {
            return None;
        }

        // Must be a valid HTML tag
        if !is_valid_span_tag(&tag_name) && !is_valid_block_tag(&tag_name) {
            return None;
        }

        // Block-level tags in span context are escaped
        if is_block_level_tag(&tag_name) && !is_also_span_tag(&tag_name) {
            return None;
        }

        let is_self_closing = inner.trim_end().ends_with('/');
        let is_void = is_void_element(&tag_name);

        // Normalize HTML attributes
        let normalized = normalize_html_tag(tag_str);

        if is_self_closing || is_void {
            // Self-closing: <br />, <img ... />
            let normalized = ensure_self_closing(&normalized, &tag_name);
            return Some((normalized, gt + 1));
        }

        // Raw content tags: <script>, <style> - content not parsed
        if is_raw_content_tag(&tag_name) {
            let close_tag = format!("</{}>", tag_name);
            let _close_tag_upper = format!("</{}>", tag_name.to_uppercase());
            // Search case-insensitively
            let search_from = start + gt + 1;
            let rest: String = chars[search_from..end].iter().collect();
            let close_pos = find_case_insensitive(&rest, &close_tag);
            if let Some(cp) = close_pos {
                let content = &rest[..cp];
                let _actual_close = &rest[cp..cp + close_tag.len()];
                let total = gt + 1 + cp + close_tag.len();
                return Some((format!("{normalized}{content}</{}>", tag_name), total));
            }
            // No closing tag found
            return Some((normalized, gt + 1));
        }

        // Regular inline tag: <span>, <em>, <strong>, etc.
        // Find the matching closing tag
        let close_tag_lower = format!("</{}>", tag_name);
        let search_start = start + gt + 1;

        // For inline tags, find the closing tag and process content between
        let rest: String = chars[search_start..end].iter().collect();
        let close_pos = find_case_insensitive(&rest, &close_tag_lower);

        if let Some(cp) = close_pos {
            // Found closing tag - process inner content as spans
            let inner_chars: Vec<char> = rest[..cp].chars().collect();
            let mut inner_html = String::new();

            // For markdown-processable tags, process inner content
            if is_markdown_processable_tag(&tag_name) {
                parse_spans(
                    &inner_chars,
                    0,
                    inner_chars.len(),
                    ctx,
                    &mut inner_html,
                    false,
                );
            } else {
                inner_html = rest[..cp].to_string();
            }

            let total_advance = gt + 1 + cp + close_tag_lower.len();
            // Also handle case-insensitive close
            return Some((
                format!("{normalized}{inner_html}</{tag_name}>"),
                total_advance,
            ));
        }

        // No closing tag - auto-close at end of content
        // Process remaining content and wrap with the tag
        let rest_content: String = chars[search_start..end].iter().collect();
        if is_markdown_processable_tag(&tag_name) {
            let inner_chars: Vec<char> = rest_content.chars().collect();
            let mut inner_html = String::new();
            parse_spans(
                &inner_chars,
                0,
                inner_chars.len(),
                ctx,
                &mut inner_html,
                false,
            );
            let total_advance = end - start;
            return Some((
                format!("{normalized}{inner_html}</{tag_name}>"),
                total_advance,
            ));
        }

        let total_advance = end - start;
        return Some((
            format!("{normalized}{rest_content}</{tag_name}>"),
            total_advance,
        ));
    }

    None
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let needle_lower = needle.to_lowercase();
    let haystack_lower = haystack.to_lowercase();
    haystack_lower.find(&needle_lower)
}

fn is_valid_span_tag(tag: &str) -> bool {
    matches!(
        tag,
        "a" | "abbr"
            | "acronym"
            | "b"
            | "bdi"
            | "bdo"
            | "big"
            | "br"
            | "button"
            | "cite"
            | "code"
            | "del"
            | "dfn"
            | "em"
            | "i"
            | "iframe"
            | "img"
            | "input"
            | "ins"
            | "kbd"
            | "label"
            | "mark"
            | "map"
            | "object"
            | "output"
            | "q"
            | "ruby"
            | "s"
            | "samp"
            | "script"
            | "select"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "textarea"
            | "time"
            | "tt"
            | "u"
            | "var"
            | "wbr"
    )
}

fn is_valid_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "dialog"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "style"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

fn is_block_level_tag(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "dialog"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

fn is_also_span_tag(_tag: &str) -> bool {
    // Tags that can appear both as block and span
    false
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
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
    )
}

fn is_raw_content_tag(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "math")
}

fn is_markdown_processable_tag(tag: &str) -> bool {
    // Tags whose inner content should be processed for markdown
    matches!(
        tag,
        "a" | "abbr"
            | "b"
            | "bdi"
            | "bdo"
            | "cite"
            | "del"
            | "dfn"
            | "em"
            | "i"
            | "ins"
            | "kbd"
            | "label"
            | "mark"
            | "q"
            | "s"
            | "samp"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "time"
            | "u"
            | "var"
    )
}

fn normalize_html_tag(tag: &str) -> String {
    // Parse and normalize HTML tag attributes
    // Convert attribute names to lowercase, normalize boolean attributes
    if !tag.starts_with('<') || !tag.ends_with('>') {
        return tag.to_string();
    }

    let inner = &tag[1..tag.len() - 1];
    let is_self_closing = inner.ends_with('/');
    let inner = if is_self_closing {
        inner[..inner.len() - 1].trim()
    } else {
        inner.trim()
    };

    // Split into tag name and attributes
    let parts: Vec<&str> = inner.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0].to_lowercase();
    let attr_str = if parts.len() > 1 { parts[1] } else { "" };

    if attr_str.is_empty() {
        if is_self_closing {
            return format!("<{tag_name} />");
        }
        return format!("<{tag_name}>");
    }

    // Parse attributes
    let attrs = parse_html_attrs(attr_str);
    let mut result = format!("<{tag_name}");
    for (name, value) in &attrs {
        let name_lower = name.to_lowercase();
        if let Some(val) = value {
            result.push_str(&format!(" {name_lower}=\"{val}\""));
        } else {
            // Boolean attribute
            result.push_str(&format!(" {name_lower}=\"\""));
        }
    }

    if is_self_closing {
        result.push_str(" />");
    } else {
        result.push('>');
    }
    result
}

fn parse_html_attrs(s: &str) -> Vec<(String, Option<String>)> {
    let mut attrs = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Check for / at end (self-closing)
        if chars[i] == '/' {
            break;
        }

        // Read attribute name
        let name_start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '/' {
            i += 1;
        }
        let name: String = chars[name_start..i].iter().collect();
        if name.is_empty() {
            break;
        }

        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        // Check for =
        if i < chars.len() && chars[i] == '=' {
            i += 1;
            // Skip whitespace
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            // Read value
            if i < chars.len() && (chars[i] == '"' || chars[i] == '\'') {
                let quote = chars[i];
                i += 1;
                let val_start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                let val: String = chars[val_start..i].iter().collect();
                if i < chars.len() {
                    i += 1; // skip closing quote
                }
                attrs.push((name, Some(val)));
            } else {
                // Unquoted value
                let val_start = i;
                while i < chars.len() && !chars[i].is_whitespace() {
                    i += 1;
                }
                let val: String = chars[val_start..i].iter().collect();
                attrs.push((name, Some(val)));
            }
        } else {
            // Boolean attribute
            attrs.push((name, None));
        }
    }

    attrs
}

fn ensure_self_closing(tag: &str, tag_name: &str) -> String {
    // Ensure void elements are self-closing with space before />
    if tag.ends_with("/>") && !tag.ends_with(" />") {
        let without = &tag[..tag.len() - 2];
        return format!("{without} />");
    }
    if tag.ends_with('>') && !tag.ends_with("/>") && is_void_element(tag_name) {
        let without = &tag[..tag.len() - 1];
        return format!("{without} />");
    }
    tag.to_string()
}

/// Try to parse emphasis/strong.
fn try_parse_emphasis(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
    in_link: bool,
) -> Option<(String, usize)> {
    let marker = chars[start];

    // Count consecutive markers
    let mut marker_count = 0;
    let mut i = start;
    while i < end && chars[i] == marker {
        marker_count += 1;
        i += 1;
    }

    if marker_count == 0 {
        return None;
    }

    // Opening delimiter rules for kramdown:
    // For * markers: must not be followed by a space/newline (left-flanking)
    // For _ markers: must not be followed by a space/newline, AND
    //               must not be preceded by an alphanumeric char (word boundary rule)
    if i >= end || chars[i] == ' ' || chars[i] == '\n' || chars[i] == '\t' {
        return None; // Not left-flanking
    }

    // For underscore: check word-boundary rule
    if marker == '_' {
        // _ can't open emphasis if preceded by alphanumeric (intra-word)
        if start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            return None;
        }
    }

    // For * marker: space before means not emphasis (e.g., "lonely * here")
    if marker == '*' {
        if start > 0 && chars[start - 1] == ' ' && marker_count <= 2 {
            // Check if this could still open emphasis
            // "* text*" - space before single * with space after is not emphasis
            // But "*text*" is emphasis
            // The rule: opening * must not have space after
            // We already checked that. But "lonely * here*" where space is before * is not emphasis
            // Actually in kramdown, `* here*` IS not emphasis when preceded by space and followed by space
            // Let's check: if preceded by space AND the marker_count is exactly what we have, it's a literal
            if chars[i] != ' ' {
                // Could still be valid: preceded by space but no space after
                // Let's try to find closing
            } else {
                return None;
            }
        }
    }

    // Try to find closing delimiter(s)
    // For ***: try strong+em (*) then em+strong (**), etc.
    if marker_count >= 3 {
        // Try: ***text*** -> <strong><em>text</em></strong>
        if let Some((inner, close_len, advance)) = find_emphasis_close(chars, i, end, marker, 3) {
            if close_len >= 3 {
                let mut inner_html = String::new();
                parse_spans(chars, i, i + inner, ctx, &mut inner_html, in_link);
                return Some((
                    format!("<strong><em>{inner_html}</em></strong>"),
                    advance + marker_count,
                ));
            }
        }
        // Try: ***text* text** -> <strong><em>text</em> text</strong>
        // Parse as em inside strong
        if let Some(result) =
            try_combined_emphasis(chars, start, end, marker, marker_count, ctx, in_link)
        {
            return Some(result);
        }
    }

    if marker_count >= 2 {
        // Try: **text** -> <strong>text</strong>
        if let Some((inner_end, advance)) = find_closing_delimiter(chars, i, end, marker, 2) {
            let mut inner_html = String::new();
            parse_spans(chars, i, inner_end, ctx, &mut inner_html, in_link);
            return Some((format!("<strong>{inner_html}</strong>"), advance + 2));
        }
    }

    if marker_count >= 1 {
        // Try: *text* -> <em>text</em>
        if let Some((inner_end, advance)) = find_closing_delimiter(chars, i, end, marker, 1) {
            let mut inner_html = String::new();
            parse_spans(chars, i, inner_end, ctx, &mut inner_html, in_link);
            return Some((format!("<em>{inner_html}</em>"), advance + 1));
        }
    }

    None
}

/// Find a closing delimiter for emphasis.
fn find_closing_delimiter(
    chars: &[char],
    start: usize,
    end: usize,
    marker: char,
    count: usize,
) -> Option<(usize, usize)> {
    let mut i = start;
    let _depth_stack: Vec<(usize, usize)> = Vec::new(); // (position, marker_count)

    while i < end {
        if chars[i] == '\\' && i + 1 < end {
            i += 2; // skip escaped char
            continue;
        }

        if chars[i] == '`' {
            // Skip code spans
            let mut bt = 0;
            while i < end && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            // Find closing backticks
            let mut found = false;
            while i < end {
                if chars[i] == '`' {
                    let mut close_bt = 0;
                    while i < end && chars[i] == '`' {
                        close_bt += 1;
                        i += 1;
                    }
                    if close_bt == bt {
                        found = true;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if !found {
                // Unclosed code span, just continue
            }
            continue;
        }

        if chars[i] == '[' {
            // Skip link/image contents
            let mut bracket_depth = 1;
            i += 1;
            while i < end && bracket_depth > 0 {
                if chars[i] == '\\' && i + 1 < end {
                    i += 2;
                    continue;
                }
                if chars[i] == '[' {
                    bracket_depth += 1;
                } else if chars[i] == ']' {
                    bracket_depth -= 1;
                }
                i += 1;
            }
            continue;
        }

        if chars[i] == marker {
            let delim_start = i;
            let mut delim_count = 0;
            while i < end && chars[i] == marker {
                delim_count += 1;
                i += 1;
            }

            // Check if this is a valid closing delimiter
            // Must be preceded by non-space (right-flanking)
            if delim_start > start
                && chars[delim_start - 1] != ' '
                && chars[delim_start - 1] != '\n'
            {
                // For underscore: must not be followed by alphanumeric
                if marker == '_' && i < end && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    continue;
                }

                if delim_count >= count {
                    return Some((delim_start, delim_start - start + count));
                }
            }
            continue;
        }

        i += 1;
    }
    None
}

/// Find emphasis close returning (inner_length, close_marker_count, total_advance)
fn find_emphasis_close(
    chars: &[char],
    start: usize,
    end: usize,
    marker: char,
    count: usize,
) -> Option<(usize, usize, usize)> {
    let mut i = start;

    while i < end {
        if chars[i] == '\\' && i + 1 < end {
            i += 2;
            continue;
        }

        if chars[i] == '`' {
            // Skip code span
            let mut bt = 0;
            while i < end && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            while i < end {
                if chars[i] == '`' {
                    let mut close_bt = 0;
                    while i < end && chars[i] == '`' {
                        close_bt += 1;
                        i += 1;
                    }
                    if close_bt == bt {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            continue;
        }

        if chars[i] == marker {
            let delim_start = i;
            let mut delim_count = 0;
            while i < end && chars[i] == marker {
                delim_count += 1;
                i += 1;
            }

            // Right-flanking check
            if delim_start > start
                && chars[delim_start - 1] != ' '
                && chars[delim_start - 1] != '\n'
            {
                if marker == '_' && i < end && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    continue;
                }
                if delim_count >= count {
                    return Some((delim_start - start, delim_count, i - start));
                }
            }
            continue;
        }

        i += 1;
    }
    None
}

/// Try combined emphasis like ***text* text** or ***text** text*
fn try_combined_emphasis(
    chars: &[char],
    start: usize,
    end: usize,
    marker: char,
    marker_count: usize,
    ctx: &mut SpanContext,
    in_link: bool,
) -> Option<(String, usize)> {
    let content_start = start + marker_count;

    // Strategy: try to find inner delimiters
    // ***text* rest** -> <strong><em>text</em> rest</strong>
    // ***text** rest* -> <em><strong>text</strong> rest</em>

    // Try: strong wrapping em (***...* ...** -> <strong><em>...</em> ...</strong>)
    if let Some((inner_end, advance2)) =
        find_closing_delimiter(chars, content_start, end, marker, 2)
    {
        // Found ** closing - this means <strong>...</strong>
        // Now the content between start+3 and inner_end might contain * delimiters
        let mut inner_html = String::new();
        parse_spans(
            chars,
            content_start,
            inner_end,
            ctx,
            &mut inner_html,
            in_link,
        );
        return Some(
            (
                format!("<strong><em>{inner_html}</em></strong>"),
                advance2 + marker_count,
            )
                .into(),
        );
    }

    // Try: em wrapping strong (***...** ...* -> <em><strong>...</strong> ...</em>)
    if let Some((inner_end, advance1)) =
        find_closing_delimiter(chars, content_start, end, marker, 1)
    {
        let mut inner_html = String::new();
        parse_spans(
            chars,
            content_start,
            inner_end,
            ctx,
            &mut inner_html,
            in_link,
        );
        return Some((
            format!("<em><strong>{inner_html}</strong></em>"),
            advance1 + marker_count,
        ));
    }

    None
}

/// Try to parse a link: [text](url) or [text][ref] or [ref]
fn try_parse_link(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
) -> Option<(String, usize)> {
    if chars[start] != '[' {
        return None;
    }

    // Find the closing bracket, handling nesting and escapes
    let text_start = start + 1;
    let mut bracket_depth = 1;
    let mut i = text_start;

    while i < end && bracket_depth > 0 {
        if chars[i] == '\\' && i + 1 < end {
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            bracket_depth += 1;
        } else if chars[i] == ']' {
            bracket_depth -= 1;
        }
        i += 1;
    }

    if bracket_depth != 0 {
        return None;
    }

    let text_end = i - 1; // position of closing ]
    let link_text: String = chars[text_start..text_end].iter().collect();
    let after_bracket = i; // position after ]

    // Empty link text [] is not a link (unless followed by (url))
    if link_text.is_empty() {
        // [](url) is allowed
        if after_bracket < end && chars[after_bracket] == '(' {
            // Continue to inline link parsing below
        } else {
            return None;
        }
    }

    // Check what follows the ]
    if after_bracket < end && chars[after_bracket] == '(' {
        // Inline link: [text](url "title")
        return try_parse_inline_link(chars, start, text_end, after_bracket, end, ctx, &link_text);
    }

    if after_bracket < end && chars[after_bracket] == '[' {
        // Reference link: [text][ref]
        let ref_start = after_bracket + 1;
        let mut ref_end = ref_start;
        while ref_end < end && chars[ref_end] != ']' {
            if chars[ref_end] == '\\' && ref_end + 1 < end {
                ref_end += 2;
                continue;
            }
            ref_end += 1;
        }
        if ref_end >= end {
            return None;
        }

        let ref_text: String = chars[ref_start..ref_end].iter().collect();
        let ref_key = if ref_text.is_empty() {
            link_text.to_lowercase()
        } else {
            ref_text.to_lowercase()
        };

        if let Some(def) = ctx.link_defs.get(&ref_key) {
            let url = escape_html_attr(&def.url);
            let title_attr = def
                .title
                .as_ref()
                .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
                .unwrap_or_default();
            let extra_attrs = format_link_def_attrs(&def.attrs);

            let mut display_html = String::new();
            let text_chars: Vec<char> = link_text.chars().collect();
            parse_spans(
                &text_chars,
                0,
                text_chars.len(),
                ctx,
                &mut display_html,
                true,
            );

            return Some((
                format!("<a{extra_attrs} href=\"{url}\"{title_attr}>{display_html}</a>"),
                ref_end + 1 - start,
            ));
        }

        // ref_text was empty and link_text not found -> literal
        if ref_text.is_empty() {
            return None;
        }

        return None;
    }

    // Implicit reference: [text] or [text]
    // Also handles the case where there's a space+newline between [] and []
    let ref_key = link_text.to_lowercase();
    // Normalize whitespace in ref key
    let ref_key_normalized: String = ref_key.split_whitespace().collect::<Vec<&str>>().join(" ");

    if let Some(def) = ctx.link_defs.get(&ref_key_normalized) {
        let url = escape_html_attr(&def.url);
        let title_attr = def
            .title
            .as_ref()
            .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
            .unwrap_or_default();
        let extra_attrs = format_link_def_attrs(&def.attrs);

        let mut display_html = String::new();
        let text_chars: Vec<char> = link_text.chars().collect();
        parse_spans(
            &text_chars,
            0,
            text_chars.len(),
            ctx,
            &mut display_html,
            true,
        );

        return Some((
            format!("<a{extra_attrs} href=\"{url}\"{title_attr}>{display_html}</a>"),
            text_end + 1 - start,
        ));
    }

    // Check if followed by optional whitespace/newline then [ref]
    let mut look = after_bracket;
    while look < end && (chars[look] == ' ' || chars[look] == '\n' || chars[look] == '\t') {
        look += 1;
    }
    if look < end && chars[look] == '[' {
        // [text] [ref] pattern
        let ref_start = look + 1;
        let mut ref_end = ref_start;
        while ref_end < end && chars[ref_end] != ']' {
            ref_end += 1;
        }
        if ref_end < end {
            let ref_text: String = chars[ref_start..ref_end].iter().collect();
            let ref_key = ref_text.to_lowercase();
            if let Some(def) = ctx.link_defs.get(&ref_key) {
                let url = escape_html_attr(&def.url);
                let title_attr = def
                    .title
                    .as_ref()
                    .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
                    .unwrap_or_default();
                let extra_attrs = format_link_def_attrs(&def.attrs);

                let mut display_html = String::new();
                let text_chars: Vec<char> = link_text.chars().collect();
                parse_spans(
                    &text_chars,
                    0,
                    text_chars.len(),
                    ctx,
                    &mut display_html,
                    true,
                );

                return Some((
                    format!("<a{extra_attrs} href=\"{url}\"{title_attr}>{display_html}</a>"),
                    ref_end + 1 - start,
                ));
            }
        }
    }

    None
}

fn format_link_def_attrs(attrs: &HashMap<String, String>) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    let mut result = String::new();
    // Order: id, class, then rest alphabetically
    if let Some(id) = attrs.get("id") {
        result.push_str(&format!(" id=\"{id}\""));
    }
    if let Some(class) = attrs.get("class") {
        result.push_str(&format!(" class=\"{class}\""));
    }
    let mut others: Vec<(&String, &String)> = attrs
        .iter()
        .filter(|(k, _)| k.as_str() != "id" && k.as_str() != "class")
        .collect();
    others.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in others {
        result.push_str(&format!(" {k}=\"{v}\""));
    }
    result
}

fn try_parse_inline_link(
    chars: &[char],
    link_start: usize,
    _text_end: usize,
    paren_start: usize,
    end: usize,
    ctx: &mut SpanContext,
    link_text: &str,
) -> Option<(String, usize)> {
    // Parse (url "title") or (url 'title') or (<url> 'title')
    let mut i = paren_start + 1; // skip (

    // Skip whitespace
    while i < end && (chars[i] == ' ' || chars[i] == '\n' || chars[i] == '\t') {
        i += 1;
    }

    if i >= end {
        return None;
    }

    // Check for closing paren immediately -> empty URL
    if chars[i] == ')' {
        let mut display_html = String::new();
        let text_chars: Vec<char> = link_text.chars().collect();
        parse_spans(
            &text_chars,
            0,
            text_chars.len(),
            ctx,
            &mut display_html,
            true,
        );
        return Some((
            format!("<a href=\"\">{display_html}</a>"),
            i + 1 - link_start,
        ));
    }

    // URL in angle brackets
    if chars[i] == '<' {
        let url_start = i + 1;
        let mut ue = url_start;
        while ue < end && chars[ue] != '>' {
            ue += 1;
        }
        if ue >= end {
            return None;
        }
        let url: String = chars[url_start..ue].iter().collect();
        let url_end_pos = ue + 1;

        // Skip whitespace
        let mut j = url_end_pos;
        while j < end && (chars[j] == ' ' || chars[j] == '\n' || chars[j] == '\t') {
            j += 1;
        }
        // Parse optional title
        let title = if j < end && (chars[j] == '"' || chars[j] == '\'') {
            let quote = chars[j];
            j += 1;
            let ts = j;
            while j < end && chars[j] != quote {
                j += 1;
            }
            if j >= end {
                return None;
            }
            let t: String = chars[ts..j].iter().collect();
            j += 1;
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        } else {
            None
        };
        // Skip whitespace
        while j < end && (chars[j] == ' ' || chars[j] == '\n' || chars[j] == '\t') {
            j += 1;
        }
        if j >= end || chars[j] != ')' {
            return None;
        }
        let url_escaped = escape_html_attr(&url);
        let title_attr = title
            .as_ref()
            .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
            .unwrap_or_default();
        let mut display_html = String::new();
        let text_chars: Vec<char> = link_text.chars().collect();
        parse_spans(
            &text_chars,
            0,
            text_chars.len(),
            ctx,
            &mut display_html,
            true,
        );
        return Some((
            format!("<a href=\"{url_escaped}\"{title_attr}>{display_html}</a>"),
            j + 1 - link_start,
        ));
    } else {
        // URL without angle brackets
        // Strategy: try multiple closing ) positions, working with and without paren nesting.
        // For each candidate ), check if the content before it forms a valid URL+title.
        let url_start = i;

        // Strategy: find the balanced closing ), and also collect inner ) positions
        // for fallback when parens are unbalanced but there's a title.
        let mut balanced_close: Option<usize> = None;
        let mut first_close: Option<usize> = None;
        {
            let mut paren_depth = 1;
            let mut scan = i;
            while scan < end {
                if chars[scan] == '\\' && scan + 1 < end {
                    scan += 2;
                    continue;
                }
                if chars[scan] == '(' {
                    paren_depth += 1;
                } else if chars[scan] == ')' {
                    paren_depth -= 1;
                    if first_close.is_none() {
                        first_close = Some(scan);
                    }
                    if paren_depth == 0 {
                        balanced_close = Some(scan);
                        break;
                    }
                }
                scan += 1;
            }
        }

        // Build candidate list: prefer balanced, try first as fallback
        let mut candidates: Vec<usize> = Vec::new();
        if let Some(bc) = balanced_close {
            candidates.push(bc);
        }
        // For unbalanced case: try to find ) preceded by a title ("..." or '...')
        // For example: /something/to(do "doit") -- the last ) is at the end
        if balanced_close.is_none() {
            // Find the last ) on the current line
            let mut scan = i;
            let mut last_close = None;
            while scan < end && chars[scan] != '\n' {
                if chars[scan] == ')' {
                    last_close = Some(scan);
                }
                scan += 1;
            }
            if let Some(lc) = last_close {
                candidates.push(lc);
            }
        }

        if candidates.is_empty() {
            return None;
        }

        for &content_end in &candidates {
            let content: String = chars[url_start..content_end].iter().collect();
            let content_trimmed = content.trim_end();

            // Try to extract title from the end
            let (url_str, title_found) =
                if content_trimmed.ends_with('"') || content_trimmed.ends_with('\'') {
                    let quote = content_trimmed.chars().last().unwrap_or_default();
                    if let Some(open_quote_pos) =
                        content_trimmed[..content_trimmed.len() - 1].rfind(quote)
                    {
                        let before_quote = content_trimmed[..open_quote_pos].trim_end();
                        if !before_quote.is_empty() {
                            let title_str =
                                &content_trimmed[open_quote_pos + 1..content_trimmed.len() - 1];
                            if title_str.is_empty() {
                                // Empty title -> not a valid link
                                continue;
                            }
                            (before_quote.to_string(), Some(title_str.to_string()))
                        } else {
                            (content_trimmed.to_string(), None::<String>)
                        }
                    } else {
                        (content_trimmed.to_string(), None)
                    }
                } else {
                    (content_trimmed.to_string(), None)
                };

            let final_url = url_str.trim().to_string();
            // Reject URLs with unbalanced parens (unless there's a title)
            if title_found.is_none() {
                let open_parens = final_url.chars().filter(|c| *c == '(').count();
                let close_parens = final_url.chars().filter(|c| *c == ')').count();
                if open_parens != close_parens {
                    continue;
                }
            }
            let url_escaped = escape_html_attr(&final_url);
            let title_attr = title_found
                .as_ref()
                .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
                .unwrap_or_default();

            let mut display_html = String::new();
            let text_chars: Vec<char> = link_text.chars().collect();
            parse_spans(
                &text_chars,
                0,
                text_chars.len(),
                ctx,
                &mut display_html,
                true,
            );

            return Some((
                format!("<a href=\"{url_escaped}\"{title_attr}>{display_html}</a>"),
                content_end + 1 - link_start,
            ));
        }
        return None;
    };
}

/// Try to parse an image: ![alt](url) or ![alt][ref]
fn try_parse_image(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
) -> Option<(String, usize)> {
    if start + 1 >= end || chars[start] != '!' || chars[start + 1] != '[' {
        return None;
    }

    // Find closing ]
    let text_start = start + 2;
    let mut bracket_depth = 1;
    let mut i = text_start;
    while i < end && bracket_depth > 0 {
        if chars[i] == '\\' && i + 1 < end {
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            bracket_depth += 1;
        } else if chars[i] == ']' {
            bracket_depth -= 1;
        }
        i += 1;
    }
    if bracket_depth != 0 {
        return None;
    }

    let text_end = i - 1;
    let alt_text: String = chars[text_start..text_end].iter().collect();
    // Unescape in alt text
    let alt_text = alt_text
        .replace("\\|", "|")
        .replace("\\[", "[")
        .replace("\\]", "]");
    let alt_escaped = escape_html_attr(&alt_text);
    let after_bracket = i;

    if after_bracket < end && chars[after_bracket] == '(' {
        // Inline image: ![alt](url "title")
        let mut j = after_bracket + 1;
        // Skip whitespace
        while j < end && (chars[j] == ' ' || chars[j] == '\n') {
            j += 1;
        }

        if j >= end {
            return None;
        }

        // Check for immediate close
        if chars[j] == ')' {
            return Some((
                format!("<img src=\"\" alt=\"{alt_escaped}\" />"),
                j + 1 - start,
            ));
        }

        // Parse URL
        let url_start = j;
        let mut paren_depth = 0;
        while j < end {
            if chars[j] == '(' {
                paren_depth += 1;
                j += 1;
            } else if chars[j] == ')' {
                if paren_depth > 0 {
                    paren_depth -= 1;
                    j += 1;
                } else {
                    break;
                }
            } else if chars[j] == ' ' || chars[j] == '\t' {
                break;
            } else {
                j += 1;
            }
        }
        let url: String = chars[url_start..j].iter().collect();

        // Skip whitespace
        while j < end && (chars[j] == ' ' || chars[j] == '\t') {
            j += 1;
        }

        // Title
        let title = if j < end && (chars[j] == '"' || chars[j] == '\'') {
            let quote = chars[j];
            j += 1;
            let ts = j;
            while j < end && chars[j] != quote {
                j += 1;
            }
            if j >= end {
                return None;
            }
            let t: String = chars[ts..j].iter().collect();
            j += 1;
            Some(t)
        } else {
            None
        };

        // Skip whitespace
        while j < end && (chars[j] == ' ' || chars[j] == '\t') {
            j += 1;
        }

        if j >= end || chars[j] != ')' {
            return None;
        }

        let url_escaped = escape_html_attr(&url);
        let title_attr = title
            .as_ref()
            .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
            .unwrap_or_default();

        return Some((
            format!("<img src=\"{url_escaped}\" alt=\"{alt_escaped}\"{title_attr} />"),
            j + 1 - start,
        ));
    }

    if after_bracket < end && chars[after_bracket] == '[' {
        // Reference image: ![alt][ref]
        let ref_start = after_bracket + 1;
        let mut ref_end = ref_start;
        while ref_end < end && chars[ref_end] != ']' {
            ref_end += 1;
        }
        if ref_end >= end {
            return None;
        }
        let ref_text: String = chars[ref_start..ref_end].iter().collect();
        let ref_key = if ref_text.is_empty() {
            alt_text.to_lowercase()
        } else {
            ref_text.to_lowercase()
        };

        if let Some(def) = ctx.link_defs.get(&ref_key) {
            let url_escaped = escape_html_attr(&def.url);
            let title_attr = def
                .title
                .as_ref()
                .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
                .unwrap_or_default();
            return Some((
                format!("<img src=\"{url_escaped}\" alt=\"{alt_escaped}\"{title_attr} />"),
                ref_end + 1 - start,
            ));
        }
        return None;
    }

    // Implicit reference: ![alt] -> look up alt as ref
    let ref_key = alt_text.to_lowercase();
    if let Some(def) = ctx.link_defs.get(&ref_key) {
        let url_escaped = escape_html_attr(&def.url);
        let title_attr = def
            .title
            .as_ref()
            .map(|t| format!(" title=\"{}\"", escape_html_attr(t)))
            .unwrap_or_default();
        return Some((
            format!("<img src=\"{url_escaped}\" alt=\"{alt_escaped}\"{title_attr} />"),
            text_end + 1 - start,
        ));
    }

    None
}

/// Try to parse a smart quote.
fn try_parse_smart_quote(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &SpanContext,
) -> Option<(String, usize)> {
    let quote_char = chars[start];

    // Get smart quote entity names from options
    let sq = &ctx.options.smart_quotes;
    let lsquo = sq.first().map(|s| s.as_str()).unwrap_or("lsquo");
    let rsquo = sq.get(1).map(|s| s.as_str()).unwrap_or("rsquo");
    let ldquo = sq.get(2).map(|s| s.as_str()).unwrap_or("ldquo");
    let rdquo = sq.get(3).map(|s| s.as_str()).unwrap_or("rdquo");

    if quote_char == '"' {
        let is_opening = is_opening_quote(chars, start, end);
        if is_opening {
            return Some((entity_str(ldquo, ctx), 1));
        } else {
            return Some((entity_str(rdquo, ctx), 1));
        }
    }

    if quote_char == '\'' {
        // Special case: contractions and possessives
        if start > 0 {
            let prev = chars[start - 1];
            if prev.is_alphanumeric() || prev == '>' || prev == ')' {
                return Some((entity_str(rsquo, ctx), 1));
            }
        }

        let is_opening = is_opening_quote(chars, start, end);
        if is_opening {
            let has_closing = has_matching_single_close(chars, start + 1, end);
            if has_closing {
                return Some((entity_str(lsquo, ctx), 1));
            } else {
                return Some((entity_str(rsquo, ctx), 1));
            }
        } else {
            return Some((entity_str(rsquo, ctx), 1));
        }
    }

    None
}

fn is_opening_quote(chars: &[char], pos: usize, end: usize) -> bool {
    let next = if pos + 1 < end {
        Some(chars[pos + 1])
    } else {
        None
    };

    // At start of text: only opening if followed by alphanumeric or clear content openers
    if pos == 0 {
        return next.map_or(false, |c| {
            c.is_alphanumeric()
                || c == '.'
                || c == '\''
                || c == '"'
                || c == '`'
                || c == '*'
                || c == '_'
                || c == '!'
        });
    }

    let prev = chars[pos - 1];

    // Must be preceded by opener context
    let prev_is_opener = prev == ' '
        || prev == '\n'
        || prev == '\t'
        || prev == '('
        || prev == '['
        || prev == '{'
        || prev == '"'
        || prev == '\''
        || prev == '`'
        || prev == '\u{201C}'
        || prev == '\u{2018}'
        || prev == '>';

    if !prev_is_opener {
        return false;
    }

    // Must be followed by non-space and non-closing-punctuation
    next.map_or(false, |c| {
        c != ' '
            && c != '\n'
            && c != '\t'
            && c != ','
            && c != ';'
            && c != ':'
            && c != ')'
            && c != ']'
            && c != '}'
    })
}

fn has_matching_single_close(chars: &[char], start: usize, end: usize) -> bool {
    let mut i = start;
    while i < end {
        if chars[i] == '\'' {
            // Check if this is a valid closing '
            if i > start && chars[i - 1] != ' ' && chars[i - 1] != '\n' {
                return true;
            }
        }
        if chars[i] == '\n' {
            // Don't match across paragraph boundaries
            break;
        }
        i += 1;
    }
    false
}

fn parse_guillemet_open(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &SpanContext,
) -> (String, usize) {
    // << with optional space after
    if start + 2 < end && chars[start + 2] == ' ' {
        // Check for typographic_symbols override for laquo_space
        if let Some(override_val) = ctx.options.typographic_symbols.get("laquo_space") {
            return (escape_html_str(override_val), 3);
        }
        return (
            format!("{}{}", entity_str("laquo", ctx), entity_str("nbsp", ctx)),
            3,
        );
    }
    (entity_str("laquo", ctx), 2)
}

fn parse_guillemet_close(
    chars: &[char],
    start: usize,
    _end: usize,
    ctx: &SpanContext,
) -> (String, usize) {
    // >> with optional space before
    if start > 0 && chars[start - 1] == ' ' {
        if let Some(override_val) = ctx.options.typographic_symbols.get("raquo_space") {
            return (escape_html_str(override_val), 2);
        }
        return (
            format!("{}{}", entity_str("nbsp", ctx), entity_str("raquo", ctx)),
            2,
        );
    }
    (entity_str("raquo", ctx), 2)
}

/// Try to parse an HTML entity: &name; or &#num; or &#xhex;
fn try_parse_entity(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &SpanContext,
) -> Option<(String, usize)> {
    if chars[start] != '&' {
        return None;
    }

    // Find the semicolon
    let mut i = start + 1;
    let max_len = 32; // reasonable max for entity names
    while i < end && i - start < max_len && chars[i] != ';' && chars[i] != ' ' && chars[i] != '\n' {
        i += 1;
    }

    if i >= end || chars[i] != ';' {
        return None;
    }

    let entity_content: String = chars[start + 1..i].iter().collect();

    // Numeric entity: &#123;
    if let Some(num_str) = entity_content.strip_prefix('#') {
        if let Some(hex_str) = num_str
            .strip_prefix('x')
            .or_else(|| num_str.strip_prefix('X'))
        {
            // Hex entity: &#xAF;
            if let Some(ch) = entities::resolve_hex_entity(hex_str) {
                let advance = i + 1 - start;
                return Some((
                    format_entity_output(
                        ch,
                        &format!("&#{};", ch as u32),
                        &format!("&#x{hex_str};"),
                        ctx,
                    ),
                    advance,
                ));
            }
            return None; // Invalid hex entity
        }
        // Decimal entity: &#343;
        if let Ok(num) = num_str.parse::<u32>() {
            if let Some(ch) = entities::resolve_numeric_entity(num) {
                let advance = i + 1 - start;
                return Some((
                    format_entity_output(ch, &format!("&#{num};"), &format!("&#{num};"), ctx),
                    advance,
                ));
            }
        }
        return None;
    }

    // Named entity: &copy;
    if entities::resolve_named_entity(&entity_content).is_some() {
        let advance = i + 1 - start;
        let original = format!("&{entity_content};");
        match ctx.options.entity_output {
            EntityOutput::AsChar => {
                let ch_str = entities::resolve_named_entity(&entity_content)
                    .unwrap_or(&entity_content);
                Some((ch_str.to_string(), advance))
            }
            EntityOutput::Symbolic => {
                // Try to find symbolic name
                Some((original, advance))
            }
            EntityOutput::Numeric => {
                let ch_str = entities::resolve_named_entity(&entity_content)
                    .unwrap_or(&entity_content);
                let cp = ch_str.chars().next().unwrap_or_default() as u32;
                Some((format!("&#{cp};"), advance))
            }
            EntityOutput::AsInput => Some((original, advance)),
        }
    } else {
        None // Unknown entity name
    }
}

fn format_entity_output(
    ch: char,
    numeric_form: &str,
    original_form: &str,
    ctx: &SpanContext,
) -> String {
    match ctx.options.entity_output {
        EntityOutput::AsChar => ch.to_string(),
        EntityOutput::Numeric => {
            let cp = ch as u32;
            format!("&#{cp};")
        }
        EntityOutput::Symbolic => {
            // Try to find the symbolic name
            // Look up in reverse entity table
            if let Some(name) = find_entity_name(ch) {
                format!("&{name};")
            } else {
                numeric_form.to_string()
            }
        }
        EntityOutput::AsInput => original_form.to_string(),
    }
}

fn find_entity_name(ch: char) -> Option<&'static str> {
    // Common reverse lookups
    let entities_map: &[(&str, char)] = &[
        ("amp", '&'),
        ("lt", '<'),
        ("gt", '>'),
        ("quot", '"'),
        ("apos", '\''),
        ("nbsp", '\u{00A0}'),
        ("copy", '\u{00A9}'),
        ("reg", '\u{00AE}'),
        ("trade", '\u{2122}'),
        ("ndash", '\u{2013}'),
        ("mdash", '\u{2014}'),
        ("hellip", '\u{2026}'),
        ("lsquo", '\u{2018}'),
        ("rsquo", '\u{2019}'),
        ("ldquo", '\u{201C}'),
        ("rdquo", '\u{201D}'),
        ("laquo", '\u{00AB}'),
        ("raquo", '\u{00BB}'),
        ("times", '\u{00D7}'),
        ("divide", '\u{00F7}'),
        ("lambda", '\u{03BB}'),
    ];
    for (name, c) in entities_map {
        if *c == ch {
            return Some(name);
        }
    }
    None
}

/// Try to parse a footnote reference: [^name]
fn try_parse_footnote_ref(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
) -> Option<(String, usize)> {
    if start + 2 >= end || chars[start] != '[' || chars[start + 1] != '^' {
        return None;
    }

    let name_start = start + 2;
    let mut i = name_start;
    while i < end && chars[i] != ']' && chars[i] != '\n' {
        i += 1;
    }
    if i >= end || chars[i] != ']' {
        return None;
    }

    let name: String = chars[name_start..i].iter().collect();
    if name.is_empty() {
        return None;
    }

    // Check if this footnote is defined
    if !ctx.footnote_defs.contains_key(&name) {
        return None;
    }

    // Get or assign footnote number
    let fn_number = if let Some(pos) = ctx.footnote_order.iter().position(|n| n == &name) {
        pos + ctx.options.footnote_nr as usize
    } else {
        let num = ctx.footnote_order.len() + ctx.options.footnote_nr as usize;
        ctx.footnote_order.push(name.clone());
        num
    };

    let prefix = &ctx.options.footnote_prefix;
    let fn_id = if prefix.is_empty() {
        format!("fn:{name}")
    } else {
        format!("fn:{prefix}:{name}")
    };

    // Track reference count for duplicate IDs
    let ref_count = ctx.footnote_ref_counts.entry(name.clone()).or_insert(0);
    *ref_count += 1;
    let suffix = if *ref_count > 1 {
        format!(":{}", *ref_count - 1)
    } else {
        String::new()
    };
    let fnref_id = if prefix.is_empty() {
        format!("fnref:{name}{suffix}")
    } else {
        format!("fnref:{prefix}:{name}{suffix}")
    };

    Some((
        format!(
            "<sup id=\"{fnref_id}\"><a href=\"#{fn_id}\" class=\"footnote\" rel=\"footnote\" role=\"doc-noteref\">{fn_number}</a></sup>",
        ),
        i + 1 - start,
    ))
}

/// Render footnotes section (called after all content is processed)
pub fn render_footnotes(ctx: &mut SpanContext) -> String {
    if ctx.footnote_order.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str("\n<div class=\"footnotes\" role=\"doc-endnotes\">\n  <ol>\n");

    for (idx, name) in ctx.footnote_order.clone().iter().enumerate() {
        let _fn_number = idx + ctx.options.footnote_nr as usize;
        let prefix = &ctx.options.footnote_prefix;
        let fn_id = if prefix.is_empty() {
            format!("fn:{name}")
        } else {
            format!("fn:{prefix}:{name}")
        };
        let fnref_id = if prefix.is_empty() {
            format!("fnref:{name}")
        } else {
            format!("fnref:{prefix}:{name}")
        };

        let content = ctx.footnote_defs.get(name).cloned().unwrap_or_default();

        let backlink = &ctx.options.footnote_backlink;
        let backlink_html = if ctx.options.footnote_backlink_inline {
            format!("&nbsp;<a href=\"#{fnref_id}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink}</a>")
        } else {
            format!(" <a href=\"#{fnref_id}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink}</a>")
        };

        output.push_str(&format!("    <li id=\"{fn_id}\">\n"));

        // Parse the footnote content as paragraphs
        let mut fn_html = String::new();
        let content_html = spans_to_html(&content, ctx);

        // Wrap in paragraph
        if content.contains('\n') && content.contains("\n\n") {
            // Multi-paragraph footnote
            let paragraphs: Vec<&str> = content.split("\n\n").collect();
            for (pi, para) in paragraphs.iter().enumerate() {
                let para_html = spans_to_html(para.trim(), ctx);
                if pi == paragraphs.len() - 1 {
                    fn_html.push_str(&format!("      <p>{para_html}{backlink_html}</p>\n"));
                } else {
                    fn_html.push_str(&format!("      <p>{para_html}</p>\n"));
                }
            }
        } else {
            fn_html.push_str(&format!("      <p>{content_html}{backlink_html}</p>\n"));
        }

        output.push_str(&fn_html);
        output.push_str("    </li>\n");
    }

    output.push_str("  </ol>\n</div>\n");
    output
}

/// Apply abbreviations to HTML output, replacing whole-word occurrences
/// but not inside HTML tags.
fn apply_abbreviations(html: &str, abbreviations: &HashMap<String, String>) -> String {
    if abbreviations.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Sort abbreviations by length (longest first) to avoid partial matches
    let mut abbrs: Vec<(&String, &String)> = abbreviations.iter().collect();
    abbrs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (abbr, full) in abbrs {
        let replacement = format!("<abbr title=\"{full}\">{abbr}</abbr>");
        result = replace_outside_tags(&result, abbr, &replacement);
    }

    result
}

/// Replace text outside of HTML tags (not inside <...> or attributes)
fn replace_outside_tags(html: &str, search: &str, replacement: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = html.chars().collect();
    let search_chars: Vec<char> = search.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inside HTML tag
        if chars[i] == '<' {
            let tag_start = i;
            i += 1;
            while i < chars.len() && chars[i] != '>' {
                i += 1;
            }
            if i < chars.len() {
                i += 1; // skip >
            }
            let tag: String = chars[tag_start..i].iter().collect();
            result.push_str(&tag);
            continue;
        }

        // Check for word match
        if i + search_chars.len() <= chars.len() {
            let candidate: String = chars[i..i + search_chars.len()].iter().collect();
            if candidate == *search {
                // Check word boundaries
                let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
                let after_ok = i + search_chars.len() >= chars.len()
                    || !chars[i + search_chars.len()].is_alphanumeric();
                if before_ok && after_ok {
                    result.push_str(replacement);
                    i += search_chars.len();
                    continue;
                }
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Replace whole-word occurrences (not inside tags)
#[allow(dead_code)]
fn replace_whole_word(text: &str, search: &str, replacement: &str) -> String {
    replace_outside_tags(text, search, replacement)
}

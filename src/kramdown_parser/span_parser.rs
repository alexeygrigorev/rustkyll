// kramdown parser - Span-level (inline) parser
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.

#![allow(clippy::manual_strip)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_return)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::manual_find)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::manual_is_ascii_check)]
#![allow(clippy::char_lit_as_u8)]
#![allow(clippy::option_map_unit_fn)]
#![allow(clippy::manual_pattern_char_comparison)]

use crate::kramdown_parser::entities;
use crate::kramdown_parser::options::{EntityOutput, Options};
use crate::syntax::highlight_code;
use std::collections::HashMap;

/// Context for span parsing, carrying link definitions and abbreviations.
pub struct SpanContext {
    /// Link definitions: id -> (url, optional title, optional attrs)
    pub link_defs: HashMap<String, LinkDef>,
    /// Abbreviations: abbreviation -> full text
    pub abbreviations: HashMap<String, String>,
    /// Abbreviation IAL attributes: abbreviation -> list of (key, value) pairs
    pub abbreviation_attrs: HashMap<String, Vec<(String, String)>>,
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
    /// TOC headers: (level, id, text, has_no_toc)
    pub toc_headers: Vec<(usize, String, String, bool)>,
    /// Index into toc_headers for sequential header ID assignment during conversion
    pub toc_header_index: usize,
    /// Options
    pub options: Options,
    /// Emphasis nesting stack: 1 = em, 2 = strong (prevents same-type nesting)
    pub emphasis_stack: Vec<u8>,
}

/// A link definition with optional IAL attributes
pub struct LinkDef {
    pub url: String,
    pub title: Option<String>,
    /// Attributes in insertion order (key, value pairs)
    pub attrs: Vec<(String, String)>,
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
                    attrs: Vec::new(),
                },
            );
        }
        Self {
            link_defs,
            abbreviations: HashMap::new(),
            abbreviation_attrs: HashMap::new(),
            footnote_defs: HashMap::new(),
            footnote_counter: options.footnote_nr as usize,
            footnote_order: Vec::new(),
            footnote_ref_counts: HashMap::new(),
            ald_defs: HashMap::new(),
            toc_headers: Vec::new(),
            toc_header_index: 0,
            options: options.clone(),
            emphasis_stack: Vec::new(),
        }
    }
}

/// Parse the full document text to extract link definitions, abbreviations, and footnotes.
/// Returns the text with those definitions removed.
pub fn extract_definitions(text: &str, ctx: &mut SpanContext) -> String {
    let mut output_lines: Vec<&str> = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    // Track whether we're at a block boundary (link defs/footnotes/abbrevs/IALs
    // can only appear at block boundaries, not in paragraph continuations)
    let mut at_block_boundary = true;

    // Collect IAL lines that appear before a link def (they apply to the next link def)
    let mut pending_ial_attrs: Vec<(String, String)> = Vec::new();
    // Track line indices of pending IAL lines (so we can output them if no link def follows)
    let mut pending_ial_lines: Vec<usize> = Vec::new();

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // Link definition: [id]: url 'title'
        // Can have 0-3 spaces of indent
        // Note: IDs starting with ^ are footnotes, not link defs
        let indent = line.len() - trimmed.len();
        if indent <= 3 && at_block_boundary {
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
                            // Start with any IAL attrs that preceded this link def
                            let mut attrs: Vec<(String, String)> = Vec::new();
                            for (k, v) in pending_ial_attrs.drain(..) {
                                merge_attr_vec(&mut attrs, k, v);
                            }
                            pending_ial_lines.clear();
                            // Check for IAL on the next line(s) after the link def
                            while i < lines.len() && is_block_ial(lines[i]) {
                                let ial_attrs = parse_ial(lines[i].trim());
                                for (k, v) in ial_attrs {
                                    merge_attr_vec(&mut attrs, k, v);
                                }
                                i += 1;
                            }
                            let key = id.to_lowercase();
                            let def = LinkDef {
                                url: link_def.0,
                                title: link_def.1,
                                attrs,
                            };
                            // Last definition wins (kramdown overwrites duplicate link IDs)
                            ctx.link_defs.insert(key, def);
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
                        at_block_boundary = true;
                        continue;
                    }
                }
            }

            // Block IAL (not ALD): {: .class #id key="value"}
            // Could be applied to the next link definition.
            // We tentatively store it; if no link def follows, we output it normally.
            if is_block_ial(trimmed) {
                let ial_attrs = parse_ial(trimmed);
                pending_ial_attrs.extend(ial_attrs);
                pending_ial_lines.push(i);
                i += 1;
                // IAL keeps us at a block boundary
                continue;
            }
        }

        // Abbreviation definition: *[ABBR]: Full text
        // Only at 0-3 spaces indent (4+ is a code block)
        if indent <= 3 {
            if let Some(rest) = trimmed.strip_prefix("*[") {
                if let Some(close) = rest.find("]:") {
                    let abbr = &rest[..close];
                    let full = rest[close + 2..].trim();
                    if !abbr.is_empty() {
                        ctx.abbreviations.insert(abbr.to_string(), full.to_string());
                        i += 1;
                        // Collect IAL attrs: pending (before) + following (after)
                        let mut abbr_attrs: Vec<(String, String)> = Vec::new();
                        for (k, v) in pending_ial_attrs.drain(..) {
                            merge_attr_vec(&mut abbr_attrs, k, v);
                        }
                        pending_ial_lines.clear();
                        // Collect IAL lines after the abbreviation definition
                        while i < lines.len() && is_block_ial(lines[i]) {
                            let ial_attrs = parse_ial(lines[i].trim());
                            for (k, v) in ial_attrs {
                                merge_attr_vec(&mut abbr_attrs, k, v);
                            }
                            i += 1;
                        }
                        if !abbr_attrs.is_empty() {
                            ctx.abbreviation_attrs.insert(abbr.to_string(), abbr_attrs);
                        }
                        at_block_boundary = true;
                        continue;
                    }
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
                // Consume any IAL lines after the footnote definition
                // (they apply to the footnote, not to the next block element)
                while i < lines.len() && is_block_ial(lines[i]) {
                    i += 1;
                }
                at_block_boundary = true;
                // Flush any pending IAL lines (they don't apply to footnote defs)
                for &ial_line_idx in &pending_ial_lines {
                    output_lines.push(lines[ial_line_idx]);
                }
                pending_ial_lines.clear();
                pending_ial_attrs.clear();
                // Insert an EOB marker so the block parser ends any
                // currently-open list/blockquote. Without this, two
                // lists separated by a footnote definition would merge
                // into one after the definition is extracted.
                // Only insert when there's more content following (not at end of doc).
                if i < lines.len() {
                    output_lines.push("^");
                }
                continue;
            }
        }

        // Not a definition line: flush any pending IAL lines to output
        // (they weren't consumed by a link def, so they belong in the output)
        for &ial_line_idx in &pending_ial_lines {
            output_lines.push(lines[ial_line_idx]);
        }
        pending_ial_lines.clear();
        pending_ial_attrs.clear();
        // Track block boundary status
        at_block_boundary = trimmed.is_empty();
        output_lines.push(line);
        i += 1;
    }

    // Flush any remaining pending IAL lines
    for &ial_line_idx in &pending_ial_lines {
        output_lines.push(lines[ial_line_idx]);
    }

    // Reconstruct text, preserving trailing blank lines
    let defs_were_removed = output_lines.len() < lines.len();
    let mut result = output_lines.join("\n");
    // Ensure trailing newline matches original
    if text.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    // If original had trailing blank lines (multiple newlines at end),
    // preserve them so the parser sees trailing Blank elements
    if text.ends_with("\n\n") && !result.ends_with("\n\n") {
        result.push('\n');
    }
    // When definitions were removed and output ends with a blank line,
    // join("\n") loses the trailing blank. Restore it.
    if defs_were_removed
        && output_lines.last().is_some_and(|l| l.is_empty())
        && !result.ends_with("\n\n")
    {
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
    if after_colon.starts_with('<') {
        if let Some(close) = after_colon[1..].find('>') {
            let url = after_colon[1..=close].to_string();
            let rest = after_colon[close + 2..].trim_start();
            let title = if rest.is_empty() {
                check_next_line_title(lines, pos)
            } else {
                extract_title(rest)
            };
            *pos += if title.is_some() && rest.is_empty() {
                2
            } else {
                1
            };
            return Some((url, title));
        }
        *pos += 1;
        return None;
    }

    // Try to split URL and title on the same line.
    // Kramdown: URL is non-greedy, title is quoted string at end.
    // First try: entire after_colon is URL (no inline title), check next line
    // Then try: split at last whitespace before a quote to separate URL and title

    // Check if there's a quoted title at the end of the line
    let trimmed_end = after_colon.trim_end();
    let mut url = trimmed_end.to_string();
    let mut title: Option<String> = None;

    // Try to find a title at the end: look for ' "title"' or " 'title'"
    if let Some(inline_title) = extract_inline_url_title(trimmed_end) {
        url = inline_title.0;
        title = Some(inline_title.1);
    }

    if url.is_empty() {
        *pos += 1;
        return None;
    }

    // Kramdown rejects link defs where the URL part contains whitespace followed by a quote
    // (this prevents mismatched quote titles from being treated as URLs)
    if title.is_none() && url_has_space_then_quote(&url) {
        // Don't advance pos; this is not a valid link definition
        return None;
    }

    // If no inline title, check next line for title
    if title.is_none() {
        if *pos + 1 < lines.len() {
            let next_trimmed = lines[*pos + 1].trim();
            if let Some(t) = extract_title(next_trimmed) {
                title = Some(t);
                *pos += 2;
                return Some((url, title));
            }
        }
    }

    *pos += 1;
    Some((url, title))
}

/// Try to extract URL and title from the same line.
/// Returns (url, title) if a quoted title is found at the end.
fn extract_inline_url_title(s: &str) -> Option<(String, String)> {
    // Look for patterns like: url "title" or url 'title'
    // The title must be at the end of the string
    for quote in ['"', '\''] {
        if s.ends_with(quote) {
            // Find the matching opening quote preceded by whitespace
            let without_end = &s[..s.len() - 1];
            // Search backwards for whitespace + opening quote
            for (i, ch) in without_end.char_indices().rev() {
                if ch == quote {
                    // Check that before the quote is whitespace (or tab)
                    if i > 0 {
                        let before = without_end.as_bytes()[i - 1];
                        if before == b' ' || before == b'\t' {
                            let url = s[..i - 1].trim_end().to_string();
                            let title = s[i + 1..s.len() - 1].to_string();
                            if !url.is_empty() {
                                return Some((url, title));
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    None
}

/// Check if the next line contains a title.
fn check_next_line_title(lines: &[&str], pos: &usize) -> Option<String> {
    if *pos + 1 < lines.len() {
        let next_trimmed = lines[*pos + 1].trim();
        extract_title(next_trimmed)
    } else {
        None
    }
}

/// Check if a URL string contains whitespace followed by a quote character.
/// Kramdown rejects such URLs as invalid link definitions.
fn url_has_space_then_quote(url: &str) -> bool {
    let bytes = url.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if (bytes[i] == b' ' || bytes[i] == b'\t')
            && (bytes[i + 1] == b'"' || bytes[i + 1] == b'\'')
        {
            return true;
        }
    }
    false
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

/// Apply inline `{::options key="value" /}` to the span context.
fn apply_inline_options(opts_str: &str, ctx: &mut SpanContext) {
    // Parse key="value" or key='value' pairs from the options string
    let mut chars = opts_str.chars().peekable();
    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        // Read key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }
        if key.is_empty() {
            chars.next();
            continue;
        }
        if chars.peek() == Some(&'=') {
            chars.next(); // consume '='
            let value = parse_ial_quoted_value(&mut chars);
            match key.as_str() {
                "parse_span_html" => {
                    ctx.options.parse_span_html = value == "true";
                }
                "parse_block_html" => {
                    ctx.options.parse_block_html = value == "true";
                }
                "footnote_nr" => {
                    if let Ok(nr) = value.parse::<u32>() {
                        ctx.options.footnote_nr = nr;
                        ctx.footnote_counter = nr as usize;
                    }
                }
                _ => {
                    // Other options are silently ignored at span level
                }
            }
        }
    }
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
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        let c = *chars.peek().unwrap();

        if c == '#' || c == '.' {
            // ID/class combo: read a sequence of (#id|.class)+ tokens
            // kramdown allows #id.class.class2 as a single multi-token
            parse_ial_id_or_class_multi(&mut chars, &mut attrs);
        } else if c.is_alphanumeric() || c == '_' {
            // Potential key=value or ALD reference
            // Read the word: \w[\w-]*
            let mut word = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' || (ch == '-' && !word.is_empty()) {
                    word.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }

            if chars.peek() == Some(&'=') {
                // key=value pair
                chars.next(); // consume =
                let value = parse_ial_quoted_value(&mut chars);
                if !word.is_empty() {
                    attrs.push((word, value));
                }
            } else if chars.peek().is_none() || chars.peek().is_some_and(|c| c.is_whitespace()) {
                // Bare word followed by whitespace or end: ALD reference
                // Must be valid ID name: starts with word char
                if !word.is_empty() {
                    attrs.push(("__ald_ref__".to_string(), word));
                }
            } else {
                // Word followed by non-whitespace non-= char (like ig.nored, as_is#this)
                // Skip the rest of this token until whitespace
                while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                    chars.next();
                }
            }
        } else {
            // Unknown character (like '-'): skip until whitespace
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                chars.next();
            }
        }
    }
    attrs
}

/// Parse a sequence of #id and .class tokens (kramdown ID_OR_CLASS_MULTI).
fn parse_ial_id_or_class_multi(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    attrs: &mut Vec<(String, String)>,
) {
    // Collect the whole run of #id/.class tokens
    let mut temp_attrs = Vec::new();
    while let Some(&c) = chars.peek() {
        if c == '#' {
            chars.next();
            // ID name: [A-Za-z][\w:-]*
            let mut id = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == ':' {
                    id.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            if !id.is_empty() {
                temp_attrs.push(("id".to_string(), id));
            }
        } else if c == '.' {
            chars.next();
            // Class name: [^\s.#]+
            let mut class = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() || ch == '.' || ch == '#' || ch == '}' {
                    break;
                }
                class.push(ch);
                chars.next();
            }
            if !class.is_empty() {
                temp_attrs.push(("class".to_string(), class));
            }
        } else {
            break;
        }
    }
    // The multi-token is valid only if followed by whitespace or end
    if chars.peek().is_none() || chars.peek().is_some_and(|c| c.is_whitespace()) {
        attrs.extend(temp_attrs);
    }
    // else: invalid token like `.foo bar` where bar starts immediately - skip
    // (but this case shouldn't really happen since . and # delimit)
}

/// Parse a quoted or unquoted value after '=' in IAL.
fn parse_ial_quoted_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    if chars.peek() == Some(&'"') {
        chars.next();
        let mut v = String::new();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    if next == '"' || next == '}' || next == '\\' {
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
                    if next == '\'' || next == '}' || next == '\\' {
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
    }
}

/// Merge an attribute into a Vec<(String, String)>, appending class values.
fn merge_attr_vec(attrs: &mut Vec<(String, String)>, key: String, value: String) {
    if key == "class" {
        // Merge class values: find existing class entry or create new one
        if let Some(entry) = attrs.iter_mut().find(|(k, _)| k == "class") {
            if !entry.1.is_empty() {
                entry.1.push(' ');
            }
            entry.1.push_str(&value);
        } else {
            attrs.push((key, value));
        }
    } else if key == "id" {
        // ID replaces existing
        if let Some(entry) = attrs.iter_mut().find(|(k, _)| k == "id") {
            entry.1 = value;
        } else {
            attrs.push((key, value));
        }
    } else {
        // Other attributes: replace existing or add new
        if let Some(entry) = attrs.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            attrs.push((key, value));
        }
    }
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
        result = apply_abbreviations(&result, &ctx.abbreviations, &ctx.abbreviation_attrs);
    }

    result
}

/// Information about a standalone image detected in a paragraph.
pub struct StandaloneImageInfo {
    /// The `src` attribute value.
    pub src: String,
    /// The `alt` attribute value (used for figcaption).
    pub alt: String,
    /// Optional `title` attribute value.
    pub title: Option<String>,
    /// Inline IAL attributes (from the span-level IAL on the image), excluding `standalone`.
    pub inline_attrs: Vec<(String, String)>,
}

/// Check if paragraph text is solely a standalone image (an image with the `standalone` IAL
/// attribute, with no other content). Returns structured data for figure rendering.
pub fn try_parse_standalone_image(text: &str) -> Option<StandaloneImageInfo> {
    let trimmed = text.trim();
    if !trimmed.starts_with("![") {
        return None;
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let end = chars.len();

    // Try to parse the image at position 0
    // We need to extract src, alt, title from the image
    let (src, alt, title, img_end) = try_extract_image_parts(&chars, 0, end)?;

    // After the image, expect an IAL with `standalone`
    let mut pos = img_end;
    let mut all_attrs: Vec<(String, String)> = Vec::new();

    // Consume IAL(s) after the image
    while pos < end {
        if let Some((attrs, ial_len)) = try_parse_span_ial(&chars, pos, end) {
            all_attrs.extend(attrs);
            pos += ial_len;
        } else {
            break;
        }
    }

    // Must be at end of text (nothing else in the paragraph)
    if pos != end {
        return None;
    }

    // Check for `standalone` attribute (stored as __ald_ref__ = "standalone")
    let has_standalone = all_attrs
        .iter()
        .any(|(k, v)| k == "__ald_ref__" && v == "standalone");

    if !has_standalone {
        return None;
    }

    // Filter out the standalone reference from inline attrs
    let inline_attrs: Vec<(String, String)> = all_attrs
        .into_iter()
        .filter(|(k, v)| !(k == "__ald_ref__" && v == "standalone"))
        // Convert __ald_ref__ entries to actual ALD lookups if needed (skip for now)
        .filter(|(k, _)| k != "__ald_ref__")
        .collect();

    Some(StandaloneImageInfo {
        src,
        alt,
        title,
        inline_attrs,
    })
}

/// Extract image parts (src, alt, title) without producing HTML.
/// Returns (src, alt, title, chars_consumed).
fn try_extract_image_parts(
    chars: &[char],
    start: usize,
    end: usize,
) -> Option<(String, String, Option<String>, usize)> {
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
    let alt_text = alt_text
        .replace("\\|", "|")
        .replace("\\[", "[")
        .replace("\\]", "]");
    let alt_text = alt_text.replace(['\n', '\t'], " ");
    let after_bracket = i;

    if after_bracket < end && chars[after_bracket] == '(' {
        // Inline image: ![alt](url "title")
        let mut j = after_bracket + 1;
        while j < end && (chars[j] == ' ' || chars[j] == '\n') {
            j += 1;
        }
        if j >= end {
            return None;
        }
        if chars[j] == ')' {
            return Some((String::new(), alt_text, None, j + 1));
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

        return Some((url, alt_text, title, j + 1));
    }

    None
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
            // \\ immediately followed by \n => line break (kramdown rule)
            if next == '\\' {
                if i + 2 < end && chars[i + 2] == '\n' {
                    // \\ + \n => <br />
                    output.push_str("<br />\n");
                    i += 3;
                    continue;
                } else if i + 2 >= end {
                    // \\ at end of text => literal backslash
                    output.push('\\');
                    i += 2;
                    continue;
                }
                // \\ followed by other chars: treat as escaped backslash (literal \)
                // Don't consume here; fall through to is_escapable_char below
            }
            if is_escapable_char(next) {
                // Special case: \$$ at start of span text.
                // In kramdown, \$$ at the beginning of a block cancels block math
                // but the \ is consumed and $$ starts inline math.
                // So \$$5+5$$ → \(5+5\) (the backslash is dropped, $$ becomes math)
                if next == '$' && i + 2 < end && chars[i + 2] == '$' && i == start {
                    // Drop the backslash, let $$ be processed as inline math
                    i += 1;
                    continue;
                }
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
                // Parse and apply inline options extension
                if let Some(end_pos) = remaining.find("/}") {
                    let opts_str = &remaining[10..end_pos].trim();
                    apply_inline_options(opts_str, ctx);
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
                let unescaped = unescape_kramdown_in_math(&math_content);
                if let Some(ref _engine) = ctx.options.math_engine {
                    output.push_str("\\(");
                    output.push_str(&escape_html_str(&unescaped));
                    output.push_str("\\)");
                } else {
                    output.push_str("<span class=\"kdmath\">$");
                    output.push_str(&escape_html_str(&unescaped));
                    output.push_str("$</span>");
                }
                i += advance;
                continue;
            }
        }

        // Backtick code span
        if chars[i] == '`' {
            if let Some((content, advance)) = try_parse_code_span(chars, i, end) {
                // Check for IAL(s) after code span before building the tag
                let mut after = i + advance;
                let mut all_ial_attrs: Vec<(String, String)> = Vec::new();
                while let Some((ial_attrs, ial_len)) = try_parse_span_ial(chars, after, end) {
                    all_ial_attrs.extend(ial_attrs);
                    after += ial_len;
                }

                // Extract language from IAL class (e.g., "language-ruby" -> "ruby")
                let lang = all_ial_attrs.iter().find_map(|(k, v)| {
                    if k == "class" {
                        v.strip_prefix("language-").map(|l| l.to_string())
                    } else {
                        None
                    }
                });

                // Try syntax highlighting if a language is detected and span highlighting
                // is not disabled
                let span_disabled = ctx.options.syntax_highlighter_opts.span_disable;
                let highlighted_content = if span_disabled {
                    None
                } else {
                    lang.as_deref().and_then(|lang_name| {
                        highlight_code(lang_name, &content)
                            .map(|h| h.trim_end_matches('\n').to_string())
                    })
                };

                // Build class list
                let mut classes = Vec::new();

                // Add base highlighter-rouge class if syntax_highlighter + guess_lang
                if ctx.options.syntax_highlighter.is_some()
                    && ctx.options.syntax_highlighter_opts.guess_lang == Some(true)
                {
                    classes.push("highlighter-rouge".to_string());
                }

                // Add IAL classes
                for (k, v) in &all_ial_attrs {
                    if k == "class" {
                        classes.push(v.clone());
                    }
                }

                // If we have highlighting, ensure highlighter-rouge is in the class list
                if highlighted_content.is_some() {
                    if !classes.iter().any(|c| c == "highlighter-rouge") {
                        classes.push("highlighter-rouge".to_string());
                    }
                }

                // Build attributes string
                let mut attrs_str = String::new();
                if !classes.is_empty() {
                    attrs_str.push_str(&format!(" class=\"{}\"", classes.join(" ")));
                }
                // Add non-class IAL attrs (id and others)
                if let Some((_, id_val)) = all_ial_attrs.iter().find(|(k, _)| k == "id") {
                    attrs_str.push_str(&format!(" id=\"{id_val}\""));
                }
                for (k, v) in &all_ial_attrs {
                    if k != "class" && k != "id" {
                        attrs_str.push_str(&format!(" {k}=\"{v}\""));
                    }
                }

                output.push_str(&format!("<code{attrs_str}>"));
                if let Some(ref highlighted) = highlighted_content {
                    output.push_str(highlighted);
                } else {
                    output.push_str(&escape_html_str(&content));
                }
                output.push_str("</code>");

                if !all_ial_attrs.is_empty() {
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

        // Image link: ![alt](url) or ![alt][ref] (allowed inside links too)
        if chars[i] == '!' && i + 1 < end && chars[i + 1] == '[' {
            if let Some((html, advance)) = try_parse_image(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                continue;
            }
        }

        // Link: [text](url) or [text][ref] or [ref]
        if chars[i] == '[' && !in_link {
            if let Some((html, advance)) = try_parse_link(chars, i, end, ctx) {
                let mut after = i + advance;
                // Check for IAL(s) after link
                let mut all_ial_attrs: Vec<(String, String)> = Vec::new();
                while let Some((ial_attrs, ial_len)) = try_parse_span_ial(chars, after, end) {
                    all_ial_attrs.extend(ial_attrs);
                    after += ial_len;
                }
                if !all_ial_attrs.is_empty() {
                    output.push_str(&apply_ial_to_a_tag(&html, &all_ial_attrs));
                    i = after;
                } else {
                    output.push_str(&html);
                    i += advance;
                }
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
                // Output extra spaces before the line break (kramdown only consumes 2)
                for _ in 0..space_count.saturating_sub(2) {
                    output.push(' ');
                }
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
#[allow(dead_code)]
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

/// Apply IAL attributes to an `<a ...>` tag string.
/// Inserts the IAL attributes into the opening `<a` tag, right before the `>`.
fn apply_ial_to_a_tag(html: &str, attrs: &[(String, String)]) -> String {
    // Find the first `<a` opening tag and its closing `>`
    let a_open = if let Some(pos) = html.find("<a ") {
        pos
    } else if let Some(pos) = html.find("<a>") {
        pos
    } else {
        return html.to_string();
    };

    // Find the closing > of this <a tag
    if let Some(close_gt) = html[a_open..].find('>') {
        let close_pos = a_open + close_gt;
        let attrs_str = format_attrs(attrs);
        let mut result = String::with_capacity(html.len() + attrs_str.len());
        result.push_str(&html[..close_pos]);
        result.push_str(&attrs_str);
        result.push_str(&html[close_pos..]);
        result
    } else {
        html.to_string()
    }
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

/// Unescape kramdown backslash escapes for braces inside math content.
/// In kramdown, `\{` becomes `{` and `\}` becomes `}` even inside math delimiters.
/// Other backslash sequences (like `\\` for line break or `\text` for LaTeX commands)
/// are left as-is, matching Jekyll's kramdown behavior.
pub fn unescape_kramdown_in_math(content: &str) -> String {
    let chars: Vec<char> = content.chars().collect();
    let mut result = String::with_capacity(content.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && (chars[i + 1] == '{' || chars[i + 1] == '}') {
            result.push(chars[i + 1]);
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
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
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some((trimmed.to_string(), i + 2 - start));
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

        // Extract tag name (may include : for XML namespaced tags)
        let tag_name_end = inner
            .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .unwrap_or(inner.len());
        let tag_name_raw = &inner[..tag_name_end];
        let tag_name = tag_name_raw.to_lowercase();
        // XML tags are: namespaced (contains ':') or unknown tags with mixed case.
        // Known HTML tags like <sPAn> are treated as regular HTML and normalized.
        let has_mixed_case = tag_name_raw.chars().any(|c| c.is_uppercase());
        let is_known_html = is_valid_span_tag(&tag_name)
            || is_valid_block_tag(&tag_name)
            || is_void_element(&tag_name);
        let is_xml_tag = tag_name_raw.contains(':') || (has_mixed_case && !is_known_html);

        if tag_name.is_empty() {
            return None;
        }

        // Tag name must start with an ASCII letter
        if !tag_name.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return None;
        }

        // Tag name must only contain valid characters (alphanumeric, -, _, ., :)
        if !tag_name_raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
        {
            return None;
        }

        // Must be a valid HTML tag or XML namespaced tag (contains :)
        let is_known_tag = is_valid_span_tag(&tag_name) || is_valid_block_tag(&tag_name);
        if !is_known_tag && !tag_name_raw.contains(':') && !is_xml_tag {
            return None;
        }

        // Block-level tags in span context are escaped (but not XML tags)
        if !is_xml_tag && is_block_level_tag(&tag_name) && !is_also_span_tag(&tag_name) {
            return None;
        }

        let is_self_closing = inner.trim_end().ends_with('/');
        let is_void = is_void_element(&tag_name);

        // Check for markdown attribute and determine processing mode
        let attrs = parse_html_attrs(if inner.len() > tag_name_end {
            &inner[tag_name_end..]
        } else {
            ""
        });
        let markdown_val = attrs
            .iter()
            .find(|(k, _)| k == "markdown")
            .and_then(|(_, v)| v.clone());

        // Normalize HTML attributes (preserve case for XML tags), removing markdown attr
        let normalized = if is_xml_tag {
            remove_attr_from_tag(&normalize_xml_tag(tag_str), "markdown")
        } else {
            remove_attr_from_tag(&normalize_html_tag(tag_str), "markdown")
        };

        if is_void {
            // Void elements: <br />, <img ... /> - always self-closing
            let normalized = ensure_self_closing(&normalized, &tag_name);
            return Some((normalized, gt + 1));
        }

        if is_self_closing && !is_void {
            // Non-void self-closing tags like <span ... /> are expanded to <tag></tag>
            // Remove the trailing / from the normalized tag
            let expanded = if let Some(slash_pos) = normalized.rfind('/') {
                let before_slash = normalized[..slash_pos].trim_end();
                format!("{}></{}>", before_slash, tag_name)
            } else {
                format!("{}</{}>", normalized, tag_name)
            };
            return Some((expanded, gt + 1));
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
        // For XML tags: use case-sensitive matching with original name
        // For HTML tags: use case-insensitive matching
        let close_tag_name = if is_xml_tag {
            tag_name_raw.to_string()
        } else {
            tag_name.clone()
        };
        let close_tag_pattern = format!("</{}>", close_tag_name);
        let search_start = start + gt + 1;

        let rest: String = chars[search_start..end].iter().collect();
        let close_pos = if is_xml_tag {
            // Case-sensitive search for XML tags
            rest.find(&close_tag_pattern)
        } else {
            find_case_insensitive(&rest, &close_tag_pattern)
        };

        if let Some(cp) = close_pos {
            let inner_chars: Vec<char> = rest[..cp].chars().collect();
            let mut inner_html = String::new();

            // Determine if content should be markdown-processed
            let should_process = match markdown_val.as_deref() {
                Some("0") => false,
                Some("1") | Some("span") | Some("block") => true,
                None => {
                    // When parse_span_html is false, don't process content
                    ctx.options.parse_span_html
                        && (is_markdown_processable_tag(&tag_name) || is_xml_tag)
                }
                _ => ctx.options.parse_span_html && is_markdown_processable_tag(&tag_name),
            };

            if should_process {
                parse_spans(
                    &inner_chars,
                    0,
                    inner_chars.len(),
                    ctx,
                    &mut inner_html,
                    false,
                );
            } else {
                // markdown="0": no markdown processing, but still handle nested
                // HTML tags with markdown attributes. Also escape autolinks.
                let raw_content = &rest[..cp];
                inner_html = process_raw_html_content(raw_content, ctx);
            }

            let total_advance = gt + 1 + cp + close_tag_pattern.len();
            return Some((
                format!("{normalized}{inner_html}</{close_tag_name}>"),
                total_advance,
            ));
        }

        // No closing tag - auto-close at end of content
        let rest_content: String = chars[search_start..end].iter().collect();
        let should_process = match markdown_val.as_deref() {
            Some("0") => false,
            Some("1") | Some("span") | Some("block") => true,
            None => is_markdown_processable_tag(&tag_name) || is_xml_tag,
            _ => is_markdown_processable_tag(&tag_name),
        };
        if should_process {
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
                format!("{normalized}{inner_html}</{close_tag_name}>"),
                total_advance,
            ));
        }

        let total_advance = end - start;
        return Some((
            format!("{normalized}{rest_content}</{close_tag_name}>"),
            total_advance,
        ));
    }

    None
}

/// Process content inside a markdown="0" HTML element.
/// Does not apply markdown processing, but still handles nested HTML tags
/// that have explicit markdown attributes. Also escapes autolinks.
fn process_raw_html_content(content: &str, ctx: &mut SpanContext) -> String {
    let mut result = String::new();
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '<' {
            // In markdown="0" mode, autolinks should be escaped, not linkified.
            // Check if this looks like an autolink before trying HTML tag parsing.
            let rest: String = chars[i..len].iter().collect();
            let is_autolink = rest.len() > 1
                && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic())
                && (rest.contains("://") || rest.contains("mailto:"))
                && rest.contains('>');

            if !is_autolink {
                // Try to parse as a valid HTML tag
                if let Some((html_out, advance)) = try_parse_html_span(&chars, i, len, ctx) {
                    result.push_str(&html_out);
                    i += advance;
                    continue;
                }
            }

            // Not a valid HTML tag or an autolink - escape the angle bracket
            result.push_str("&lt;");
            i += 1;
            // Find the matching > and escape it too
            while i < len && chars[i] != '>' {
                result.push(chars[i]);
                i += 1;
            }
            if i < len && chars[i] == '>' {
                result.push_str("&gt;");
                i += 1;
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }
    result
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
    // Tags whose inner content should be processed for markdown.
    // Raw-like tags (kbd, samp, var) are NOT processed to preserve literal content.
    matches!(
        tag,
        "a" | "abbr"
            | "b"
            | "bdi"
            | "bdo"
            | "button"
            | "cite"
            | "del"
            | "dfn"
            | "em"
            | "i"
            | "ins"
            | "label"
            | "mark"
            | "q"
            | "s"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "time"
            | "u"
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
            // Normalize newlines in attribute values to spaces
            let normalized_val = val.replace('\n', " ");
            result.push_str(&format!(" {name_lower}=\"{normalized_val}\""));
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

/// Normalize an XML tag: preserve case for tag name and attribute names,
/// but normalize attribute quotes to double quotes.
fn normalize_xml_tag(tag: &str) -> String {
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

    let parts: Vec<&str> = inner.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0]; // preserve case
    let attr_str = if parts.len() > 1 { parts[1] } else { "" };

    if attr_str.is_empty() {
        if is_self_closing {
            return format!("<{tag_name} />");
        }
        return format!("<{tag_name}>");
    }

    let attrs = parse_html_attrs(attr_str);
    let mut result = format!("<{tag_name}");
    for (name, value) in &attrs {
        // Preserve case for XML attribute names
        if let Some(val) = value {
            let normalized_val = val.replace('\n', " ");
            result.push_str(&format!(" {name}=\"{normalized_val}\""));
        } else {
            result.push_str(&format!(" {name}=\"\""));
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

/// Remove an attribute from an HTML tag string.
fn remove_attr_from_tag(tag: &str, attr_name: &str) -> String {
    // Simple approach: use regex-like removal of attr="value"
    // Handle: attr="value", attr='value', attr=value
    let mut result = tag.to_string();
    let patterns = [
        format!(" {attr_name}=\""),
        format!(" {attr_name}='"),
        format!(" {attr_name}="),
    ];
    for pattern in &patterns {
        if let Some(start) = result.find(pattern.as_str()) {
            let after_eq = start + pattern.len();
            let quote_char = if pattern.ends_with('"') {
                Some('"')
            } else if pattern.ends_with('\'') {
                Some('\'')
            } else {
                None
            };
            if let Some(qc) = quote_char {
                // Find closing quote
                if let Some(end_q) = result[after_eq..].find(qc) {
                    let end = after_eq + end_q + 1;
                    result = format!("{}{}", &result[..start], &result[end..]);
                }
            } else {
                // Unquoted value: ends at space or >
                let end = result[after_eq..]
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .map(|p| after_eq + p)
                    .unwrap_or(result.len());
                result = format!("{}{}", &result[..start], &result[end..]);
            }
        }
    }
    result
}

/// Try to parse emphasis/strong following the kramdown algorithm.
///
/// Kramdown matches 1 or 2 markers at a time (never 3+).
/// `***` is handled by matching `*` then recursively finding `**` inside.
///
/// Key rules:
/// - Same emphasis type cannot nest (e.g., can't have `<em>` inside `<em>`)
/// - For `**`, if no closing `**` found and not inside `:em`, fallback to `*`
/// - Closing delimiter must not be preceded by whitespace
/// - For `:em`, closing `*` must not be followed by `**` (without `***`)
/// - For `_`, closing must not be followed by alphanumeric
fn try_parse_emphasis(
    chars: &[char],
    start: usize,
    end: usize,
    ctx: &mut SpanContext,
    in_link: bool,
) -> Option<(String, usize)> {
    let marker = chars[start];

    // Count consecutive markers (kramdown only uses 1 or 2 at a time)
    let mut total_markers = 0;
    {
        let mut j = start;
        while j < end && chars[j] == marker {
            total_markers += 1;
            j += 1;
        }
    }
    if total_markers == 0 {
        return None;
    }

    // Determine: 1 marker = em, 2 markers = strong
    let use_count = std::cmp::min(total_markers, 2);
    let element: u8 = if use_count == 2 { 2 } else { 1 }; // 1=em, 2=strong
    let content_start = start + use_count;

    // Opening delimiter rules:
    // Must not be followed by whitespace
    if content_start >= end
        || chars[content_start] == ' '
        || chars[content_start] == '\n'
        || chars[content_start] == '\t'
    {
        return None;
    }

    // For underscore: check word-boundary rule.
    // Kramdown: /[[:alpha:]]-?[[:alpha:]]*_*\z/ on pre_match
    // Meaning: the text before ends with alpha (optionally hyphen+alpha) followed by
    // zero or more underscores. So "word_" blocks, but just "_" or "__" doesn't.
    if marker == '_' && start > 0 {
        // Skip past any trailing underscores before start
        let mut j = start;
        while j > 0 && chars[j - 1] == '_' {
            j -= 1;
        }
        // Now check if there's an alphabetic char (the kramdown regex core)
        if j > 0 && chars[j - 1].is_alphabetic() {
            return None;
        }
    }

    // Stack check: can't nest same element type
    // If tree already contains this element type, or stack has it, output as literal
    if ctx.emphasis_stack.contains(&element) {
        return None;
    }

    // For * marker with space before: kramdown doesn't open emphasis
    // "lonely * here*" -> the * preceded by space doesn't open
    if marker == '*' && start > 0 && chars[start - 1] == ' ' {
        // Check if followed by space too - definitely not emphasis
        if chars[content_start] == ' ' {
            return None;
        }
        // Preceded by space but not followed by space: still could be emphasis
        // But kramdown only opens if it can find a valid close
    }

    // Save footnote state before trying emphasis (we may need to restore if emphasis fails)
    let saved_footnote_counter = ctx.footnote_counter;
    let saved_footnote_order = ctx.footnote_order.clone();
    let saved_footnote_ref_counts = ctx.footnote_ref_counts.clone();

    // Try to parse spans until we find a matching close delimiter
    let sub_parse_result = parse_spans_until_emphasis_close(
        chars,
        content_start,
        end,
        marker,
        use_count,
        element,
        ctx,
        in_link,
    );

    if let Some((inner_html, close_pos)) = sub_parse_result {
        // Successfully found closing delimiter
        let tag = if element == 2 { "strong" } else { "em" };
        let html = format!("<{tag}>{inner_html}</{tag}>");
        let advance = close_pos - start;
        return Some((html, advance));
    }

    // Emphasis failed - restore footnote state to avoid double-counting
    ctx.footnote_counter = saved_footnote_counter;
    ctx.footnote_order = saved_footnote_order;
    ctx.footnote_ref_counts = saved_footnote_ref_counts;

    // If strong failed and we're not inside em, try fallback to single marker (em)
    if element == 2 && !ctx.emphasis_stack.contains(&1) {
        // Save state again for the fallback attempt
        let saved_footnote_counter2 = ctx.footnote_counter;
        let saved_footnote_order2 = ctx.footnote_order.clone();
        let saved_footnote_ref_counts2 = ctx.footnote_ref_counts.clone();

        // Revert: only consume 1 marker, try em
        let content_start_1 = start + 1;
        if content_start_1 < end
            && chars[content_start_1] != ' '
            && chars[content_start_1] != '\n'
            && chars[content_start_1] != '\t'
        {
            let fallback_result = parse_spans_until_emphasis_close(
                chars,
                content_start_1,
                end,
                marker,
                1,
                1, // em
                ctx,
                in_link,
            );
            if let Some((inner_html, close_pos)) = fallback_result {
                let html = format!("<em>{inner_html}</em>");
                let advance = close_pos - start;
                return Some((html, advance));
            }
        }

        // Fallback also failed - restore state
        ctx.footnote_counter = saved_footnote_counter2;
        ctx.footnote_order = saved_footnote_order2;
        ctx.footnote_ref_counts = saved_footnote_ref_counts2;
    }

    None
}

/// Parse spans until a matching emphasis close delimiter is found.
///
/// This implements kramdown's recursive span parsing with stop condition.
/// Returns Some((inner_html, position_after_close_delimiter)) on success.
#[allow(clippy::too_many_arguments)]
fn parse_spans_until_emphasis_close(
    chars: &[char],
    start: usize,
    end: usize,
    marker: char,
    close_count: usize,
    element: u8, // 1=em, 2=strong
    ctx: &mut SpanContext,
    in_link: bool,
) -> Option<(String, usize)> {
    // We parse character by character, similar to parse_spans, but also check for
    // the closing delimiter at each position where we see the marker char.
    let mut output = String::new();
    let mut i = start;
    let mut has_content = false; // Must have non-empty children

    // Push our element type onto the emphasis stack
    ctx.emphasis_stack.push(element);

    let result = loop {
        if i >= end {
            break None; // Reached end without finding close
        }

        // Check for closing delimiter before processing other spans
        if chars[i] == marker {
            let delim_start = i;
            let mut delim_count = 0;
            let mut j = i;
            while j < end && chars[j] == marker {
                delim_count += 1;
                j += 1;
            }

            // Check closing conditions (kramdown's stop_re check):
            // 1. Must be preceded by non-whitespace (right-flanking)
            let preceded_by_nonspace = delim_start > start
                && chars[delim_start - 1] != ' '
                && chars[delim_start - 1] != '\n'
                && chars[delim_start - 1] != '\t';

            // 2. For :em, must not be followed by double delimiter without triple
            //    i.e., `*` closing em must not be at `***` (followed by **)
            //    but `*` at `*` or `****` is OK
            // For :em, the closing delimiter must not be at a position where
            // there are exactly 2 consecutive markers (which would be a strong
            // delimiter). Kramdown checks: !@src.match?(/\*\*(?!\*)/)
            // This means: if there are exactly 2 markers here, em can't close.
            // If 1 or 3+, em CAN close.
            let em_double_check = if element == 1 { delim_count != 2 } else { true };

            // 3. For _ type, closing delimiter must not be followed by alphanumeric.
            // Kramdown: !@src.match?(/_{close_count}[[:alnum:]]/)
            let underscore_check = if marker == '_' {
                let after_close = delim_start + close_count;
                !(after_close < end && chars[after_close].is_alphanumeric())
            } else {
                true
            };

            // 4. Must have content (non-empty children)
            let content_check = has_content;

            if preceded_by_nonspace
                && em_double_check
                && underscore_check
                && content_check
                && delim_count >= close_count
            {
                // Valid close found! Consume close_count markers.
                let close_end = delim_start + close_count;
                break Some((output, close_end));
            }

            // Not a valid close - treat as content and try to parse as emphasis opener
            // or fall through to regular span parsing
        }

        // Regular span parsing (mirror of parse_spans logic)
        // Backslash escape
        if chars[i] == '\\' && i + 1 < end {
            let next = chars[i + 1];
            if next == '\\' {
                if i + 2 < end && chars[i + 2] == '\n' {
                    output.push_str("<br />\n");
                    i += 3;
                    has_content = true;
                    continue;
                } else if i + 2 >= end {
                    output.push('\\');
                    i += 2;
                    has_content = true;
                    continue;
                }
            }
            if is_escapable_char(next) {
                output.push_str(&escape_html_char(next));
                i += 2;
                has_content = true;
                continue;
            }
        }

        // Backtick code span
        if chars[i] == '`' {
            if let Some((content, advance)) = try_parse_code_span(chars, i, end) {
                output.push_str("<code>");
                output.push_str(&escape_html_str(&content));
                output.push_str("</code>");
                i += advance;
                has_content = true;
                continue;
            }
            output.push('`');
            i += 1;
            has_content = true;
            continue;
        }

        // Link
        if chars[i] == '[' && !in_link {
            if let Some((html, advance)) = try_parse_link(chars, i, end, ctx) {
                let mut after = i + advance;
                // Check for IAL(s) after link
                let mut all_ial_attrs: Vec<(String, String)> = Vec::new();
                while let Some((ial_attrs, ial_len)) = try_parse_span_ial(chars, after, end) {
                    all_ial_attrs.extend(ial_attrs);
                    after += ial_len;
                }
                if !all_ial_attrs.is_empty() {
                    output.push_str(&apply_ial_to_a_tag(&html, &all_ial_attrs));
                    i = after;
                } else {
                    output.push_str(&html);
                    i += advance;
                }
                has_content = true;
                continue;
            }
        }

        // Image
        if chars[i] == '!' && i + 1 < end && chars[i + 1] == '[' {
            if let Some((html, advance)) = try_parse_image(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // Nested emphasis (recursive)
        if (chars[i] == '*' || chars[i] == '_') && i < end {
            if let Some((html, advance)) = try_parse_emphasis(chars, i, end, ctx, in_link) {
                output.push_str(&html);
                i += advance;
                has_content = true;
                continue;
            }
            // Emphasis failed - output markers as literal text.
            // Kramdown's EMPHASIS_START matches 1 or 2 markers; on failure,
            // all matched markers are added as literal text.
            let emph_marker = chars[i];
            let mut mc = 0;
            let mut j = i;
            while j < end && chars[j] == emph_marker && mc < 2 {
                mc += 1;
                j += 1;
            }
            for _ in 0..mc {
                output.push(emph_marker);
            }
            i += mc;
            has_content = true;
            continue;
        }

        // HTML comment
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
                has_content = true;
                continue;
            }
        }

        // Autolinks
        if chars[i] == '<' {
            if let Some((link_html, advance)) = try_parse_autolink(chars, i, end, ctx) {
                output.push_str(&link_html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // HTML span elements
        if chars[i] == '<' {
            if let Some((html, advance)) = try_parse_html_span(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // Smart quotes and typography
        if chars[i] == '"' || chars[i] == '\'' {
            if let Some((html, advance)) = try_parse_smart_quote(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // Guillemets
        if chars[i] == '<' && i + 1 < end && chars[i + 1] == '<' {
            let (html, advance) = parse_guillemet_open(chars, i, end, ctx);
            output.push_str(&html);
            i += advance;
            has_content = true;
            continue;
        }
        if chars[i] == '>' && i + 1 < end && chars[i + 1] == '>' {
            let (html, advance) = parse_guillemet_close(chars, i, end, ctx);
            output.push_str(&html);
            i += advance;
            has_content = true;
            continue;
        }

        // Em-dash and en-dash
        if chars[i] == '-' && i + 1 < end && chars[i + 1] == '-' {
            if i + 2 < end && chars[i + 2] == '-' {
                output.push_str(&entity_str("mdash", ctx));
                i += 3;
                has_content = true;
                continue;
            }
            output.push_str(&entity_str("ndash", ctx));
            i += 2;
            has_content = true;
            continue;
        }

        // Ellipsis
        if chars[i] == '.' && i + 2 < end && chars[i + 1] == '.' && chars[i + 2] == '.' {
            output.push_str(&entity_str("hellip", ctx));
            i += 3;
            has_content = true;
            continue;
        }

        // Entity references
        if chars[i] == '&' {
            if let Some((entity_html, advance)) = try_parse_entity(chars, i, end, ctx) {
                output.push_str(&entity_html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // Footnote references
        if chars[i] == '[' && i + 1 < end && chars[i + 1] == '^' {
            if let Some((html, advance)) = try_parse_footnote_ref(chars, i, end, ctx) {
                output.push_str(&html);
                i += advance;
                has_content = true;
                continue;
            }
        }

        // HTML entities
        if chars[i] == '&' {
            output.push_str("&amp;");
            i += 1;
            has_content = true;
            continue;
        }
        if chars[i] == '<' {
            output.push_str("&lt;");
            i += 1;
            has_content = true;
            continue;
        }
        if chars[i] == '>' {
            output.push_str("&gt;");
            i += 1;
            has_content = true;
            continue;
        }

        // Regular character
        output.push(chars[i]);
        i += 1;
        has_content = true;
    };

    // Pop our element type from the emphasis stack
    ctx.emphasis_stack.pop();

    result
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

fn format_link_def_attrs(attrs: &[(String, String)]) -> String {
    if attrs.is_empty() {
        return String::new();
    }
    let mut result = String::new();
    // Output attributes in insertion order (matching kramdown Ruby behavior)
    for (k, v) in attrs {
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
    // Normalize whitespace in alt attribute: collapse newlines and tabs to spaces
    // (standard HTML attribute value normalization per the spec)
    let alt_text = alt_text.replace(['\n', '\t'], " ");
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
        // Special case: contractions, possessives, and closing after other close chars
        // Kramdown SQ_CLOSE: anything that's not space/escape/bracket/paren
        if start > 0 {
            let prev = chars[start - 1];
            // Special case: "'word" or '"word" patterns -> opening quote combo
            // In kramdown: '"(?=\w) -> lsquo, ldquo; "'(?=\w) -> rsquo, lsquo
            if (prev == '"' || prev == '\'')
                && start + 1 < end
                && chars[start + 1].is_alphanumeric()
            {
                // Don't treat as closing; fall through to opening logic below
            } else {
                let is_close_context = prev.is_alphanumeric()
                    || prev == '>'
                    || prev == ')'
                    || prev == ']'
                    || prev == '"'
                    || prev == '\''
                    || prev == '.'
                    || prev == '!'
                    || prev == '?'
                    || prev == ','
                    || prev == ';'
                    || prev == ':'
                    || prev == '-'
                    || prev == '}'
                    || prev == '\u{2019}' // rsquo
                    || prev == '\u{201D}'; // rdquo
                if is_close_context {
                    return Some((entity_str(rsquo, ctx), 1));
                }
            }
        }

        // Special case: decade abbreviation ('80s, '90s, etc.)
        if start + 3 < end
            && chars[start + 1].is_ascii_digit()
            && chars[start + 2].is_ascii_digit()
            && chars[start + 3] == 's'
        {
            return Some((entity_str(rsquo, ctx), 1));
        }

        // Rule 1: quote before emphasis markers (_*) followed by non-space -> opening
        if start + 1 < end && (chars[start + 1] == '_' || chars[start + 1] == '*') {
            // Check there's a non-space char after the markers
            let mut j = start + 1;
            while j < end && (chars[j] == '_' || chars[j] == '*') {
                j += 1;
            }
            if j < end && !chars[j].is_whitespace() {
                return Some((entity_str(lsquo, ctx), 1));
            }
        }

        let is_opening = is_opening_quote(chars, start, end);
        if is_opening {
            // Check if followed by word character or quote+word (kramdown opening rules)
            if start + 1 < end {
                let next = chars[start + 1];
                if next.is_alphanumeric()
                    || next == '"'
                    || next == '\''
                    || next == '`'
                    || next == '*'
                    || next == '_'
                    || next == '!'
                    || next == '.'
                {
                    return Some((entity_str(lsquo, ctx), 1));
                }
            }
            // Otherwise it's an unmatched opening - use rsquo for safety
            return Some((entity_str(rsquo, ctx), 1));
        }

        // Before space, end of string, or 's' word boundary -> rsquo
        if start + 1 >= end
            || chars[start + 1].is_whitespace()
            || (chars[start + 1] == 's'
                && (start + 2 >= end || !chars[start + 2].is_alphanumeric()))
        {
            return Some((entity_str(rsquo, ctx), 1));
        }

        // Remaining single quotes are opening (kramdown fallback rule)
        return Some((entity_str(lsquo, ctx), 1));
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
        return next.is_some_and(|c| {
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
    next.is_some_and(|c| {
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

#[allow(dead_code)]
fn has_matching_single_close(chars: &[char], start: usize, end: usize) -> bool {
    let mut i = start;
    while i < end {
        if chars[i] == '\'' {
            // Check if this is a valid closing ' (adjacent or preceded by non-space)
            if i == start || (i > start && chars[i - 1] != ' ' && chars[i - 1] != '\n') {
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
                let ch_str =
                    entities::resolve_named_entity(&entity_content).unwrap_or(&entity_content);
                Some((ch_str.to_string(), advance))
            }
            EntityOutput::Symbolic => {
                // Try to find symbolic name
                Some((original, advance))
            }
            EntityOutput::Numeric => {
                let ch_str =
                    entities::resolve_named_entity(&entity_content).unwrap_or(&entity_content);
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
    let prefixed_name = if prefix.is_empty() {
        name.clone()
    } else {
        format!("{prefix}{name}")
    };
    let fn_id = format!("fn:{prefixed_name}");

    // Track reference count for duplicate IDs
    let ref_count = ctx.footnote_ref_counts.entry(name.clone()).or_insert(0);
    *ref_count += 1;
    let suffix = if *ref_count > 1 {
        format!(":{}", *ref_count - 1)
    } else {
        String::new()
    };
    let fnref_id = format!("fnref:{prefixed_name}{suffix}");

    // Format the link text using footnote_link_text option
    let link_text = if ctx.options.footnote_link_text.is_empty() {
        fn_number.to_string()
    } else {
        ctx.options
            .footnote_link_text
            .replace("%s", &fn_number.to_string())
    };

    Some((
        format!(
            "<sup id=\"{fnref_id}\"><a href=\"#{fn_id}\" class=\"footnote\" rel=\"footnote\" role=\"doc-noteref\">{link_text}</a></sup>",
        ),
        i + 1 - start,
    ))
}

/// Get the footnote data needed for rendering. Returns Vec of (name, content, ref_count).
/// The actual rendering is done in html.rs which has access to the block parser.
pub fn get_footnote_data(ctx: &SpanContext) -> Vec<(String, String, usize)> {
    ctx.footnote_order
        .iter()
        .map(|name| {
            let content = ctx.footnote_defs.get(name).cloned().unwrap_or_default();
            let ref_count = ctx.footnote_ref_counts.get(name).copied().unwrap_or(1);
            (name.clone(), content, ref_count)
        })
        .collect()
}

/// Convert backlink text to HTML entity form.
/// Default backlink chars U+21A9 U+FE0E become `&#8617;`.
/// Also escapes HTML special chars.
pub fn encode_backlink_text(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        // Variation selectors are ignored in entity output
        if ('\u{FE00}'..='\u{FE0F}').contains(&ch) {
            continue;
        }
        match ch {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            c if !c.is_ascii() => {
                result.push_str(&format!("&#{};", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

/// Apply abbreviations to HTML output, replacing whole-word occurrences
/// but not inside HTML tags.
fn apply_abbreviations(
    html: &str,
    abbreviations: &HashMap<String, String>,
    abbreviation_attrs: &HashMap<String, Vec<(String, String)>>,
) -> String {
    if abbreviations.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();

    // Sort abbreviations by length (longest first) to avoid partial matches
    let mut abbrs: Vec<(&String, &String)> = abbreviations.iter().collect();
    abbrs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (abbr, full) in abbrs {
        // Build attribute string from IAL attrs
        let extra_attrs = if let Some(attrs) = abbreviation_attrs.get(abbr.as_str()) {
            let mut attr_str = String::new();
            for (k, v) in attrs {
                if k == "__ald_ref__" {
                    continue;
                }
                attr_str.push(' ');
                attr_str.push_str(k);
                attr_str.push_str("=\"");
                attr_str.push_str(&escape_html_attr(v));
                attr_str.push('"');
            }
            attr_str
        } else {
            String::new()
        };

        let replacement = if full.is_empty() {
            format!("<abbr{extra_attrs}>{abbr}</abbr>")
        } else {
            let escaped_full = escape_html_attr(full);
            format!("<abbr{extra_attrs} title=\"{escaped_full}\">{abbr}</abbr>")
        };
        result = replace_outside_tags(&result, abbr, &replacement);
    }

    result
}

/// Check if a character is whitespace for abbreviation matching purposes.
/// Matches both standard whitespace and Unicode space separators (like \u{a0}).
fn is_abbr_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{a0}'
}

/// Try to match abbreviation at position i in chars, with flexible whitespace matching.
/// Returns the length of the match if successful, or None.
/// Spaces in the search string match any sequence of whitespace/nbsp characters.
fn try_match_abbr(chars: &[char], i: usize, search_chars: &[char]) -> Option<usize> {
    let mut ci = i;
    let mut si = 0;

    while si < search_chars.len() {
        if ci >= chars.len() {
            return None;
        }
        if search_chars[si] == ' ' {
            // Search has a space: match one or more whitespace chars in the text
            if !is_abbr_whitespace(chars[ci]) {
                return None;
            }
            // Consume all whitespace in the search
            while si < search_chars.len() && search_chars[si] == ' ' {
                si += 1;
            }
            // Consume one or more whitespace in the text
            ci += 1;
            while ci < chars.len() && is_abbr_whitespace(chars[ci]) {
                ci += 1;
            }
        } else {
            if chars[ci] != search_chars[si] {
                return None;
            }
            ci += 1;
            si += 1;
        }
    }

    Some(ci - i)
}

/// Replace text outside of HTML tags (not inside <...> or attributes).
/// For abbreviations containing spaces, matches any whitespace sequence (including
/// newlines and nbsp) and preserves the original matched text in the replacement.
fn replace_outside_tags(html: &str, search: &str, replacement: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = html.chars().collect();
    let search_chars: Vec<char> = search.chars().collect();
    let has_space = search.contains(' ');
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

        // Try to match abbreviation at this position
        let match_len = if has_space {
            try_match_abbr(&chars, i, &search_chars)
        } else if i + search_chars.len() <= chars.len() {
            let candidate: String = chars[i..i + search_chars.len()].iter().collect();
            if candidate == *search {
                Some(search_chars.len())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(mlen) = match_len {
            // Check word boundaries
            let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
            let after_ok = i + mlen >= chars.len() || !chars[i + mlen].is_alphanumeric();
            if before_ok && after_ok {
                if has_space {
                    // For multi-word abbreviations, preserve the actual matched text
                    // The replacement has the format <abbr ...>SEARCH</abbr>
                    // We need to replace SEARCH with the actual matched text
                    let matched: String = chars[i..i + mlen].iter().collect();
                    let actual_replacement = replacement.replace(search, &matched);
                    result.push_str(&actual_replacement);
                } else {
                    result.push_str(replacement);
                }
                i += mlen;
                continue;
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

// kramdown parser - HTML to native element conversion
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// When html_to_native is enabled, this module converts HTML block elements
// (like <p>, <em>, <strong>, <h1>-<h6>, <ul>, <ol>, <table>, <pre>, <code>)
// into native kramdown elements so they are processed and rendered natively.

use crate::kramdown_parser::element::{Element, ElementType};
use crate::kramdown_parser::options::Options;

/// Convert inline HTML tags in the final output when html_to_native is enabled.
/// - `<b>` -> `<strong>`, `<i>` -> `<em>`
/// - Strip all tags inside `<code>` elements (html_to_native code conversion)
pub fn convert_inline_html_tags(html: &str) -> String {
    let mut result = html.to_string();

    // First, strip tags inside <code>...</code> spans
    result = strip_tags_in_code(&result);

    // Then convert <b> -> <strong>, </b> -> </strong>
    result = result.replace("<b>", "<strong>");
    result = result.replace("</b>", "</strong>");
    result = result.replace("<i>", "<em>");
    result = result.replace("</i>", "</em>");
    result
}

/// Strip HTML tags inside <code>...</code> spans, keeping only text content.
fn strip_tags_in_code(html: &str) -> String {
    let mut result = String::new();
    let mut remaining = html;

    while let Some(code_start) = remaining.find("<code>") {
        // Output everything before <code>
        result.push_str(&remaining[..code_start + 6]); // include <code>
        remaining = &remaining[code_start + 6..];

        // Find matching </code>
        if let Some(code_end) = remaining.find("</code>") {
            let code_content = &remaining[..code_end];
            // Strip all HTML tags from code content
            let stripped = strip_html_tags(code_content);
            result.push_str(&stripped);
            result.push_str("</code>");
            remaining = &remaining[code_end + 7..];
        } else {
            // No closing tag, output rest as-is
            result.push_str(remaining);
            return result;
        }
    }

    result.push_str(remaining);
    result
}

/// Strip all HTML tags from a string, keeping only text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    result
}

/// Convert HTML blocks to native elements throughout the document tree.
/// This is called after parsing when html_to_native is enabled.
pub fn convert_html_to_native(children: &mut Vec<Element>, options: &Options) {
    let mut i = 0;
    while i < children.len() {
        if children[i].element_type == ElementType::HtmlBlock {
            let html_value = children[i].value.clone().unwrap_or_default();
            if let Some(mut converted) = try_convert_block(&html_value, options) {
                // Replace the HtmlBlock with the converted native elements
                children.remove(i);
                let count = converted.len();
                for (j, elem) in converted.drain(..).enumerate() {
                    children.insert(i + j, elem);
                }
                // Merge trailing paragraph into the last converted paragraph.
                // When html_to_native converts `<p>content</p> trailing text`,
                // the trailing text becomes a separate Paragraph that should be
                // merged with the converted paragraph.
                let last_converted_idx = i + count - 1;
                if count > 0
                    && children[last_converted_idx].element_type == ElementType::Paragraph
                    && last_converted_idx + 1 < children.len()
                    && children[last_converted_idx + 1].element_type == ElementType::Paragraph
                {
                    let next_para = children.remove(last_converted_idx + 1);
                    let next_text = get_element_text_value(&next_para);
                    if !next_text.is_empty() {
                        let existing_text = get_element_text_value(&children[last_converted_idx]);
                        let merged = format!("{} {}", existing_text.trim_end(), next_text);
                        children[last_converted_idx].children.clear();
                        let text_elem = Element::with_value(ElementType::Text, merged);
                        children[last_converted_idx].children.push(text_elem);
                    }
                }
                i += count;
                continue;
            }
        }
        // Recurse into children
        convert_html_to_native(&mut children[i].children, options);
        i += 1;
    }
}

/// Get the text value from an element's children.
fn get_element_text_value(elem: &Element) -> String {
    let mut text = String::new();
    for child in &elem.children {
        if let Some(ref val) = child.value {
            text.push_str(val);
        }
    }
    text
}

/// Try to convert an HTML block string to native elements.
/// Returns None if the HTML cannot be converted (passes through as raw).
fn try_convert_block(html: &str, options: &Options) -> Option<Vec<Element>> {
    let trimmed = html.trim();

    // Parse the HTML to get a list of top-level nodes
    let nodes = parse_html_nodes(trimmed);
    if nodes.is_empty() {
        return None;
    }

    let mut result = Vec::new();
    for node in &nodes {
        match node {
            HtmlNode::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                let tag_lc = tag.to_lowercase();
                match tag_lc.as_str() {
                    "p" => {
                        let mut para = Element::new(ElementType::Paragraph);
                        // Merge attributes (excluding markdown)
                        for (k, v) in attrs {
                            if k != "markdown" {
                                para.attr.insert(k.clone(), v.clone());
                            }
                        }
                        let text_content = nodes_to_text(children);
                        let stripped = strip_whitespace_text(&text_content);
                        // Decode typographic entities (like &hellip;, &ldquo;, etc.)
                        let decoded = decode_typographic_entities(&stripped);
                        let text_elem = Element::with_value(ElementType::Text, decoded);
                        para.children.push(text_elem);
                        // Mark as pre-formatted HTML (don't markdown-process)
                        para.options
                            .insert("raw_html".to_string(), "true".to_string());
                        result.push(para);
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        let level = tag_lc[1..].parse::<usize>().unwrap_or(1);
                        let mut header = Element::new(ElementType::Header);
                        header
                            .options
                            .insert("level".to_string(), level.to_string());
                        // Copy attributes
                        for (k, v) in attrs {
                            if k != "markdown" {
                                header.attr.insert(k.clone(), v.clone());
                            }
                        }
                        // Extract text content for raw_text (used for auto_id generation)
                        let raw_text = extract_plain_text(children);
                        header.options.insert("raw_text".to_string(), raw_text);
                        // Convert children to text element
                        let text_content = nodes_to_text(children);
                        let stripped = strip_whitespace_text(&text_content);
                        let text_elem = Element::with_value(ElementType::Text, stripped);
                        header.children.push(text_elem);
                        // Mark as raw HTML (don't markdown-process)
                        header
                            .options
                            .insert("raw_html".to_string(), "true".to_string());
                        result.push(header);
                    }
                    "em" | "i" => {
                        // Inline emphasis - only convert if it's at block level
                        // (inside a paragraph context). At block level, wrap in paragraph.
                        // Actually, for block-level <em>, kramdown keeps it as-is
                        return None;
                    }
                    "ul" | "ol" => {
                        if let Some(list_elem) = convert_list(tag_lc.as_str(), attrs, children) {
                            result.push(list_elem);
                        } else {
                            return None;
                        }
                    }
                    "pre" => {
                        if let Some(code_elem) = convert_pre(children) {
                            result.push(code_elem);
                        } else {
                            return None;
                        }
                    }
                    "table" => {
                        if let Some(table_elem) = convert_table(attrs, children, options) {
                            result.push(table_elem);
                        } else {
                            return None;
                        }
                    }
                    "div" | "section" | "article" | "aside" | "header" | "footer" | "nav"
                    | "main" | "figure" | "figcaption" | "fieldset" | "form" => {
                        // Block container - process content to remove whitespace-only text nodes
                        // but keep as raw HTML block (don't convert to native element)
                        let processed = process_block_container_content(tag, attrs, children);
                        let mut elem = Element::with_value(ElementType::HtmlBlock, processed);
                        elem.options.insert("type".to_string(), "raw".to_string());
                        result.push(elem);
                    }
                    _ => {
                        // Unknown block element - keep as raw HTML
                        return None;
                    }
                }
            }
            HtmlNode::Text(text) => {
                let trimmed_text = text.trim();
                if !trimmed_text.is_empty() {
                    // Text at block level - wrap in paragraph? Or handle inline?
                    // For html_to_native at block level, text content is kept raw
                    return None;
                }
            }
            HtmlNode::Comment(_) => {
                // Comments are preserved as raw HTML
                return None;
            }
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// -----------------------------------------------------------------------
// HTML parsing (simple DOM parser)
// -----------------------------------------------------------------------

#[derive(Debug, Clone)]
enum HtmlNode {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<HtmlNode>,
        self_closing: bool,
    },
    Text(String),
    Comment(String),
}

/// Parse HTML string into a list of top-level nodes.
fn parse_html_nodes(html: &str) -> Vec<HtmlNode> {
    let mut nodes = Vec::new();
    let mut pos = 0;
    let bytes = html.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Check for comment
            if html[pos..].starts_with("<!--") {
                if let Some(end) = html[pos + 4..].find("-->") {
                    let comment_content = &html[pos..pos + 4 + end + 3];
                    nodes.push(HtmlNode::Comment(comment_content.to_string()));
                    pos += 4 + end + 3;
                    continue;
                }
            }

            // Check for CDATA
            if html[pos..].starts_with("<![CDATA[") {
                if let Some(end) = html[pos + 9..].find("]]>") {
                    let cdata_content = &html[pos + 9..pos + 9 + end];
                    nodes.push(HtmlNode::Text(cdata_content.to_string()));
                    pos += 9 + end + 3;
                    continue;
                }
            }

            // Check for closing tag
            if pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
                // Skip closing tags at top level (orphans)
                if let Some(gt) = html[pos..].find('>') {
                    pos += gt + 1;
                    continue;
                }
            }

            // Try to parse opening tag
            if let Some(parsed) = parse_opening_tag(&html[pos..]) {
                pos += parsed.bytes_consumed;
                if parsed.self_closing || is_void_element(&parsed.tag_name.to_lowercase()) {
                    nodes.push(HtmlNode::Element {
                        tag: parsed.tag_name,
                        attrs: parsed.attrs,
                        children: Vec::new(),
                        self_closing: true,
                    });
                } else {
                    let (children, consumed) = parse_children(&html[pos..], &parsed.tag_name);
                    pos += consumed;
                    nodes.push(HtmlNode::Element {
                        tag: parsed.tag_name,
                        attrs: parsed.attrs,
                        children,
                        self_closing: false,
                    });
                }
                continue;
            }

            // Not a valid tag - treat as text
            nodes.push(HtmlNode::Text("<".to_string()));
            pos += 1;
        } else {
            // Collect text until next <
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'<' {
                pos += 1;
            }
            nodes.push(HtmlNode::Text(html[start..pos].to_string()));
        }
    }

    nodes
}

/// Parse children of an element until the matching closing tag is found.
/// Returns (children, bytes consumed including closing tag).
fn parse_children(html: &str, parent_tag: &str) -> (Vec<HtmlNode>, usize) {
    let mut children = Vec::new();
    let mut pos = 0;
    let bytes = html.as_bytes();
    let parent_lc = parent_tag.to_lowercase();
    let is_known_html = is_known_html_element(&parent_lc);

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Check for closing tag matching parent
            if pos + 1 < bytes.len() && bytes[pos + 1] == b'/' {
                let close_start = pos;
                if let Some(gt) = html[pos..].find('>') {
                    let tag_name = html[pos + 2..pos + gt].trim().to_string();
                    let matches = if is_known_html {
                        tag_name.to_lowercase() == parent_lc
                    } else {
                        tag_name == *parent_tag
                    };
                    if matches {
                        return (children, pos + gt + 1);
                    }
                    // Non-matching close tag - treat as text
                    children.push(HtmlNode::Text(html[close_start..pos + gt + 1].to_string()));
                    pos += gt + 1;
                    continue;
                }
            }

            // Check for comment
            if html[pos..].starts_with("<!--") {
                if let Some(end) = html[pos + 4..].find("-->") {
                    let comment = &html[pos..pos + 4 + end + 3];
                    children.push(HtmlNode::Comment(comment.to_string()));
                    pos += 4 + end + 3;
                    continue;
                }
            }

            // Check for CDATA
            if html[pos..].starts_with("<![CDATA[") {
                if let Some(end) = html[pos + 9..].find("]]>") {
                    let cdata_content = &html[pos + 9..pos + 9 + end];
                    children.push(HtmlNode::Text(cdata_content.to_string()));
                    pos += 9 + end + 3;
                    continue;
                }
            }

            // Try to parse opening tag
            if let Some(parsed) = parse_opening_tag(&html[pos..]) {
                pos += parsed.bytes_consumed;
                if parsed.self_closing || is_void_element(&parsed.tag_name.to_lowercase()) {
                    children.push(HtmlNode::Element {
                        tag: parsed.tag_name,
                        attrs: parsed.attrs,
                        children: Vec::new(),
                        self_closing: true,
                    });
                } else {
                    let (sub_children, consumed) = parse_children(&html[pos..], &parsed.tag_name);
                    pos += consumed;
                    children.push(HtmlNode::Element {
                        tag: parsed.tag_name,
                        attrs: parsed.attrs,
                        children: sub_children,
                        self_closing: false,
                    });
                }
                continue;
            }

            // Not a valid tag
            children.push(HtmlNode::Text("<".to_string()));
            pos += 1;
        } else {
            let start = pos;
            while pos < bytes.len() && bytes[pos] != b'<' {
                pos += 1;
            }
            children.push(HtmlNode::Text(html[start..pos].to_string()));
        }
    }

    // No closing tag found - return what we have
    (children, pos)
}

/// Result of parsing an HTML opening tag.
struct ParsedTag {
    tag_name: String,
    attrs: Vec<(String, String)>,
    self_closing: bool,
    bytes_consumed: usize,
}

/// Parse an HTML opening tag from the start of the string.
fn parse_opening_tag(s: &str) -> Option<ParsedTag> {
    if !s.starts_with('<') {
        return None;
    }
    let after_lt = &s[1..];
    if after_lt.starts_with('/') || after_lt.starts_with('!') || after_lt.starts_with('?') {
        return None;
    }

    // Extract tag name
    let tag_name: String = after_lt
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    if tag_name.is_empty() {
        return None;
    }

    let after_name = &after_lt[tag_name.len()..];

    // Find the closing >
    let gt_pos = after_name.find('>')?;
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

    let attrs = parse_attrs(attr_str);
    let bytes_consumed = 1 + tag_name.len() + gt_pos + 1;

    Some(ParsedTag {
        tag_name,
        attrs,
        self_closing: is_self_closing,
        bytes_consumed,
    })
}

/// Parse HTML attributes string into key-value pairs.
fn parse_attrs(attr_str: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut chars = attr_str.chars().peekable();

    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        let mut name = String::new();
        while chars.peek().is_some_and(|c| {
            c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ':' || *c == '.'
        }) {
            if let Some(c) = chars.next() {
                name.push(c);
            }
        }
        if name.is_empty() {
            chars.next();
            continue;
        }

        // Lowercase for known HTML attributes
        let name = name.to_lowercase();

        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        if chars.peek() == Some(&'=') {
            chars.next();
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                chars.next();
            }
            let value = if chars.peek() == Some(&'"') {
                chars.next();
                chars.by_ref().take_while(|c| *c != '"').collect()
            } else if chars.peek() == Some(&'\'') {
                chars.next();
                chars.by_ref().take_while(|c| *c != '\'').collect()
            } else {
                chars
                    .by_ref()
                    .take_while(|c| !c.is_whitespace() && *c != '>' && *c != '/')
                    .collect()
            };
            attrs.push((name, value));
        } else {
            attrs.push((name, String::new()));
        }
    }

    attrs
}

/// Check if a tag name is a void (self-closing) HTML element.
fn is_void_element(tag_lc: &str) -> bool {
    matches!(
        tag_lc,
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

/// Check if a tag is a known HTML element (for case-insensitive matching).
fn is_known_html_element(tag_lc: &str) -> bool {
    const KNOWN: &[&str] = &[
        "a",
        "abbr",
        "acronym",
        "address",
        "applet",
        "area",
        "article",
        "aside",
        "b",
        "base",
        "bdo",
        "big",
        "blockquote",
        "body",
        "br",
        "button",
        "caption",
        "cite",
        "code",
        "col",
        "colgroup",
        "dd",
        "del",
        "dfn",
        "div",
        "dl",
        "dt",
        "em",
        "embed",
        "fieldset",
        "figcaption",
        "figure",
        "font",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hgroup",
        "hr",
        "html",
        "i",
        "iframe",
        "img",
        "input",
        "ins",
        "kbd",
        "label",
        "legend",
        "li",
        "link",
        "main",
        "map",
        "mark",
        "menu",
        "meta",
        "nav",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "pre",
        "q",
        "rb",
        "rbc",
        "rp",
        "rt",
        "rtc",
        "ruby",
        "s",
        "samp",
        "script",
        "section",
        "select",
        "small",
        "source",
        "span",
        "strike",
        "strong",
        "style",
        "sub",
        "summary",
        "sup",
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
        "tt",
        "u",
        "ul",
        "var",
        "wbr",
    ];
    KNOWN.contains(&tag_lc)
}

// -----------------------------------------------------------------------
// Content extraction helpers
// -----------------------------------------------------------------------

/// Convert HTML nodes back to HTML text (for text content of elements).
fn nodes_to_text(nodes: &[HtmlNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            HtmlNode::Text(t) => result.push_str(t),
            HtmlNode::Comment(c) => result.push_str(c),
            HtmlNode::Element {
                tag,
                attrs,
                children,
                self_closing,
            } => {
                result.push('<');
                result.push_str(tag);
                for (k, v) in attrs {
                    result.push(' ');
                    result.push_str(k);
                    result.push_str("=\"");
                    result.push_str(v);
                    result.push('"');
                }
                if *self_closing {
                    result.push_str(" />");
                } else {
                    result.push('>');
                    result.push_str(&nodes_to_text(children));
                    result.push_str("</");
                    result.push_str(tag);
                    result.push('>');
                }
            }
        }
    }
    result
}

/// Extract plain text from HTML nodes (strips all tags).
fn extract_plain_text(nodes: &[HtmlNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            HtmlNode::Text(t) => {
                // Decode common entities for text extraction
                let decoded = decode_entities(t);
                result.push_str(&decoded);
            }
            HtmlNode::Comment(_) => {}
            HtmlNode::Element { children, .. } => {
                result.push_str(&extract_plain_text(children));
            }
        }
    }
    // Compress whitespace
    let mut prev_space = false;
    let mut compressed = String::new();
    for c in result.chars() {
        if c.is_whitespace() {
            if !prev_space {
                compressed.push(' ');
            }
            prev_space = true;
        } else {
            compressed.push(c);
            prev_space = false;
        }
    }
    compressed.trim().to_string()
}

/// Strip leading and trailing whitespace in text, compress inner whitespace.
fn strip_whitespace_text(text: &str) -> String {
    let mut result = String::new();
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space && !result.is_empty() {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }
    result
}

/// Decode common HTML entities.
fn decode_entities(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::from("&");
            while let Some(&next) = chars.peek() {
                entity.push(next);
                chars.next();
                if next == ';' {
                    break;
                }
                if !next.is_ascii_alphanumeric() && next != '#' {
                    break;
                }
            }
            if entity.ends_with(';') {
                if let Some(decoded) = decode_named_entity(&entity) {
                    result.push_str(decoded);
                    continue;
                }
            }
            result.push_str(&entity);
        } else {
            result.push(c);
        }
    }

    result
}

/// Decode a named HTML entity.
fn decode_named_entity(entity: &str) -> Option<&'static str> {
    match entity {
        "&amp;" => Some("&"),
        "&lt;" => Some("<"),
        "&gt;" => Some(">"),
        "&quot;" => Some("\""),
        "&apos;" => Some("'"),
        "&nbsp;" => Some("\u{00a0}"),
        "&hellip;" => Some("\u{2026}"),
        "&ldquo;" => Some("\u{201c}"),
        "&rdquo;" => Some("\u{201d}"),
        "&lsquo;" => Some("\u{2018}"),
        "&rsquo;" => Some("\u{2019}"),
        "&mdash;" => Some("\u{2014}"),
        "&ndash;" => Some("\u{2013}"),
        "&laquo;" => Some("\u{00ab}"),
        "&raquo;" => Some("\u{00bb}"),
        _ => None,
    }
}

/// Process content of a block container element (div, section, etc.) for html_to_native.
/// Removes whitespace-only text nodes between block elements.
fn process_block_container_content(
    tag: &str,
    attrs: &[(String, String)],
    children: &[HtmlNode],
) -> String {
    let tag_lc = tag.to_lowercase();
    let mut output = String::new();

    // Opening tag
    output.push('<');
    output.push_str(&tag_lc);
    for (k, v) in attrs {
        if k != "markdown" {
            output.push(' ');
            output.push_str(k);
            output.push_str("=\"");
            output.push_str(v);
            output.push('"');
        }
    }
    output.push('>');

    // Process children - remove whitespace-only text between block elements
    // and add proper indentation
    let indent = "  ";
    for child in children {
        match child {
            HtmlNode::Text(t) => {
                if !t.trim().is_empty() {
                    output.push_str(t);
                }
                // Skip whitespace-only text (removes blank lines)
            }
            HtmlNode::Comment(c) => {
                output.push('\n');
                output.push_str(indent);
                output.push_str(c);
            }
            HtmlNode::Element {
                tag: child_tag,
                attrs: child_attrs,
                children: child_children,
                self_closing,
            } => {
                output.push('<');
                output.push_str(child_tag);
                for (k, v) in child_attrs {
                    output.push(' ');
                    output.push_str(k);
                    output.push_str("=\"");
                    output.push_str(v);
                    output.push('"');
                }
                if *self_closing {
                    output.push_str(" />");
                } else {
                    output.push('>');
                    output.push_str(&nodes_to_text(child_children));
                    output.push_str("</");
                    output.push_str(child_tag);
                    output.push('>');
                }
            }
        }
    }

    // Closing tag on new line
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output.push_str("</");
    output.push_str(&tag_lc);
    output.push('>');

    output
}

/// Decode only typographic HTML entities (hellip, ldquo, rdquo, etc.) to Unicode chars.
/// Preserves &lt;, &gt;, &amp;, &quot; and other structural entities.
fn decode_typographic_entities(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::from("&");
            while let Some(&next) = chars.peek() {
                entity.push(next);
                chars.next();
                if next == ';' {
                    break;
                }
                if !next.is_ascii_alphanumeric() && next != '#' {
                    break;
                }
            }
            if entity.ends_with(';') {
                if let Some(decoded) = decode_typographic_entity(&entity) {
                    result.push_str(decoded);
                    continue;
                }
            }
            result.push_str(&entity);
        } else {
            result.push(c);
        }
    }

    result
}

/// Decode only typographic entities (not structural ones like &lt; &gt; &amp;).
fn decode_typographic_entity(entity: &str) -> Option<&'static str> {
    match entity {
        "&hellip;" => Some("\u{2026}"),
        "&ldquo;" => Some("\u{201c}"),
        "&rdquo;" => Some("\u{201d}"),
        "&lsquo;" => Some("\u{2018}"),
        "&rsquo;" => Some("\u{2019}"),
        "&mdash;" => Some("\u{2014}"),
        "&ndash;" => Some("\u{2013}"),
        "&laquo;" => Some("\u{00ab}"),
        "&raquo;" => Some("\u{00bb}"),
        "&nbsp;" => Some("\u{00a0}"),
        _ => None,
    }
}

// -----------------------------------------------------------------------
// Element converters
// -----------------------------------------------------------------------

/// Convert <ul>/<ol> to a native List element.
fn convert_list(tag: &str, attrs: &[(String, String)], children: &[HtmlNode]) -> Option<Element> {
    let mut list = Element::new(ElementType::List);
    list.options.insert(
        "list_type".to_string(),
        if tag == "ul" {
            "ul".to_string()
        } else {
            "ol".to_string()
        },
    );
    for (k, v) in attrs {
        if k != "markdown" {
            list.attr.insert(k.clone(), v.clone());
        }
    }

    for child in children {
        match child {
            HtmlNode::Element {
                tag,
                children: li_children,
                ..
            } if tag.to_lowercase() == "li" => {
                let li_elem = convert_list_item(li_children)?;
                list.children.push(li_elem);
            }
            HtmlNode::Text(t) if t.trim().is_empty() => {
                // Skip whitespace between li elements
            }
            _ => {
                // Non-li content in list - can't convert
                return None;
            }
        }
    }

    Some(list)
}

/// Convert a <li>'s children to a native ListItem element.
fn convert_list_item(children: &[HtmlNode]) -> Option<Element> {
    let mut item = Element::new(ElementType::ListItem);

    // Determine if this is a "tight" item (just text) or "loose" (has <p> elements)
    let has_p = children
        .iter()
        .any(|c| matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "p"));

    if has_p {
        // Loose item: each <p> becomes a child paragraph
        for child in children {
            match child {
                HtmlNode::Element {
                    tag,
                    children: p_children,
                    ..
                } if tag.to_lowercase() == "p" => {
                    let mut para = Element::new(ElementType::Paragraph);
                    let text = nodes_to_text(p_children);
                    let stripped = strip_whitespace_text(&text);
                    para.children
                        .push(Element::with_value(ElementType::Text, stripped));
                    item.children.push(para);
                }
                HtmlNode::Text(t) if t.trim().is_empty() => {}
                _ => {
                    // Can't convert complex item content
                    return None;
                }
            }
        }
        item.options
            .insert("is_paragraph".to_string(), "true".to_string());
    } else {
        // Tight item: just text content
        let text = nodes_to_text(children);
        // Compress whitespace, strip leading/trailing
        let stripped = strip_whitespace_text(&text);
        item.children
            .push(Element::with_value(ElementType::Text, stripped));
    }

    Some(item)
}

/// Convert a <pre> element to a native CodeBlock.
fn convert_pre(children: &[HtmlNode]) -> Option<Element> {
    // Check if it's <pre><code>...</code></pre> or plain <pre>...</pre>
    let code_child = children
        .iter()
        .find(|c| matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "code"));

    let (text, _has_code_wrapper) = if let Some(HtmlNode::Element {
        children: code_children,
        ..
    }) = code_child
    {
        (extract_raw_text(code_children), true)
    } else {
        (extract_raw_text(children), false)
    };

    let mut elem = Element::new(ElementType::CodeBlock);
    // Ensure code block content ends with newline (kramdown convention)
    let text = if text.ends_with('\n') {
        text
    } else {
        format!("{text}\n")
    };
    elem.value = Some(text);
    Some(elem)
}

/// Extract raw text from nodes for code blocks - decodes HTML entities
/// and strips all tags (for html_to_native code/pre conversion).
fn extract_raw_text(nodes: &[HtmlNode]) -> String {
    let mut result = String::new();
    for node in nodes {
        match node {
            HtmlNode::Text(t) => {
                // Decode HTML entities so escape_html in the converter can re-encode
                result.push_str(&decode_entities(t));
            }
            HtmlNode::Element { children, .. } => {
                // For code blocks, tags inside are stripped, content preserved
                result.push_str(&extract_raw_text(children));
            }
            HtmlNode::Comment(_) => {}
        }
    }
    result
}

/// Convert a <table> to a native Table element (only for "simple" tables).
fn convert_table(
    attrs: &[(String, String)],
    children: &[HtmlNode],
    _options: &Options,
) -> Option<Element> {
    // Check if this is a simple table (all cells only contain phrasing content)
    // and all rows have the same number of cells
    if !is_simple_table(children) {
        return None;
    }

    let mut table = Element::new(ElementType::Table);
    for (k, v) in attrs {
        if k != "markdown" {
            table.attr.insert(k.clone(), v.clone());
        }
    }

    // Calculate alignment from style attributes
    let mut alignment: Vec<String> = Vec::new();

    // Process table sections
    let mut has_explicit_sections = false;
    for child in children {
        match child {
            HtmlNode::Element {
                tag,
                children: section_children,
                ..
            } => {
                let tag_lc = tag.to_lowercase();
                match tag_lc.as_str() {
                    "thead" | "tbody" | "tfoot" => {
                        has_explicit_sections = true;
                        let section_type = match tag_lc.as_str() {
                            "thead" => "thead",
                            "tfoot" => "tfoot",
                            _ => "tbody",
                        };
                        let mut section = Element::new(ElementType::Table);
                        section
                            .options
                            .insert("section".to_string(), section_type.to_string());

                        for row_node in section_children {
                            if let HtmlNode::Element {
                                tag,
                                children: cells,
                                ..
                            } = row_node
                            {
                                if tag.to_lowercase() == "tr" {
                                    let (row, row_align) =
                                        convert_table_row(cells, section_type == "thead");
                                    if alignment.is_empty() && !row_align.is_empty() {
                                        alignment = row_align;
                                    }
                                    section.children.push(row);
                                }
                            }
                        }

                        table.children.push(section);
                    }
                    "tr" => {
                        // Direct tr children (no explicit sections)
                        // Will be wrapped in tbody below
                        let first_cell_is_th = section_children.iter().any(|c| {
                            matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "th")
                        }) && !section_children.iter().any(|c| {
                            matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "td")
                        });

                        let (row, row_align) =
                            convert_table_row(section_children, first_cell_is_th);
                        if alignment.is_empty() && !row_align.is_empty() {
                            alignment = row_align;
                        }

                        // Check if first row is all <th> - if so, it's a header row
                        // For now, store all direct rows for later wrapping
                        table.children.push(row);
                    }
                    _ => {}
                }
            }
            HtmlNode::Text(t) if t.trim().is_empty() => {}
            _ => {}
        }
    }

    // If no explicit sections, wrap rows in tbody
    if !has_explicit_sections && !table.children.is_empty() {
        // Check if first row should be thead (all th cells)
        let first_row_all_th = table
            .children
            .first()
            .is_some_and(|row| row.options.get("all_th").is_some_and(|v| v == "true"));

        if first_row_all_th && table.children.len() > 1 {
            // First row becomes thead, rest becomes tbody
            let header_row = table.children.remove(0);
            let mut thead = Element::new(ElementType::Table);
            thead
                .options
                .insert("section".to_string(), "thead".to_string());
            thead.children.push(header_row);

            let mut tbody = Element::new(ElementType::Table);
            tbody
                .options
                .insert("section".to_string(), "tbody".to_string());
            for row in table.children.drain(..) {
                tbody.children.push(row);
            }

            table.children.push(thead);
            table.children.push(tbody);
        } else {
            // All rows in tbody
            let rows: Vec<Element> = table.children.drain(..).collect();
            let mut tbody = Element::new(ElementType::Table);
            tbody
                .options
                .insert("section".to_string(), "tbody".to_string());
            for row in rows {
                tbody.children.push(row);
            }
            table.children.push(tbody);
        }
    }

    // Store alignment
    if !alignment.is_empty() {
        let align_str = alignment.join(",");
        table.options.insert("alignment".to_string(), align_str);
    }

    // If table has no rows, return a minimal empty table
    if table.children.is_empty() || table.children.iter().all(|s| s.children.is_empty()) {
        // Empty table - return as raw HTML
        return None;
    }

    Some(table)
}

/// Convert a <tr>'s children to a native TableRow element.
/// Returns the row element and alignment info.
fn convert_table_row(cells: &[HtmlNode], _is_header: bool) -> (Element, Vec<String>) {
    let mut row = Element::new(ElementType::TableRow);
    let mut alignment = Vec::new();
    let mut all_th = true;

    for cell_node in cells {
        if let HtmlNode::Element {
            tag,
            attrs,
            children,
            ..
        } = cell_node
        {
            let tag_lc = tag.to_lowercase();
            if tag_lc == "td" || tag_lc == "th" {
                if tag_lc == "td" {
                    all_th = false;
                }
                let mut cell = Element::new(ElementType::TableCell);
                if tag_lc == "th" {
                    cell.options
                        .insert("is_header".to_string(), "true".to_string());
                }

                // Keep all attributes on the cell (including style)
                for (k, v) in attrs {
                    cell.attr.insert(k.clone(), v.clone());
                }
                // Extract alignment from style for the table alignment option
                let cell_align = if let Some(style) = attrs.iter().find(|(k, _)| k == "style") {
                    extract_text_align(&style.1).unwrap_or_else(|| "default".to_string())
                } else {
                    "default".to_string()
                };
                alignment.push(cell_align);

                // Cell content - mark as raw HTML (don't markdown-process)
                let text = nodes_to_text(children);
                let stripped = strip_whitespace_text(&text);
                cell.children
                    .push(Element::with_value(ElementType::Text, stripped));
                cell.options
                    .insert("raw_html".to_string(), "true".to_string());
                row.children.push(cell);
            }
        }
    }

    if all_th {
        row.options.insert("all_th".to_string(), "true".to_string());
    }

    (row, alignment)
}

/// Check if a table is "simple" per kramdown Ruby rules:
/// - All cells contain only phrasing (inline) content
/// - All rows have the same number of cells
/// - Either: all direct rows have only <td> cells
///   OR: has explicit thead(th) + tbody(td) + optional tfoot(td) sections
fn is_simple_table(children: &[HtmlNode]) -> bool {
    let mut nr_cells: Option<usize> = None;

    let check_row_cells = |row_children: &[HtmlNode]| -> (bool, usize) {
        let count = row_children
            .iter()
            .filter(|c| {
                matches!(c, HtmlNode::Element { tag, .. }
                    if tag.to_lowercase() == "td" || tag.to_lowercase() == "th")
            })
            .count();
        for cell in row_children {
            if let HtmlNode::Element { tag, children, .. } = cell {
                let tag_lc = tag.to_lowercase();
                if (tag_lc == "td" || tag_lc == "th") && !only_phrasing_content(children) {
                    return (false, 0);
                }
            }
        }
        (true, count)
    };

    // Check cell counts and phrasing content
    for child in children {
        if let HtmlNode::Element {
            tag,
            children: section_children,
            ..
        } = child
        {
            let tag_lc = tag.to_lowercase();
            match tag_lc.as_str() {
                "tr" => {
                    let (ok, count) = check_row_cells(section_children);
                    if !ok || count == 0 {
                        return false;
                    }
                    if let Some(prev) = nr_cells {
                        if prev != count {
                            return false;
                        }
                    } else {
                        nr_cells = Some(count);
                    }
                }
                "thead" | "tbody" | "tfoot" => {
                    for row_node in section_children {
                        if let HtmlNode::Element {
                            tag,
                            children: row_children,
                            ..
                        } = row_node
                        {
                            if tag.to_lowercase() == "tr" {
                                let (ok, count) = check_row_cells(row_children);
                                if !ok || count == 0 {
                                    return false;
                                }
                                if let Some(prev) = nr_cells {
                                    if prev != count {
                                        return false;
                                    }
                                } else {
                                    nr_cells = Some(count);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if nr_cells.is_none() || nr_cells == Some(0) {
        return false;
    }

    // Check structure: either all direct <tr> with <td> only,
    // or explicit sections (thead+tbody)
    let has_direct_rows = children
        .iter()
        .any(|c| matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "tr"));
    let has_sections = children.iter().any(|c| {
        matches!(c, HtmlNode::Element { tag, .. }
            if tag.to_lowercase() == "thead" || tag.to_lowercase() == "tbody" || tag.to_lowercase() == "tfoot")
    });

    if has_direct_rows && !has_sections {
        // All direct rows must have only <td> cells (no <th>)
        let check_all_td = |row_children: &[HtmlNode]| -> bool {
            row_children
                .iter()
                .all(|c| !matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "th"))
        };
        for child in children {
            if let HtmlNode::Element {
                tag,
                children: row_children,
                ..
            } = child
            {
                if tag.to_lowercase() == "tr" && !check_all_td(row_children) {
                    return false;
                }
            }
        }
    }

    if has_sections {
        // Must have at least one tbody
        let has_tbody = children
            .iter()
            .any(|c| matches!(c, HtmlNode::Element { tag, .. } if tag.to_lowercase() == "tbody"));
        if !has_tbody {
            return false;
        }
    }

    true
}

/// Check if children contain only phrasing (inline) content.
fn only_phrasing_content(children: &[HtmlNode]) -> bool {
    const BLOCK_ELEMENTS: &[&str] = &[
        "address",
        "article",
        "aside",
        "blockquote",
        "div",
        "dl",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "header",
        "hgroup",
        "hr",
        "li",
        "main",
        "nav",
        "ol",
        "p",
        "pre",
        "section",
        "table",
        "ul",
    ];

    for child in children {
        if let HtmlNode::Element {
            tag, children: sub, ..
        } = child
        {
            if BLOCK_ELEMENTS.contains(&tag.to_lowercase().as_str()) {
                return false;
            }
            if !only_phrasing_content(sub) {
                return false;
            }
        }
    }
    true
}

/// Extract text-align value from a CSS style string.
fn extract_text_align(style: &str) -> Option<String> {
    let re_pattern = "text-align:";
    if let Some(idx) = style.find(re_pattern) {
        let after = &style[idx + re_pattern.len()..];
        let value: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

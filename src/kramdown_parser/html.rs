// kramdown parser - HTML converter
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use crate::kramdown_parser::element::{Document, Element, ElementType};
use crate::kramdown_parser::options::Options;

/// Converts a kramdown Document AST into HTML output.
pub struct HtmlConverter;

impl HtmlConverter {
    /// Convert a Document AST to an HTML string.
    pub fn convert(doc: &Document, options: &Options) -> String {
        let mut output = String::new();
        convert_children(&doc.root.children, &mut output, options, 0);

        // Ensure trailing newline (kramdown always outputs at least \n)
        if output.is_empty() || !output.ends_with('\n') {
            output.push('\n');
        }

        output
    }
}

/// Convert a list of child elements to HTML.
fn convert_children(children: &[Element], output: &mut String, options: &Options, indent: usize) {
    let mut prev_was_block = false;
    let mut first = true;

    for (i, child) in children.iter().enumerate() {
        match child.element_type {
            ElementType::Blank | ElementType::Eob => {
                // These produce no output
                continue;
            }
            _ => {}
        }

        // Add blank line between block elements (except first, and except
        // elements that follow blanks)
        if !first && prev_was_block && is_visible_block(child) {
            // Check if there was a blank line between this and previous visible element
            let had_blank = had_blank_between(children, i);
            if had_blank {
                output.push('\n');
            }
        }

        convert_element(child, output, options, indent);
        if is_visible_block(child) {
            prev_was_block = true;
            first = false;
        }
    }
}

/// Check if there was a Blank element between position i and the previous visible element.
fn had_blank_between(children: &[Element], i: usize) -> bool {
    // Look backwards from i for a Blank element before the previous visible element
    let mut j = i.saturating_sub(1);
    loop {
        match children[j].element_type {
            ElementType::Blank => return true,
            ElementType::Eob => {
                if j == 0 {
                    return false;
                }
                j -= 1;
            }
            _ => return false,
        }
    }
}

/// Check if an element produces visible block output.
fn is_visible_block(elem: &Element) -> bool {
    !matches!(elem.element_type, ElementType::Blank | ElementType::Eob)
}

/// Convert a single element to HTML.
fn convert_element(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    match elem.element_type {
        ElementType::Blank | ElementType::Eob => {
            // No output
        }
        ElementType::Paragraph => {
            convert_paragraph(elem, output, options, indent);
        }
        ElementType::Header => {
            convert_header(elem, output, options, indent);
        }
        ElementType::CodeBlock => {
            convert_code_block(elem, output, options, indent);
        }
        ElementType::Blockquote => {
            convert_blockquote(elem, output, options, indent);
        }
        ElementType::HorizontalRule => {
            convert_horizontal_rule(elem, output, indent);
        }
        ElementType::List => {
            convert_list(elem, output, options, indent);
        }
        ElementType::ListItem => {
            convert_list_item(elem, output, options, indent);
        }
        ElementType::Table => {
            convert_table(elem, output, options, indent);
        }
        ElementType::Text => {
            if let Some(ref val) = elem.value {
                output.push_str(val);
            }
        }
        _ => {
            // Unsupported element types: output nothing for now
        }
    }
}

/// Convert paragraph to HTML.
fn convert_paragraph(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    write_indent(output, indent);
    output.push_str("<p>");

    let text = get_element_text(elem);
    let processed = process_paragraph_text(&text, options);
    output.push_str(&processed);

    output.push_str("</p>\n");
}

/// Process paragraph text for typographic conversions and inline code spans.
fn process_paragraph_text(text: &str, _options: &Options) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Backtick code spans
        if chars[i] == '`' {
            let mut bt = 0;
            let bt_start = i;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            // Find matching closing backticks
            let content_start = i;
            let mut found = false;
            while i < chars.len() {
                if chars[i] == '`' {
                    let mut close_bt = 0;
                    let close_start = i;
                    while i < chars.len() && chars[i] == '`' {
                        close_bt += 1;
                        i += 1;
                    }
                    if close_bt == bt {
                        let content: String = chars[content_start..close_start].iter().collect();
                        // Trim single leading/trailing space if content has them and isn't all spaces
                        let trimmed = if content.len() >= 2
                            && content.starts_with(' ')
                            && content.ends_with(' ')
                            && content.trim().len() < content.len() - 2 + content.trim().len()
                        {
                            &content[1..content.len() - 1]
                        } else {
                            content.trim()
                        };
                        result.push_str("<code>");
                        result.push_str(&escape_html(trimmed));
                        result.push_str("</code>");
                        found = true;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            if !found {
                // No closing backticks - output as literal
                for ch in &chars[bt_start..content_start] {
                    result.push(*ch);
                }
                i = content_start;
            }
            continue;
        }

        if chars[i] == '-' {
            // Count consecutive dashes
            let start = i;
            while i < chars.len() && chars[i] == '-' {
                i += 1;
            }
            let count = i - start;
            // Convert: every 3 dashes -> em-dash, remaining 2 -> en-dash, remaining 1 -> dash
            let em_dashes = count / 3;
            let remaining = count % 3;
            let en_dashes = remaining / 2;
            let single_dashes = remaining % 2;

            for _ in 0..em_dashes {
                result.push('\u{2014}'); // em-dash
            }
            for _ in 0..en_dashes {
                result.push('\u{2013}'); // en-dash
            }
            for _ in 0..single_dashes {
                result.push('-');
            }
        } else if chars[i] == '\\' && i + 1 < chars.len() && is_escapable_char(chars[i + 1]) {
            // Escaped character: \- \. \# \| etc.
            result.push(chars[i + 1]);
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Get the text content of an element (from its Text children).
fn get_element_text(elem: &Element) -> String {
    let mut text = String::new();
    for child in &elem.children {
        if let Some(ref val) = child.value {
            text.push_str(val);
        }
    }
    text
}

/// Convert header to HTML.
fn convert_header(elem: &Element, output: &mut String, _options: &Options, indent: usize) {
    let level = elem
        .options
        .get("level")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(1);

    write_indent(output, indent);
    output.push_str(&format!("<h{level}"));
    write_attrs(&elem.attr, output);
    output.push('>');

    let text = get_element_text(elem);
    // Process escaped hashes in header text
    let processed = text.replace("\\#", "#");
    output.push_str(&processed);

    output.push_str(&format!("</h{level}>\n"));
}

/// Convert code block to HTML.
fn convert_code_block(elem: &Element, output: &mut String, _options: &Options, indent: usize) {
    let content = elem.value.as_deref().unwrap_or("");
    let fence_language = elem.options.get("language");

    write_indent(output, indent);

    // Determine code language and pre attributes
    // Language from fence goes on <code> as class="language-X"
    // IAL attributes generally go on <pre>, except:
    // - If IAL has a class starting with "language-" and there's no fence language,
    //   that class goes on <code> instead
    let mut code_lang_class: Option<String> = fence_language.map(|l| format!("language-{l}"));
    let mut pre_attrs: Vec<(String, String)> = Vec::new();

    for (key, value) in &elem.attr {
        if key == "class" {
            // Check if any class is a language-* class
            let classes: Vec<&str> = value.split_whitespace().collect();
            let mut pre_classes = Vec::new();
            for cls in &classes {
                if cls.starts_with("language-") && code_lang_class.is_none() {
                    // Move language-* class to <code> if no fence language
                    code_lang_class = Some(cls.to_string());
                } else {
                    pre_classes.push(*cls);
                }
            }
            if !pre_classes.is_empty() {
                pre_attrs.push(("class".to_string(), pre_classes.join(" ")));
            }
        } else {
            pre_attrs.push((key.clone(), value.clone()));
        }
    }

    // Write <pre> with attributes
    output.push_str("<pre");
    for (key, value) in &pre_attrs {
        output.push_str(&format!(" {key}=\"{value}\""));
    }
    output.push_str("><code");

    // Language class on <code>
    if let Some(ref lang_class) = code_lang_class {
        output.push_str(&format!(" class=\"{lang_class}\""));
    }

    output.push('>');

    // Escape HTML entities in code content
    let escaped = escape_html(content);
    output.push_str(&escaped);

    output.push_str("</code></pre>\n");
}

/// Convert blockquote to HTML.
fn convert_blockquote(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    write_indent(output, indent);
    output.push_str("<blockquote");
    write_attrs(&elem.attr, output);
    output.push_str(">\n");

    convert_children(&elem.children, output, options, indent + 2);

    write_indent(output, indent);
    output.push_str("</blockquote>\n");
}

/// Convert horizontal rule to HTML.
fn convert_horizontal_rule(elem: &Element, output: &mut String, indent: usize) {
    write_indent(output, indent);
    if elem.attr.is_empty() {
        output.push_str("<hr />\n");
    } else {
        output.push_str("<hr");
        write_attrs(&elem.attr, output);
        output.push_str(" />\n");
    }
}

/// Convert list to HTML.
fn convert_list(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    let tag = elem
        .options
        .get("list_type")
        .map(|s| s.as_str())
        .unwrap_or("ul");

    write_indent(output, indent);
    output.push('<');
    output.push_str(tag);
    write_attrs(&elem.attr, output);
    output.push_str(">\n");

    for child in &elem.children {
        convert_list_item(child, output, options, indent + 2);
    }

    write_indent(output, indent);
    output.push_str("</");
    output.push_str(tag);
    output.push_str(">\n");
}

/// Convert list item to HTML.
fn convert_list_item(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    write_indent(output, indent);
    output.push_str("<li");
    write_attrs(&elem.attr, output);
    output.push('>');

    if elem.children.is_empty() {
        output.push_str("</li>\n");
        return;
    }

    // Determine rendering mode:
    // 1. Simple: only text children -> inline after <li>
    // 2. Mixed: starts with text, then block elements -> text inline, blocks indented
    // 3. Block: starts with block element -> newline after <li>, all indented

    let has_block_children = elem.children.iter().any(|c| {
        matches!(
            c.element_type,
            ElementType::Paragraph
                | ElementType::Blockquote
                | ElementType::CodeBlock
                | ElementType::Header
                | ElementType::List
                | ElementType::HorizontalRule
                | ElementType::Table
        )
    });

    if !has_block_children {
        // Simple item: inline content on same line as <li>
        for child in &elem.children {
            convert_element(child, output, options, 0);
        }
        output.push_str("</li>\n");
    } else {
        // Check if first child is text (mixed mode)
        let first_is_text = elem
            .children
            .first()
            .is_some_and(|c| c.element_type == ElementType::Text);

        if first_is_text {
            // Mixed: text inline after <li>, then newline, then indented blocks
            let text_child = &elem.children[0];
            if let Some(ref val) = text_child.value {
                output.push_str(val);
            }
            output.push('\n');
            convert_list_item_children(&elem.children[1..], output, options, indent + 2);
            write_indent(output, indent);
            output.push_str("</li>\n");
        } else {
            // Pure block content: newline after <li>, then indented content
            output.push('\n');
            convert_list_item_children(&elem.children, output, options, indent + 2);
            write_indent(output, indent);
            output.push_str("</li>\n");
        }
    }
}

/// Convert list item children to HTML, handling spacing between block elements.
fn convert_list_item_children(
    children: &[Element],
    output: &mut String,
    options: &Options,
    indent: usize,
) {
    let mut prev_was_visible = false;

    for (i, child) in children.iter().enumerate() {
        match child.element_type {
            ElementType::Blank | ElementType::Eob => continue,
            _ => {}
        }

        // Add blank line between visible block elements if there was a blank between them
        if prev_was_visible && is_visible_block(child) {
            let had_blank = had_blank_between(children, i);
            if had_blank {
                output.push('\n');
            }
        }

        convert_element(child, output, options, indent);
        if is_visible_block(child) {
            prev_was_visible = true;
        }
    }
}

/// Convert table to HTML.
fn convert_table(elem: &Element, output: &mut String, _options: &Options, indent: usize) {
    // Parse alignments from options
    let alignments: Vec<&str> = elem
        .options
        .get("alignments")
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let max_cols: usize = elem
        .options
        .get("max_cols")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    write_indent(output, indent);
    output.push_str("<table");
    write_attrs(&elem.attr, output);
    output.push_str(">\n");

    for section in &elem.children {
        let section_tag = section
            .options
            .get("section")
            .map(|s| s.as_str())
            .unwrap_or("tbody");
        let is_head = section_tag == "thead";

        write_indent(output, indent + 2);
        output.push('<');
        output.push_str(section_tag);
        output.push_str(">\n");

        for row in &section.children {
            write_indent(output, indent + 4);
            output.push_str("<tr>\n");

            let cell_count = row.children.len();
            // Render actual cells
            for (ci, cell) in row.children.iter().enumerate() {
                let tag = if is_head { "th" } else { "td" };
                let alignment = alignments.get(ci).copied().unwrap_or("default");

                write_indent(output, indent + 6);
                output.push('<');
                output.push_str(tag);
                if alignment != "default" && !alignment.is_empty() {
                    output.push_str(&format!(" style=\"text-align: {alignment}\""));
                }
                output.push('>');

                // Write cell content with inline processing
                let text = get_element_text(cell);
                if text.is_empty() {
                    output.push('\u{a0}');
                } else {
                    let processed = process_table_cell_content(&text);
                    output.push_str(&processed);
                }

                output.push_str("</");
                output.push_str(tag);
                output.push_str(">\n");
            }

            // Fill missing cells
            for ci in cell_count..max_cols {
                let tag = if is_head { "th" } else { "td" };
                let alignment = alignments.get(ci).copied().unwrap_or("default");

                write_indent(output, indent + 6);
                output.push('<');
                output.push_str(tag);
                if alignment != "default" && !alignment.is_empty() {
                    output.push_str(&format!(" style=\"text-align: {alignment}\""));
                }
                output.push_str(">\u{a0}</");
                output.push_str(tag);
                output.push_str(">\n");
            }

            write_indent(output, indent + 4);
            output.push_str("</tr>\n");
        }

        write_indent(output, indent + 2);
        output.push_str("</");
        output.push_str(section_tag);
        output.push_str(">\n");
    }

    write_indent(output, indent);
    output.push_str("</table>\n");
}

/// Process table cell content: convert backtick code spans to <code> tags,
/// handle escaped pipes, and balance/escape HTML tags.
fn process_table_cell_content(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut open_tags: Vec<String> = Vec::new();

    while i < chars.len() {
        // Backtick code span
        if chars[i] == '`' {
            let mut bt = 0;
            let _start = i;
            while i < chars.len() && chars[i] == '`' {
                bt += 1;
                i += 1;
            }
            // Find matching closing backticks
            let mut found = false;
            let content_start = i;
            while i < chars.len() {
                if chars[i] == '`' {
                    let mut close_bt = 0;
                    let close_start = i;
                    while i < chars.len() && chars[i] == '`' {
                        close_bt += 1;
                        i += 1;
                    }
                    if close_bt == bt {
                        // Found matching close
                        let content: String = chars[content_start..close_start].iter().collect();
                        let trimmed = content.trim();
                        result.push_str("<code>");
                        result.push_str(&escape_html(trimmed));
                        result.push_str("</code>");
                        found = true;
                        break;
                    }
                    // Not enough backticks, keep looking
                } else {
                    i += 1;
                }
            }
            if !found {
                // No closing backticks - output as literal
                for _ in 0..bt {
                    result.push('`');
                }
                // Re-process from content_start
                i = content_start;
            }
            continue;
        }

        // HTML tag detection
        if chars[i] == '<' {
            let remaining: String = chars[i..].iter().collect();
            // Check for closing tag
            if remaining.starts_with("</") {
                if let Some(gt) = remaining.find('>') {
                    let tag_content = &remaining[2..gt];
                    let tag_name = tag_content.split_whitespace().next().unwrap_or("");
                    // Check if this closing tag matches an open tag
                    if let Some(pos) = open_tags.iter().rposition(|t| t == tag_name) {
                        // Valid close
                        result.push_str(&remaining[..=gt]);
                        open_tags.remove(pos);
                        i += gt + 1;
                    } else {
                        // Stray closing tag - escape the entire tag
                        let tag_str = &remaining[..=gt];
                        result.push_str(&escape_html(tag_str));
                        i += gt + 1;
                    }
                    continue;
                }
            }
            // Check for opening tag
            if let Some(gt) = remaining.find('>') {
                let tag_content = &remaining[1..gt];
                let tag_name = tag_content
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('/');
                // Self-closing tags
                let is_self_closing = tag_content.ends_with('/');
                let is_void = matches!(
                    tag_name,
                    "br" | "hr" | "img" | "input" | "col" | "area" | "base" | "link" | "meta"
                );
                result.push_str(&remaining[..=gt]);
                i += gt + 1;
                if !is_self_closing && !is_void && !tag_name.is_empty() {
                    open_tags.push(tag_name.to_string());
                }
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    // Auto-close any unclosed tags
    for tag in open_tags.iter().rev() {
        result.push_str(&format!("</{tag}>"));
    }

    result
}

/// Write HTML attributes.
fn write_attrs(attrs: &std::collections::HashMap<String, String>, output: &mut String) {
    // Write id first, then class, then rest alphabetically
    if let Some(id) = attrs.get("id") {
        output.push_str(&format!(" id=\"{id}\""));
    }
    if let Some(class) = attrs.get("class") {
        output.push_str(&format!(" class=\"{class}\""));
    }
    let mut other_keys: Vec<&String> = attrs
        .keys()
        .filter(|k| *k != "id" && *k != "class")
        .collect();
    other_keys.sort();
    for key in other_keys {
        let value = &attrs[key];
        output.push_str(&format!(" {key}=\"{value}\""));
    }
}

/// Write indentation.
fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
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
    )
}

/// Escape HTML entities in code block content.
/// Only escapes &, <, > (not quotes, as they're safe in code blocks).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

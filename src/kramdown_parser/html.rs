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
use crate::kramdown_parser::parser::KramdownParser;
use crate::kramdown_parser::span_parser::{self, SpanContext};

/// Converts a kramdown Document AST into HTML output.
pub struct HtmlConverter;

impl HtmlConverter {
    /// Convert a Document AST to an HTML string (without span context - legacy path).
    pub fn convert(doc: &Document, options: &Options) -> String {
        let mut ctx = SpanContext::new(options);
        Self::convert_with_context(doc, options, &mut ctx)
    }

    /// Convert a Document AST to an HTML string with a span context for inline processing.
    pub fn convert_with_context(
        doc: &Document,
        options: &Options,
        ctx: &mut SpanContext,
    ) -> String {
        let mut output = String::new();
        convert_children(&doc.root.children, &mut output, options, 0, ctx);

        // Render footnotes if any were referenced
        let footnotes = render_footnotes(options, ctx);
        output.push_str(&footnotes);

        // Ensure trailing newline (kramdown always outputs at least \n)
        if output.is_empty() || !output.ends_with('\n') {
            output.push('\n');
        }

        output
    }
}

/// Convert a list of child elements to HTML.
fn convert_children(
    children: &[Element],
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
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

        convert_element(child, output, options, indent, ctx);
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
    match elem.element_type {
        ElementType::Blank | ElementType::Eob => false,
        ElementType::BlockExtension => {
            // Empty block extensions (options, self-closing) produce no output
            let ext_type = elem
                .options
                .get("ext_type")
                .map(|s| s.as_str())
                .unwrap_or("");
            match ext_type {
                "comment" | "nomarkdown" => elem.value.as_ref().is_some_and(|v| !v.is_empty()),
                _ => false,
            }
        }
        _ => true,
    }
}

/// Convert a single element to HTML.
fn convert_element(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
    match elem.element_type {
        ElementType::Blank | ElementType::Eob => {
            // No output
        }
        ElementType::Paragraph => {
            convert_paragraph(elem, output, options, indent, ctx);
        }
        ElementType::Header => {
            convert_header(elem, output, options, indent, ctx);
        }
        ElementType::CodeBlock => {
            convert_code_block(elem, output, options, indent);
        }
        ElementType::Blockquote => {
            convert_blockquote(elem, output, options, indent, ctx);
        }
        ElementType::HorizontalRule => {
            convert_horizontal_rule(elem, output, indent);
        }
        ElementType::List => {
            convert_list(elem, output, options, indent, ctx);
        }
        ElementType::ListItem => {
            convert_list_item(elem, output, options, indent, ctx);
        }
        ElementType::Table => {
            convert_table(elem, output, options, indent, ctx);
        }
        ElementType::HtmlBlock => {
            convert_html_block(elem, output, indent, options, ctx);
        }
        ElementType::DefinitionList => {
            convert_definition_list(elem, output, options, indent, ctx);
        }
        ElementType::MathBlock => {
            convert_math_block(elem, output, options, indent);
        }
        ElementType::BlockExtension => {
            convert_block_extension(elem, output, indent);
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

/// Convert paragraph to HTML.
fn convert_paragraph(
    elem: &Element,
    output: &mut String,
    _options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
    write_indent(output, indent);
    output.push_str("<p>");

    let text = get_element_text(elem);
    let processed = span_parser::spans_to_html(&text, ctx);
    output.push_str(&processed);

    output.push_str("</p>\n");
}

/// Convert header to HTML.
fn convert_header(
    elem: &Element,
    output: &mut String,
    _options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
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
    let processed = span_parser::spans_to_html(&text, ctx);
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
fn convert_blockquote(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
    write_indent(output, indent);
    output.push_str("<blockquote");
    write_attrs(&elem.attr, output);
    output.push_str(">\n");

    convert_children(&elem.children, output, options, indent + 2, ctx);

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
fn convert_list(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
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
        convert_list_item(child, output, options, indent + 2, ctx);
    }

    write_indent(output, indent);
    output.push_str("</");
    output.push_str(tag);
    output.push_str(">\n");
}

/// Convert list item to HTML.
fn convert_list_item(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
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
        // Process inline text through span parser
        let text = get_element_text_from_children(&elem.children);
        if !text.is_empty() {
            let processed = span_parser::spans_to_html(&text, ctx);
            output.push_str(&processed);
        } else {
            for child in &elem.children {
                convert_element(child, output, options, 0, ctx);
            }
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
                let processed = span_parser::spans_to_html(val, ctx);
                output.push_str(&processed);
            }
            output.push('\n');
            convert_list_item_children(&elem.children[1..], output, options, indent + 2, ctx);
            write_indent(output, indent);
            output.push_str("</li>\n");
        } else {
            // Pure block content: newline after <li>, then indented content
            output.push('\n');
            convert_list_item_children(&elem.children, output, options, indent + 2, ctx);
            write_indent(output, indent);
            output.push_str("</li>\n");
        }
    }
}

/// Get text from a list of children elements (for simple list items).
fn get_element_text_from_children(children: &[Element]) -> String {
    let mut text = String::new();
    for child in children {
        if child.element_type == ElementType::Text {
            if let Some(ref val) = child.value {
                text.push_str(val);
            }
        }
    }
    text
}

/// Convert list item children to HTML, handling spacing between block elements.
fn convert_list_item_children(
    children: &[Element],
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
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

        convert_element(child, output, options, indent, ctx);
        if is_visible_block(child) {
            prev_was_visible = true;
        }
    }
}

/// Convert table to HTML.
fn convert_table(
    elem: &Element,
    output: &mut String,
    _options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
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

                // Write cell content with span processing
                let text = get_element_text(cell);
                if text.is_empty() {
                    output.push('\u{a0}');
                } else {
                    let processed = span_parser::spans_to_html(&text, ctx);
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

/// Escape invalid HTML inside a block-level HTML element.
/// Orphan closing tags (no matching opener) and non-HTML angle bracket content
/// get their `<` and `>` escaped to `&lt;` and `&gt;`.
fn escape_invalid_html_in_block(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let lines: Vec<&str> = html.split('\n').collect();

    // Track which tags are open to detect orphan closing tags
    let mut open_tags: Vec<String> = Vec::new();
    // Track if we're inside the outermost block tag (skip escaping there)
    let mut depth = 0;
    let mut _first_line = true;

    for (li, line) in lines.iter().enumerate() {
        if li > 0 {
            result.push('\n');
        }

        // Process char by char to find and validate HTML tags
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '<' {
                // Try to parse as an HTML tag
                let remaining: String = chars[i..].iter().collect();

                if remaining.starts_with("</") {
                    // Closing tag
                    if let Some((tag_name, end_pos)) = extract_tag_name_from_closing(&remaining) {
                        let tag_lc = tag_name.to_lowercase();
                        // Check if this closing tag has a matching opener
                        if depth <= 1 && !open_tags.iter().rev().any(|t| t == &tag_lc) {
                            // Orphan closing tag - escape it
                            result.push_str("&lt;");
                            i += 1;
                            continue;
                        }
                        // Valid closing tag
                        if let Some(pos) = open_tags.iter().rposition(|t| t == &tag_lc) {
                            open_tags.truncate(pos);
                        }
                        depth -= 1;
                        // Pass through
                        for ch in remaining[..end_pos].chars() {
                            result.push(ch);
                        }
                        i += end_pos;
                        continue;
                    }
                    // Not a valid closing tag - escape
                    result.push_str("&lt;");
                    i += 1;
                    continue;
                } else if remaining.starts_with("<!") || remaining.starts_with("<?") {
                    // Comment or PI - pass through
                    result.push(chars[i]);
                    i += 1;
                    continue;
                } else {
                    // Opening tag candidate
                    if let Some((tag_name, end_pos, is_self_closing)) =
                        extract_tag_info_from_opening(&remaining)
                    {
                        let tag_lc = tag_name.to_lowercase();
                        if !is_self_closing && !is_void_tag_name(&tag_lc) {
                            open_tags.push(tag_lc);
                            depth += 1;
                        }
                        // Pass through the tag
                        for ch in remaining[..end_pos].chars() {
                            result.push(ch);
                        }
                        i += end_pos;
                        continue;
                    }
                    // Not a valid opening tag - escape the <
                    result.push_str("&lt;");
                    i += 1;
                    continue;
                }
            } else if chars[i] == '>' {
                // Standalone > that's not part of a tag
                result.push_str("&gt;");
                i += 1;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }

        _first_line = false;
    }

    result
}

/// Extract tag name from a closing tag string like `</div>`.
/// Returns (tag_name, end_position).
fn extract_tag_name_from_closing(s: &str) -> Option<(String, usize)> {
    if !s.starts_with("</") {
        return None;
    }
    let after = &s[2..];
    let tag_name: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    if tag_name.is_empty() {
        return None;
    }
    let tag_len = tag_name.len();
    let after_name = &after[tag_len..];
    let gt_pos = after_name.find('>')?;
    Some((tag_name, 2 + tag_len + gt_pos + 1))
}

/// Extract tag info from an opening tag string like `<div class="test">`.
/// Returns (tag_name, end_position, is_self_closing).
fn extract_tag_info_from_opening(s: &str) -> Option<(String, usize, bool)> {
    if !s.starts_with('<') || s.starts_with("</") || s.starts_with("<!") || s.starts_with("<?") {
        return None;
    }
    let after = &s[1..];
    let tag_name: String = after
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == ':')
        .collect();
    if tag_name.is_empty() {
        return None;
    }
    let tag_len = tag_name.len();
    // Tag name must be followed by space, >, or / (but not // which is a URL)
    let after_name = &after[tag_len..];
    if !after_name.is_empty() {
        let next = after_name.chars().next()?;
        if next != ' ' && next != '\t' && next != '>' && next != '/' && next != '\n' {
            return None;
        }
        // Reject URL-like patterns: <http://...>
        if after_name.starts_with("//") {
            return None;
        }
    }
    let gt_pos = after_name.find('>')?;
    let before_gt = &after_name[..gt_pos];
    let is_self_closing = before_gt.trim_end().ends_with('/');
    Some((tag_name, 1 + tag_len + gt_pos + 1, is_self_closing))
}

/// Check if a tag name is a void (self-closing) HTML element.
fn is_void_tag_name(tag: &str) -> bool {
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

/// Escape HTML entities in code block content.
/// Only escapes &, <, > (not quotes, as they're safe in code blocks).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert HTML block to output.
fn convert_html_block(
    elem: &Element,
    output: &mut String,
    indent: usize,
    options: &Options,
    ctx: &mut SpanContext,
) {
    let parse_mode = elem.options.get("parse_mode").map(|s| s.as_str());

    match parse_mode {
        Some("block") => {
            // Block-parsed HTML: output opening tag, then children, then closing tag
            let tag = elem.options.get("tag").map(|s| s.as_str()).unwrap_or("div");
            let attrs = elem.options.get("attrs").map(|s| s.as_str()).unwrap_or("");
            write_indent(output, indent);
            output.push('<');
            output.push_str(tag);
            output.push_str(attrs);
            output.push_str(">\n");
            convert_children(&elem.children, output, options, indent + 2, ctx);
            write_indent(output, indent);
            output.push_str("</");
            output.push_str(tag);
            output.push_str(">\n");
        }
        Some("span") => {
            // Span-parsed HTML: output opening tag, then span-processed content, then closing tag
            let tag = elem.options.get("tag").map(|s| s.as_str()).unwrap_or("p");
            let attrs = elem.options.get("attrs").map(|s| s.as_str()).unwrap_or("");
            write_indent(output, indent);
            output.push('<');
            output.push_str(tag);
            output.push_str(attrs);
            output.push_str(">\n");
            if let Some(ref val) = elem.value {
                let processed = span_parser::spans_to_html(val.trim(), ctx);
                output.push_str(&processed);
                output.push('\n');
            }
            write_indent(output, indent);
            output.push_str("</");
            output.push_str(tag);
            output.push_str(">\n");
        }
        _ => {
            // Raw HTML: pass through
            let elem_type = elem.options.get("type").map(|s| s.as_str());
            let skip_escape = matches!(elem_type, Some("comment") | Some("raw"));
            if let Some(ref val) = elem.value {
                if !val.is_empty() {
                    if skip_escape {
                        output.push_str(val);
                    } else {
                        let processed = escape_invalid_html_in_block(val);
                        output.push_str(&processed);
                    }
                    if !val.ends_with('\n') {
                        output.push('\n');
                    }
                }
            }
            // Handle trailing text after HTML comments (e.g., --> para)
            if let Some(trailing) = elem.options.get("trailing_text") {
                output.push_str("<p>");
                output.push_str(trailing);
                output.push_str("</p>\n");
            }
        }
    }
}

/// Convert definition list to HTML.
fn convert_definition_list(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
    // Check for auto_ids directive in attributes
    let mut auto_ids_prefix: Option<String> = None;
    let mut dl_attrs = elem.attr.clone();
    // Look for auto_ids or auto_ids-prefix- style attributes
    let auto_ids_keys: Vec<String> = dl_attrs
        .keys()
        .filter(|k| k.starts_with("auto_ids"))
        .cloned()
        .collect();
    for key in &auto_ids_keys {
        if key == "auto_ids" {
            auto_ids_prefix = Some(String::new());
        } else if key.starts_with("auto_ids-") && key.ends_with('-') {
            let prefix = &key[9..key.len() - 1];
            auto_ids_prefix = Some(format!("{prefix}-"));
        }
        dl_attrs.remove(key);
    }

    write_indent(output, indent);
    output.push_str("<dl");
    write_attrs(&dl_attrs, output);
    output.push_str(">\n");

    for child in &elem.children {
        match child.element_type {
            ElementType::DefinitionTerm => {
                write_indent(output, indent + 2);
                output.push_str("<dt");
                // Auto-generate ID if auto_ids is set and dt doesn't have one
                if let Some(ref prefix) = auto_ids_prefix {
                    if !child.attr.contains_key("id") {
                        if let Some(ref val) = child.value {
                            let id = generate_term_id(val, prefix);
                            output.push_str(&format!(" id=\"{id}\""));
                        }
                    }
                }
                write_attrs(&child.attr, output);
                output.push('>');
                if let Some(ref val) = child.value {
                    let processed = span_parser::spans_to_html(val, ctx);
                    output.push_str(&processed);
                }
                output.push_str("</dt>\n");
            }
            ElementType::DefinitionDefinition => {
                let has_block = child.options.contains_key("block_content");
                let has_para = child.options.contains_key("para_wrap");

                if has_block {
                    // Block content in dd
                    write_indent(output, indent + 2);
                    output.push_str("<dd");
                    write_attrs(&child.attr, output);
                    output.push_str(">\n");
                    convert_children(&child.children, output, options, indent + 4, ctx);
                    write_indent(output, indent + 2);
                    output.push_str("</dd>\n");
                } else if has_para {
                    // Para-wrapped dd
                    write_indent(output, indent + 2);
                    output.push_str("<dd");
                    write_attrs(&child.attr, output);
                    output.push_str(">\n");
                    write_indent(output, indent + 4);
                    output.push_str("<p>");
                    if let Some(ref val) = child.value {
                        let processed = span_parser::spans_to_html(val, ctx);
                        output.push_str(&processed);
                    }
                    output.push_str("</p>\n");
                    write_indent(output, indent + 2);
                    output.push_str("</dd>\n");
                } else {
                    // Simple dd
                    write_indent(output, indent + 2);
                    output.push_str("<dd");
                    write_attrs(&child.attr, output);
                    output.push('>');
                    if let Some(ref val) = child.value {
                        if !val.is_empty() {
                            let processed = span_parser::spans_to_html(val, ctx);
                            output.push_str(&processed);
                        }
                    }
                    output.push_str("</dd>\n");
                }
            }
            _ => {}
        }
    }

    write_indent(output, indent);
    output.push_str("</dl>\n");
}

/// Generate an ID for a definition term from its text.
fn generate_term_id(text: &str, prefix: &str) -> String {
    let cleaned = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    // Remove leading/trailing hyphens and collapse multiple
    let collapsed: String = cleaned
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!("{prefix}{collapsed}")
}

/// Convert math block to HTML.
fn convert_math_block(elem: &Element, output: &mut String, options: &Options, indent: usize) {
    if let Some(ref content) = elem.value {
        let escaped = escape_html(content);

        // Check if math engine is disabled (None = disabled via `~`)
        let no_engine = options.math_engine.is_none();

        if no_engine {
            write_indent(output, indent);
            output.push_str("<div");
            write_attrs(&elem.attr, output);
            if !elem.attr.contains_key("class") {
                output.push_str(" class=\"kdmath\"");
            } else {
                // Append kdmath to existing class
            }
            output.push_str(">$$\n");
            output.push_str(&escaped);
            output.push_str("\n$$</div>\n");
        } else {
            // Default: use MathJax-style delimiters
            if elem.attr.is_empty() {
                write_indent(output, indent);
                output.push_str("\\[");
                output.push_str(&escaped);
                output.push_str("\\]\n");
            } else {
                // With attributes, wrap in a div
                write_indent(output, indent);
                output.push_str("<div");
                write_attrs(&elem.attr, output);
                output.push('>');
                output.push_str("\\[");
                output.push_str(&escaped);
                output.push_str("\\]\n");
                output.push_str("</div>\n");
            }
        }
    }
}

/// Convert block extension to HTML.
fn convert_block_extension(elem: &Element, output: &mut String, _indent: usize) {
    let ext_type = elem
        .options
        .get("ext_type")
        .map(|s| s.as_str())
        .unwrap_or("");
    match ext_type {
        "comment" => {
            // Output as HTML comment
            if let Some(ref val) = elem.value {
                if !val.is_empty() {
                    output.push_str("<!-- ");
                    output.push_str(val);
                    output.push_str(" -->\n");
                }
            }
        }
        "nomarkdown" => {
            // Output raw content
            if let Some(ref val) = elem.value {
                if !val.is_empty() {
                    output.push_str(val);
                    output.push('\n');
                }
            }
        }
        _ => {
            // Empty/options - no output
        }
    }
}

/// Render the footnotes section at the end of the document.
/// Uses the block parser to process footnote content (which can contain
/// headings, blockquotes, code blocks, lists, etc.).
fn render_footnotes(options: &Options, ctx: &mut SpanContext) -> String {
    if ctx.footnote_order.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let start_nr = options.footnote_nr as usize;
    if start_nr != 1 {
        output.push_str(&format!(
            "\n<div class=\"footnotes\" role=\"doc-endnotes\">\n  <ol start=\"{start_nr}\">\n"
        ));
    } else {
        output.push_str("\n<div class=\"footnotes\" role=\"doc-endnotes\">\n  <ol>\n");
    }

    let backlink_text = span_parser::encode_backlink_text(&options.footnote_backlink);
    let backlink_empty = options.footnote_backlink.is_empty();
    let prefix = options.footnote_prefix.clone();

    let mut i = 0;
    while i < ctx.footnote_order.len() {
        let name = ctx.footnote_order[i].clone();
        let content = ctx.footnote_defs.get(&name).cloned().unwrap_or_default();

        let prefixed_name = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}{name}")
        };
        let fn_id = format!("fn:{prefixed_name}");
        let fnref_id = format!("fnref:{prefixed_name}");

        output.push_str(&format!("    <li id=\"{fn_id}\">\n"));

        let fn_content_html = render_footnote_content(&content, options, ctx);

        if backlink_empty {
            output.push_str(&fn_content_html);
        } else {
            let ref_count = ctx.footnote_ref_counts.get(&name).copied().unwrap_or(1);
            let nbsp = "\u{a0}";
            let mut bl = format!(
                "{nbsp}<a href=\"#{fnref_id}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink_text}</a>"
            );
            for ref_num in 1..ref_count {
                bl.push_str(&format!(
                    "{nbsp}<a href=\"#{fnref_id}:{ref_num}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink_text}<sup>{}</sup></a>",
                    ref_num + 1
                ));
            }

            let inserted = insert_backlink_into_content(&fn_content_html, &bl);
            output.push_str(&inserted);
        }

        output.push_str("    </li>\n");
        i += 1;
    }

    output.push_str("  </ol>\n</div>\n");
    output
}

/// Render footnote content through the block parser for full block-level support.
fn render_footnote_content(content: &str, options: &Options, ctx: &mut SpanContext) -> String {
    if content.is_empty() {
        return String::new();
    }

    // Add trailing newline for the block parser
    let input = if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    };

    // Parse through block parser
    let doc = KramdownParser::parse(&input, options);

    // Convert to HTML with span processing (reuse the same context for footnote refs)
    let mut html_output = String::new();
    convert_children(&doc.root.children, &mut html_output, options, 6, ctx);

    html_output
}

/// Insert backlink HTML into the last paragraph (or last block element) of footnote content.
/// If the content ends with a <p>...</p>, insert the backlink before the closing </p>.
/// If it ends with another block element, append a new <p> with just the backlink.
fn insert_backlink_into_content(content: &str, backlink_html: &str) -> String {
    // Find the last </p> in the content
    if let Some(last_p_close) = content.rfind("</p>\n") {
        let mut result = String::new();
        result.push_str(&content[..last_p_close]);
        result.push_str(backlink_html);
        result.push_str(&content[last_p_close..]);
        return result;
    }

    // No paragraph found - append a new paragraph with the backlink
    let mut result = content.to_string();
    result.push_str(&format!("      <p>{backlink_html}</p>\n"));
    result
}

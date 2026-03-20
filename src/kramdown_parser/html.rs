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

/// Process paragraph text for typographic conversions.
fn process_paragraph_text(text: &str, _options: &Options) -> String {
    // Apply typographic conversions:
    // `---` -> em-dash, `--` -> en-dash
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
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
        } else if chars[i] == '\\' && i + 1 < chars.len() && chars[i + 1] == '#' {
            // Escaped hash
            result.push('#');
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

/// Escape HTML entities in code block content.
/// Only escapes &, <, > (not quotes, as they're safe in code blocks).
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

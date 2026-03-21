// kramdown parser - HTML converter
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use crate::kramdown_parser::element::{Document, Element, ElementType};
use crate::kramdown_parser::options::{EntityOutput, Options};
use crate::kramdown_parser::parser::KramdownParser;
use crate::kramdown_parser::span_parser::{self, SpanContext};
use crate::syntax::highlight_code;

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
        // Pre-collect headers for TOC generation
        if options.auto_ids {
            collect_headers(&doc.root.children, options, ctx);
        }

        let mut output = String::new();
        convert_children(&doc.root.children, &mut output, options, 0, ctx);

        // Render footnotes if any were referenced.
        // Use ctx.options which may have been updated by {::options} extensions.
        let effective_options = ctx.options.clone();
        let footnotes = render_footnotes(&effective_options, ctx);

        // Check if the output has a footnote placement marker
        let has_footnote_placeholder = output.contains("__FOOTNOTE_PLACEHOLDER__");

        if has_footnote_placeholder {
            // Replace the placeholder with footnote content
            output = output.replacen("__FOOTNOTE_PLACEHOLDER__", &footnotes, 1);
        } else {
            if !footnotes.is_empty() {
                // Before appending footnotes, handle trailing blank like kramdown Ruby:
                // If the document has trailing blank/invisible elements at the end, add separator.
                let has_trailing_blank = doc
                    .root
                    .children
                    .iter()
                    .rev()
                    .take_while(|e| {
                        e.element_type == ElementType::Blank
                            || e.element_type == ElementType::Eob
                            || (e.element_type == ElementType::BlockExtension
                                && !is_visible_block(e))
                    })
                    .any(|e| e.element_type == ElementType::Blank);
                if has_trailing_blank && !output.ends_with("\n\n") {
                    output.push('\n');
                }
                // Trim excess trailing newlines (keep at most one blank line before footnotes)
                while output.ends_with("\n\n\n") {
                    output.pop();
                }
            }

            output.push_str(&footnotes);
        }

        // Ensure trailing newline (kramdown always outputs at least \n)
        if output.is_empty() || !output.ends_with('\n') {
            output.push('\n');
        }

        // If the document ends with a trailing Blank element AND there was
        // visible content before it AND no footnotes were rendered,
        // kramdown outputs an extra trailing newline.
        if footnotes.is_empty() {
            let has_visible_before_blank = doc.root.children.iter().any(is_visible_block);
            let has_trailing_blank = doc
                .root
                .children
                .last()
                .is_some_and(|e| e.element_type == ElementType::Blank);
            if has_trailing_blank && has_visible_before_blank && !output.ends_with("\n\n") {
                output.push('\n');
            }
        }

        output
    }
}

/// Collect headers from the document for TOC generation.
fn collect_headers(children: &[Element], options: &Options, ctx: &mut SpanContext) {
    let mut used_ids: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for child in children {
        if child.element_type == ElementType::Header {
            let level = child
                .options
                .get("level")
                .and_then(|l| l.parse::<usize>().ok())
                .unwrap_or(1);
            // For html_to_native headers, use raw_text for ID generation
            let id_text = if let Some(raw_text) = child.options.get("raw_text") {
                raw_text.clone()
            } else {
                get_element_text(child)
            };
            let has_no_toc = child
                .attr
                .get("class")
                .is_some_and(|c| c.split_whitespace().any(|cls| cls == "no_toc"));

            // Generate ID from text (slug)
            let id = if let Some(existing_id) = child.attr.get("id") {
                existing_id.clone()
            } else {
                let base_id = generate_header_id(&id_text, options);
                let count = used_ids.entry(base_id.clone()).or_insert(0);
                let final_id = if *count == 0 {
                    base_id.clone()
                } else {
                    format!("{}-{}", base_id, count)
                };
                *count += 1;
                final_id
            };

            ctx.toc_headers.push((level, id, id_text, has_no_toc));
        }
    }
}

/// Generate a header ID from text (slug), matching kramdown Ruby `basic_generate_id`.
/// The raw source text is used directly (not span-processed), matching kramdown Ruby behavior
/// where `raw_text` is set from source during header creation.
fn generate_header_id(text: &str, options: &Options) -> String {
    // Process kramdown backslash escapes (e.g. \\\` -> \`)
    let clean = process_kramdown_escapes(text);

    // Match kramdown Ruby basic_generate_id:
    // 1. Strip leading non-alpha chars
    let stripped = clean.trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    // 2. Remove chars that are not [a-zA-Z0-9 -]
    let filtered: String = stripped
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect();
    // 3. Convert spaces to hyphens
    let result = filtered.replace(' ', "-");
    // 4. Downcase
    let result = result.to_lowercase();
    // 5. Apply auto_id_prefix
    format!("{}{}", options.auto_id_prefix, result)
}

/// Process kramdown backslash escapes in text (for ID generation).
/// E.g. `\\` -> `\`, `\`` -> `` ` ``, etc.
fn process_kramdown_escapes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                // Kramdown allows escaping these characters
                if "\\`*_{}[]()#+-.!>~|\"'/$".contains(next) {
                    result.push(next);
                    chars.next();
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Strip footnote reference markers from header text.
fn strip_footnote_refs_from_text(text: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == '^' {
            // Skip until closing ]
            let mut j = i + 2;
            while j < chars.len() && chars[j] != ']' {
                j += 1;
            }
            if j < chars.len() {
                i = j + 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Strip link reference syntax like [Text] from header text.
fn strip_link_refs_from_text(text: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        if chars[i] == '[' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != ']' {
                j += 1;
            }
            if j < chars.len() {
                // Extract text inside brackets
                let inner: String = chars[i + 1..j].iter().collect();
                result.push_str(&inner);
                i = j + 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// Generate TOC HTML from collected headers.
fn generate_toc(output: &mut String, options: &Options, ctx: &mut SpanContext) {
    // Parse toc_levels
    let (min_level, max_level) = parse_toc_levels(&options.toc_levels);

    // Filter headers for TOC (clone to avoid borrow conflict with ctx)
    let headers: Vec<(usize, String, String, bool)> = ctx
        .toc_headers
        .iter()
        .filter(|(level, _, _, no_toc)| !no_toc && *level >= min_level && *level <= max_level)
        .cloned()
        .collect();

    if headers.is_empty() {
        return;
    }

    output.push_str("<ul id=\"markdown-toc\">\n");

    let mut stack: Vec<usize> = Vec::new(); // stack of levels for nesting

    for (i, (level, id, text, _)) in headers.iter().enumerate() {
        let level = *level;

        // Close nested lists as needed
        while stack.last().is_some_and(|&l| l >= level) {
            stack.pop();
            let indent = 2 + stack.len() * 4;
            output.push_str(&" ".repeat(indent + 2));
            output.push_str("</ul>\n");
            output.push_str(&" ".repeat(indent));
            output.push_str("</li>\n");
        }

        let indent = 2 + stack.len() * 4;

        // Check if next header is deeper
        let next_is_deeper = headers.get(i + 1).is_some_and(|(nl, _, _, _)| *nl > level);

        let clean_text = strip_footnote_refs_from_text(text);
        let clean_text = strip_link_refs_from_text(&clean_text);
        // Process through span parser for escapes, typography, etc.
        let processed_text = span_parser::spans_to_html(&clean_text, ctx);

        output.push_str(&" ".repeat(indent));
        output.push_str(&format!(
            "<li><a href=\"#{}\" id=\"markdown-toc-{}\">{}</a>",
            id, id, processed_text
        ));

        if next_is_deeper {
            let ul_indent = "    ".repeat(stack.len() + 1);
            output.push_str(&ul_indent);
            output.push_str("<ul>\n");
            stack.push(level);
        } else {
            output.push_str("</li>\n");
        }
    }

    // Close remaining open lists
    while stack.pop().is_some() {
        let indent = 2 + stack.len() * 4;
        output.push_str(&" ".repeat(indent + 2));
        output.push_str("</ul>\n");
        output.push_str(&" ".repeat(indent));
        output.push_str("</li>\n");
    }

    output.push_str("</ul>\n");
}

/// Parse toc_levels string like "1..6" or "2..3" into (min, max).
fn parse_toc_levels(s: &str) -> (usize, usize) {
    if let Some((min_s, max_s)) = s.split_once("..") {
        let min = min_s.trim().parse().unwrap_or(1);
        let max = max_s.trim().parse().unwrap_or(6);
        (min, max)
    } else {
        (1, 6)
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

        // When the first visible block is preceded by invisible block extensions
        // and blank lines, output the leading blank (kramdown preserves it).
        if first && is_visible_block(child) && i > 0 {
            let had_invisible_before = children[..i]
                .iter()
                .any(|e| e.element_type == ElementType::BlockExtension && !is_visible_block(e));
            let had_blank_before = children[..i]
                .iter()
                .any(|e| e.element_type == ElementType::Blank);
            if had_invisible_before && had_blank_before {
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
            convert_code_block(elem, output, options, indent, ctx);
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
            convert_block_extension(elem, output, indent, options, ctx);
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
    output.push_str("<p");
    write_attrs(&elem.attr, output);
    output.push('>');

    let text = get_element_text(elem);
    // Strip CDATA sections before processing: <![CDATA[content]]> -> escaped content
    let text = strip_inline_cdata(&text);
    // If marked as raw_html (from html_to_native), don't markdown-process the text
    let is_raw_html = elem.options.get("raw_html").is_some_and(|v| v == "true");
    if is_raw_html {
        output.push_str(&text);
    } else {
        let processed = span_parser::spans_to_html(&text, ctx);
        output.push_str(&processed);
    }

    output.push_str("</p>\n");
}

/// Convert header to HTML.
fn convert_header(
    elem: &Element,
    output: &mut String,
    options: &Options,
    indent: usize,
    ctx: &mut SpanContext,
) {
    let level = elem
        .options
        .get("level")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(1);

    let is_raw_html = elem.options.get("raw_html").is_some_and(|v| v == "true");

    // Build attributes, adding auto-ID if needed
    let mut attrs = elem.attr.clone();
    if options.auto_ids {
        if !attrs.contains_key("id") {
            // Use sequential index into toc_headers to get the pre-computed ID
            // (handles duplicate headers correctly)
            if let Some((_, id, _, _)) = ctx.toc_headers.get(ctx.toc_header_index) {
                attrs.insert("id".to_string(), id.clone());
            } else {
                // Fallback: generate ID directly
                let id_text = if let Some(raw_text) = elem.options.get("raw_text") {
                    raw_text.clone()
                } else {
                    get_element_text(elem)
                };
                let id = generate_header_id(&id_text, options);
                attrs.insert("id".to_string(), id);
            }
        }
        // Always advance index to stay in sync with collect_headers
        ctx.toc_header_index += 1;
    }

    write_indent(output, indent);
    output.push_str(&format!("<h{level}"));
    write_attrs(&attrs, output);
    output.push('>');

    let text = get_element_text(elem);
    if is_raw_html {
        output.push_str(&text);
    } else {
        let processed = span_parser::spans_to_html(&text, ctx);
        output.push_str(&processed);
    }

    output.push_str(&format!("</h{level}>\n"));
}

/// Convert code block to HTML.
fn convert_code_block(
    elem: &Element,
    output: &mut String,
    _options: &Options,
    indent: usize,
    ctx: &SpanContext,
) {
    let content = elem.value.as_deref().unwrap_or("");
    let fence_language = elem.options.get("language");

    write_indent(output, indent);

    // Determine code language from fence, IAL class, or default_lang option
    let mut code_lang: Option<String> = fence_language.cloned();
    let mut pre_attrs: Vec<(String, String)> = Vec::new();

    for (key, value) in &elem.attr {
        if key == "class" {
            let classes: Vec<&str> = value.split_whitespace().collect();
            let mut pre_classes = Vec::new();
            for cls in &classes {
                if cls.starts_with("language-") && code_lang.is_none() {
                    code_lang = Some(cls.strip_prefix("language-").unwrap_or(cls).to_string());
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

    // Fall back to default_lang from syntax_highlighter_opts
    if code_lang.is_none() {
        if let Some(ref default_lang) = ctx.options.syntax_highlighter_opts.default_lang {
            code_lang = Some(default_lang.clone());
        }
    }

    // Try syntax highlighting with Rouge-compatible output
    let use_rouge = ctx.options.syntax_highlighter.as_deref() == Some("rouge");
    if use_rouge {
        if let Some(ref lang) = code_lang {
            if let Some(highlighted) = highlight_code(lang, content) {
                // Rouge-compatible output format
                output.push_str(&format!(
                    "<div class=\"language-{lang} highlighter-rouge\"><div class=\"highlight\"><pre class=\"highlight\"><code>"
                ));
                output.push_str(&highlighted);
                output.push_str("</code></pre>\n</div></div>\n");
                return;
            }
        }
    }

    // Plain output (no highlighting)
    output.push_str("<pre");
    for (key, value) in &pre_attrs {
        output.push_str(&format!(" {key}=\"{value}\""));
    }
    output.push_str("><code");

    if let Some(ref lang) = code_lang {
        output.push_str(&format!(" class=\"language-{lang}\""));
    }

    output.push('>');

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
    // TOC list: replace with generated TOC or suppress
    if elem.options.get("toc").is_some_and(|v| v == "true") {
        if options.auto_ids {
            // Generate TOC from collected headers
            generate_toc(output, options, ctx);
        }
        // If auto_ids is false, suppress the list (no output)
        return;
    }

    // Footnote placement marker: {: footnotes}
    // Output a unique placeholder that will be replaced with footnote content later
    if elem.options.get("footnotes").is_some_and(|v| v == "true") {
        output.push_str("__FOOTNOTE_PLACEHOLDER__");
        return;
    }

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
        let text = text.trim_end().to_string();
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
                let cell_is_header = cell.options.get("is_header").is_some_and(|v| v == "true");
                let tag = if is_head || cell_is_header {
                    "th"
                } else {
                    "td"
                };
                let alignment = alignments.get(ci).copied().unwrap_or("default");

                write_indent(output, indent + 6);
                output.push('<');
                output.push_str(tag);
                // If cell has its own attributes (from html_to_native), use those
                if !cell.attr.is_empty() {
                    write_attrs(&cell.attr, output);
                } else if alignment != "default" && !alignment.is_empty() {
                    output.push_str(&format!(" style=\"text-align: {alignment}\""));
                }
                output.push('>');

                // Write cell content
                let is_raw = cell.options.get("raw_html").is_some_and(|v| v == "true");
                let text = get_element_text(cell);
                if text.is_empty() {
                    output.push('\u{a0}');
                } else if is_raw {
                    output.push_str(&text);
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

/// Write HTML attributes in insertion order (matching kramdown Ruby behavior).
fn write_attrs(attrs: &indexmap::IndexMap<String, String>, output: &mut String) {
    for (key, value) in attrs {
        if key == "id" && value.trim().is_empty() {
            continue;
        }
        let escaped = escape_html_attr(value);
        output.push_str(&format!(" {key}=\"{escaped}\""));
    }
}

/// Escape HTML attribute value: escape &, <, >, and ".
fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Write indentation.
fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

/// Escape invalid HTML inside a block-level HTML element.
/// Process `markdown="span"` attributes inside raw HTML blocks.
/// Finds tags with `markdown="span"`, removes the attribute, and processes
/// the element's text content through span parsing (for abbreviations, etc.).
/// Process markdown attributes in raw HTML blocks.
/// Handles markdown="span", markdown="block", markdown="1", and markdown="0".
/// Tags with these attributes have their content processed as markdown accordingly.
fn process_markdown_span_in_raw_html(html: &str, ctx: &mut SpanContext) -> String {
    if !html.contains("markdown=") {
        return html.to_string();
    }

    let mut result = String::new();
    let mut remaining = html;

    // Match any markdown="..." attribute
    while let Some(md_start) = remaining.find("markdown=\"") {
        // Find the start of the opening tag containing this attribute
        let tag_open = match remaining[..md_start].rfind('<') {
            Some(pos) => pos,
            None => {
                result.push_str(&remaining[..md_start + 10]);
                remaining = &remaining[md_start + 10..];
                continue;
            }
        };

        // Extract the markdown attribute value
        let after_eq = &remaining[md_start + 10..]; // after markdown="
        let md_value_end = match after_eq.find('"') {
            Some(pos) => pos,
            None => {
                result.push_str(&remaining[..md_start + 10]);
                remaining = &remaining[md_start + 10..];
                continue;
            }
        };
        let md_value = &after_eq[..md_value_end];
        let marker = format!("markdown=\"{}\"", md_value);

        // If markdown="0", just strip the attribute and pass through
        if md_value == "0" {
            let full_marker_end = md_start + marker.len();
            result.push_str(&remaining[..md_start]);
            // Skip the markdown="0" attribute (and surrounding space)
            let before_marker = &remaining[..md_start];
            let trim_space = before_marker.ends_with(' ');
            if trim_space && !result.is_empty() && result.ends_with(' ') {
                // Already copied space, skip
            }
            remaining = &remaining[full_marker_end..];
            continue;
        }

        // Extract tag name
        let after_lt = &remaining[tag_open + 1..];
        let tag_name_end = after_lt
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(after_lt.len());
        let tag_name = &after_lt[..tag_name_end];

        if tag_name.is_empty() {
            result.push_str(&remaining[..md_start + marker.len()]);
            remaining = &remaining[md_start + marker.len()..];
            continue;
        }

        let tag_name_lc = tag_name.to_lowercase();

        // Find end of opening tag (the >)
        let after_full_marker = &remaining[md_start + marker.len()..];
        let gt_pos = match after_full_marker.find('>') {
            Some(pos) => md_start + marker.len() + pos,
            None => {
                result.push_str(&remaining[..md_start + marker.len()]);
                remaining = &remaining[md_start + marker.len()..];
                continue;
            }
        };

        // Build clean opening tag (without markdown="...")
        let full_tag = &remaining[tag_open..=gt_pos];
        let clean_tag = full_tag
            .replace(&format!(" {}", marker), "")
            .replace(&format!("{} ", marker), "")
            .replace(&marker, "");

        // Find the closing tag
        let content_start = gt_pos + 1;
        let close_tag_lc = format!("</{}>", tag_name_lc);
        // Find closing tag (case-insensitive for known HTML)
        let after_content = &remaining[content_start..];
        let close_pos = match find_close_tag_ci(after_content, &tag_name_lc) {
            Some(pos) => content_start + pos,
            None => {
                result.push_str(&remaining[..=gt_pos]);
                remaining = &remaining[gt_pos + 1..];
                continue;
            }
        };

        let inner_content = &remaining[content_start..close_pos];
        // Find actual close tag text (including >)
        let close_tag_actual = &remaining[close_pos..];
        let close_end = close_tag_actual
            .find('>')
            .map(|p| p + 1)
            .unwrap_or(close_tag_lc.len());

        // Determine parse mode based on markdown attribute value and tag content model
        let is_span_content = is_span_content_model(&tag_name_lc);
        let parse_as_block = match md_value {
            "block" => true,
            "span" => false,
            "1" => !is_span_content,
            _ => false,
        };

        // Output everything before the tag
        result.push_str(&remaining[..tag_open]);

        // Calculate indentation of the opening tag
        let tag_indent = {
            let before = &remaining[..tag_open];
            let nl = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_prefix = &before[nl..];
            let sp = line_prefix.len() - line_prefix.trim_start().len();
            " ".repeat(sp)
        };

        if parse_as_block {
            // Block-level processing
            let clean_tag_lc = clean_tag.to_lowercase();
            result.push_str(&clean_tag_lc);
            result.push('\n');
            // Indentation depends on context:
            // With parse_block_html, use deeper indent (tag+4 content, tag+2 close).
            // With standalone markdown attr, use simpler indent (tag+2 content, tag close).
            let (content_extra, close_extra) = if ctx.options.parse_block_html {
                (4usize, 2usize)
            } else {
                (2, 0)
            };
            let inner_trimmed = inner_content.trim();
            if !inner_trimmed.is_empty() {
                let options = ctx.options.clone();
                let inner_html = super::to_html_with_options(inner_trimmed, &options);
                let child_indent_str = " ".repeat(tag_indent.len() + content_extra);
                for line in inner_html.trim_end().lines() {
                    result.push_str(&child_indent_str);
                    result.push_str(line);
                    result.push('\n');
                }
            }
            let close_indent_str = " ".repeat(tag_indent.len() + close_extra);
            result.push_str(&close_indent_str);
            result.push_str(&close_tag_lc);
        } else {
            // Span-level processing
            result.push_str(&clean_tag);
            let processed = span_parser::spans_to_html(inner_content, ctx);
            result.push_str(&processed);
            result.push_str(&close_tag_lc);
        }

        remaining = &remaining[close_pos + close_end..];
    }

    result.push_str(remaining);
    result
}

/// Check if a tag has span-level content model (content parsed as inline).
fn is_span_content_model(tag_lc: &str) -> bool {
    matches!(
        tag_lc,
        "a" | "abbr"
            | "acronym"
            | "b"
            | "bdo"
            | "big"
            | "button"
            | "cite"
            | "caption"
            | "del"
            | "dfn"
            | "dt"
            | "em"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "i"
            | "ins"
            | "label"
            | "legend"
            | "optgroup"
            | "p"
            | "q"
            | "rb"
            | "rbc"
            | "rp"
            | "rt"
            | "rtc"
            | "ruby"
            | "select"
            | "small"
            | "span"
            | "strong"
            | "sub"
            | "sup"
            | "th"
            | "tt"
    )
}

/// Find a closing tag case-insensitively. Returns position of `</tag>` in the string.
fn find_close_tag_ci(s: &str, tag_lc: &str) -> Option<usize> {
    let s_lower = s.to_lowercase();
    let pattern = format!("</{}", tag_lc);
    s_lower.find(&pattern)
}

/// Orphan closing tags (no matching opener) and non-HTML angle bracket content
/// get their `<` and `>` escaped to `&lt;` and `&gt;`.
fn escape_invalid_html_in_block(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let lines: Vec<&str> = html.split('\n').collect();

    // Track which tags are open to detect orphan closing tags.
    // Each entry is (match_name, is_known_html) where match_name is lowercased
    // for known HTML elements and original case for unknown elements.
    let mut open_tags: Vec<(String, bool)> = Vec::new();
    // Track if we're inside the outermost block tag (skip escaping there)
    let mut depth = 0;
    let mut _first_line = true;
    // Track if we're inside a multi-line HTML comment
    let mut in_comment = false;
    // Track if we're inside a multi-line CDATA section
    let mut in_cdata = false;

    for (li, line) in lines.iter().enumerate() {
        if li > 0 {
            result.push('\n');
        }

        // If we're in a multi-line CDATA section, output content until ]]>
        if in_cdata {
            if let Some(end_idx) = line.find("]]>") {
                let inner = &line[..end_idx];
                result.push_str(&inner.replace('<', "&lt;").replace('>', "&gt;"));
                in_cdata = false;
                // Process rest of line normally after ]]>
                let rest = &line[end_idx + 3..];
                if !rest.is_empty() {
                    result.push_str(rest);
                }
                _first_line = false;
                continue;
            } else {
                // Still in CDATA - output line as text with entity escaping
                result.push_str(&line.replace('<', "&lt;").replace('>', "&gt;"));
                _first_line = false;
                continue;
            }
        }

        // If we're in a multi-line comment, pass through until -->
        if in_comment {
            if let Some(end_idx) = line.find("-->") {
                let comment_end = end_idx + 3;
                result.push_str(&line[..comment_end]);
                in_comment = false;
                // Process rest of line normally
                let rest = &line[comment_end..];
                if !rest.is_empty() {
                    result.push_str(rest);
                }
                _first_line = false;
                continue;
            } else {
                result.push_str(line);
                _first_line = false;
                continue;
            }
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
                        let is_known = is_known_html_element_name(&tag_lc);
                        // For known HTML elements, match case-insensitively (using lowercased name).
                        // For unknown elements, match case-sensitively (using original name).
                        let match_name = if is_known {
                            tag_lc.clone()
                        } else {
                            tag_name.clone()
                        };
                        // Check if this closing tag has a matching opener
                        let has_match = open_tags.iter().rev().any(|(name, _)| *name == match_name);
                        if depth <= 1 && !has_match {
                            // Orphan closing tag - escape it
                            result.push_str("&lt;");
                            i += 1;
                            continue;
                        }
                        // Valid closing tag
                        if let Some(pos) =
                            open_tags.iter().rposition(|(name, _)| *name == match_name)
                        {
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
                } else if let Some(after_comment_start) = remaining.strip_prefix("<!--") {
                    // HTML comment - pass through everything until -->
                    if let Some(end_idx) = after_comment_start.find("-->") {
                        let comment_end =
                            (remaining.len() - after_comment_start.len()) + end_idx + 3;
                        result.push_str(&remaining[..comment_end]);
                        i += comment_end;
                    } else {
                        // Comment continues past this line
                        result.push_str(&remaining);
                        i += remaining.len();
                        in_comment = true;
                    }
                    continue;
                } else if remaining.starts_with("<![CDATA[") {
                    // CDATA section - strip the CDATA wrapper and output content as text.
                    // Content inside CDATA has < and > escaped.
                    let content_start = 9; // length of "<![CDATA["
                    if let Some(end_idx) = remaining[content_start..].find("]]>") {
                        let inner = &remaining[content_start..content_start + end_idx];
                        result.push_str(&inner.replace('<', "&lt;").replace('>', "&gt;"));
                        i += content_start + end_idx + 3;
                    } else {
                        // CDATA continues past this line - strip opening marker, rest is text
                        let inner = &remaining[content_start..];
                        result.push_str(&inner.replace('<', "&lt;").replace('>', "&gt;"));
                        i += remaining.len();
                        in_cdata = true;
                    }
                    continue;
                } else if remaining.starts_with("<!") || remaining.starts_with("<?") {
                    // Other PI/declaration - pass through the < and continue
                    result.push(chars[i]);
                    i += 1;
                    continue;
                } else {
                    // Opening tag candidate
                    if let Some((tag_name, end_pos, is_self_closing)) =
                        extract_tag_info_from_opening(&remaining)
                    {
                        let tag_lc = tag_name.to_lowercase();
                        let is_known = is_known_html_element_name(&tag_lc);
                        if !is_self_closing && !is_void_tag_name(&tag_lc) {
                            // For known HTML elements, store lowercased name.
                            // For unknown elements, store original case for case-sensitive matching.
                            let match_name = if is_known { tag_lc } else { tag_name.clone() };
                            open_tags.push((match_name, is_known));
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

    // Add synthetic closing tags for any tags left open (auto-close).
    // Use the stored match_name which preserves original case for unknown elements.
    for (match_name, _is_known) in open_tags.iter().rev() {
        result.push('\n');
        result.push_str("</");
        result.push_str(match_name);
        result.push('>');
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

/// Check if a lowercased tag name is a known HTML element (block, span, or void).
/// Used to determine case-sensitivity: known HTML elements match case-insensitively,
/// unknown elements (like XML or custom tags) match case-sensitively.
fn is_known_html_element_name(tag_lc: &str) -> bool {
    // Combine all known HTML element categories
    matches!(
        tag_lc,
        // Block elements
        "address" | "article" | "aside" | "blockquote" | "body" | "caption" | "center"
        | "col" | "colgroup" | "dd" | "details" | "dialog" | "dir" | "div" | "dl" | "dt"
        | "fieldset" | "figcaption" | "figure" | "footer" | "form" | "frame" | "frameset"
        | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "head" | "header" | "hgroup" | "hr"
        | "html" | "iframe" | "legend" | "li" | "link" | "main" | "map" | "math" | "menu"
        | "menuitem" | "meta" | "nav" | "noframes" | "noscript" | "ol" | "optgroup"
        | "option" | "p" | "param" | "pre" | "section" | "source" | "summary" | "svg"
        | "table" | "tbody" | "td" | "textarea" | "tfoot" | "th" | "thead" | "title"
        | "tr" | "track" | "ul"
        // Span/inline elements
        | "a" | "abbr" | "acronym" | "b" | "bdi" | "bdo" | "big" | "button" | "cite"
        | "code" | "del" | "dfn" | "em" | "font" | "i" | "ins" | "kbd" | "label" | "mark"
        | "q" | "rb" | "rbc" | "rp" | "rt" | "rtc" | "ruby" | "s" | "samp" | "select"
        | "small" | "span" | "strike" | "strong" | "sub" | "sup" | "time" | "tt" | "u"
        | "var"
        // Void elements
        | "area" | "base" | "br" | "embed" | "img" | "input" | "wbr"
        // Content model elements
        | "script" | "style" | "applet"
    )
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

/// Strip inline CDATA sections from text.
/// `<![CDATA[content]]>` -> escaped content (< and > become &lt; and &gt;).
/// This follows kramdown behavior where CDATA in span-level content is stripped
/// and the inner content is treated as text with HTML entities escaped.
fn strip_inline_cdata(text: &str) -> String {
    if !text.contains("<![CDATA[") {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find("<![CDATA[") {
        result.push_str(&remaining[..start]);
        let after_marker = &remaining[start + 9..];
        if let Some(end) = after_marker.find("]]>") {
            let inner = &after_marker[..end];
            // Strip CDATA markers, preserve content as-is.
            // The span parser will handle escaping of < and > in the content.
            result.push_str(inner);
            remaining = &after_marker[end + 3..];
        } else {
            result.push_str(after_marker);
            remaining = "";
        }
    }
    result.push_str(remaining);
    result
}

/// Inject IAL attributes into the first HTML tag in a raw HTML string.
/// E.g., inject class="cls" into `<div>` -> `<div class="cls">`
fn inject_attrs_into_html(html: &str, attrs: &indexmap::IndexMap<String, String>) -> String {
    if attrs.is_empty() {
        return html.to_string();
    }
    // Find the first `>` that closes an opening tag
    if let Some(gt_pos) = html.find('>') {
        let before_gt = &html[..gt_pos];
        // Build attribute string
        let mut attr_str = String::new();
        write_attrs(attrs, &mut attr_str);
        let mut result = String::with_capacity(html.len() + attr_str.len());
        result.push_str(before_gt);
        result.push_str(&attr_str);
        result.push_str(&html[gt_pos..]);
        result
    } else {
        html.to_string()
    }
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
            let orig_attrs = elem.options.get("attrs").map(|s| s.as_str()).unwrap_or("");
            write_indent(output, indent);
            output.push('<');
            output.push_str(tag);
            output.push_str(orig_attrs);
            // Also inject IAL attributes
            write_attrs(&elem.attr, output);
            output.push_str(">\n");
            convert_children(&elem.children, output, options, indent + 2, ctx);
            write_indent(output, indent);
            output.push_str("</");
            output.push_str(tag);
            output.push_str(">\n");
        }
        Some("span") => {
            // Span-parsed HTML: content is parsed as spans while preserving
            // the original whitespace structure (leading/trailing newlines).
            let tag = elem.options.get("tag").map(|s| s.as_str()).unwrap_or("p");
            let attrs = elem.options.get("attrs").map(|s| s.as_str()).unwrap_or("");
            let multiline = elem
                .options
                .get("multiline_span")
                .is_some_and(|v| v == "true");
            let close_solo = elem.options.get("close_solo").is_some_and(|v| v == "true");
            write_indent(output, indent);
            output.push('<');
            output.push_str(tag);
            output.push_str(attrs);
            output.push('>');
            if let Some(ref val) = elem.value {
                let processed = span_parser::spans_to_html(val.trim(), ctx);
                if multiline {
                    output.push('\n');
                    output.push_str(&processed);
                    output.push('\n');
                } else {
                    output.push_str(&processed);
                    if close_solo {
                        output.push('\n');
                    }
                }
            }
            output.push_str("</");
            output.push_str(tag);
            output.push_str(">\n");
        }
        _ => {
            // If parse_block_html is enabled mid-document via {::options},
            // re-process raw HTML blocks as block-parsed HTML.
            let parse_mode_check = elem.options.get("parse_mode").map(|s| s.as_str());
            // Only reprocess if parse_block_html was CHANGED mid-document (not initial)
            let was_changed_mid_doc = ctx.options.parse_block_html && !options.parse_block_html;
            if parse_mode_check.is_none() && was_changed_mid_doc {
                if let Some(ref val) = elem.value {
                    let opts = ctx.options.clone();
                    let reprocessed = super::to_html_with_options(val.trim(), &opts);
                    write_indent(output, indent);
                    output.push_str(reprocessed.trim());
                    output.push('\n');
                    return;
                }
            }
            // Raw HTML: pass through, but inject IAL attributes into opening tag
            let elem_type = elem.options.get("type").map(|s| s.as_str());
            let skip_escape = matches!(elem_type, Some("comment") | Some("raw"));
            if let Some(ref val) = elem.value {
                if !val.is_empty() {
                    let html_str = if !elem.attr.is_empty() {
                        inject_attrs_into_html(val, &elem.attr)
                    } else {
                        val.clone()
                    };
                    // Process markdown="span" elements inside raw HTML
                    let html_str = process_markdown_span_in_raw_html(&html_str, ctx);
                    // Indent the first line of raw HTML when nested (e.g., in footnotes)
                    write_indent(output, indent);
                    if skip_escape {
                        output.push_str(&html_str);
                    } else {
                        let processed = escape_invalid_html_in_block(&html_str);
                        output.push_str(&processed);
                    }
                    if !html_str.ends_with('\n') {
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
    // Check for auto_ids directive in IAL refs
    let mut auto_ids_prefix: Option<String> = None;
    let dl_attrs = elem.attr.clone();
    // Look for auto_ids or auto_ids-prefix- style refs in ial_refs option
    if let Some(refs_str) = elem.options.get("ial_refs") {
        for ref_name in refs_str.split(',') {
            if ref_name == "auto_ids" {
                auto_ids_prefix = Some(String::new());
            } else if let Some(rest) = ref_name.strip_prefix("auto_ids-") {
                // auto_ids-prefix- -> prefix is everything between "auto_ids-" and trailing "-"
                if let Some(prefix) = rest.strip_suffix('-') {
                    auto_ids_prefix = Some(format!("{prefix}-"));
                } else {
                    auto_ids_prefix = Some(rest.to_string());
                }
            }
        }
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
    // Match kramdown Ruby basic_generate_id:
    let stripped = text.trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    let filtered: String = stripped
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect();
    let result = filtered.replace(' ', "-").to_lowercase();
    format!("{prefix}{result}")
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
            // MathJax \[...\] output is always at column 0 (no indent),
            // matching kramdown Ruby behavior.
            if elem.attr.is_empty() {
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
fn convert_block_extension(
    elem: &Element,
    output: &mut String,
    _indent: usize,
    _options: &Options,
    ctx: &mut SpanContext,
) {
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
        "options" => {
            // Apply inline options changes
            if let Some(opts_str) = elem.options.get("options_str") {
                apply_inline_options(&opts_str.clone(), ctx);
            }
        }
        _ => {
            // Empty - no output
        }
    }
}

/// Apply inline option changes from a {::options ...} extension.
fn apply_inline_options(opts_str: &str, ctx: &mut SpanContext) {
    let mut remaining = opts_str.trim();

    while !remaining.is_empty() {
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            break;
        }

        let eq_pos = match remaining.find('=') {
            Some(p) => p,
            None => break,
        };
        let key = remaining[..eq_pos].trim();
        remaining = &remaining[eq_pos + 1..];

        let value = if remaining.starts_with('"') {
            let rest = &remaining[1..];
            let end = find_end_of_quoted_value(rest, '"');
            let val = &rest[..end];
            remaining = if end + 1 < rest.len() {
                &rest[end + 1..]
            } else {
                ""
            };
            val.replace("\\}", "}").replace("\\\"", "\"")
        } else if remaining.starts_with('\'') {
            let rest = &remaining[1..];
            let end = find_end_of_quoted_value(rest, '\'');
            let val = &rest[..end];
            remaining = if end + 1 < rest.len() {
                &rest[end + 1..]
            } else {
                ""
            };
            val.replace("\\}", "}").replace("\\'", "'")
        } else {
            let end = remaining
                .find(char::is_whitespace)
                .unwrap_or(remaining.len());
            let val = &remaining[..end];
            remaining = &remaining[end..];
            val.to_string()
        };

        apply_single_option(key, &value, ctx);
    }
}

fn find_end_of_quoted_value(s: &str, quote: char) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        if chars[i] == quote {
            return i;
        }
        i += 1;
    }
    s.len()
}

fn apply_single_option(key: &str, value: &str, ctx: &mut SpanContext) {
    if key == "template" {
        return;
    }

    match key {
        "parse_block_html" => ctx.options.parse_block_html = parse_bool_val(value),
        "parse_span_html" => ctx.options.parse_span_html = parse_bool_val(value),
        "footnote_nr" => {
            if let Ok(nr) = value.parse::<u32>() {
                ctx.options.footnote_nr = nr;
                ctx.footnote_counter = nr as usize;
            }
        }
        "auto_ids" => ctx.options.auto_ids = parse_bool_val(value),
        "auto_id_prefix" => ctx.options.auto_id_prefix = value.to_string(),
        "syntax_highlighter" => {
            ctx.options.syntax_highlighter = if value == "~" || value.is_empty() {
                None
            } else {
                Some(value.to_string())
            };
        }
        "syntax_highlighter_opts" => {
            let inner = value.trim_start_matches('{').trim_end_matches('}');
            for pair in inner.split(',') {
                let pair = pair.trim();
                if let Some(colon) = pair.find(':') {
                    let k = pair[..colon].trim();
                    let v = pair[colon + 1..].trim();
                    match k {
                        "default_lang" => {
                            ctx.options.syntax_highlighter_opts.default_lang = Some(v.to_string())
                        }
                        "css" => {
                            ctx.options.syntax_highlighter_opts.block_css = Some(v.to_string())
                        }
                        "wrap" => ctx.options.syntax_highlighter_opts.wrap = Some(v.to_string()),
                        "guess_lang" => {
                            ctx.options.syntax_highlighter_opts.guess_lang = Some(parse_bool_val(v))
                        }
                        _ => {}
                    }
                }
            }
        }
        "entity_output" => {
            ctx.options.entity_output = match value {
                "as_char" => crate::kramdown_parser::options::EntityOutput::AsChar,
                "as_input" => crate::kramdown_parser::options::EntityOutput::AsInput,
                "symbolic" => crate::kramdown_parser::options::EntityOutput::Symbolic,
                "numeric" => crate::kramdown_parser::options::EntityOutput::Numeric,
                _ => ctx.options.entity_output.clone(),
            };
        }
        "footnote_backlink_inline" => ctx.options.footnote_backlink_inline = parse_bool_val(value),
        _ => {}
    }
}

fn parse_bool_val(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "yes" | "1")
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
            "<div class=\"footnotes\" role=\"doc-endnotes\">\n  <ol start=\"{start_nr}\">\n"
        ));
    } else {
        output.push_str("<div class=\"footnotes\" role=\"doc-endnotes\">\n  <ol>\n");
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

        // In kramdown Ruby, a blank line is added after <li> when the first child
        // is NOT a transparent paragraph. We detect this by checking:
        // - content is empty (blank footnote)
        // - content starts with a blank line (explicit block separation)
        // - the rendered HTML does NOT start with a <p> tag (e.g., code block, list, etc.)
        let starts_with_blank = content.starts_with('\n');
        let is_non_para = !fn_content_html.trim_start().starts_with("<p");
        if content.is_empty() || starts_with_blank || is_non_para {
            output.push('\n');
        }

        if backlink_empty {
            output.push_str(&fn_content_html);
        } else {
            let ref_count = ctx.footnote_ref_counts.get(&name).copied().unwrap_or(1);
            let nbsp = entity_to_str('\u{a0}', "nbsp", &ctx.options);
            let mut bl = format!(
                "{nbsp}<a href=\"#{fnref_id}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink_text}</a>"
            );
            for ref_num in 1..ref_count {
                bl.push_str(&format!(
                    "{nbsp}<a href=\"#{fnref_id}:{ref_num}\" class=\"reversefootnote\" role=\"doc-backlink\">{backlink_text}<sup>{}</sup></a>",
                    ref_num + 1
                ));
            }

            let inserted = insert_backlink_into_content(
                &fn_content_html,
                &bl,
                options.footnote_backlink_inline,
            );
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

    // Pre-collect headers for auto-ID generation in footnote content
    if options.auto_ids {
        collect_headers(&doc.root.children, options, ctx);
    }

    // Convert to HTML with span processing (reuse the same context for footnote refs)
    let mut html_output = String::new();
    convert_children(&doc.root.children, &mut html_output, options, 6, ctx);

    // Remove any trailing blank lines from footnote content
    while html_output.ends_with("\n\n") {
        html_output.pop();
    }

    html_output
}

/// Insert backlink HTML into the last paragraph of footnote content.
/// In kramdown, the backlink goes into the last top-level `<p>` element.
/// If the last top-level element is not a `<p>`, a new `<p>` is appended.
/// If `backlink_inline` is true, traverse into nested elements to find deepest last p/header.
fn insert_backlink_into_content(
    content: &str,
    backlink_html: &str,
    backlink_inline: bool,
) -> String {
    if backlink_inline {
        return insert_backlink_inline(content, backlink_html);
    }

    // Find the last top-level </p> (not nested inside other elements)
    if let Some(pos) = find_last_top_level_p_close(content) {
        let mut result = String::new();
        result.push_str(&content[..pos]);
        result.push_str(backlink_html);
        result.push_str(&content[pos..]);
        return result;
    }

    // No top-level paragraph found - append a new paragraph with the backlink.
    // Strip leading nbsp from backlink since it's in its own paragraph (no space needed).
    let trimmed_bl = strip_leading_nbsp(backlink_html);
    let mut result = content.to_string();
    result.push_str(&format!("      <p>{trimmed_bl}</p>\n"));
    result
}

/// Strip a leading non-breaking space character or entity from the string.
fn strip_leading_nbsp(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix('\u{a0}') {
        return rest;
    }
    if let Some(rest) = s.strip_prefix("&nbsp;") {
        return rest;
    }
    if let Some(rest) = s.strip_prefix("&#160;") {
        return rest;
    }
    s
}

/// Check if the content ends with a top-level `</p>` and return its position.
/// Only returns a position if the LAST top-level block element is a `<p>`.
/// This matches kramdown's behavior: backlink goes in the last `<p>` only if
/// the last child is a paragraph.
fn find_last_top_level_p_close(content: &str) -> Option<usize> {
    let trimmed = content.trim_end_matches('\n');
    // Check if the content ends with </p>
    if !trimmed.ends_with("</p>") {
        return None;
    }
    // Find the position of this last </p>
    let p_close_pos = trimmed.rfind("</p>")?;
    // Verify it's at top level by scanning depth up to this position
    let mut depth: usize = 0;
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < p_close_pos {
        if bytes[i] == b'<' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                let tag_start = i + 2;
                let tag_end = content[tag_start..]
                    .find('>')
                    .map(|p| tag_start + p)
                    .unwrap_or(bytes.len());
                let tag_name = content[tag_start..tag_end].trim().to_lowercase();
                if is_nesting_block_tag(&tag_name) {
                    depth = depth.saturating_sub(1);
                }
            } else {
                let tag_start = i + 1;
                let tag_end = content[tag_start..]
                    .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                    .map(|p| tag_start + p)
                    .unwrap_or(bytes.len());
                let tag_name = content[tag_start..tag_end].to_lowercase();
                if is_nesting_block_tag(&tag_name) {
                    depth += 1;
                }
            }
        }
        i += 1;
    }
    if depth == 0 {
        Some(p_close_pos)
    } else {
        None
    }
}

/// Check if a tag name is a block element that creates nesting depth.
fn is_nesting_block_tag(tag: &str) -> bool {
    matches!(
        tag,
        "blockquote" | "div" | "ul" | "ol" | "table" | "li" | "dd" | "dl" | "details" | "section"
    )
}

/// Insert backlink into the deepest last p/header element (for footnote_backlink_inline mode).
fn insert_backlink_inline(content: &str, backlink_html: &str) -> String {
    // For inline mode, find the deepest last paragraph or header element
    // Try </p> first, then </h1> through </h6>
    let close_tags = [
        "</p>\n", "</h1>\n", "</h2>\n", "</h3>\n", "</h4>\n", "</h5>\n", "</h6>\n",
    ];

    // Find the last occurrence of any of these closing tags
    let mut best_pos = None;
    for tag in &close_tags {
        if let Some(pos) = content.rfind(tag) {
            match best_pos {
                None => best_pos = Some(pos),
                Some(bp) if pos > bp => best_pos = Some(pos),
                _ => {}
            }
        }
    }

    if let Some(pos) = best_pos {
        let mut result = String::new();
        result.push_str(&content[..pos]);
        // For inline mode, keep the nbsp from backlink_html (it already has the nbsp prefix)
        result.push_str(backlink_html);
        result.push_str(&content[pos..]);
        return result;
    }

    // No p/header found - append a new paragraph with the backlink (strip leading nbsp)
    let trimmed_bl = strip_leading_nbsp(backlink_html);
    let mut result = content.to_string();
    result.push_str(&format!("      <p>{trimmed_bl}</p>\n"));
    result
}

/// Convert a character to its HTML entity string, respecting entity_output option.
fn entity_to_str(ch: char, name: &str, options: &Options) -> String {
    match options.entity_output {
        EntityOutput::AsChar => ch.to_string(),
        EntityOutput::Symbolic => format!("&{name};"),
        EntityOutput::Numeric => format!("&#{};", ch as u32),
        EntityOutput::AsInput => format!("&{name};"),
    }
}

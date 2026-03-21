// kramdown parser - Rust reimplementation
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
//
// Some test cases are based on the MDTest test suite:
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>
//
// See LICENSE-kramdown in this directory for the full license text.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.

pub mod element;
pub mod entities;
pub mod html;
pub mod html_to_native;
pub mod options;
pub mod parser;
pub mod span_parser;

use element::Document;
use html::HtmlConverter;
use options::Options;
use parser::KramdownParser;
use span_parser::SpanContext;

/// Parse kramdown input and convert to HTML using default options.
pub fn to_html(input: &str) -> String {
    let options = Options::default();
    to_html_with_options(input, &options)
}

/// Parse kramdown input and convert to HTML using the given options.
pub fn to_html_with_options(input: &str, options: &Options) -> String {
    // Create span context and extract definitions (link defs, abbreviations, footnotes)
    let mut span_ctx = SpanContext::new(options);
    let cleaned = span_parser::extract_definitions(input, &mut span_ctx);

    // Parse blocks from the cleaned text (definitions removed)
    // Pass ALDs extracted by extract_definitions so IAL can resolve ALD references
    let mut doc: Document =
        KramdownParser::parse_with_alds(&cleaned, options, &mut span_ctx.ald_defs);

    // Convert HTML blocks to native kramdown elements when html_to_native is enabled
    if options.html_to_native {
        html_to_native::convert_html_to_native(&mut doc.root.children, options);
    }

    // Convert to HTML with span processing
    let mut html = HtmlConverter::convert_with_context(&doc, options, &mut span_ctx);

    // When html_to_native is enabled, convert inline HTML tags in the final output:
    // <b> -> <strong>, <i> -> <em>, and strip tags inside <code> spans.
    if options.html_to_native {
        html = html_to_native::convert_inline_html_tags(&html);
    }

    // When definitions were extracted from the start of the document, the cleaned text
    // starts with blank lines. In kramdown, blank elements produce "\n" in output.
    // The block parser creates Blank elements but the converter skips them.
    // We need to preserve leading blank lines that result from definition extraction.
    if cleaned.starts_with('\n') && !input.starts_with('\n') && !html.starts_with('\n') {
        html.insert(0, '\n');
    }

    // Kramdown preserves trailing blank lines: if the cleaned text (after def extraction)
    // ended with a blank line AND there are no footnotes (which consume trailing blanks),
    // ensure the output also ends with \n\n.
    if cleaned.ends_with("\n\n")
        && html.len() > 1
        && !html.ends_with("\n\n")
        && span_ctx.footnote_order.is_empty()
    {
        html.push('\n');
    }

    // Raw HTML block tags (script, style) at the end of input with a trailing newline
    // produce an extra blank in kramdown (the trailing \n becomes a blank element in the AST).
    // If the output ends with a raw HTML closing tag followed by \n, add the extra \n.
    if cleaned.ends_with('\n')
        && !cleaned.ends_with("\n\n")
        && html.ends_with('\n')
        && !html.ends_with("\n\n")
    {
        let trimmed_html = html.trim_end();
        if trimmed_html.ends_with("</script>")
            || trimmed_html.ends_with("</style>")
            || trimmed_html.ends_with("</math>")
        {
            html.push('\n');
        }
    }

    html
}

#[cfg(test)]
mod tests;

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
    let doc: Document = KramdownParser::parse(&cleaned, options);

    // Convert to HTML with span processing
    HtmlConverter::convert_with_context(&doc, options, &mut span_ctx)
}

#[cfg(test)]
mod tests;

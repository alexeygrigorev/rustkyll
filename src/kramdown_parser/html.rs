// kramdown parser - HTML converter stub
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use crate::kramdown_parser::element::Document;
use crate::kramdown_parser::options::Options;

/// Converts a kramdown Document AST into HTML output.
///
/// This is currently a stub that returns an empty string. Actual HTML conversion
/// will be implemented in Phase 2 (block elements) and Phase 3 (span elements).
pub struct HtmlConverter;

impl HtmlConverter {
    /// Convert a Document AST to an HTML string.
    ///
    /// Currently returns an empty string. Real conversion is Phase 2/3 work.
    pub fn convert(_doc: &Document, _options: &Options) -> String {
        String::new()
    }
}

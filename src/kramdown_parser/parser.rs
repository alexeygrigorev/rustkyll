// kramdown parser - Parser stub
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use crate::kramdown_parser::element::{Document, Element, ElementType};
use crate::kramdown_parser::options::Options;

/// The kramdown parser. Converts kramdown-flavored Markdown text into a Document AST.
///
/// This is currently a stub that returns a minimal document. Actual parsing
/// will be implemented in Phase 2 (block elements) and Phase 3 (span elements).
pub struct KramdownParser;

impl KramdownParser {
    /// Parse kramdown input text into a Document AST.
    ///
    /// Currently returns a Document with a single Text child containing the raw input.
    pub fn parse(input: &str, _options: &Options) -> Document {
        let mut doc = Document::new();
        if !input.is_empty() {
            let text = Element::with_value(ElementType::Text, input);
            doc.root.children.push(text);
        }
        doc
    }
}

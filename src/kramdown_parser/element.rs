// kramdown parser - Element types
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use std::collections::HashMap;

/// HTML attributes map (id, class, arbitrary key-value pairs).
/// Supports IAL (inline attribute lists) from kramdown.
pub type Attr = HashMap<String, String>;

/// All kramdown element types, covering block and span categories.
/// Reference: kramdown 2.5.2 `element.rb`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementType {
    // Block elements
    Root,
    Blank,
    Paragraph,
    Header,
    Blockquote,
    CodeBlock,
    HorizontalRule,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    HtmlBlock,
    DefinitionList,
    DefinitionTerm,
    DefinitionDefinition,
    MathBlock,
    Toc,
    Eob,
    BlockExtension,

    // Span elements
    Text,
    Emphasis,
    Strong,
    Link,
    Image,
    CodeSpan,
    LineBreak,
    SmartQuote,
    TypedSymbol,
    HtmlSpan,
    FootnoteRef,
    FootnoteMarker,
    Abbreviation,
    MathInline,
    SpanExtension,
    EscapedChar,
}

/// A single node in the kramdown AST.
#[derive(Debug, Clone)]
pub struct Element {
    /// The type of this element.
    pub element_type: ElementType,
    /// The text value of this element (for Text, CodeBlock, etc.).
    pub value: Option<String>,
    /// HTML attributes for this element.
    pub attr: Attr,
    /// Child elements.
    pub children: Vec<Element>,
    /// Additional options specific to this element (e.g., header level).
    pub options: HashMap<String, String>,
}

impl Element {
    /// Create a new element of the given type.
    pub fn new(element_type: ElementType) -> Self {
        Self {
            element_type,
            value: None,
            attr: Attr::new(),
            children: Vec::new(),
            options: HashMap::new(),
        }
    }

    /// Create a new element with a text value.
    pub fn with_value(element_type: ElementType, value: impl Into<String>) -> Self {
        Self {
            element_type,
            value: Some(value.into()),
            attr: Attr::new(),
            children: Vec::new(),
            options: HashMap::new(),
        }
    }
}

/// The top-level document, wrapping a root element.
#[derive(Debug, Clone)]
pub struct Document {
    /// The root element of the document tree.
    pub root: Element,
}

impl Document {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            root: Element::new(ElementType::Root),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

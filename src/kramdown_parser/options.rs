// kramdown parser - Options
//
// Based on kramdown by Thomas Leitner (MIT License)
// Copyright (C) 2009-2013 Thomas Leitner <t_leitner@gmx.at>
// See LICENSE-kramdown in this directory for the full license text.
//
// Some test cases based on MDTest by Michel Fortin
// Copyright (c) 2007 Michel Fortin <http://www.michelf.com/>

use std::collections::HashMap;
use std::path::Path;

/// How HTML entities should be output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum EntityOutput {
    /// Output as actual characters (e.g., &amp; -> &)
    #[default]
    AsChar,
    /// Output as the original input form
    AsInput,
    /// Output as symbolic entities (e.g., &amp;)
    Symbolic,
    /// Output as numeric entities (e.g., &#38;)
    Numeric,
}

/// Syntax highlighter options (block/span sub-keys and other settings).
#[derive(Debug, Clone, Default)]
pub struct SyntaxHighlighterOpts {
    /// CSS class mode for block code ("class" or "style")
    pub block_css: Option<String>,
    /// Default language for code blocks
    pub default_lang: Option<String>,
    /// Wrap mode (e.g., "span", "div")
    pub wrap: Option<String>,
    /// Line numbers setting
    pub line_numbers: Option<String>,
    /// Whether to guess the language
    pub guess_lang: Option<bool>,
}

/// A link definition: (url, optional title).
pub type LinkDef = (String, Option<String>);

/// Parser and converter options, matching kramdown 2.5.2 defaults.
#[derive(Debug, Clone)]
pub struct Options {
    // Header/ID options
    pub auto_ids: bool,
    pub auto_id_prefix: String,
    pub auto_id_stripping: bool,
    pub transliterated_header_ids: bool,
    pub header_offset: i32,
    pub header_links: bool,

    // Entity output
    pub entity_output: EntityOutput,

    // Footnote options
    pub footnote_nr: u32,
    pub footnote_prefix: String,
    pub footnote_backlink: String,
    pub footnote_backlink_inline: bool,
    pub footnote_link_text: String,

    // Math
    pub math_engine: Option<String>,

    // HTML parsing
    pub parse_block_html: bool,
    pub html_to_native: bool,

    // Smart quotes and typography
    pub smart_quotes: Vec<String>,
    pub typographic_symbols: HashMap<String, String>,

    // Syntax highlighting
    pub syntax_highlighter: Option<String>,
    pub syntax_highlighter_opts: SyntaxHighlighterOpts,
    pub enable_coderay: bool,

    // Link definitions
    pub link_defs: HashMap<String, LinkDef>,

    // TOC
    pub toc_levels: String,

    // CJK
    pub remove_line_breaks_for_cjk: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            auto_ids: false,
            auto_id_prefix: String::new(),
            auto_id_stripping: false,
            transliterated_header_ids: false,
            header_offset: 0,
            header_links: false,
            entity_output: EntityOutput::default(),
            footnote_nr: 1,
            footnote_prefix: String::new(),
            footnote_backlink: "\u{21A9}\u{FE0E}".to_string(),
            footnote_backlink_inline: false,
            footnote_link_text: String::new(),
            math_engine: Some("mathjax".to_string()),
            parse_block_html: false,
            html_to_native: false,
            smart_quotes: vec![
                "lsquo".to_string(),
                "rsquo".to_string(),
                "ldquo".to_string(),
                "rdquo".to_string(),
            ],
            typographic_symbols: HashMap::new(),
            syntax_highlighter: Some("rouge".to_string()),
            syntax_highlighter_opts: SyntaxHighlighterOpts::default(),
            enable_coderay: false,
            link_defs: HashMap::new(),
            toc_levels: "1..6".to_string(),
            remove_line_breaks_for_cjk: false,
        }
    }
}

impl Options {
    /// Load options from a `.options` file (kramdown YAML format with Ruby-style `:symbol` keys).
    /// Returns default options if the file does not exist.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        Self::parse_options_str(&content)
    }

    /// Parse options from a string in kramdown `.options` format.
    pub fn parse_options_str(content: &str) -> Result<Self, String> {
        let mut opts = Self::default();

        // Track if we're inside a nested block (like syntax_highlighter_opts or typographic_symbols)
        let mut in_syntax_highlighter_opts = false;
        let mut in_typographic_symbols = false;
        let mut in_link_defs = false;
        let mut current_link_key: Option<String> = None;

        for line in content.lines() {
            // Detect nested blocks by indentation
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                // Top-level key
                in_syntax_highlighter_opts = false;
                in_typographic_symbols = false;
                in_link_defs = false;
                current_link_key = None;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Handle indented sub-keys for syntax_highlighter_opts
            if in_syntax_highlighter_opts && (line.starts_with(' ') || line.starts_with('\t')) {
                Self::parse_syntax_highlighter_opt(trimmed, &mut opts.syntax_highlighter_opts);
                continue;
            }

            // Handle indented sub-keys for typographic_symbols
            if in_typographic_symbols && (line.starts_with(' ') || line.starts_with('\t')) {
                if let Some((key, val)) = Self::split_yaml_kv(trimmed) {
                    opts.typographic_symbols.insert(key, Self::unquote(&val));
                }
                continue;
            }

            // Handle indented sub-keys for link_defs
            if in_link_defs && (line.starts_with(' ') || line.starts_with('\t')) {
                if let Some((key, val)) = Self::split_yaml_kv(trimmed) {
                    // link_defs values are YAML arrays like [url] or [url, title]
                    let link_def = Self::parse_link_def_value(&val);
                    opts.link_defs.insert(key, link_def);
                    current_link_key = None;
                } else if current_link_key.is_some() {
                    // continuation line
                }
                continue;
            }

            // Parse top-level key: value
            let (key, val) = match Self::split_yaml_kv(trimmed) {
                Some(kv) => kv,
                None => continue,
            };

            match key.as_str() {
                "auto_ids" => opts.auto_ids = Self::parse_bool(&val),
                "auto_id_prefix" => opts.auto_id_prefix = val,
                "auto_id_stripping" => opts.auto_id_stripping = Self::parse_bool(&val),
                "transliterated_header_ids" => {
                    opts.transliterated_header_ids = Self::parse_bool(&val)
                }
                "header_offset" => opts.header_offset = val.parse().unwrap_or(0),
                "header_links" => opts.header_links = Self::parse_bool(&val),
                "entity_output" => opts.entity_output = Self::parse_entity_output(&val),
                "footnote_nr" => opts.footnote_nr = val.parse().unwrap_or(1),
                "footnote_prefix" => opts.footnote_prefix = val,
                "footnote_backlink" => opts.footnote_backlink = Self::unquote(&val),
                "footnote_backlink_inline" => {
                    opts.footnote_backlink_inline = Self::parse_bool(&val)
                }
                "footnote_link_text" => opts.footnote_link_text = Self::unquote(&val),
                "math_engine" => {
                    opts.math_engine = if val == "~" || val.is_empty() {
                        None
                    } else {
                        Some(val)
                    };
                }
                "parse_block_html" => opts.parse_block_html = Self::parse_bool(&val),
                "html_to_native" => opts.html_to_native = Self::parse_bool(&val),
                "syntax_highlighter" => {
                    opts.syntax_highlighter = if val == "~" || val.is_empty() {
                        None
                    } else {
                        Some(val)
                    };
                }
                "syntax_highlighter_opts" => {
                    in_syntax_highlighter_opts = true;
                }
                "enable_coderay" => opts.enable_coderay = Self::parse_bool(&val),
                "typographic_symbols" => {
                    in_typographic_symbols = true;
                }
                "link_defs" => {
                    in_link_defs = true;
                }
                "toc_levels" => opts.toc_levels = val,
                "remove_line_breaks_for_cjk" => {
                    opts.remove_line_breaks_for_cjk = Self::parse_bool(&val)
                }
                "smart_quotes" => {
                    // Could be a YAML array like [lsquo, rsquo, ldquo, rdquo]
                    if val.starts_with('[') && val.ends_with(']') {
                        let inner = &val[1..val.len() - 1];
                        opts.smart_quotes =
                            inner.split(',').map(|s| s.trim().to_string()).collect();
                    }
                }
                _ => {
                    // Unknown options are silently ignored
                }
            }
        }

        Ok(opts)
    }

    /// Split a YAML key: value line, stripping the leading `:` from Ruby-style symbol keys.
    fn split_yaml_kv(line: &str) -> Option<(String, String)> {
        // For Ruby-style symbol keys like `:auto_ids: true`, skip the leading `:`
        // to find the real separator colon.
        let line_trimmed = line.trim();
        let search_start = if line_trimmed.starts_with(':') { 1 } else { 0 };

        let colon_pos = line_trimmed[search_start..].find(':')? + search_start;
        let key = line_trimmed[..colon_pos].trim();
        // Strip leading `:` from Ruby symbol keys
        let key = key.strip_prefix(':').unwrap_or(key).to_string();

        if key.is_empty() {
            return None;
        }

        let rest = line_trimmed[colon_pos + 1..].trim();
        if rest.is_empty() {
            // Key with no value on this line (nested block follows)
            return Some((key, String::new()));
        }

        // Strip leading `:` from Ruby symbol values (like `:symbolic`)
        let val = rest.strip_prefix(':').unwrap_or(rest).to_string();
        Some((key, val))
    }

    fn parse_bool(s: &str) -> bool {
        matches!(s.to_lowercase().as_str(), "true" | "yes" | "1")
    }

    fn parse_entity_output(s: &str) -> EntityOutput {
        match s {
            "as_char" => EntityOutput::AsChar,
            "as_input" => EntityOutput::AsInput,
            "symbolic" => EntityOutput::Symbolic,
            "numeric" => EntityOutput::Numeric,
            _ => EntityOutput::default(),
        }
    }

    fn parse_syntax_highlighter_opt(line: &str, opts: &mut SyntaxHighlighterOpts) {
        // Handle sub-keys like "block:", "default_lang: ruby", etc.
        // Also handle nested "block: \n  css: class" patterns
        if let Some((key, val)) = Self::split_yaml_kv(line) {
            match key.as_str() {
                "css" => opts.block_css = Some(val),
                "default_lang" => opts.default_lang = Some(val),
                "wrap" => opts.wrap = Some(val),
                "line_numbers" => {
                    opts.line_numbers = if val == "null" || val == "~" {
                        None
                    } else {
                        Some(val)
                    };
                }
                "guess_lang" => opts.guess_lang = Some(Self::parse_bool(&val)),
                "block" | "span" => {
                    // These are sub-section headers, ignore the key itself
                }
                _ => {}
            }
        }
    }

    fn parse_link_def_value(val: &str) -> (String, Option<String>) {
        // Parse [url] or [url, title]
        let inner = val
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_string();
        let parts: Vec<&str> = inner.splitn(2, ',').collect();
        let url = parts[0].trim().to_string();
        let title = parts.get(1).map(|t| t.trim().to_string());
        (url, title)
    }

    fn unquote(s: &str) -> String {
        let s = s.trim();
        if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
            s[1..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }
}

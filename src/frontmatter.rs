use std::collections::HashMap;

use pulldown_cmark::{html, Event, Options, Parser};

/// Errors that can occur when parsing a document.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("failed to parse YAML front matter: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("failed to parse YAML front matter (lenient): {0}")]
    YamlLenient(#[from] crate::yaml::YamlParseError),
}

/// YAML front matter as a flexible key-value map.
pub type FrontMatter = HashMap<String, serde_yaml::Value>;

/// The excerpt separator used in Jekyll-style markdown files.
const EXCERPT_SEPARATOR: &str = "<!--more-->";

/// A parsed document consisting of YAML front matter and markdown body.
#[derive(Debug)]
pub struct Document {
    /// Parsed YAML front matter key-value pairs.
    pub front_matter: FrontMatter,
    /// Raw markdown body (everything after front matter).
    pub content: String,
    /// Content before `<!--more-->` separator, if present.
    pub excerpt: Option<String>,
}

/// Split raw text into optional YAML front matter string and markdown body.
///
/// Front matter is delimited by `---` on its own line at the very start of the file.
/// Returns `(yaml_str, body)`. If no front matter is detected, returns `(None, full_input)`.
fn split_front_matter(input: &str) -> (Option<&str>, &str) {
    // Front matter must start with "---" on the first line.
    let trimmed = input.trim_start_matches('\u{feff}'); // strip BOM if present
    if !trimmed.starts_with("---") {
        return (None, input);
    }

    // Find the closing "---" delimiter. It must appear on its own line
    // after the opening one.
    let after_opening = &trimmed[3..];
    // Skip past the newline after the opening ---
    let rest = if let Some(stripped) = after_opening.strip_prefix('\n') {
        stripped
    } else if let Some(stripped) = after_opening.strip_prefix("\r\n") {
        stripped
    } else {
        // Opening --- is not followed by a newline -- not valid front matter.
        return (None, input);
    };

    // Search for closing --- on its own line by scanning for line boundaries
    // directly in the byte string. This correctly handles both LF and CRLF
    // line endings without cumulative byte offset drift (which was the cause
    // of the Unicode slicing panic in issue #78).
    let mut line_start = 0;
    while line_start < rest.len() {
        // Find where this line ends (at the next \n, or end of string).
        let newline_pos = rest[line_start..].find('\n');
        let line_end = newline_pos.map(|p| line_start + p).unwrap_or(rest.len());

        // Extract the line content (without the trailing \n).
        let line = &rest[line_start..line_end];

        if line.trim() == "---" {
            // YAML content is everything before this line's start.
            let yaml_str = &rest[..line_start];
            // Body starts after the closing --- line (past the \n).
            let body = if line_end < rest.len() {
                &rest[line_end + 1..]
            } else {
                ""
            };
            return (Some(yaml_str), body);
        }

        // Advance to the next line. If no newline was found, we're done.
        match newline_pos {
            Some(_) => line_start = line_end + 1,
            None => break,
        }
    }

    // No closing delimiter found -- treat entire input as body with no front matter.
    (None, input)
}

/// Extract the excerpt from markdown content.
///
/// First tries to find `<!--more-->` separator. If not found, falls back to
/// the first paragraph (text before the first blank line), matching Jekyll's
/// default behavior where `page.excerpt` is auto-generated from content.
fn extract_excerpt(content: &str) -> Option<String> {
    // Try <!--more--> separator first
    if let Some(pos) = content.find(EXCERPT_SEPARATOR) {
        let excerpt = content[..pos].trim().to_string();
        return if excerpt.is_empty() {
            Some(String::new())
        } else {
            Some(excerpt)
        };
    }

    // Fall back to first paragraph (text before first blank line)
    let trimmed = content.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    // Find the first blank line (two consecutive newlines)
    if let Some(pos) = trimmed.find("\n\n") {
        let first_para = trimmed[..pos].trim().to_string();
        if first_para.is_empty() {
            None
        } else {
            Some(first_para)
        }
    } else {
        // No blank line -- entire content is one paragraph
        let para = trimmed.trim().to_string();
        if para.is_empty() {
            None
        } else {
            Some(para)
        }
    }
}

/// Parse a string containing optional YAML front matter and a markdown body.
///
/// Returns a `Document` with parsed front matter, raw markdown content,
/// and an optional excerpt (text before `<!--more-->`).
///
/// # Errors
///
/// Returns `ParseError::Yaml` if the front matter block contains invalid YAML.
pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let (yaml_str, body) = split_front_matter(input);

    let front_matter = match yaml_str {
        Some(yaml) => {
            let parsed: Option<FrontMatter> = crate::yaml::from_str_lenient(yaml)?;
            parsed.unwrap_or_default()
        }
        None => FrontMatter::new(),
    };

    let content = body.to_string();
    let excerpt = extract_excerpt(&content);

    Ok(Document {
        front_matter,
        content,
        excerpt,
    })
}

/// Convert a markdown string to HTML.
///
/// Transform pulldown-cmark events so that inline `Code` spans are emitted
/// with `class="language-plaintext highlighter-rouge"`, matching kramdown behavior.
///
/// Raw HTML `<code>` tags (passed through as `Html`/`InlineHtml` events) are
/// left untouched -- Jekyll/kramdown only adds the class to markdown-rendered
/// backtick code, not to `<code>` tags already present in the source HTML.
fn add_inline_code_class_to_events<'a>(
    parser: impl Iterator<Item = (Event<'a>, std::ops::Range<usize>)>,
    source: &'a str,
) -> Vec<Event<'a>> {
    add_inline_code_class_to_events_impl(parser, source, true, false)
}

/// Implementation of inline code class transformation with configurable behavior.
///
/// When `add_code_classes` is true (kramdown mode), inline `Code` spans get
/// `class="language-plaintext highlighter-rouge"`. When false (CommonMark mode),
/// inline code is left as bare `<code>` elements.
///
/// When `hardbreaks` is true (CommonMarkGhPages HARDBREAKS option), every
/// `SoftBreak` event is converted to an inline `<br>` element instead.
fn add_inline_code_class_to_events_impl<'a>(
    parser: impl Iterator<Item = (Event<'a>, std::ops::Range<usize>)>,
    source: &'a str,
    add_code_classes: bool,
    hardbreaks: bool,
) -> Vec<Event<'a>> {
    let mut events = Vec::new();
    for (event, range) in parser {
        match event {
            Event::Code(text) if add_code_classes => {
                // Emit raw HTML instead of the Code event so that push_html
                // produces <code class="...">text</code> rather than bare <code>.
                let escaped = html_escape_for_code(&text);
                let html = format!(
                    "<code class=\"language-plaintext highlighter-rouge\">{escaped}</code>"
                );
                events.push(Event::InlineHtml(html.into()));
            }
            Event::SoftBreak if hardbreaks => {
                // Issue 223: When HARDBREAKS is enabled, convert soft breaks
                // to <br> elements matching Jekyll's CommonMarkGhPages output.
                // We emit Event::HardBreak which pulldown-cmark renders as
                // "<br />\n". The LayoutEngine's final output step converts
                // <br /> back to <br> when enable_hardbreaks is true.
                events.push(Event::HardBreak);
            }
            Event::SoftBreak => {
                // Kramdown preserves trailing whitespace from source lines before
                // soft breaks. pulldown-cmark strips it (CommonMark behavior).
                // Restore any trailing whitespace that was stripped to match kramdown.
                //
                // The range for SoftBreak covers the newline in the source.
                // Check the source byte just before the range start for whitespace.
                if range.start > 0 {
                    let before = &source[..range.start];
                    let trailing_ws: String = before
                        .chars()
                        .rev()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    if !trailing_ws.is_empty() {
                        events.push(Event::Text(trailing_ws.into()));
                    }
                }
                events.push(Event::SoftBreak);
            }
            other => events.push(other),
        }
    }
    events
}

/// Escape HTML special characters for code content, matching pulldown-cmark behavior.
fn html_escape_for_code(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Normalize extra whitespace between block HTML tags to match kramdown output.
pub fn normalize_block_whitespace(html: &str) -> std::borrow::Cow<'_, str> {
    // Short-circuit: only active when `>\n\n` appears (closing tag followed by 2+ newlines).
    if !html.contains(">\n\n") {
        return std::borrow::Cow::Borrowed(html);
    }
    let mut result = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'>' {
            result.push('>');
            i += 1;
            let nl_start = i;
            while i < len && bytes[i] == b'\n' {
                i += 1;
            }
            let nl_count = i - nl_start;
            if nl_count >= 2 && i < len && bytes[i] == b'<' {
                result.push('\n');
            } else {
                for _ in 0..nl_count {
                    result.push('\n');
                }
            }
        } else if bytes[i] < 0x80 {
            result.push(bytes[i] as char);
            i += 1;
        } else {
            let ch_start = i;
            i += 1;
            while i < len && (bytes[i] & 0xC0) == 0x80 {
                i += 1;
            }
            result.push_str(&html[ch_start..i]);
        }
    }
    std::borrow::Cow::Owned(result)
}

/// Escape double quotes (`"`) to `&quot;` in HTML text nodes, matching kramdown behavior.
///
/// kramdown escapes `"` in text content to `&quot;`, which is important because
/// templates like `{{ page.content | strip_html | truncate: 240 }}` place the
/// result in HTML attribute values like `content="..."`. Unescaped `"` in text
/// would break the attribute parsing.
///
/// This function only escapes `"` that appear outside of HTML tags (i.e., in text
/// nodes). Quotes inside tag attributes (e.g., `href="..."`) are left unchanged.
/// Quotes inside `<script>` and `<style>` blocks are also left unchanged, since
/// those blocks contain code (e.g., JSON-LD) where `"` is required.
pub fn escape_quotes_in_text_nodes(html: &str) -> std::borrow::Cow<'_, str> {
    // Short-circuit: if no double-quote characters at all, nothing to escape.
    if !html.contains('"') {
        return std::borrow::Cow::Borrowed(html);
    }
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let bytes = html.as_bytes();

    for (byte_pos, ch) in html.char_indices() {
        match ch {
            '<' => {
                in_tag = true;
                let rest = &bytes[byte_pos..];
                // Check for <script, </script>, <style, </style>
                // All tag names are ASCII so byte comparison is safe.
                if rest.len() >= 7 && rest[..7].eq_ignore_ascii_case(b"<script") {
                    in_script = true;
                } else if rest.len() >= 6 && rest[..6].eq_ignore_ascii_case(b"<style") {
                    in_style = true;
                } else if rest.len() >= 9 && rest[..9].eq_ignore_ascii_case(b"</script>") {
                    in_script = false;
                } else if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case(b"</style>") {
                    in_style = false;
                }
                result.push(ch);
            }
            '>' => {
                in_tag = false;
                result.push(ch);
            }
            '"' if !in_tag && !in_script && !in_style => {
                result.push_str("&quot;");
            }
            _ => {
                result.push(ch);
            }
        }
    }
    std::borrow::Cow::Owned(result)
}

/// Supports headings, paragraphs, links, images, bold/italic, blockquotes,
/// code blocks, lists, horizontal rules, and raw HTML passthrough
/// (including Liquid-like tags such as `{% include ... %}`).
pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    // D5: Enable smart punctuation to match kramdown's smart quote behavior.
    // kramdown converts straight quotes to curly quotes by default.
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Issue 228: Process markdown="1" attribute on HTML elements before the
    // main markdown pass. This renders markdown inside <aside markdown="1">,
    // <div markdown="1">, etc. and strips the attribute.
    let markdown = crate::kramdown::process_markdown_attribute(markdown);

    // Protect pre-existing curly quotes from being re-processed by
    // fix_smart_quote_directions. kramdown only converts straight quotes to
    // curly; pre-existing curly quotes pass through unchanged.
    let markdown = protect_preexisting_curly_quotes(&markdown);

    // Escape parenthesis-style ordered list markers (e.g., "1) text") because
    // kramdown does not support `)` as a list delimiter -- only `.`.
    // pulldown-cmark (CommonMark) would treat these as ordered lists.
    let markdown = escape_paren_list_markers(&markdown);

    // Issue 227: Protect math content from backslash-escape processing.
    let (markdown, math_saved) = protect_math_content(&markdown);

    // Issue 204: Escape heading markers inside list context to match kramdown.
    // In kramdown, headings after list items without a blank line are text.
    let markdown = crate::kramdown::escape_headings_in_list_context(&markdown);

    // Issue 204: Collapse blank lines between list items to match kramdown's
    // tight list behavior. CommonMark makes entire list loose on any blank line.
    let markdown = crate::kramdown::collapse_blank_lines_between_list_items(&markdown);
    // Issue 200: Convert kramdown-style pipe tables to HTML.
    let markdown = crate::kramdown::convert_kramdown_pipe_tables(&markdown);

    // Issue 203: Split text that follows HTML block close tags onto new lines.
    // In kramdown, `</figure>Text with [links](url)` treats the text as a new
    // paragraph, but CommonMark treats it as part of the HTML block.
    let markdown = crate::kramdown::split_text_after_html_block_close(&markdown);

    // Issue 206: Normalize zero-width spaces before emphasis markers so
    // pulldown-cmark recognizes them as word boundaries for emphasis.
    let markdown = normalize_zwsp_for_emphasis(&markdown);

    // Issue 206: Fix emphasis patterns that CommonMark doesn't parse but
    // kramdown does (e.g., word*.*).
    let markdown = fix_kramdown_emphasis_patterns(&markdown);

    // Issue 275: Escape inner delimiters in mixed-delimiter emphasis patterns
    // (e.g., _*text*_ -> _\*text\*_) to match kramdown behavior.
    let markdown = crate::kramdown::escape_mixed_delimiter_emphasis(&markdown);

    // Issue 270: Convert runs of 4+ consecutive underscores to match kramdown
    // behavior. CommonMark leaves them as literal text but kramdown treats
    // them as emphasis (4 -> em, 6+ -> strong).
    let markdown = crate::kramdown::convert_kramdown_underscore_runs(&markdown);

    // Issue 198: Protect consecutive single quotes ('' and ''') from smart
    // punctuation to match kramdown behavior for MediaWiki-style markup.
    let markdown = protect_consecutive_single_quotes(&markdown);

    // Protect Liquid tags from smart punctuation by replacing quotes inside
    // {% %} and {{ }} patterns with placeholders.
    let protected = protect_liquid_quotes(&markdown);

    let parser = Parser::new_ext(&protected, options);
    let events = add_inline_code_class_to_events(parser.into_offset_iter(), &protected);
    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    // Restore protected quotes
    let html_output = restore_liquid_quotes(&html_output);
    let html_output = restore_consecutive_single_quotes(&html_output);

    // Issue 227: Restore math content
    let html_output = restore_math_content(&html_output, &math_saved);

    // Issue 207: Decode pulldown-cmark's percent-encoding of special chars in URLs
    // to match Jekyll/kramdown behavior.
    let html_output = decode_pulldown_url_encoding(&html_output);

    // Issue 211: Fix smart quote directions to match kramdown
    let html_output = crate::kramdown::fix_smart_quote_directions(&html_output);

    // Issue 247: Apply kramdown SQ_RULES to straight quotes from restored ''/'''' sequences.
    // This runs AFTER fix_smart_quote_directions because that function would re-process
    // the smart quotes we produce. By running after, the straight quotes from ''/'''' are
    // still straight (fix_smart_quote_directions only processes curly quotes).
    let html_output = crate::kramdown::apply_kramdown_smart_quotes_to_straight(&html_output);

    // Restore pre-existing curly quotes after direction fix
    let html_output = restore_preexisting_curly_quotes(&html_output);

    // Apply kramdown compatibility post-processing
    crate::kramdown::postprocess(&html_output)
}

/// Convert Markdown to HTML with configurable inline code class, smart punctuation,
/// and hard breaks behavior.
///
/// When `add_code_classes` is true (kramdown mode, the default), inline backtick
/// code gets `class="language-plaintext highlighter-rouge"`. When false (CommonMark
/// mode), inline code is rendered as bare `<code>` elements.
///
/// When `enable_smart_punctuation` is true (kramdown mode), straight quotes are
/// converted to curly quotes and `...` becomes an ellipsis character. When false
/// (CommonMarkGhPages mode), punctuation is left as-is.
///
/// When `enable_hardbreaks` is true (CommonMarkGhPages with HARDBREAKS option),
/// every soft line break (single newline within a paragraph) is converted to a
/// `<br>` element, matching Jekyll's HARDBREAKS behavior.
///
/// This is used when the site config specifies a non-kramdown markdown processor
/// (e.g., `markdown: CommonMarkGhPages`).
pub fn markdown_to_html_with_options(
    markdown: &str,
    add_code_classes: bool,
    enable_smart_punctuation: bool,
    enable_hardbreaks: bool,
    enable_autolink: bool,
) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    // Issue 220: Only enable smart punctuation for kramdown sites.
    // CommonMarkGhPages does not enable smart punctuation by default.
    if enable_smart_punctuation {
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
    }

    // Issue 228: Process markdown="1" attribute on HTML elements
    let markdown = crate::kramdown::process_markdown_attribute(markdown);

    // Protect pre-existing curly quotes from fix_smart_quote_directions
    let markdown = protect_preexisting_curly_quotes(&markdown);

    let markdown = escape_paren_list_markers(&markdown);
    // Issue 227: Protect math content from backslash-escape processing
    let (markdown, math_saved) = protect_math_content(&markdown);
    let markdown = crate::kramdown::escape_headings_in_list_context(&markdown);
    let markdown = crate::kramdown::collapse_blank_lines_between_list_items(&markdown);
    let markdown = crate::kramdown::convert_kramdown_pipe_tables(&markdown);
    let markdown = crate::kramdown::split_text_after_html_block_close(&markdown);
    let markdown = normalize_zwsp_for_emphasis(&markdown);
    let markdown = fix_kramdown_emphasis_patterns(&markdown);
    // Issue 275: Escape inner delimiters in mixed-delimiter emphasis patterns
    let markdown = crate::kramdown::escape_mixed_delimiter_emphasis(&markdown);
    // Issue 270: Convert runs of 4+ consecutive underscores to match kramdown
    let markdown = crate::kramdown::convert_kramdown_underscore_runs(&markdown);
    let markdown = protect_consecutive_single_quotes(&markdown);
    // Issue 294: Auto-link bare URLs for CommonMarkGhPages sites with autolink extension.
    // Wraps bare http/https URLs in angle brackets so pulldown-cmark treats them as autolinks.
    let markdown = if enable_autolink {
        autolink_bare_urls(&markdown)
    } else {
        markdown
    };

    let protected = protect_liquid_quotes(&markdown);

    let parser = Parser::new_ext(&protected, options);
    let events = add_inline_code_class_to_events_impl(
        parser.into_offset_iter(),
        &protected,
        add_code_classes,
        enable_hardbreaks,
    );
    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    let html_output = restore_liquid_quotes(&html_output);
    let html_output = restore_consecutive_single_quotes(&html_output);
    let html_output = restore_math_content(&html_output, &math_saved);
    let html_output = decode_pulldown_url_encoding(&html_output);
    // Issue 211: Fix smart quote directions to match kramdown
    let html_output = crate::kramdown::fix_smart_quote_directions(&html_output);
    // Issue 247: Apply kramdown SQ_RULES to straight quotes from restored ''/'''' sequences.
    // Only when smart punctuation is enabled (kramdown mode). Runs AFTER fix_smart_quote_directions.
    let html_output = if enable_smart_punctuation {
        crate::kramdown::apply_kramdown_smart_quotes_to_straight(&html_output)
    } else {
        html_output
    };
    // Restore pre-existing curly quotes after direction fix
    let html_output = restore_preexisting_curly_quotes(&html_output);
    crate::kramdown::postprocess(&html_output)
}

/// Convert XHTML-style `<br />` to HTML5-style `<br>` for CommonMarkGhPages
/// sites with HARDBREAKS enabled.
///
/// Jekyll's CommonMarkGhPages renderer outputs `<br>` (HTML5 style), not
/// `<br />` (XHTML style). This function is called at the very end of the
/// rendering pipeline to match Jekyll's output format.
pub fn normalize_br_to_html5(html: &str) -> String {
    html.replace("<br />", "<br>")
}

/// Convert Markdown to HTML with lighter postprocessing, for the `markdownify` filter.
///
/// Jekyll's `markdownify` filter runs kramdown which outputs `<p>text</p>\n`.
/// The full `markdown_to_html` applies `add_block_spacing` which adds an extra
/// newline after block tags, but that's only needed for page body content.
/// In a filter context the template supplies the trailing newline.
pub fn markdown_to_html_for_filter(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    // Protect pre-existing curly quotes from fix_smart_quote_directions
    let markdown = protect_preexisting_curly_quotes(markdown);

    let markdown = escape_paren_list_markers(&markdown);
    // Issue 227: Protect math content from backslash-escape processing
    let (markdown, math_saved) = protect_math_content(&markdown);
    let markdown = crate::kramdown::escape_headings_in_list_context(&markdown);
    let markdown = crate::kramdown::collapse_blank_lines_between_list_items(&markdown);
    // Issue 200: Convert kramdown-style pipe tables to HTML.
    let markdown = crate::kramdown::convert_kramdown_pipe_tables(&markdown);

    let markdown = crate::kramdown::split_text_after_html_block_close(&markdown);

    // Issue 198/206: Same ZWSP and emphasis handling as markdown_to_html
    let markdown = normalize_zwsp_for_emphasis(&markdown);
    let markdown = fix_kramdown_emphasis_patterns(&markdown);
    // Issue 275: Escape inner delimiters in mixed-delimiter emphasis patterns
    let markdown = crate::kramdown::escape_mixed_delimiter_emphasis(&markdown);
    let markdown = protect_consecutive_single_quotes(&markdown);

    let protected = protect_liquid_quotes(&markdown);

    let parser = Parser::new_ext(&protected, options);
    let events = add_inline_code_class_to_events(parser.into_offset_iter(), &protected);
    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    let html_output = restore_liquid_quotes(&html_output);
    let html_output = restore_consecutive_single_quotes(&html_output);
    let html_output = restore_math_content(&html_output, &math_saved);
    let html_output = decode_pulldown_url_encoding(&html_output);
    // Issue 211: Fix smart quote directions to match kramdown
    let html_output = crate::kramdown::fix_smart_quote_directions(&html_output);
    // Issue 247: Apply kramdown SQ_RULES to straight quotes from restored ''/'''' sequences
    let html_output = crate::kramdown::apply_kramdown_smart_quotes_to_straight(&html_output);
    // Restore pre-existing curly quotes after direction fix
    let html_output = restore_preexisting_curly_quotes(&html_output);

    crate::kramdown::postprocess_for_filter(&html_output)
}

/// Escape parenthesis-style ordered list markers to prevent pulldown-cmark
/// from treating them as ordered lists. Kramdown only uses `.` as a list
/// delimiter, not `)`, so `1) text` should be treated as a paragraph.
///
/// This converts `1) ` at the start of a line to `1\) ` so the backslash
/// escapes the parenthesis in CommonMark. Only applies outside of code blocks
/// and HTML blocks.
/// Issue 198: Normalize zero-width spaces (U+200B) before underscore/asterisk
/// emphasis markers so that pulldown-cmark recognizes them as word boundaries.
///
/// In CommonMark, `_emphasis_` requires the opening `_` to be preceded by
/// whitespace or punctuation (a "left-flanking delimiter run"). Zero-width
/// space (U+200B) is Unicode category Cf (format), which CommonMark does not
/// classify as whitespace. So `\u{200b}_word_` is treated as mid-word and the
/// emphasis is not applied.
///
/// Issue 206: Fix emphasis patterns that kramdown handles but CommonMark doesn't.
/// In CommonMark, `word*X*` is not a left-flanking delimiter. kramdown is more
/// permissive. This inserts ZWSP+space before such patterns to enable emphasis.
fn fix_kramdown_emphasis_patterns(markdown: &str) -> String {
    if !markdown.contains('*') {
        return markdown.to_string();
    }
    let mut result = String::with_capacity(markdown.len() + 32);
    let chars: Vec<char> = markdown.chars().collect();
    let len = chars.len();
    let mut i = 0;
    while i < len {
        if chars[i] == '*' && i > 0 && chars[i - 1].is_alphanumeric() {
            let mut j = i + 1;
            while j < len && j < i + 5 && chars[j] != '*' && !chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && j > i + 1 && chars[j] == '*' {
                result.push('\u{200b}');
                result.push(' ');
                for ch in &chars[i..=j] {
                    result.push(*ch);
                }
                i = j + 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

/// kramdown, however, does recognize ZWSP as a boundary. This function inserts
/// a regular space after ZWSP when followed by `_` or `*` to enable emphasis.
fn normalize_zwsp_for_emphasis(markdown: &str) -> String {
    if !markdown.contains('\u{200b}') {
        return markdown.to_string();
    }

    let mut result = String::with_capacity(markdown.len() + 32);
    let chars: Vec<char> = markdown.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\u{200b}' && i + 1 < len && (chars[i + 1] == '_' || chars[i + 1] == '*') {
            result.push('\u{200b}');
            result.push(' ');
            i += 1;
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Placeholder for consecutive single quotes used in MediaWiki-style markup.
const SINGLE_QUOTE_3_PLACEHOLDER: &str = "\x00SQ3\x00";
const SINGLE_QUOTE_2_PLACEHOLDER: &str = "\x00SQ2\x00";

/// Placeholder prefix for protected math content (Issue 227).
const MATH_PLACEHOLDER_PREFIX: &str = "\x00MATH";
const MATH_PLACEHOLDER_SUFFIX: &str = "MATH\x00";

/// Issue 198: Protect consecutive single quotes (`''` and `'''`) from smart
/// punctuation conversion.
///
/// kramdown does NOT convert `''text''` or `'''text'''` to curly quotes --
/// it keeps them as literal straight single quotes. pulldown-cmark's smart
/// punctuation converts them to curly quotes (\u2018/\u2019).
///
/// Replace `'''` and `''` with placeholders before markdown processing,
/// restore after. Single `'` is left alone for normal smart quote conversion.
fn protect_consecutive_single_quotes(input: &str) -> String {
    if !input.contains("''") {
        return input.to_string();
    }
    // Replace ''' first (3 quotes) then '' (2 quotes) to avoid partial matching
    let result = input.replace("'''", SINGLE_QUOTE_3_PLACEHOLDER);
    result.replace("''", SINGLE_QUOTE_2_PLACEHOLDER)
}

/// Restore consecutive single quote placeholders back to their original form.
fn restore_consecutive_single_quotes(input: &str) -> String {
    let result = input.replace(SINGLE_QUOTE_3_PLACEHOLDER, "'''");
    result.replace(SINGLE_QUOTE_2_PLACEHOLDER, "''")
}

/// Placeholders for pre-existing curly quotes in the markdown source.
/// kramdown only converts STRAIGHT quotes to curly -- pre-existing curly quotes
/// pass through unchanged. pulldown-cmark also passes them through, but our
/// fix_smart_quote_directions post-processing incorrectly re-processes them.
/// By protecting them before markdown processing and restoring after the
/// direction fix, we ensure they remain exactly as written in the source.
const CURLY_LSINGLE_PLACEHOLDER: &str = "\x00CLS\x00";
const CURLY_RSINGLE_PLACEHOLDER: &str = "\x00CRS\x00";
const CURLY_LDOUBLE_PLACEHOLDER: &str = "\x00CLD\x00";
const CURLY_RDOUBLE_PLACEHOLDER: &str = "\x00CRD\x00";

/// Protect pre-existing curly quotes in the markdown source from being
/// re-processed by fix_smart_quote_directions.
fn protect_preexisting_curly_quotes(input: &str) -> String {
    if !input.contains('\u{2018}')
        && !input.contains('\u{2019}')
        && !input.contains('\u{201C}')
        && !input.contains('\u{201D}')
    {
        return input.to_string();
    }
    input
        .replace('\u{2018}', CURLY_LSINGLE_PLACEHOLDER)
        .replace('\u{2019}', CURLY_RSINGLE_PLACEHOLDER)
        .replace('\u{201C}', CURLY_LDOUBLE_PLACEHOLDER)
        .replace('\u{201D}', CURLY_RDOUBLE_PLACEHOLDER)
}

/// Restore pre-existing curly quote placeholders back to their original Unicode chars.
fn restore_preexisting_curly_quotes(input: &str) -> String {
    input
        .replace(CURLY_LSINGLE_PLACEHOLDER, "\u{2018}")
        .replace(CURLY_RSINGLE_PLACEHOLDER, "\u{2019}")
        .replace(CURLY_LDOUBLE_PLACEHOLDER, "\u{201C}")
        .replace(CURLY_RDOUBLE_PLACEHOLDER, "\u{201D}")
}

/// Issue 227: Protect content inside $...$ and $$...$$ math delimiters.
///
/// pulldown-cmark treats \, as an escaped comma and strips the backslash.
/// kramdown passes \, through literally inside math blocks.
/// This replaces math block contents with placeholders before markdown processing,
/// then restores them after HTML generation.
fn protect_math_content(input: &str) -> (String, Vec<String>) {
    if !input.contains('$') {
        return (input.to_string(), Vec::new());
    }

    let mut result = String::with_capacity(input.len());
    let mut saved: Vec<String> = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' {
            // Check for $$ (display math) or $ (inline math)
            let is_display = i + 1 < len && bytes[i + 1] == b'$';
            let delimiter = if is_display { "$$" } else { "$" };
            let delim_len = delimiter.len();
            let start = i;
            let content_start = i + delim_len;

            // Find closing delimiter
            let mut j = content_start;
            let mut found_end = false;
            while j < len {
                if is_display {
                    if j + 1 < len && bytes[j] == b'$' && bytes[j + 1] == b'$' {
                        // Found closing $$
                        let content = &input[content_start..j];
                        let idx = saved.len();
                        saved.push(content.to_string());
                        result.push_str(delimiter);
                        result.push_str(MATH_PLACEHOLDER_PREFIX);
                        result.push_str(&idx.to_string());
                        result.push_str(MATH_PLACEHOLDER_SUFFIX);
                        result.push_str(delimiter);
                        i = j + 2;
                        found_end = true;
                        break;
                    }
                } else {
                    // Inline math: do NOT cross line boundaries.
                    // If we hit a newline without finding a closing $,
                    // the opening $ is unmatched -- treat it as literal.
                    if bytes[j] == b'\n' {
                        break;
                    }
                    if bytes[j] == b'$' {
                        // For inline math, make sure it's not $$
                        if j + 1 < len && bytes[j + 1] == b'$' {
                            j += 2;
                            continue;
                        }
                        let content = &input[content_start..j];
                        let idx = saved.len();
                        saved.push(content.to_string());
                        result.push_str(delimiter);
                        result.push_str(MATH_PLACEHOLDER_PREFIX);
                        result.push_str(&idx.to_string());
                        result.push_str(MATH_PLACEHOLDER_SUFFIX);
                        result.push_str(delimiter);
                        i = j + 1;
                        found_end = true;
                        break;
                    }
                }
                // Skip escaped characters inside math (but we keep tracking)
                j += 1;
            }

            if !found_end {
                // No closing delimiter found -- output literally
                result.push_str(&input[start..start + delim_len]);
                i = content_start;
            }
        } else {
            // Safe to push UTF-8 char
            let ch = input[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    (result, saved)
}

/// Issue 227: Restore protected math content from placeholders.
fn restore_math_content(html: &str, saved: &[String]) -> String {
    if saved.is_empty() {
        return html.to_string();
    }

    let mut result = html.to_string();
    for (idx, content) in saved.iter().enumerate() {
        let placeholder = format!(
            "{}{}{}",
            MATH_PLACEHOLDER_PREFIX, idx, MATH_PLACEHOLDER_SUFFIX
        );
        result = result.replace(&placeholder, content);
    }
    result
}

/// Issue 207/212: Decode percent-encoding that pulldown-cmark adds to URLs in href/src attributes.
///
/// pulldown-cmark percent-encodes some ASCII characters in URLs (like `]`) that
/// Jekyll/kramdown preserves as-is. This post-processes the HTML to decode those
/// characters back to their literal form in href and src attributes.
///
/// Characters decoded:
/// - `]` (0x5D) -- closing brackets in URLs (pulldown-cmark encodes these)
///
/// Characters kept encoded:
/// - Non-ASCII bytes (> 0x7F) -- pulldown-cmark does NOT encode these, so any
///   percent-encoded non-ASCII in the output was already encoded in the source
/// - Space (%20) -- must remain encoded in URLs
/// - Other ASCII control characters
fn decode_pulldown_url_encoding(html: &str) -> String {
    // Quick check: if there's no percent-encoding at all, return as-is
    if !html.contains('%') {
        return html.to_string();
    }

    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while !remaining.is_empty() {
        // Find next href=" or src="
        let attr_start = remaining
            .find("href=\"")
            .map(|p| (p, 6)) // "href=\"" is 6 chars
            .or_else(|| remaining.find("src=\"").map(|p| (p, 5))); // "src=\"" is 5 chars

        if let Some((pos, prefix_len)) = attr_start {
            // Copy everything before and including the attribute prefix
            result.push_str(&remaining[..pos + prefix_len]);
            let after_quote = &remaining[pos + prefix_len..];

            // Find closing quote
            if let Some(end_quote) = after_quote.find('"') {
                let url = &after_quote[..end_quote];
                result.push_str(&decode_url_for_jekyll_compat(url));
                result.push('"');
                remaining = &after_quote[end_quote + 1..];
            } else {
                result.push_str(after_quote);
                break;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Decode percent-encoded characters in a URL to match Jekyll's behavior.
///
/// Decodes:
/// - `]` (0x5D) back to literal `]` (pulldown-cmark encodes this)
///
/// Preserves encoding for:
/// - Non-ASCII bytes (> 0x7F) -- pulldown-cmark never encodes these, so any
///   percent-encoded non-ASCII in the output was already encoded in the source
/// - Space (%20)
/// - Other ASCII characters that should remain encoded
fn decode_url_for_jekyll_compat(url: &str) -> String {
    if !url.contains('%') {
        return url.to_string();
    }

    let bytes = url.as_bytes();
    let len = bytes.len();
    let mut decoded: Vec<u8> = Vec::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'%' && i + 2 < len {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                let byte_val = (h << 4) | l;
                // Only decode ] (0x5D) which pulldown-cmark encodes.
                // Do NOT decode non-ASCII bytes (> 127) -- pulldown-cmark
                // passes those through as raw UTF-8, so any %XX with byte > 127
                // was already percent-encoded in the markdown source.
                if byte_val == b']' {
                    decoded.push(byte_val);
                    i += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(decoded).unwrap_or_else(|_| url.to_string())
}

/// Convert a hex digit to its numeric value.
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Issue 294: Wrap bare http/https URLs in angle brackets so pulldown-cmark
/// treats them as autolinks. This implements GFM-style autolink behavior for
/// CommonMarkGhPages sites with the autolink extension.
///
/// Only wraps URLs that are NOT:
/// - Already inside angle brackets (`<url>`)
/// - Inside markdown links (`[text](url)` or `](url)`)
/// - Inside code spans or code blocks
/// - Already wrapped in an HTML `<a>` tag
fn autolink_bare_urls(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_fenced_code = false;
    let mut fence_char = ' ';
    let mut fence_len = 0;

    for line in input.split('\n') {
        if !result.is_empty() {
            result.push('\n');
        }

        let trimmed = line.trim_start();
        if !in_fenced_code {
            if let Some(fc) = autolink_detect_fence_start(trimmed) {
                in_fenced_code = true;
                fence_char = fc.0;
                fence_len = fc.1;
                result.push_str(line);
                continue;
            }
        } else if autolink_is_fence_end(trimmed, fence_char, fence_len) {
            in_fenced_code = false;
            result.push_str(line);
            continue;
        }

        if in_fenced_code {
            result.push_str(line);
            continue;
        }

        // Skip indented code blocks (4+ spaces or tab at start)
        if line.starts_with("    ") || line.starts_with('\t') {
            result.push_str(line);
            continue;
        }

        let processed = autolink_process_line(line);
        result.push_str(&processed);
    }

    result
}

/// Detect the start of a fenced code block. Returns (fence_char, fence_length).
fn autolink_detect_fence_start(trimmed: &str) -> Option<(char, usize)> {
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == first).count();
    if count >= 3 {
        Some((first, count))
    } else {
        None
    }
}

/// Check if a line ends a fenced code block.
fn autolink_is_fence_end(trimmed: &str, fence_char: char, fence_len: usize) -> bool {
    let count = trimmed.chars().take_while(|&c| c == fence_char).count();
    count >= fence_len && trimmed.trim().chars().all(|c| c == fence_char)
}

/// Process a single line for bare URL autolinking, respecting code spans,
/// angle-bracket autolinks, markdown links, and HTML `<a>` tags.
fn autolink_process_line(line: &str) -> String {
    // Build a list of byte-offset regions to skip
    let skip_regions = {
        let mut regions = Vec::new();
        autolink_find_code_spans(line, &mut regions);
        autolink_find_angle_brackets(line, &mut regions);
        autolink_find_markdown_links(line, &mut regions);
        autolink_find_html_a_tags(line, &mut regions);
        regions.sort_by_key(|r| r.0);
        regions
    };

    let mut result = String::with_capacity(line.len());
    let mut pos = 0;
    let bytes = line.as_bytes();
    let len = bytes.len();

    while pos < len {
        // Check if we're inside a skip region
        if let Some(&(_, end)) = skip_regions.iter().find(|&&(s, e)| pos >= s && pos < e) {
            // Copy the skip region as-is
            let end = end.min(len);
            result.push_str(&line[pos..end]);
            pos = end;
            continue;
        }

        // Look for http:// or https://
        if bytes[pos] == b'h'
            && (line[pos..].starts_with("https://") || line[pos..].starts_with("http://"))
        {
            // Check that the char before is not part of a URL context
            let prev_char = if pos > 0 {
                line[..pos].chars().last()
            } else {
                None
            };
            let skip = matches!(prev_char, Some('<') | Some('(') | Some('"') | Some('\''));

            if !skip {
                // Find the end of the URL
                let url_end = find_url_end(line, pos);
                if url_end > pos {
                    result.push('<');
                    result.push_str(&line[pos..url_end]);
                    result.push('>');
                    pos = url_end;
                    continue;
                }
            }
        }

        // Copy character as-is
        let ch = line[pos..].chars().next().unwrap();
        result.push(ch);
        pos += ch.len_utf8();
    }

    result
}

/// Find the end of a URL starting at `start`. Returns the byte offset after the URL.
/// Strips trailing punctuation that is typically not part of URLs.
fn find_url_end(line: &str, start: usize) -> usize {
    let mut end = start;
    let bytes = line.as_bytes();
    let len = bytes.len();

    while end < len {
        let b = bytes[end];
        // URL terminates at whitespace, angle brackets, or certain delimiters
        if b == b' ' || b == b'\t' || b == b'<' || b == b'>' || b == b'[' || b == b']' {
            break;
        }
        end += 1;
    }

    // Strip trailing punctuation that is typically not part of the URL
    while end > start {
        let last_byte = bytes[end - 1];
        if matches!(
            last_byte,
            b'.' | b',' | b';' | b':' | b'!' | b'?' | b')' | b'\'' | b'"'
        ) {
            // Special case: closing paren is part of URL if there's a matching open paren
            if last_byte == b')' {
                let url = &line[start..end];
                let open_count = url.bytes().filter(|&b| b == b'(').count();
                let close_count = url.bytes().filter(|&b| b == b')').count();
                if open_count >= close_count {
                    break;
                }
            }
            end -= 1;
        } else {
            break;
        }
    }

    end
}

/// Find byte ranges of backtick code spans in a line.
fn autolink_find_code_spans(line: &str, regions: &mut Vec<(usize, usize)>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'`' {
            let start = i;
            let backtick_count = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            i += backtick_count;
            // Find matching closing backticks
            let mut found = false;
            while i + backtick_count <= len {
                if bytes[i] == b'`' {
                    let close_count = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                    if close_count == backtick_count {
                        regions.push((start, i + close_count));
                        i += close_count;
                        found = true;
                        break;
                    }
                    i += close_count;
                } else {
                    i += 1;
                }
            }
            if !found {
                i = start + backtick_count;
            }
        } else {
            i += 1;
        }
    }
}

/// Find byte ranges of angle-bracket autolinks `<url>`.
fn autolink_find_angle_brackets(line: &str, regions: &mut Vec<(usize, usize)>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'<' {
            if let Some(close) = line[i + 1..].find('>') {
                let end = i + 1 + close + 1;
                let inner = &line[i + 1..end - 1];
                if inner.starts_with("http://") || inner.starts_with("https://") {
                    regions.push((i, end));
                }
                i = end;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

/// Find byte ranges of markdown link targets: `](url)`.
fn autolink_find_markdown_links(line: &str, regions: &mut Vec<(usize, usize)>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b']' && i + 1 < len && bytes[i + 1] == b'(' {
            let paren_start = i + 1;
            let mut depth = 0;
            let mut j = paren_start;
            while j < len {
                if bytes[j] == b'(' {
                    depth += 1;
                } else if bytes[j] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        regions.push((paren_start, j + 1));
                        i = j + 1;
                        break;
                    }
                }
                j += 1;
            }
            if j >= len {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
}

/// Find byte ranges of HTML `<a ...>...</a>` regions.
fn autolink_find_html_a_tags(line: &str, regions: &mut Vec<(usize, usize)>) {
    let lower = line.to_ascii_lowercase();
    let mut search_from = 0;

    while search_from < lower.len() {
        let start = lower[search_from..]
            .find("<a ")
            .or_else(|| lower[search_from..].find("<a>"));
        if let Some(rel_start) = start {
            let abs_start = search_from + rel_start;
            if let Some(rel_end) = lower[abs_start..].find("</a>") {
                let abs_end = abs_start + rel_end + 4;
                regions.push((abs_start, abs_end));
                search_from = abs_end;
            } else {
                search_from = abs_start + 1;
            }
        } else {
            break;
        }
    }
}

fn escape_paren_list_markers(markdown: &str) -> String {
    let mut result = String::with_capacity(markdown.len());
    let mut in_code_block = false;
    let mut in_html_block = false;

    for line in markdown.split('\n') {
        if !result.is_empty() {
            result.push('\n');
        }

        // Track fenced code blocks
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            result.push_str(line);
            continue;
        }

        if in_code_block {
            result.push_str(line);
            continue;
        }

        // Track HTML blocks (simple heuristic: lines starting with <)
        if trimmed.starts_with('<') && !trimmed.starts_with("</") {
            in_html_block = true;
        }
        if in_html_block {
            result.push_str(line);
            // End HTML block on blank line or closing tag
            if trimmed.is_empty() {
                in_html_block = false;
            }
            continue;
        }

        // Check for N) pattern at start of line (with optional leading whitespace)
        let leading_spaces = line.len() - trimmed.len();
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            // Check for more digits followed by ") "
            let digits_end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            let after_digits = &rest[digits_end..];
            if after_digits.starts_with(") ") || after_digits == ")" {
                // Escape the closing parenthesis
                result.push_str(&line[..leading_spaces]);
                let digit_count = trimmed.len() - rest.len() + digits_end;
                result.push_str(&trimmed[..digit_count]);
                result.push_str("\\)");
                result.push_str(&after_digits[1..]); // skip the original )
                continue;
            }
        }

        result.push_str(line);
    }

    result
}

/// Replace double quotes inside Liquid tags and kramdown IALs with a
/// placeholder to prevent smart punctuation from converting them to curly
/// quotes.
///
/// Protects both Liquid tags (`{{`, `{%`) and kramdown inline attribute
/// lists (`{:`) since pulldown-cmark's smart punctuation would otherwise
/// turn `"_blank"` into curly quotes, breaking IAL attribute parsing.
fn protect_liquid_quotes(input: &str) -> String {
    // Sentinel that won't appear in normal text and won't be modified by markdown
    const QUOTE_PLACEHOLDER: &str = "\x00QUOT\x00";

    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while !remaining.is_empty() {
        // Find next Liquid tag or kramdown IAL opening
        let tag_start = remaining
            .find("{%")
            .or_else(|| remaining.find("{{"))
            .or_else(|| remaining.find("{:"));

        if let Some(start) = tag_start {
            // Copy everything before the tag
            result.push_str(&remaining[..start]);

            let opener = &remaining[start..start + 2];
            let closer = if opener == "{%" {
                "%}"
            } else if opener == "{{" {
                "}}"
            } else {
                // kramdown IAL: {:...}
                "}"
            };

            if let Some(end) = remaining[start + 2..].find(closer) {
                let tag_end = start + 2 + end + closer.len();
                let tag_content = &remaining[start..tag_end];
                // Replace double quotes inside the tag with placeholder
                result.push_str(&tag_content.replace('"', QUOTE_PLACEHOLDER));
                remaining = &remaining[tag_end..];
            } else {
                // No closing tag found, copy rest as-is
                result.push_str(remaining);
                return result;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }

    result
}

/// Restore placeholders back to double quotes.
fn restore_liquid_quotes(input: &str) -> String {
    const QUOTE_PLACEHOLDER: &str = "\x00QUOT\x00";
    input.replace(QUOTE_PLACEHOLDER, "\"")
}

/// Dedent lines inside HTML blocks that have 4+ spaces of leading whitespace.
///
/// In CommonMark, 4+ spaces of indentation creates an indented code block.
/// When Liquid includes produce HTML output with indentation (e.g., from
/// `{% for %}` loops), the indented `<a>`, `<div>`, `<h3>` tags get treated
/// as code blocks by pulldown-cmark, causing them to be HTML-escaped inside
/// `<pre><code>` blocks.
///
/// Jekyll uses kramdown, which is more lenient about indentation inside HTML.
/// This function normalizes the indentation to prevent the code-block issue
/// while preserving actual indented code blocks (those not containing HTML tags).
///
/// The algorithm: reduce any line indented with 4+ spaces to 2 spaces if it
/// looks like it contains an HTML tag (starts with `<` after trimming) or is
/// a blank line within an HTML context.
pub fn dedent_html_lines(content: &str) -> String {
    let mut result = String::with_capacity(content.len());

    for line in content.split('\n') {
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();

        // Only modify lines with 4+ spaces that look like HTML
        if leading_spaces >= 4 && looks_like_html(trimmed) {
            // Reduce to at most 3 spaces (prevent code-block interpretation)
            let new_indent = leading_spaces.min(3);
            for _ in 0..new_indent {
                result.push(' ');
            }
            result.push_str(trimmed);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline that we added if original didn't end with one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Check if a trimmed line looks like it contains HTML content.
///
/// Returns true for lines that start with an HTML tag, end with an HTML tag,
/// or contain common HTML patterns. Returns false for plain text that should
/// be treated as potential indented code blocks.
fn looks_like_html(trimmed: &str) -> bool {
    if trimmed.is_empty() {
        return false;
    }

    // Lines starting with HTML tags
    if trimmed.starts_with('<') {
        return true;
    }

    // Lines starting with HTML closing tags
    if trimmed.starts_with("</") {
        return true;
    }

    // Lines that end with an HTML tag (e.g., content followed by </div>)
    if trimmed.ends_with('>') {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    // ========================================================================
    // Front matter splitting tests
    // ========================================================================

    #[test]
    fn test_parse_standard_front_matter() {
        let input = "---\ntitle: Hello\nlayout: post\n---\nBody content here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello")
        );
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
        assert_eq!(doc.content, "Body content here");
    }

    #[test]
    fn test_parse_no_front_matter() {
        let input = "Just some markdown\n\nWith paragraphs.";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, input);
    }

    #[test]
    fn test_parse_empty_front_matter() {
        let input = "---\n---\nBody after empty front matter";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, "Body after empty front matter");
    }

    #[test]
    fn test_hr_in_body_not_confused_with_front_matter() {
        let input = "---\ntitle: Test\n---\nSome text\n\n---\n\nMore text after HR";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test")
        );
        assert!(doc.content.contains("---"));
        assert!(doc.content.contains("More text after HR"));
    }

    #[test]
    fn test_front_matter_with_blank_line_after_opening() {
        let input = "---\n\ntitle: Test\n---\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test")
        );
        assert_eq!(doc.content, "Body");
    }

    // ========================================================================
    // YAML value types tests
    // ========================================================================

    #[test]
    fn test_yaml_simple_string() {
        let input = "---\ntitle: \"Test Title\"\n---\n";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Test Title")
        );
    }

    #[test]
    fn test_yaml_inline_list() {
        let input = "---\nauthors: [alice, bob]\n---\n";
        let doc = parse_document(input).unwrap();
        let authors = doc.front_matter.get("authors").unwrap();
        let seq = authors.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].as_str(), Some("alice"));
        assert_eq!(seq[1].as_str(), Some("bob"));
    }

    #[test]
    fn test_yaml_block_list() {
        let input = "---\ntags:\n- analytics\n- clustering\n---\n";
        let doc = parse_document(input).unwrap();
        let tags = doc.front_matter.get("tags").unwrap();
        let seq = tags.as_sequence().unwrap();
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].as_str(), Some("analytics"));
        assert_eq!(seq[1].as_str(), Some("clustering"));
    }

    #[test]
    fn test_yaml_nested_object() {
        let input = "---\nids:\n  anchor: ABC\n  youtube: XYZ\n---\n";
        let doc = parse_document(input).unwrap();
        let ids = doc.front_matter.get("ids").unwrap();
        let map = ids.as_mapping().unwrap();
        assert_eq!(
            map.get(Value::String("anchor".into()))
                .and_then(Value::as_str),
            Some("ABC")
        );
        assert_eq!(
            map.get(Value::String("youtube".into()))
                .and_then(Value::as_str),
            Some("XYZ")
        );
    }

    #[test]
    fn test_yaml_date_value() {
        let input = "---\nstart: 2020-12-14 00:00:00\n---\n";
        let doc = parse_document(input).unwrap();
        // serde_yaml parses bare dates/datetimes as strings
        let start = doc.front_matter.get("start").unwrap();
        // It should be preserved as some kind of value (string or tagged)
        assert!(start.as_str().is_some() || start.is_string());
    }

    #[test]
    fn test_yaml_null_empty_value() {
        let input = "---\ndescription:\n---\n";
        let doc = parse_document(input).unwrap();
        let desc = doc.front_matter.get("description").unwrap();
        assert!(desc.is_null());
    }

    // ========================================================================
    // Excerpt extraction tests
    // ========================================================================

    #[test]
    fn test_excerpt_with_separator() {
        let input = "---\ntitle: Test\n---\nFirst paragraph.\n\n<!--more-->\n\nRest of content.";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.excerpt, Some("First paragraph.".to_string()));
        assert!(doc.content.contains("Rest of content."));
    }

    #[test]
    fn test_excerpt_without_separator() {
        // Without <!--more-->, Jekyll auto-generates excerpt from first paragraph
        let input = "---\ntitle: Test\n---\nJust content, no separator.";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.excerpt,
            Some("Just content, no separator.".to_string()),
            "Should auto-generate excerpt from first paragraph"
        );
    }

    #[test]
    fn test_excerpt_first_paragraph_only() {
        // Auto-excerpt should only include first paragraph (before blank line)
        let input =
            "---\ntitle: Test\n---\nFirst paragraph here.\n\nSecond paragraph here.\n\nThird.";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.excerpt,
            Some("First paragraph here.".to_string()),
            "Auto-excerpt should be first paragraph only"
        );
    }

    #[test]
    fn test_excerpt_auto_with_unicode() {
        // Non-ASCII content should be preserved in auto-excerpt
        let input = "---\ntitle: Test\n---\n\u{1F382} Hubberversary! \u{4F60}\u{597D}\u{4E16}\u{754C}\n\nMore content.";
        let doc = parse_document(input).unwrap();
        let excerpt = doc.excerpt.unwrap();
        assert!(
            excerpt.contains("\u{1F382}"),
            "Unicode emoji should be in excerpt. Got: {}",
            excerpt
        );
        assert!(
            excerpt.contains("\u{4F60}\u{597D}"),
            "CJK chars should be in excerpt. Got: {}",
            excerpt
        );
    }

    #[test]
    fn test_excerpt_separator_at_beginning() {
        let input = "---\ntitle: Test\n---\n<!--more-->\nContent after.";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.excerpt, Some(String::new()));
    }

    // ========================================================================
    // Markdown to HTML conversion tests
    // ========================================================================

    #[test]
    fn test_md_heading() {
        let html = markdown_to_html("## Hello");
        assert!(html.contains("<h2"), "Should contain h2 tag. Got: {}", html);
        assert!(html.contains("Hello"));
        assert!(html.contains("</h2>"));
    }

    #[test]
    fn test_md_bold_italic() {
        let html = markdown_to_html("This is **bold** and *italic* text.");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_md_link() {
        let html = markdown_to_html("[text](https://example.com)");
        assert!(html.contains("<a href=\"https://example.com\">text</a>"));
    }

    #[test]
    fn test_md_code_block() {
        let html = markdown_to_html("```\ncode here\n```");
        assert!(
            html.contains("<pre"),
            "Should contain a pre tag. Got: {}",
            html
        );
        assert!(
            html.contains("<code>"),
            "Should contain a code tag. Got: {}",
            html
        );
        assert!(
            html.contains("code here"),
            "Should contain code content. Got: {}",
            html
        );
    }

    #[test]
    fn test_md_blockquote() {
        let html = markdown_to_html("> This is a quote");
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("This is a quote"));
    }

    #[test]
    fn test_md_raw_html_passthrough() {
        let html = markdown_to_html("<figure><img src=\"test.jpg\"></figure>");
        assert!(html.contains("<figure>"));
        // img tags from raw HTML are converted to XHTML-style self-closing
        // (issue 222: normalize_bare_void_elements converts ALL void elements)
        assert!(
            html.contains("<img src=\"test.jpg\" />"),
            "Should convert img tag to XHTML-style self-closing. Got: {}",
            html
        );
        assert!(html.contains("</figure>"));
    }

    #[test]
    fn test_md_liquid_tags_preserved() {
        let input = "Some text\n\n{% include youtube.html video_id=\"abc123\" %}\n\nMore text";
        let html = markdown_to_html(input);
        // The Liquid tag structure is preserved. Note: smart punctuation (D5)
        // converts straight quotes to curly quotes in text context, which is
        // the same behavior as kramdown. In the real pipeline, Liquid tags are
        // resolved before markdown conversion, so this only affects edge cases.
        assert!(
            html.contains("{% include youtube.html"),
            "Liquid tag should be preserved. Got: {}",
            html
        );
        assert!(
            html.contains("abc123"),
            "Liquid tag parameters should be preserved. Got: {}",
            html
        );
    }

    // ========================================================================
    // Integration tests with real Jekyll content patterns
    // ========================================================================

    #[test]
    fn test_real_post_pattern() {
        let input = r#"---
layout: post
title: 'Customer Segmentation with RFM+'
subtitle: Build a 5D RFM+ framework
description: Customer segmentation with limited data.
image: images/posts/2020-11-29-segmentation/cover.jpg
authors:
- nishantmohan
tags:
- analytics
- clustering
datepublished: '2020-11-29'
date: '2020-11-29'
---

## Background

There's a specific part of job-hunting that I look forward to.

<!--more-->

## Introduction

They asked me to perform customer segmentation.

<figure>
<img src="/images/posts/test.jpg" />
</figure>

{% include youtube.html video_id="pWqD7SGuihs" %}
"#;
        let doc = parse_document(input).unwrap();

        // Verify front matter fields
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Customer Segmentation with RFM+")
        );

        // Authors list
        let authors = doc.front_matter.get("authors").unwrap();
        let seq = authors.as_sequence().unwrap();
        assert_eq!(seq.len(), 1);
        assert_eq!(seq[0].as_str(), Some("nishantmohan"));

        // Tags list
        let tags = doc.front_matter.get("tags").unwrap();
        let tag_seq = tags.as_sequence().unwrap();
        assert_eq!(tag_seq.len(), 2);

        // Date string
        assert_eq!(
            doc.front_matter.get("date").and_then(Value::as_str),
            Some("2020-11-29")
        );

        // Excerpt
        assert!(doc.excerpt.is_some());
        let excerpt = doc.excerpt.unwrap();
        assert!(excerpt.contains("Background"));
        assert!(!excerpt.contains("Introduction"));

        // Content
        assert!(doc.content.contains("Introduction"));
        assert!(doc.content.contains("{% include youtube.html"));

        // HTML conversion
        let html = markdown_to_html(&doc.content);
        assert!(html.contains("<h2"), "Should contain h2 tag. Got: {}", html);
        assert!(html.contains("<figure>"));
        assert!(html.contains("{% include youtube.html"));
    }

    #[test]
    fn test_real_people_pattern() {
        let input = r#"---
short: 16rahuljain
title: "Rahul Jain"
picture: "images/authors/16rahuljain.jpg"
linkedin: 16rahuljain

---

Rahul has over 12 years of experience in data and engineering."#;
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("short").and_then(Value::as_str),
            Some("16rahuljain")
        );
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Rahul Jain")
        );
        assert_eq!(
            doc.front_matter.get("picture").and_then(Value::as_str),
            Some("images/authors/16rahuljain.jpg")
        );
        assert_eq!(
            doc.front_matter.get("linkedin").and_then(Value::as_str),
            Some("16rahuljain")
        );
        assert!(doc.content.contains("Rahul has over 12 years"));
    }

    #[test]
    fn test_real_book_pattern_deeply_nested() {
        let input = r#"---
title: "Machine Learning Bookcamp"
description: "Book of the Week"
start: 2020-12-14 00:00:00
end: 2020-12-18 23:59:59
authors: [alexeygrigorev]
links:
  - text: Book's page on Manning
    link: http://bit.ly/mlbookcamp
  - text: Book's GitHub repository
    link: https://github.com/alexeygrigorev/mlbookcamp-code
archive:
- name: Vladimir Finkelshtein
  text: "First question."
  replies:
  - name: Alexey Grigorev
    text: "Answer here."
---

Book description body.
"#;
        let doc = parse_document(input).unwrap();

        // Inline list
        let authors = doc.front_matter.get("authors").unwrap();
        assert_eq!(authors.as_sequence().unwrap().len(), 1);

        // Nested links
        let links = doc.front_matter.get("links").unwrap();
        let links_seq = links.as_sequence().unwrap();
        assert_eq!(links_seq.len(), 2);
        let first_link = links_seq[0].as_mapping().unwrap();
        assert_eq!(
            first_link
                .get(Value::String("text".into()))
                .and_then(Value::as_str),
            Some("Book's page on Manning")
        );

        // Deeply nested archive with replies
        let archive = doc.front_matter.get("archive").unwrap();
        let archive_seq = archive.as_sequence().unwrap();
        assert_eq!(archive_seq.len(), 1);
        let first_entry = archive_seq[0].as_mapping().unwrap();
        let replies = first_entry
            .get(Value::String("replies".into()))
            .unwrap()
            .as_sequence()
            .unwrap();
        assert_eq!(replies.len(), 1);
        assert_eq!(
            replies[0]
                .as_mapping()
                .unwrap()
                .get(Value::String("name".into()))
                .and_then(Value::as_str),
            Some("Alexey Grigorev")
        );
    }

    #[test]
    fn test_real_podcast_pattern_nested_ids_and_links() {
        let input = r#"---
title: 'A/B Testing'
short: A/B Testing
season: 7
episode: 6
guests:
- jakobgraff
image: images/podcast/ab-testing.jpg
ids:
  anchor: AB-Testing-e1eq73v
  youtube: 0Gqx1LtqRZU
links:
  anchor: https://anchor.fm/datatalksclub/episodes/AB-Testing-e1eq73v
  apple: https://podcasts.apple.com/podcast/id1541710331
  spotify: https://open.spotify.com/episode/3LhBOO1UANCGbOwkntZt4j
  youtube: https://www.youtube.com/watch?v=0Gqx1LtqRZU
---

Transcript content here.
"#;
        let doc = parse_document(input).unwrap();

        // Nested ids map
        let ids = doc.front_matter.get("ids").unwrap();
        let ids_map = ids.as_mapping().unwrap();
        assert_eq!(
            ids_map
                .get(Value::String("anchor".into()))
                .and_then(Value::as_str),
            Some("AB-Testing-e1eq73v")
        );
        assert_eq!(
            ids_map
                .get(Value::String("youtube".into()))
                .and_then(Value::as_str),
            Some("0Gqx1LtqRZU")
        );

        // Nested links map (as a mapping, not a sequence)
        let links = doc.front_matter.get("links").unwrap();
        let links_map = links.as_mapping().unwrap();
        assert_eq!(
            links_map
                .get(Value::String("spotify".into()))
                .and_then(Value::as_str),
            Some("https://open.spotify.com/episode/3LhBOO1UANCGbOwkntZt4j")
        );

        // Season/episode as integers
        let season = doc.front_matter.get("season").unwrap();
        assert_eq!(season.as_u64(), Some(7));
    }

    // ========================================================================
    // Issue 43: Duplicate keys in front matter
    // ========================================================================

    #[test]
    fn test_front_matter_duplicate_keys_last_wins() {
        let input = "---\ntitle: First Title\nlayout: post\ntitle: Second Title\n---\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Second Title")
        );
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
    }

    // ========================================================================
    // Issue 71: dedent_html_lines tests
    // ========================================================================

    #[test]
    fn test_dedent_html_lines_reduces_indented_html_tags() {
        let input = "    <a href=\"/test.html\">Link</a>\n    <div>Content</div>";
        let result = dedent_html_lines(input);
        // Should reduce 4-space indent to 3 spaces (below code-block threshold)
        assert!(
            !result.starts_with("    <a"),
            "Should reduce indentation below 4 spaces, got: {:?}",
            result
        );
        assert!(
            result.contains("<a href=\"/test.html\">Link</a>"),
            "HTML content should be preserved, got: {:?}",
            result
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_non_html_indentation() {
        // Plain text with 4+ spaces should NOT be dedented (it's a code block)
        let input = "    let x = 42;";
        let result = dedent_html_lines(input);
        assert_eq!(
            result, input,
            "Non-HTML indented lines should be preserved as-is"
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_less_than_4_spaces() {
        let input = "  <div>OK</div>";
        let result = dedent_html_lines(input);
        assert_eq!(result, input, "Lines with <4 spaces should be unchanged");
    }

    #[test]
    fn test_dedent_html_lines_handles_deep_indentation() {
        let input = "        <h3>Title</h3>";
        let result = dedent_html_lines(input);
        assert!(
            result.contains("<h3>Title</h3>"),
            "Content should be preserved"
        );
        let leading_spaces = result.len() - result.trim_start().len();
        assert!(
            leading_spaces <= 3,
            "Leading spaces should be at most 3, got {}",
            leading_spaces
        );
    }

    #[test]
    fn test_dedent_html_lines_mixed_content() {
        let input = "## Heading\n\n<div class=\"wrapper\">\n    <a href=\"/test\">Link</a>\n    <h3>Title</h3>\n</div>\n\n## Another heading";
        let result = dedent_html_lines(input);
        assert!(
            result.contains("## Heading"),
            "Markdown headings should be preserved"
        );
        assert!(
            result.contains("## Another heading"),
            "Markdown headings should be preserved"
        );
        assert!(
            result.contains("<a href=\"/test\">Link</a>"),
            "HTML links should be preserved"
        );
        // The indented <a> tag should no longer have 4+ spaces
        assert!(
            !result.contains("    <a href"),
            "Indented HTML should be dedented"
        );
    }

    #[test]
    fn test_dedent_html_lines_related_posts_pattern() {
        // Simulates what Liquid outputs after processing related-posts.html include
        let input = r#"<div class="related-posts-section">
  <h2 class="related-posts-title">Related Posts</h2>
  <div class="related-posts-grid">
    <a href="/blog/test.html" class="related-post-card">
      <div class="related-post-content">
        <h3 class="related-post-title">Test Course</h3>
      </div>
    </a>
  </div>
</div>"#;
        let result = dedent_html_lines(input);
        // After dedenting, the markdown processor should not escape the HTML
        let html = markdown_to_html(&result);
        assert!(
            html.contains("<h3 class=\"related-post-title\">Test Course</h3>"),
            "h3 tags should render as HTML, not be escaped. Got: {}",
            html
        );
        assert!(
            html.contains("<a href=\"/blog/test.html\""),
            "Links should render as HTML. Got: {}",
            html
        );
        assert!(
            !html.contains("&lt;a href"),
            "Links should NOT be HTML-escaped. Got: {}",
            html
        );
        assert!(
            !html.contains("<pre><code>"),
            "Should not produce code blocks. Got: {}",
            html
        );
    }

    #[test]
    fn test_dedent_html_lines_preserves_fenced_code_blocks() {
        // Fenced code blocks (```) should not be affected since they use
        // backtick fencing, not indentation
        let input = "```\n    <div>code example</div>\n```";
        let result = dedent_html_lines(input);
        // The <div> inside fenced code is still HTML-looking, but the fenced
        // code block markers ensure it's treated as code by the markdown parser
        let html = markdown_to_html(&result);
        assert!(
            html.contains("<code>"),
            "Fenced code block should still work"
        );
    }

    #[test]
    fn test_markdown_with_embedded_html_after_liquid() {
        // Simulates a markdown file that contains HTML from a Liquid include,
        // which is the pattern for blog posts with {% include related-posts.html %}
        let input = r#"## Introduction

Some markdown text here.

<div class="related-posts-section">
  <h2 class="related-posts-title">Related Posts</h2>
  <div class="related-posts-grid">
    <a href="/blog/course.html" class="related-post-card">
      <div class="related-post-content">
        <h3 class="related-post-title">Course Title</h3>
        <p class="related-post-excerpt">Description here</p>
      </div>
    </a>
  </div>
</div>
"#;
        let dedented = dedent_html_lines(input);
        let html = markdown_to_html(&dedented);

        // Markdown heading should be converted
        assert!(
            html.contains("Introduction</h2>") && html.contains("<h2"),
            "Markdown heading should be converted to HTML. Got: {}",
            html
        );

        // Embedded HTML should be preserved as-is
        assert!(
            html.contains("<h3 class=\"related-post-title\">Course Title</h3>"),
            "Include output HTML should not be escaped. Got: {}",
            html
        );
        assert!(
            !html.contains("&lt;h3"),
            "HTML tags should not be escaped. Got: {}",
            html
        );
    }

    #[test]
    fn test_markdown_headings_with_liquid_html() {
        // Simulates a standalone page like books.md with markdown headings
        // mixed with Liquid-generated HTML
        let input = r#"# Book of the Week

Each week we have a book author coming.

## How it works

* Register on DataTalks.Club
* Join the channel

## Upcoming books

<section class="upcoming-books">
  <div class="books">
    <div class="book-card">Book 1</div>
  </div>
</section>

## Archive

<ul>
  <li>Past book 1</li>
</ul>
"#;
        let dedented = dedent_html_lines(input);
        let html = markdown_to_html(&dedented);

        assert!(
            html.contains("Book of the Week</h1>") && html.contains("<h1"),
            "h1 missing. Got: {}",
            html
        );
        assert!(
            html.contains("How it works</h2>") && html.contains("<h2"),
            "h2 'How it works' missing. Got: {}",
            html
        );
        assert!(
            html.contains("Upcoming books</h2>"),
            "h2 'Upcoming books' missing. Got: {}",
            html
        );
        assert!(
            html.contains("Archive</h2>"),
            "h2 'Archive' missing. Got: {}",
            html
        );
        assert!(
            html.contains("<li>Register on DataTalks.Club</li>"),
            "list items missing"
        );
    }

    // ========================================================================
    // Issue 78: Unicode byte boundary panic with CRLF line endings
    // ========================================================================

    #[test]
    fn test_unicode_curly_quote_lf() {
        // U+2019 RIGHT SINGLE QUOTATION MARK (3 bytes in UTF-8)
        let input = "---\ntitle: 'Strategic Positioning\u{2019}'\n---\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Strategic Positioning\u{2019}")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_unicode_curly_quote_crlf() {
        // This is the exact reproduction case from issue #78.
        // CRLF line endings + U+2019 curly quote caused a byte boundary panic.
        let input = "---\r\ntitle: 'Strategic Positioning\u{2019}'\r\n---\r\nBody here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Strategic Positioning\u{2019}")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_unicode_emoji_crlf() {
        // 4-byte emoji with CRLF line endings
        let input = "---\r\ntitle: 'Hello \u{1F600} World'\r\nlayout: post\r\n---\r\nBody content";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello \u{1F600} World")
        );
        assert_eq!(doc.content, "Body content");
    }

    #[test]
    fn test_unicode_cjk_crlf() {
        // CJK characters (3 bytes each) with CRLF
        let input = "---\r\ntitle: '\u{4F60}\u{597D}\u{4E16}\u{754C}'\r\n---\r\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("\u{4F60}\u{597D}\u{4E16}\u{754C}")
        );
        assert_eq!(doc.content, "Body");
    }

    #[test]
    fn test_unicode_in_body_crlf() {
        let input = "---\r\ntitle: Test\r\n---\r\nBody with \u{2019}curly\u{2019} quotes";
        let doc = parse_document(input).unwrap();
        assert_eq!(doc.content, "Body with \u{2019}curly\u{2019} quotes");
    }

    #[test]
    fn test_crlf_ascii_only() {
        let input = "---\r\ntitle: Hello\r\nlayout: post\r\n---\r\nBody content here";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello")
        );
        assert_eq!(
            doc.front_matter.get("layout").and_then(Value::as_str),
            Some("post")
        );
        assert_eq!(doc.content, "Body content here");
    }

    #[test]
    fn test_crlf_long_frontmatter_with_unicode() {
        // 50+ lines to accumulate offset drift, with Unicode on the last line
        let mut input = String::from("---\r\n");
        for i in 0..55 {
            input.push_str(&format!("key{}: value{}\r\n", i, i));
        }
        input.push_str("special: 'quote\u{2019}mark'\r\n");
        input.push_str("---\r\n");
        input.push_str("Body after long frontmatter");

        let doc = parse_document(&input).unwrap();
        assert_eq!(
            doc.front_matter.get("special").and_then(Value::as_str),
            Some("quote\u{2019}mark")
        );
        assert_eq!(doc.content, "Body after long frontmatter");
    }

    #[test]
    fn test_mixed_line_endings() {
        // Mix of LF and CRLF within the same file
        let input = "---\ntitle: 'Mixed \u{2019} endings'\r\nlayout: post\n---\r\nBody here";
        let doc = parse_document(&input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Mixed \u{2019} endings")
        );
        assert_eq!(doc.content, "Body here");
    }

    #[test]
    fn test_empty_front_matter_crlf() {
        let input = "---\r\n---\r\nBody after empty front matter";
        let doc = parse_document(input).unwrap();
        assert!(doc.front_matter.is_empty());
        assert_eq!(doc.content, "Body after empty front matter");
    }

    #[test]
    fn test_bom_crlf_unicode() {
        // BOM + CRLF + multi-byte characters
        let input = "\u{feff}---\r\ntitle: 'Hello \u{2019}World\u{2019}'\r\n---\r\nBody";
        let doc = parse_document(input).unwrap();
        assert_eq!(
            doc.front_matter.get("title").and_then(Value::as_str),
            Some("Hello \u{2019}World\u{2019}")
        );
        assert_eq!(doc.content, "Body");
    }

    #[test]
    fn test_crlf_podcast_pattern_with_curly_quotes() {
        // Simulates the actual DTC podcast episode that triggered the panic.
        // The curly quote U+2019 appears in the title value.
        let input = "---\r\ntitle: \"Building a Sustainable Data Freelancing Career: Market Validation, Client Acquisition & Strategic Positioning\u{2019}\"\r\nseason: 7\r\nepisode: 6\r\nguests:\r\n- jakobgraff\r\nimage: images/podcast/ab-testing.jpg\r\nids:\r\n  anchor: AB-Testing-e1eq73v\r\n  youtube: 0Gqx1LtqRZU\r\n---\r\nTranscript content here.";
        let doc = parse_document(input).unwrap();
        assert!(
            doc.front_matter.get("title").is_some(),
            "title should be parsed"
        );
        assert_eq!(doc.content, "Transcript content here.");
    }

    #[test]
    fn test_split_front_matter_crlf_closing_delimiter() {
        // Verify that the closing --- with CRLF is detected correctly
        let input = "---\r\ntitle: Test\r\n---\r\n";
        let (yaml, body) = split_front_matter(input);
        assert!(yaml.is_some(), "YAML should be detected");
        assert!(body.is_empty() || body.trim().is_empty());
    }

    #[test]
    fn test_unicode_at_exact_offset_boundary() {
        // Construct input where the multi-byte character would be at the exact
        // position where the old code's cumulative drift would cause a panic.
        // With CRLF, each line undercounts by 1 byte. After N lines the drift is N bytes.
        // Place a 3-byte character so the old offset would land inside it.
        let mut input = String::from("---\r\n");
        // 10 lines of short content to create 10-byte drift
        for _ in 0..10 {
            input.push_str("k: v\r\n");
        }
        // Add a line with a multi-byte char near where the drift would cause slicing
        input.push_str("z: '\u{2019}\u{2019}\u{2019}'\r\n");
        input.push_str("---\r\n");
        input.push_str("Body");

        let doc = parse_document(&input).unwrap();
        assert_eq!(doc.content, "Body");
        assert_eq!(
            doc.front_matter.get("z").and_then(Value::as_str),
            Some("\u{2019}\u{2019}\u{2019}")
        );
    }

    // ========================================================================
    // escape_paren_list_markers tests
    // ========================================================================

    #[test]
    fn test_escape_paren_list_markers_basic() {
        let input = "1) First item\n2) Second item";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, "1\\) First item\n2\\) Second item");
    }

    #[test]
    fn test_escape_paren_list_markers_dot_style_unaffected() {
        let input = "1. First item\n2. Second item";
        let result = escape_paren_list_markers(input);
        assert_eq!(
            result, input,
            "Dot-style list markers should not be escaped"
        );
    }

    #[test]
    fn test_escape_paren_list_markers_inside_code_block() {
        let input = "```\n1) code line\n```\n1) outside code";
        let result = escape_paren_list_markers(input);
        assert!(
            result.contains("1) code line"),
            "Should not escape inside code blocks. Got: {}",
            result
        );
        assert!(
            result.contains("1\\) outside code"),
            "Should escape outside code blocks. Got: {}",
            result
        );
    }

    #[test]
    fn test_escape_paren_list_markers_mid_sentence() {
        let input = "This has 1) in the middle";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, input, "Should not escape when not at start of line");
    }

    #[test]
    fn test_escape_paren_list_markers_multi_digit() {
        let input = "10) Tenth item";
        let result = escape_paren_list_markers(input);
        assert_eq!(result, "10\\) Tenth item");
    }

    #[test]
    fn test_escape_paren_list_markers_renders_as_paragraph() {
        // Verify that after escaping, markdown_to_html produces a <p> tag, not <ol>
        let input = "1) First item";
        let html = markdown_to_html(input);
        assert!(
            !html.contains("<ol>"),
            "Escaped paren marker should not produce <ol>. Got: {}",
            html
        );
        assert!(
            html.contains("<p>"),
            "Escaped paren marker should produce <p>. Got: {}",
            html
        );
    }

    #[test]
    fn test_protect_liquid_quotes_covers_kramdown_ial() {
        // IAL quotes should be protected from smart punctuation
        let input = r#"[link](/url){:target="_blank"}"#;
        let protected = protect_liquid_quotes(input);
        // The quotes inside {:...} should be replaced with placeholders
        assert!(
            !protected.contains(r#"target="_blank""#),
            "IAL quotes should be replaced. Got: {}",
            protected
        );
        // Restore should bring them back
        let restored = restore_liquid_quotes(&protected);
        assert!(
            restored.contains(r#"target="_blank""#),
            "Restored quotes should match original. Got: {}",
            restored
        );
    }

    #[test]
    fn test_markdown_to_html_kramdown_ial_target_blank() {
        // Smart punctuation must not convert IAL quotes to curly quotes
        let input = r#"[Register](/slack.html){:target="_blank"}"#;
        let html = markdown_to_html(input);
        assert!(
            html.contains(r#"target="_blank""#),
            "IAL target attribute should have straight quotes. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{201c}') && !html.contains('\u{201d}'),
            "Should not contain curly quotes. Got: {}",
            html
        );
    }

    #[test]
    fn test_markdown_to_html_single_paragraph_trailing_newline() {
        // Jekyll/kramdown outputs <p>text</p>\n for a single paragraph.
        // The html_content should end with a single \n, not \n\n.
        // This matters because collection item content (e.g., person bios)
        // gets embedded in JSON-LD via strip_html | jsonify, and an extra
        // trailing newline causes description fields to have \n\n instead of \n.
        let input = "Valeriia Kuka is a Content Manager at DataTalks.Club.\n";
        let html = markdown_to_html(input);
        assert_eq!(
            html, "<p>Valeriia Kuka is a Content Manager at DataTalks.Club.</p>\n",
            "Single paragraph should end with exactly one trailing newline"
        );
    }

    #[test]
    fn test_markdown_to_html_no_trailing_newline_source() {
        // When source content has no trailing newline, pulldown-cmark still adds \n.
        // The add_block_spacing should NOT double it to \n\n.
        let input = "Some text without trailing newline";
        let html = markdown_to_html(input);
        assert!(
            html.ends_with("</p>\n"),
            "Output should end with </p>\\n, got: {:?}",
            &html[html.len().saturating_sub(30)..]
        );
        assert!(
            !html.ends_with("</p>\n\n"),
            "Output should NOT end with </p>\\n\\n, got: {:?}",
            &html[html.len().saturating_sub(30)..]
        );
    }

    #[test]
    fn test_markdown_to_html_multi_paragraph_trailing_newline() {
        // Multiple paragraphs should have \n\n between them but end with single \n.
        let input = "First paragraph.\n\nSecond paragraph.\n";
        let html = markdown_to_html(input);
        assert!(
            html.ends_with("</p>\n"),
            "Multi-paragraph content should end with </p>\\n, got: {:?}",
            &html[html.len().saturating_sub(30)..]
        );
        assert!(
            !html.ends_with("</p>\n\n"),
            "Multi-paragraph content should NOT end with </p>\\n\\n, got: {:?}",
            &html[html.len().saturating_sub(30)..]
        );
    }

    #[test]
    fn test_script_block_ampersand_not_escaped() {
        // When a <script type="application/ld+json"> block contains &, the markdown
        // parser should pass it through as a raw HTML block without escaping & to &amp;.
        // This matters for course structured data includes with names like
        // "Infrastructure & Prerequisites".
        let input = r#"Some text before.

<script type="application/ld+json">
{
  "name": "Infrastructure & Prerequisites"
}
</script>

Some text after.
"#;
        let html = markdown_to_html(input);
        assert!(
            html.contains("Infrastructure & Prerequisites"),
            "& in <script> block should not be escaped to &amp;, got:\n{}",
            html
        );
        assert!(
            !html.contains("Infrastructure &amp; Prerequisites"),
            "& should not become &amp; in script blocks"
        );
    }

    // ========================================================================
    // Issue 162: figcaption <p> preserved through full markdown_to_html pipeline
    // ========================================================================

    #[test]
    fn test_issue162_figcaption_p_preserved_through_pipeline() {
        // Real-world case from blog/how-to-setup-lightweight-local-version-for-airflow:
        // <figure> block with <figcaption><p>...</p></figcaption>.
        // The <p> is in the source markdown. Jekyll preserves it.
        let input =
            "<figure>\n<img src=\"/images/test.png\"  />\n<figcaption><p>Caption text here</p></figcaption>\n</figure>";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<figcaption><p>Caption text here</p></figcaption>"),
            "figcaption <p> should be preserved through full pipeline. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue162_figcaption_p_with_links_preserved_through_pipeline() {
        // Real-world case: figcaption with inline links inside <p>
        let input = "<figure>\n<img src=\"/images/test.png\"  />\n<figcaption><p>Forget about issues (logos from <a href=\"https://example.com\"><u>Example</u></a> and <a href=\"https://other.com\"><u>Other</u></a>)</p></figcaption>\n</figure>";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<figcaption><p>Forget about issues"),
            "figcaption <p> with links should be preserved. Got:\n{}",
            html
        );
        assert!(
            html.contains("</a>)</p></figcaption>"),
            "figcaption closing </p> should be preserved. Got:\n{}",
            html
        );
    }

    #[test]
    fn test_issue162_figcaption_p_preceded_by_markdown() {
        // <figure> block preceded by markdown text (as in the airflow blog post)
        let input = "\n\n<figure>\n<img src=\"/images/test.png\"  />\n<figcaption><p>Caption text here</p></figcaption>\n</figure>\n\nSome text after.\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<figcaption><p>Caption text here</p></figcaption>"),
            "figcaption <p> should be preserved when preceded by markdown. Got:\n{}",
            html
        );
    }

    // ========================================================================
    // Issue 176: Inline code class -- backtick vs raw HTML <code>
    // ========================================================================

    #[test]
    fn test_issue176_backtick_code_gets_class() {
        // Markdown backtick inline code should get language-plaintext class
        let html = markdown_to_html("Use `pip install` to install.\n");
        assert!(
            html.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Backtick inline code should get language-plaintext class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue176_raw_html_code_no_class() {
        // Raw HTML <code> in markdown source should NOT get the class
        let html =
            markdown_to_html("You start in a directory named <code>working</code>. Keep going.\n");
        assert!(
            html.contains("<code>working</code>"),
            "Raw HTML <code> should NOT get language-plaintext class. Got: {}",
            html
        );
        assert!(
            !html.contains("language-plaintext highlighter-rouge\">working</code>"),
            "Raw HTML <code> must NOT have language-plaintext class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue176_mixed_backtick_and_raw_html_code() {
        // Same document with both backtick code and raw HTML code
        let input = "Use `pip` to install.\n\n<p>See the <code>README</code> file.</p>\n";
        let html = markdown_to_html(input);
        // Backtick code gets class
        assert!(
            html.contains("<code class=\"language-plaintext highlighter-rouge\">pip</code>"),
            "Backtick code should have class. Got: {}",
            html
        );
        // Raw HTML code does NOT get class
        assert!(
            html.contains("<code>README</code>"),
            "Raw HTML code should NOT have class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue176_markdownify_backtick_code_gets_class() {
        // markdownify filter should also add classes to backtick code
        let html = markdown_to_html_for_filter("Use `code` here\n");
        assert!(
            html.contains("<code class=\"language-plaintext highlighter-rouge\">code</code>"),
            "markdownify backtick code should have class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue176_code_with_special_chars() {
        // Inline code with HTML special characters should be properly escaped
        let html = markdown_to_html("Use `a < b && c > d` in code.\n");
        assert!(
            html.contains("a &lt; b &amp;&amp; c &gt; d</code>"),
            "Special chars should be escaped in inline code. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 216: Inline code classes conditional on markdown processor
    // ========================================================================

    #[test]
    fn test_issue216_commonmark_no_inline_code_class() {
        // When markdown processor is CommonMark (not kramdown), backtick inline
        // code should NOT get language-plaintext highlighter-rouge class
        let html = markdown_to_html_with_options(
            "Use `pip install` to set up.\n",
            false,
            true,
            false,
            false,
        );
        assert!(
            html.contains("<code>pip install</code>"),
            "CommonMark mode should produce bare <code> tags. Got: {}",
            html
        );
        assert!(
            !html.contains("language-plaintext"),
            "CommonMark mode should NOT add language-plaintext class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue216_kramdown_keeps_inline_code_class() {
        // Default (kramdown) mode should still add the class
        let html = markdown_to_html_with_options(
            "Use `pip install` to set up.\n",
            true,
            true,
            false,
            false,
        );
        assert!(
            html.contains(
                "<code class=\"language-plaintext highlighter-rouge\">pip install</code>"
            ),
            "Kramdown mode should add language-plaintext class. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue216_commonmark_unicode_inline_code() {
        // Non-ASCII content in inline code under CommonMark mode
        let html = markdown_to_html_with_options(
            "Use `einrichten` to configure.\n",
            false,
            true,
            false,
            false,
        );
        assert!(
            html.contains("<code>einrichten</code>"),
            "CommonMark mode with Unicode content should produce bare <code>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue216_commonmark_fenced_code_keeps_language_class() {
        // Fenced code blocks with language specifier should still get language class
        // regardless of markdown processor setting (regression guard)
        let input = "```python\nprint('hello')\n```\n";
        let html = markdown_to_html_with_options(input, false, true, false, false);
        assert!(
            html.contains("language-python"),
            "Fenced code blocks should keep language class even in CommonMark mode. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 220: Conditional smart punctuation (kramdown vs CommonMarkGhPages)
    // ========================================================================

    #[test]
    fn test_issue220_smart_punctuation_off_preserves_straight_apostrophe() {
        // When smart punctuation is disabled (CommonMarkGhPages mode),
        // straight apostrophes should remain as-is (U+0027), not curly quotes.
        let html = markdown_to_html_with_options("it's great\n", false, false, false, false);
        assert!(
            html.contains("it's great"),
            "Smart punctuation OFF should preserve straight apostrophe. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{2019}'),
            "Smart punctuation OFF should NOT produce curly right single quote. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue220_smart_punctuation_off_preserves_straight_double_quotes() {
        // When smart punctuation is disabled, straight double quotes should remain.
        let html =
            markdown_to_html_with_options("She said \"hello\"\n", false, false, false, false);
        assert!(
            !html.contains('\u{201C}') && !html.contains('\u{201D}'),
            "Smart punctuation OFF should NOT produce curly double quotes. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue220_smart_punctuation_off_preserves_three_dots() {
        // When smart punctuation is disabled, three dots should remain as three
        // separate U+002E characters, not become the ellipsis character U+2026.
        let html = markdown_to_html_with_options("Wait for it...\n", false, false, false, false);
        assert!(
            html.contains("..."),
            "Smart punctuation OFF should preserve three dots. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{2026}'),
            "Smart punctuation OFF should NOT produce ellipsis character. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue220_smart_punctuation_off_unicode_content() {
        // Non-ASCII content with quotes should also be preserved when smart punctuation is off.
        let html = markdown_to_html_with_options(
            "c'est la vie, \"Gem\u{00FC}tlichkeit\"\n",
            false,
            false,
            false,
            false,
        );
        assert!(
            html.contains("c'est"),
            "Smart punctuation OFF should preserve apostrophe in Unicode content. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{2019}'),
            "Smart punctuation OFF should NOT produce curly quote in Unicode content. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{201C}') && !html.contains('\u{201D}'),
            "Smart punctuation OFF should NOT produce curly double quotes in Unicode content. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue220_smart_punctuation_on_converts_apostrophe() {
        // Regression guard: kramdown mode (smart punctuation ON) should still
        // convert straight apostrophes to curly quotes.
        let html = markdown_to_html_with_options("it's great\n", true, true, false, false);
        assert!(
            html.contains('\u{2019}'),
            "Smart punctuation ON should convert apostrophe to curly quote. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue220_smart_punctuation_on_converts_ellipsis() {
        // Regression guard: kramdown mode should still convert ... to ellipsis.
        let html = markdown_to_html_with_options("Wait for it...\n", true, true, false, false);
        assert!(
            html.contains('\u{2026}'),
            "Smart punctuation ON should convert three dots to ellipsis. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 202: Preserve trailing whitespace before soft breaks (kramdown compat)
    // ========================================================================

    #[test]
    fn test_issue202_soft_break_preserves_trailing_space() {
        // Kramdown preserves trailing whitespace before newlines in paragraphs.
        // Source: "with a \n$500" (trailing space before \n)
        // Kramdown HTML: "<p>with a \n$500</p>\n" (space preserved before \n)
        // pulldown-cmark strips trailing whitespace before soft breaks.
        // We must restore it for kramdown compatibility.
        let input = "with a \n$500";
        let html = markdown_to_html(input);
        assert!(
            html.contains("with a \n$500") || html.contains("with a\n$500"),
            "Soft break should preserve trailing space. Got: {:?}",
            html
        );
        // The critical test: after strip_html | strip_newlines, space before $ must remain
        let stripped = html
            .replace("<p>", "")
            .replace("</p>", "")
            .replace('\n', "");
        assert!(
            stripped.contains("with a $500"),
            "After removing tags and newlines, space before $ must be preserved. Got: {:?}",
            stripped
        );
    }

    #[test]
    fn test_issue202_soft_break_no_trailing_space() {
        // When source has NO trailing space before newline:
        // Source: "side of\nML" (no trailing space)
        // Kramdown HTML: "<p>side of\nML</p>\n" (no space before \n)
        // Both should produce "side ofML" after strip_html | strip_newlines
        let input = "side of\nML";
        let html = markdown_to_html(input);
        let stripped = html
            .replace("<p>", "")
            .replace("</p>", "")
            .replace('\n', "");
        // Note: when there's no trailing space in the source, kramdown also
        // produces no space. Both renderers should agree.
        assert!(
            stripped.contains("side ofML"),
            "No trailing space in source means no space after strip_newlines. Got: {:?}",
            stripped
        );
    }

    // ========================================================================
    // Issue 206: Inline formatting tests
    // ========================================================================

    #[test]
    fn test_emphasis_after_zero_width_space() {
        // Zero-width space before _word_ should still produce <em>
        let md = "connect with \u{200b}_everyone_";
        let html = markdown_to_html(md);
        assert!(
            html.contains("<em>everyone</em>"),
            "Emphasis after zero-width space should be applied. Got: {html}"
        );
    }

    #[test]
    fn test_emphasis_after_zero_width_space_unicode() {
        // Non-ASCII: ZWSP before emphasis with Cyrillic content
        let md = "\u{200b}_\u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}_";
        let html = markdown_to_html(md);
        assert!(
            html.contains("<em>\u{043F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}</em>"),
            "Emphasis with Cyrillic after ZWSP should work. Got: {html}"
        );
    }

    #[test]
    fn test_emphasis_dot_pattern() {
        // *.*  should produce <em>.</em>
        let md = "not straightforward*.*";
        let html = markdown_to_html(md);
        assert!(
            html.contains("<em>.</em>"),
            "Single-char emphasis with dot should be applied. Got: {html}"
        );
    }

    #[test]
    fn test_kramdown_link_with_target_blank_full() {
        let md =
            r#"[Wikipedia](https://en.wikipedia.org/wiki/Docker_(software)){:target="_blank"}"#;
        let html = markdown_to_html(md);
        assert!(
            html.contains(r#"target="_blank""#),
            "Kramdown IAL target should be applied. Got: {html}"
        );
        assert!(html.contains("href="), "Should produce a link. Got: {html}");
        assert!(
            !html.contains("{:target"),
            "IAL syntax should be consumed. Got: {html}"
        );
    }

    #[test]
    fn test_inline_link_rendered() {
        let md = "Visit our [homepage](/) for more info";
        let html = markdown_to_html(md);
        assert!(
            html.contains("<a href=\"/\">homepage</a>"),
            "Inline link should be rendered. Got: {html}"
        );
    }

    #[test]
    fn test_normalize_zwsp_for_emphasis_preserves_normal_text() {
        // Normal text without ZWSP should pass through unchanged
        let input = "Hello _world_ and *bold*";
        assert_eq!(normalize_zwsp_for_emphasis(input), input);
    }

    #[test]
    fn test_normalize_zwsp_for_emphasis_no_zwsp() {
        // No ZWSP means early return
        let input = "plain text";
        assert_eq!(normalize_zwsp_for_emphasis(input), input);
    }

    // ========================================================================
    // Issue 207: URL encoding tests
    // ========================================================================

    #[test]
    fn test_url_with_non_ascii_stays_percent_encoded() {
        // Issue 212: pulldown-cmark percent-encodes non-ASCII in markdown link URLs.
        // We no longer decode these back, because we cannot distinguish pulldown-cmark-
        // encoded bytes from bytes that were already percent-encoded in the source.
        // Preserving the encoding is correct for sources that pre-encode non-ASCII URLs.
        let md =
            "[link](/page/\u{043D}\u{0430}\u{0437}\u{0432}\u{0430}\u{043D}\u{0438}\u{0435}.html)";
        let html = markdown_to_html(md);
        assert!(
            html.contains("%D0%BD%D0%B0%D0%B7%D0%B2%D0%B0%D0%BD%D0%B8%D0%B5"),
            "Non-ASCII in URL should stay percent-encoded. Got: {html}"
        );
    }

    #[test]
    fn test_url_bracket_not_percent_encoded() {
        // Test that ] in URLs is decoded back to literal
        let html = decode_pulldown_url_encoding(r#"<a href="http://example.com/page%5D">link</a>"#);
        assert!(
            html.contains("page]"),
            "Bracket ] should be decoded from %5D. Got: {html}"
        );
    }

    #[test]
    fn test_url_cyrillic_preserved() {
        // Cyrillic percent-encoding should be preserved (pulldown-cmark never
        // encodes non-ASCII, so any %XX with byte > 127 was already in the source)
        let html = decode_pulldown_url_encoding(
            r#"<a href="/page/%D0%BD%D0%B0%D0%B7%D0%B2%D0%B0%D0%BD%D0%B8%D0%B5.html">link</a>"#,
        );
        assert!(
            html.contains("%D0%BD%D0%B0%D0%B7%D0%B2%D0%B0%D0%BD%D0%B8%D0%B5"),
            "Cyrillic percent-encoding should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_url_space_kept_encoded() {
        // Spaces should stay percent-encoded
        let html = decode_pulldown_url_encoding(r#"<a href="/page%20name">link</a>"#);
        assert!(
            html.contains("%20"),
            "Space should remain percent-encoded. Got: {html}"
        );
    }

    #[test]
    fn test_decode_preserves_non_url_content() {
        // Content outside href/src attributes should not be modified
        let html = r#"<p>some %5D text</p>"#;
        let result = decode_pulldown_url_encoding(html);
        assert_eq!(result, html, "Non-attribute content should be unchanged");
    }

    // ========================================================================
    // Issue 212: URL percent-encoding fix
    // ========================================================================

    #[test]
    fn test_212_raw_html_href_preserved() {
        // Raw HTML with pre-encoded non-ASCII URL should be preserved as-is
        let input = r#"Some text.

<a href="https://example.com/caf%c3%a9">cafe link</a>

More text.
"#;
        let html = markdown_to_html(input);
        assert!(
            html.contains("%c3%a9"),
            "Pre-encoded non-ASCII in raw HTML href should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_212_markdown_link_pre_encoded_url() {
        // Markdown link with pre-encoded non-ASCII URL should preserve encoding
        let html = decode_pulldown_url_encoding(
            r#"<a href="https://example.com/niar%C3%A9-data/">link</a>"#,
        );
        assert!(
            html.contains("%C3%A9"),
            "Pre-encoded non-ASCII in markdown link should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_212_bracket_still_decoded() {
        // pulldown-cmark encodes ] as %5D; we should still decode it
        let html = decode_pulldown_url_encoding(r#"<a href="http://example.com/page%5D">link</a>"#);
        assert!(
            html.contains("page]"),
            "Bracket ] should still be decoded from %5D. Got: {html}"
        );
    }

    #[test]
    fn test_212_cyrillic_percent_encoding_preserved() {
        // Cyrillic percent-encoded URLs should be preserved (not decoded)
        let html = decode_pulldown_url_encoding(
            r#"<a href="https://example.com/%D0%B0%D0%B1%D0%B2">link</a>"#,
        );
        assert!(
            html.contains("%D0%B0%D0%B1%D0%B2"),
            "Cyrillic percent-encoding should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_212_mixed_pre_encoded_preserved() {
        // Pre-encoded combining accent should be preserved
        let html =
            decode_pulldown_url_encoding(r#"<a href="https://example.com/a-cafe%CC%81">link</a>"#);
        assert!(
            html.contains("%CC%81"),
            "Pre-encoded combining accent should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_212_arrow_encoding_preserved() {
        // %E2%86%92 (arrow ->) should be preserved, not decoded
        let html = decode_pulldown_url_encoding(
            r#"<a href="https://example.com/path%E2%86%92next">link</a>"#,
        );
        assert!(
            html.contains("%E2%86%92"),
            "Arrow percent-encoding should be preserved. Got: {html}"
        );
    }

    // ========================================================================
    // Issue 223: HARDBREAKS support (soft break -> <br>)
    // ========================================================================

    #[test]
    fn test_issue223_soft_break_becomes_hard_break_with_hardbreaks() {
        // When hardbreaks is enabled, a single newline within a paragraph
        // should produce a hard break element in the HTML output.
        // Note: markdown_to_html_with_options outputs <br /> (XHTML style)
        // because pulldown-cmark's HardBreak renders as <br />. The final
        // conversion to <br> (HTML5) happens in LayoutEngine's render methods
        // via normalize_br_to_html5().
        let html = markdown_to_html_with_options("line one\nline two\n", false, false, true, false);
        assert!(
            html.contains("<br"),
            "Hardbreaks enabled should produce a break element. Got: {}",
            html
        );
        assert!(
            html.contains("line one<br"),
            "Expected break right after 'line one'. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_soft_break_stays_soft_without_hardbreaks() {
        // When hardbreaks is disabled, soft breaks should NOT produce <br>.
        // This is a regression guard for existing behavior.
        let html =
            markdown_to_html_with_options("line one\nline two\n", false, false, false, false);
        assert!(
            !html.contains("<br"),
            "Hardbreaks disabled should NOT produce <br>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_multiple_newlines_produce_multiple_breaks() {
        // Multiple soft breaks within a paragraph should each produce a break.
        let html = markdown_to_html_with_options("a\nb\nc\n", false, false, true, false);
        let br_count = html.matches("<br").count();
        assert_eq!(
            br_count, 2,
            "Expected 2 break elements for 'a\\nb\\nc'. Got {} in: {}",
            br_count, html
        );
    }

    #[test]
    fn test_issue223_hardbreaks_with_unicode() {
        // Non-ASCII content with hardbreaks should work correctly.
        let html = markdown_to_html_with_options(
            "Gem\u{00fc}tlichkeit\nSch\u{00f6}n\n",
            false,
            false,
            true,
            false,
        );
        assert!(
            html.contains("<br"),
            "Hardbreaks with Unicode should produce a break. Got: {}",
            html
        );
        assert!(
            html.contains("Gem\u{00fc}tlichkeit"),
            "German word should be preserved. Got: {}",
            html
        );
        assert!(
            html.contains("Sch\u{00f6}n"),
            "Second German word should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_hardbreaks_inside_blockquote() {
        // Soft breaks inside blockquotes should also become hard breaks when enabled.
        let html =
            markdown_to_html_with_options("> line one\n> line two\n", false, false, true, false);
        assert!(
            html.contains("<br"),
            "Hardbreaks inside blockquote should produce a break. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_hardbreaks_inside_list_item() {
        // Soft breaks inside list items should become hard breaks when enabled.
        let html =
            markdown_to_html_with_options("- item\n  continued\n", false, false, true, false);
        assert!(
            html.contains("<br"),
            "Hardbreaks inside list item should produce a break. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_explicit_hard_break_still_works() {
        // Two trailing spaces before newline should produce <br> even without
        // hardbreaks enabled. This is standard CommonMark behavior.
        let html =
            markdown_to_html_with_options("line one  \nline two\n", false, false, false, false);
        assert!(
            html.contains("<br"),
            "Explicit hard break (2 spaces) should produce <br>. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue223_normalize_br_to_html5() {
        // The normalize_br_to_html5 function converts <br /> to <br>
        assert_eq!(
            normalize_br_to_html5("<p>line one<br />\nline two</p>\n"),
            "<p>line one<br>\nline two</p>\n"
        );
    }

    #[test]
    fn test_issue223_normalize_br_preserves_when_no_br() {
        // No change when there's no <br />
        let input = "<p>hello world</p>\n";
        assert_eq!(normalize_br_to_html5(input), input);
    }

    #[test]
    fn test_issue223_normalize_br_multiple() {
        // Multiple <br /> should all be converted
        assert_eq!(
            normalize_br_to_html5("<p>a<br />\nb<br />\nc</p>"),
            "<p>a<br>\nb<br>\nc</p>"
        );
    }

    // ========================================================================
    // Issue 227: Pattern 3 -- Math backslash protection
    // ========================================================================

    #[test]
    fn test_issue227_math_backslash_comma_inline() {
        // \, inside $...$ should be preserved literally
        let html = markdown_to_html("Text $a \\, b$ more\n");
        assert!(
            html.contains("\\,"),
            "\\, inside inline math should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue227_math_backslash_comma_display() {
        // \, inside $$...$$ should be preserved
        let html = markdown_to_html("$$f(x) \\, g(x)$$\n");
        assert!(
            html.contains("\\,"),
            "\\, inside display math should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue227_backslash_comma_outside_math_stripped() {
        // \, outside math should still be processed by pulldown-cmark (backslash stripped)
        let html = markdown_to_html("Regular \\, text\n");
        assert!(
            !html.contains("\\,"),
            "\\, outside math should have backslash stripped. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue227_multiple_math_blocks_on_one_line() {
        // Multiple $...$ on one line should all preserve \,
        let html = markdown_to_html("Inline $a \\, b$ and $c \\, d$ text\n");
        // Count occurrences of \,
        let count = html.matches("\\,").count();
        assert!(
            count >= 2,
            "Both math blocks should preserve \\,. Got {} occurrences in: {}",
            count,
            html
        );
    }

    #[test]
    fn test_issue227_math_backslash_sequences_preserved() {
        // Various LaTeX backslash sequences inside math should be preserved
        let html = markdown_to_html("$\\mathbf{v} \\in \\{1 \\, .. \\, C\\}$\n");
        assert!(
            html.contains("\\,"),
            "\\, inside math should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue227_math_protection_survives_unmatched_dollar() {
        let html = markdown_to_html("text with lone $ sign\n\nmath $a \\, b$ here\n");
        assert!(
            html.contains("\\,"),
            "\\, should be preserved despite earlier unmatched $. Got: {}",
            html
        );
    }

    // ========================================================================
    // Issue 211: Fix smart quote direction mismatches vs kramdown
    // ========================================================================

    #[test]
    fn test_issue211_english_basic_double_quotes() {
        // Basic English: "hello" should produce LEFT_DOUBLE...RIGHT_DOUBLE
        let html = markdown_to_html("He said \"hello\" quietly\n");
        assert!(
            html.contains("\u{201C}hello\u{201D}"),
            "English double quotes should be LEFT...RIGHT. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_inch_mark_after_digit() {
        // kramdown: 13" produces U+201D (right double quote / closing)
        // pulldown-cmark incorrectly produces U+201C (left/opening)
        let html = markdown_to_html("on 13\" laptop screens\n");
        assert!(
            html.contains("13\u{201D}"),
            "Quote after digit should be RIGHT_DOUBLE (inch mark). Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_cjk_double_quotes() {
        // CJK: "なに" -- kramdown produces RIGHT...RIGHT (both U+201D)
        // because CJK chars match SQ_CLOSE (rule 7), making both quotes closing.
        // pulldown-cmark produces LEFT...LEFT; our fix should make both RIGHT.
        let html = markdown_to_html("オープンソースとは\"なに\"\n");
        assert!(
            html.contains("\u{201D}\u{306A}\u{306B}\u{201D}"),
            "CJK double quotes should both be RIGHT (kramdown compat). Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_korean_single_quotes() {
        // Korean: 'outdated' should produce LEFT_SINGLE...RIGHT_SINGLE
        // pulldown-cmark produces RIGHT_SINGLE...RIGHT_SINGLE (wrong opening direction)
        let html = markdown_to_html("표시해 'outdated'로\n");
        assert!(
            html.contains("\u{2018}outdated\u{2019}"),
            "Korean single quotes should be LEFT...RIGHT. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_cyrillic_double_quotes() {
        // Cyrillic: "ерунда" should produce LEFT_DOUBLE...RIGHT_DOUBLE
        let html = markdown_to_html("мол \"ерунда\"\n");
        assert!(
            html.contains("\u{201C}\u{0435}\u{0440}\u{0443}\u{043D}\u{0434}\u{0430}\u{201D}"),
            "Cyrillic double quotes should be LEFT...RIGHT. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_apostrophe_not_regressed() {
        // Apostrophes must still be RIGHT_SINGLE (U+2019)
        let html = markdown_to_html("it's a test\n");
        assert!(
            html.contains("it\u{2019}s"),
            "Apostrophe should still be RIGHT_SINGLE. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_cats_apostrophe_not_regressed() {
        let html = markdown_to_html("the cat's whiskers\n");
        assert!(
            html.contains("cat\u{2019}s"),
            "Possessive apostrophe should still be RIGHT_SINGLE. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_contraction_with_quotes_in_same_paragraph() {
        // don't + "hello" in same paragraph should both work
        let html = markdown_to_html("don't stop, he said \"hello\"\n");
        assert!(
            html.contains("don\u{2019}t"),
            "Contraction apostrophe should be RIGHT_SINGLE. Got: {}",
            html
        );
        assert!(
            html.contains("\u{201C}hello\u{201D}"),
            "Double quotes should be LEFT...RIGHT. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_quote_after_dash_kramdown_compat() {
        // kramdown treats " after - as LEFT (opening), because - is excluded from SQ_CLOSE.
        // "3x TACOS 230,-" -> kramdown produces LEFT...LEFT (both U+201C).
        // This matches the actual taco-stand page from kids-horror-stories-ru.
        let html = markdown_to_html("плакатом: \"3x TACOS 230,-\". Симпатичный\n");
        // kramdown: both quotes are U+201C (LEFT DOUBLE)
        assert!(
            html.contains("\u{201C}3x TACOS 230,-\u{201C}"),
            "Quote after dash should be LEFT (kramdown compat). Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_quote_after_dash_simple() {
        // Simpler test: "hello,-" -> kramdown produces LEFT...LEFT
        let html = markdown_to_html("test \"hello,-\". text\n");
        assert!(
            html.contains("\u{201C}hello,-\u{201C}"),
            "Quote after dash should be LEFT (kramdown compat). Got: {}",
            html
        );
    }

    #[test]
    fn test_issue211_german_quotes_preserved() {
        // German-style quotes use U+201E as opener and U+201C as closer.
        // The fix_smart_quote_directions function must NOT convert U+201C to U+201D
        // when it follows a U+201E opener.
        // Source: kids-horror-stories-ru/254-dark-container.md
        let html =
            markdown_to_html("\u{201E}\u{041C}\u{0443}\u{0441}\u{043E}\u{0440}\u{043D}\u{044B}\u{0439}, \u{2014} \u{043F}\u{043E}\u{0434}\u{0443}\u{043C}\u{0430}\u{043B} \u{043E}\u{043D}, \u{2014} \u{043D}\u{0438}\u{0447}\u{0435}\u{0433}\u{043E} \u{043D}\u{0435}\u{043E}\u{0431}\u{044B}\u{0447}\u{043D}\u{043E}\u{0433}\u{043E}.\u{201C}\n");
        // The U+201E...U+201C pair must remain intact
        assert!(
            html.contains("\u{201E}") && html.contains("\u{201C}"),
            "German quotes: U+201E opener and U+201C closer must be preserved. Got: {}",
            html
        );
        // Specifically, U+201C must NOT be converted to U+201D
        assert!(
            !html.contains("\u{201D}"),
            "German closing quote U+201C must not be converted to U+201D. Got: {}",
            html
        );
    }

    // --- Regression: pre-existing curly quotes must pass through unchanged ---

    #[test]
    fn test_preexisting_left_single_quote_preserved() {
        // Source: muan-blog notes/2023-05-27-uu.md has U+2018 (LEFT SINGLE) already
        // in the source: "Kendrick\u{2018}a role?" -- kramdown passes it through as-is.
        // fix_smart_quote_directions must NOT convert it to U+2019.
        let input = "Anna Kendrick\u{2018}a role?\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("Kendrick\u{2018}a"),
            "Pre-existing U+2018 must be preserved, not converted to U+2019. Got: {}",
            html
        );
    }

    #[test]
    fn test_preexisting_left_double_quote_as_closer_preserved() {
        // Source: muan-blog stories/8e9c0186 has U+201C used for both opening
        // AND closing: \u{201C}Show the world...\u{201C}
        // kramdown passes these through unchanged.
        let input = "\u{201C}Show the world how much you love Javascript.\u{201C} - @phae\n";
        let html = markdown_to_html(input);
        // Both should remain as U+201C
        let count_201c = html.matches('\u{201C}').count();
        assert!(
            count_201c >= 2,
            "Both pre-existing U+201C quotes must be preserved. Got: {}",
            html
        );
        assert!(
            !html.contains('\u{201D}'),
            "Pre-existing U+201C must not be converted to U+201D. Got: {}",
            html
        );
    }

    #[test]
    fn test_preexisting_curly_quotes_coexist_with_straight() {
        // Mix of pre-existing curly quotes and straight quotes that need conversion.
        // The straight quotes should still be converted by smart punctuation,
        // but the pre-existing curly quotes must pass through unchanged.
        let input = "He said \"hello\" and Kendrick\u{2018}a role\n";
        let html = markdown_to_html(input);
        // Straight "hello" should be converted to curly quotes
        assert!(
            html.contains('\u{201C}') && html.contains('\u{201D}'),
            "Straight quotes should be converted. Got: {}",
            html
        );
        // Pre-existing U+2018 between 'k' and 'a' must stay as U+2018
        assert!(
            html.contains("Kendrick\u{2018}a"),
            "Pre-existing U+2018 must not change. Got: {}",
            html
        );
    }

    #[test]
    fn test_preexisting_right_single_apostrophe_preserved() {
        // Source: muan-blog notes/2024-04-27-mm.md has U+2019 (RIGHT) as
        // I\u{2019}m -- this is already correct and should pass through.
        let input = "I\u{2019}m working through films\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("I\u{2019}m"),
            "Pre-existing U+2019 apostrophe must be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_preexisting_curly_double_quotes_in_cjk_context() {
        // Source: muan-blog notes/2020-09-11-zz.md has pre-existing U+201C
        // before "..." (ellipsis): #TIFF20 \u{201C}... stopping
        // kramdown passes it through as-is.
        let input = "#TIFF20 \u{201C}... stopping the virus\u{201D}\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{201C}") && html.contains("\u{201D}"),
            "Pre-existing curly double quotes must be preserved. Got: {}",
            html
        );
    }

    // Issue 247: kramdown SQ_RULES for consecutive quote sequences

    #[test]
    fn test_issue247_double_quotes_mid_sentence_lsquo_pair() {
        // ''implicit'' mid-sentence: space before '' -> both become lsquo (U+2018)
        // closing '' -> both become rsquo (U+2019)
        let input = "A place is ''implicit'' if\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}implicit\u{2019}\u{2019}"),
            "Mid-sentence ''word'' should produce lsquo+lsquo...rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_double_quotes_start_of_line() {
        // ''Atomicity'' at start: Rule 7 matches '' as text(') + rsquo
        // closing '' -> rsquo + rsquo
        let input = "''Atomicity''\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("'\u{2019}Atomicity\u{2019}\u{2019}"),
            "Start-of-line ''word'' should produce straight+rsquo...rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_double_quotes_before_punctuation() {
        // ''Views'': at start, closing '' before colon
        let input = "''Views'': definition\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("'\u{2019}Views\u{2019}\u{2019}:"),
            "''word'': should produce straight+rsquo...rsquo+rsquo before colon. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_triple_quotes() {
        // '''Bold''' triple quotes
        let input = "'''Bold'''\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{2019}'\u{2019}Bold\u{2019}\u{2019}\u{2019}"),
            "'''word''' should produce rsquo+straight+rsquo...rsquo+rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_apostrophe_inside_double_quotes() {
        // ''cat's'' with apostrophe inside
        let input = "The ''cat's'' whiskers\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}cat\u{2019}s\u{2019}\u{2019}"),
            "''cat's'' should produce lsquo+lsquo...rsquo(apostrophe)...rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_smart_punctuation_disabled_straight_quotes() {
        // When smart punctuation is disabled, '' stays as literal straight quotes
        let input = "''Atomicity''\n";
        let html = markdown_to_html_with_options(input, false, false, false, false);
        assert!(
            html.contains("''Atomicity''"),
            "With smart punctuation off, ''word'' should stay as straight quotes. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_no_regression_normal_apostrophe() {
        // Normal apostrophes should still work
        let input = "It's a cat's life\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("It\u{2019}s") && html.contains("cat\u{2019}s"),
            "Normal apostrophes should produce rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_unicode_consecutive_quotes() {
        // Cyrillic text with consecutive quotes
        let input = "\u{0422}\u{0435}\u{043A}\u{0441}\u{0442} ''\u{0422}\u{0435}\u{043E}\u{0440}\u{0435}\u{043C}\u{0430}'' \u{0432}\u{0430}\u{0436}\u{043D}\u{0430}\n";
        let html = markdown_to_html(input);
        // Mid-sentence: space before '' -> lsquo pair, closing '' -> rsquo pair
        assert!(
            html.contains("\u{2018}\u{2018}\u{0422}\u{0435}\u{043E}\u{0440}\u{0435}\u{043C}\u{0430}\u{2019}\u{2019}"),
            "Cyrillic ''word'' mid-sentence should produce lsquo+lsquo...rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_after_dollar_math() {
        // $X$ is called the ''body'' -- space before '' triggers lsquo pair
        let input = "$X$ is called the ''body''\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{2018}\u{2018}body\u{2019}\u{2019}"),
            "''body'' after math should produce lsquo+lsquo...rsquo+rsquo. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_preexisting_curly_quotes_unchanged() {
        // Pre-existing curly quotes in source should pass through unchanged
        let input = "He said \u{2018}hello\u{2019} and left\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("\u{2018}hello\u{2019}"),
            "Pre-existing curly quotes should be preserved. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_math_and_consecutive_quotes() {
        // Math content f'(x) preserved AND ''term'' gets correct smart quotes
        let input = "$f'(x)$ and ''term''\n";
        let html = markdown_to_html(input);
        // Math should be preserved
        assert!(
            html.contains("f'(x)") || html.contains("f\u{2019}(x)"),
            "Math content should be preserved. Got: {}",
            html
        );
        // ''term'' at end of line (after space) -> lsquo pair, then rsquo pair at end
        assert!(
            html.contains("\u{2018}\u{2018}term\u{2019}\u{2019}"),
            "''term'' after math should get correct smart quotes. Got: {}",
            html
        );
    }

    #[test]
    fn test_issue247_with_options_mid_sentence() {
        // Same test via markdown_to_html_with_options with smart punctuation on
        let input = "A place is ''implicit'' if\n";
        let html = markdown_to_html_with_options(input, true, true, false, false);
        assert!(
            html.contains("\u{2018}\u{2018}implicit\u{2019}\u{2019}"),
            "markdown_to_html_with_options mid-sentence ''word'' should work. Got: {}",
            html
        );
    }

    // ========================================================================
    // escape_quotes_in_text_nodes unit tests
    // ========================================================================

    #[test]
    fn test_escape_quotes_in_text_simple() {
        let input = r#"<p>"hello"</p>"#;
        let result = escape_quotes_in_text_nodes(input);
        assert_eq!(result, "<p>&quot;hello&quot;</p>");
    }

    #[test]
    fn test_escape_quotes_preserves_attribute_quotes() {
        let input = r#"<a href="http://example.com">link</a>"#;
        let result = escape_quotes_in_text_nodes(input);
        assert_eq!(result, r#"<a href="http://example.com">link</a>"#);
    }

    #[test]
    fn test_escape_quotes_skips_script_blocks() {
        let input = r#"<script>{"key": "value"}</script>"#;
        let result = escape_quotes_in_text_nodes(input);
        assert_eq!(result, r#"<script>{"key": "value"}</script>"#);
    }

    #[test]
    fn test_escape_quotes_skips_style_blocks() {
        let input = r#"<style>body { font-family: "Arial"; }</style>"#;
        let result = escape_quotes_in_text_nodes(input);
        assert_eq!(result, r#"<style>body { font-family: "Arial"; }</style>"#);
    }

    #[test]
    fn test_escape_quotes_unicode_content() {
        let input = "<p>\u{201C}curly\u{201D} and \"straight\"</p>";
        let result = escape_quotes_in_text_nodes(input);
        assert!(result.contains("&quot;straight&quot;"));
        // Curly quotes should not be affected
        assert!(result.contains("\u{201C}curly\u{201D}"));
    }

    #[test]
    fn test_escape_quotes_mixed_tags_and_text() {
        let input = "<meta content=\"test\"><p>\"Email Mu-An\"</p>";
        let result = escape_quotes_in_text_nodes(input);
        // Attribute quotes unchanged, text quotes escaped
        assert_eq!(
            result,
            "<meta content=\"test\"><p>&quot;Email Mu-An&quot;</p>"
        );
    }

    // ========================================================================
    // normalize_block_whitespace unit tests
    // ========================================================================

    #[test]
    fn test_normalize_block_whitespace_collapses_double_newline() {
        let input = "<div>\n\n<p>text</p>";
        assert_eq!(normalize_block_whitespace(input), "<div>\n<p>text</p>");
    }

    #[test]
    fn test_normalize_block_whitespace_preserves_single_newline() {
        let input = "<div>\n<p>text</p>";
        assert_eq!(normalize_block_whitespace(input), "<div>\n<p>text</p>");
    }

    #[test]
    fn test_normalize_block_whitespace_preserves_non_tag_after_newlines() {
        let input = "<div>\n\ntext";
        assert_eq!(normalize_block_whitespace(input), "<div>\n\ntext");
    }

    #[test]
    fn test_normalize_block_whitespace_collapses_triple_newline() {
        let input = "<div>\n\n\n<p>text</p>";
        assert_eq!(normalize_block_whitespace(input), "<div>\n<p>text</p>");
    }

    #[test]
    fn test_normalize_block_whitespace_empty() {
        assert_eq!(normalize_block_whitespace(""), "");
    }

    #[test]
    fn test_normalize_block_whitespace_no_tags() {
        let input = "just plain text";
        assert_eq!(normalize_block_whitespace(input), "just plain text");
    }

    #[test]
    fn test_normalize_block_whitespace_utf8() {
        let input = "<div>\n\n<p>Привет мир</p>";
        assert_eq!(
            normalize_block_whitespace(input),
            "<div>\n<p>Привет мир</p>"
        );
    }

    // Issue 270: Underscore emphasis runs (kramdown-specific behavior)

    #[test]
    fn test_markdown_underscore_emphasis_seven_in_quotes() {
        let input = "\"I can has _______\" was also\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<strong>__</strong>_"),
            "Expected <strong>__</strong>_ for 7 underscores, got: {html}"
        );
    }

    #[test]
    fn test_markdown_underscore_emphasis_six() {
        let input = "text ______\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<strong>__</strong>"),
            "Expected <strong>__</strong> for 6 underscores, got: {html}"
        );
    }

    #[test]
    fn test_markdown_underscore_emphasis_four() {
        let input = "text ____\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<em>__</em>"),
            "Expected <em>__</em> for 4 underscores, got: {html}"
        );
    }

    #[test]
    fn test_markdown_underscore_emphasis_three_literal() {
        let input = "text ___\n";
        let html = markdown_to_html(input);
        assert!(
            !html.contains("<em>") && !html.contains("<strong>"),
            "3 underscores inline should stay literal, got: {html}"
        );
    }

    #[test]
    fn test_markdown_underscore_emphasis_unicode() {
        let input = "__\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}__\n";
        let html = markdown_to_html(input);
        assert!(
            html.contains("<strong>\u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442}</strong>"),
            "Expected strong around Unicode content, got: {html}"
        );
    }

    // ========================================================================
    // Issue 294: GFM autolink for bare URLs
    // ========================================================================

    #[test]
    fn test_issue294_bare_https_url_autolinked() {
        let html = markdown_to_html_with_options(
            "Visit https://example.com for details.\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "Bare HTTPS URL should be auto-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_bare_http_url_autolinked() {
        let html = markdown_to_html_with_options(
            "Check http://old.example.com too.\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            html.contains(r#"<a href="http://old.example.com">http://old.example.com</a>"#),
            "Bare HTTP URL should be auto-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_bare_url_end_of_line() {
        let html =
            markdown_to_html_with_options("See https://example.com\n", false, false, false, true);
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "Bare URL at end of line should be auto-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_bare_url_followed_by_period() {
        let html = markdown_to_html_with_options(
            "Go to https://example.com.\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "URL followed by period should link URL without the period. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_bare_url_in_parens() {
        let html =
            markdown_to_html_with_options("(see https://example.com)\n", false, false, false, true);
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "URL in parens should link URL without the paren. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_no_double_linking_markdown_link() {
        let html = markdown_to_html_with_options(
            "[Example](https://example.com)\n",
            false,
            false,
            false,
            true,
        );
        let a_count = html.matches("<a ").count();
        assert_eq!(
            a_count, 1,
            "Markdown link should not be double-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_no_autolink_in_code_span() {
        let html =
            markdown_to_html_with_options("`https://example.com`\n", false, false, false, true);
        assert!(
            !html.contains("<a "),
            "URL inside code span should NOT be auto-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_autolink_disabled_by_default() {
        let html = markdown_to_html_with_options(
            "Visit https://example.com for details.\n",
            false,
            false,
            false,
            false,
        );
        assert!(
            !html.contains("<a "),
            "Bare URL should NOT be auto-linked when autolink is disabled. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_kramdown_mode_no_autolink() {
        let html = markdown_to_html("Visit https://example.com for details.\n");
        assert!(
            !html.contains(r#"<a href="https://example.com">"#),
            "kramdown mode should NOT auto-link bare URLs. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_angle_bracket_autolink_still_works() {
        let html = markdown_to_html_with_options(
            "Visit <https://example.com> for details.\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "Angle bracket autolinks should still work. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_bare_url_unicode_text() {
        let html = markdown_to_html_with_options(
            "\u{041f}\u{043e}\u{0441}\u{0435}\u{0442}\u{0438}\u{0442}\u{0435} https://example.com \u{0434}\u{043b}\u{044f} \u{0438}\u{043d}\u{0444}\u{043e}\u{0440}\u{043c}\u{0430}\u{0446}\u{0438}\u{0438}.\n",
            false, false, false, true,
        );
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "Bare URL with surrounding Unicode text should be auto-linked. Got: {html}"
        );
        assert!(
            html.contains("\u{041f}\u{043e}\u{0441}\u{0435}\u{0442}\u{0438}\u{0442}\u{0435}"),
            "Surrounding Unicode text should be preserved. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_no_autolink_in_code_block() {
        let html = markdown_to_html_with_options(
            "```\nhttps://example.com\n```\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            !html.contains("<a "),
            "URL inside code block should NOT be auto-linked. Got: {html}"
        );
    }

    #[test]
    fn test_issue294_multiple_urls_in_paragraph() {
        let html = markdown_to_html_with_options(
            "Check https://example.com and https://other.org for info.\n",
            false,
            false,
            false,
            true,
        );
        assert!(
            html.contains(r#"<a href="https://example.com">https://example.com</a>"#),
            "First URL should be auto-linked. Got: {html}"
        );
        assert!(
            html.contains(r#"<a href="https://other.org">https://other.org</a>"#),
            "Second URL should be auto-linked. Got: {html}"
        );
    }
}

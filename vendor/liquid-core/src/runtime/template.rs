use std::io::Write;

use crate::error::Result;

use super::Renderable;
use super::Runtime;

/// An executable template block.
#[derive(Debug)]
pub struct Template {
    elements: Vec<Box<dyn Renderable>>,
}

impl Template {
    /// Create an executable template block.
    pub fn new(elements: Vec<Box<dyn Renderable>>) -> Template {
        Template { elements }
    }
}

impl Renderable for Template {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        // Check if any element in this template uses runtime whitespace stripping.
        // If so, we need to buffer the output so we can strip trailing whitespace
        // before elements that use `{{-`.
        let needs_buffering = self
            .elements
            .iter()
            .any(|el| el.needs_leading_whitespace_strip() || el.needs_trailing_whitespace_strip());

        if needs_buffering {
            return self.render_to_buffered(writer, runtime);
        }

        for el in &self.elements {
            el.render_to(writer, runtime)?;

            // Did the last element we processed set an interrupt? If so, we
            // need to abandon the rest of our child elements and just
            // return what we've got. This is usually in response to a
            // `break` or `continue` tag being rendered.
            //
            // Optimization: check the fast Cell<bool> flag first to avoid
            // the expensive AnyMap lookup on every element. The flag is only
            // set when break/continue actually fires, so this is a simple
            // branch prediction win for the common (no-interrupt) case.
            if runtime.registers().interrupted_fast.get() {
                break;
            }
        }
        Ok(())
    }
}

impl Template {
    /// Buffered rendering that supports runtime whitespace stripping for `{{-` and `-}}`.
    ///
    /// This is used when the template contains expressions with dash whitespace control.
    /// It buffers the output so that `{{-` can strip trailing whitespace from previously
    /// rendered elements, matching Ruby Liquid's behavior.
    ///
    /// Heuristic: `{{-` strips trailing whitespace from the buffer only when the
    /// trailing whitespace region contains at least one newline character. This
    /// distinguishes template-originating whitespace (which always includes newlines
    /// from line breaks between tags) from expression-originating whitespace (like
    /// a single trailing space in `' | '`).
    fn render_to_buffered(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()> {
        let mut buffer = Vec::with_capacity(4096);
        let mut rstrip_next = false;
        // Track the buffer position right after the last expression/tag that
        // produced output. Text nodes (template literal whitespace) do NOT
        // update this boundary. This lets `{{-` strip all trailing template
        // whitespace (from Text nodes) without reaching into rendered output
        // from expressions like `{{ ' ' }}`.
        let mut expression_output_end: usize = 0;

        for el in &self.elements {
            // If this element needs leading whitespace stripped ({{-),
            // strip trailing whitespace from the buffer, but only in the
            // region after the last expression output. This ensures we
            // only strip Text-originated (template) whitespace and never
            // consume rendered output from expressions.
            if el.needs_leading_whitespace_strip() {
                strip_trailing_template_whitespace_in_range(&mut buffer, expression_output_end);
            }

            let before_len = buffer.len();

            // If the previous element used -}}, strip leading whitespace
            // from this element's output, but only template whitespace
            // (containing a newline). This prevents stripping rendered
            // output like the space from `{{ ' ' }}`.
            if rstrip_next {
                let mut temp = Vec::new();
                el.render_to(&mut temp, runtime)?;
                if el.is_raw_text() {
                    // Text node: apply newline-based stripping
                    let s = std::str::from_utf8(&temp).unwrap_or("");
                    let trimmed = strip_leading_template_whitespace(s);
                    buffer.extend_from_slice(trimmed.as_bytes());
                } else {
                    // Expression/tag output: never strip
                    buffer.extend_from_slice(&temp);
                }
            } else {
                el.render_to(&mut buffer, runtime)?;
            }

            // Update expression_output_end only when an expression element
            // (`{{ ... }}`) produced output. Expression output is intentional
            // content that must be protected from whitespace stripping.
            // Block tags and text nodes do NOT advance this boundary,
            // allowing `{{-` to strip through incidental whitespace from
            // template structure.
            if el.is_expression_output() && buffer.len() > before_len {
                expression_output_end = buffer.len();
            }

            rstrip_next = el.needs_trailing_whitespace_strip();

            if runtime.registers().interrupted_fast.get() {
                break;
            }
        }

        writer.write_all(&buffer).map_err(|e| {
            crate::error::Error::with_msg(format!("Failed to write buffered output: {}", e))
        })?;
        Ok(())
    }
}

/// Strip trailing whitespace from a byte buffer within the range
/// `[range_start..]`, but only if the trailing whitespace region in that
/// range contains at least one newline (`\n`).
///
/// By scoping the strip to the last element's output region, we avoid
/// consuming rendered output from earlier elements (e.g., the space from
/// `{{ ' ' }}`). Only template-originating whitespace (Text nodes with
/// newlines) gets stripped.
///
/// Examples (with range_start covering only the last element):
/// - Last element wrote `"\n"` -> stripped (newline found)
/// - Last element wrote `" "` (from `{{ ' ' }}`) -> not stripped (no newline)
/// - Last element wrote `"\n  "` -> stripped (newline found)
fn strip_trailing_template_whitespace_in_range(buffer: &mut Vec<u8>, range_start: usize) {
    if range_start >= buffer.len() {
        return;
    }

    // Find where trailing whitespace starts within the range
    let trailing_ws_start = buffer[range_start..]
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|pos| range_start + pos + 1)
        .unwrap_or(range_start);

    // Check if the trailing whitespace region contains a newline
    let has_newline = buffer[trailing_ws_start..].contains(&b'\n');

    if has_newline {
        buffer.truncate(trailing_ws_start);
    }
}

/// Strip leading whitespace from a string, but only if that leading
/// whitespace region contains at least one newline (`\n`).
///
/// This is the right-strip (`-}}`) counterpart: it only strips template
/// whitespace (Text nodes with newlines between tags), not rendered output
/// like the space from `{{ ' ' }}`.
fn strip_leading_template_whitespace(s: &str) -> &str {
    let trimmed = s.trim_start();
    let leading_ws = &s[..s.len() - trimmed.len()];
    if leading_ws.contains('\n') {
        trimmed
    } else {
        s
    }
}

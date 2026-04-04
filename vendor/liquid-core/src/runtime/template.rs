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

        for el in &self.elements {
            // If this element needs leading whitespace stripped ({{-),
            // strip trailing whitespace from the buffer, but only if the
            // trailing whitespace contains a newline (template whitespace).
            if el.needs_leading_whitespace_strip() {
                strip_trailing_template_whitespace(&mut buffer);
            }

            // If the previous element used -}}, strip leading whitespace
            // from this element's output by rendering to a temp buffer first.
            if rstrip_next {
                let mut temp = Vec::new();
                el.render_to(&mut temp, runtime)?;
                let s = std::str::from_utf8(&temp).unwrap_or("");
                let trimmed = s.trim_start();
                buffer.extend_from_slice(trimmed.as_bytes());
            } else {
                el.render_to(&mut buffer, runtime)?;
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

/// Strip trailing whitespace from a byte buffer, but only if the trailing
/// whitespace region contains at least one newline (`\n`).
///
/// This heuristic distinguishes template-originating whitespace (which always
/// includes newlines from line breaks between Liquid tags) from expression-
/// originating whitespace (like a single trailing space in `' | '`).
///
/// Examples:
/// - `"About | "` -> not stripped (trailing ` ` has no newline)
/// - `"text\n    \n  "` -> stripped to `"text"` (newline found)
/// - `"text\n"` -> stripped to `"text"` (newline found)
fn strip_trailing_template_whitespace(buffer: &mut Vec<u8>) {
    // First check if there's any trailing whitespace at all
    let trailing_ws_start = buffer
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|pos| pos + 1)
        .unwrap_or(0);

    // Check if the trailing whitespace region contains a newline
    let has_newline = buffer[trailing_ws_start..].contains(&b'\n');

    if has_newline {
        buffer.truncate(trailing_ws_start);
    }
}

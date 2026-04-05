use std::fmt::Debug;
use std::io::Write;

use crate::error::Result;

use super::Runtime;

/// Any object (tag/block) that can be rendered by liquid must implement this trait.
pub trait Renderable: Send + Sync + Debug {
    /// Renders the Renderable instance given a Liquid runtime.
    fn render(&self, runtime: &dyn Runtime) -> Result<String> {
        // Pre-size the buffer to avoid repeated reallocations for typical
        // layout templates (10-50KB output). 16KB covers most pages without
        // over-allocating for small templates.
        let mut data = Vec::with_capacity(16 * 1024);
        self.render_to(&mut data, runtime)?;
        Ok(String::from_utf8(data).expect("render only writes UTF-8"))
    }

    /// Renders the Renderable instance given a Liquid runtime.
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> Result<()>;

    /// Returns `true` if this element uses `{{-` (left-strip) whitespace control.
    ///
    /// When `true`, `Template::render_to` will strip trailing whitespace from the
    /// output buffer before rendering this element. This implements runtime
    /// whitespace stripping that matches Ruby Liquid's behavior, where `{{-`
    /// strips whitespace from the output buffer (not just adjacent template text).
    fn needs_leading_whitespace_strip(&self) -> bool {
        false
    }

    /// Returns `true` if this element uses `-}}` (right-strip) whitespace control.
    ///
    /// When `true`, `Template::render_to` will strip leading whitespace from the
    /// output of subsequent elements. This implements runtime whitespace stripping
    /// that matches Ruby Liquid's behavior.
    fn needs_trailing_whitespace_strip(&self) -> bool {
        false
    }

    /// Returns `true` if this element is raw template text (a `Text` node).
    ///
    /// Used by `Template::render_to_buffered` to distinguish template-originating
    /// whitespace (which can be stripped by `{{-` / `-}}`) from expression-rendered
    /// output (which must not be stripped).
    fn is_raw_text(&self) -> bool {
        false
    }

    /// Returns `true` if this element is a Liquid expression (`{{ ... }}`).
    ///
    /// Expression output is user-intentional content that must be protected
    /// from whitespace stripping by adjacent `{{-` / `-}}` tags. Block tags
    /// (`{% if %}`, `{% assign %}`) return false — their incidental whitespace
    /// output (from template text in their body) can be stripped.
    fn is_expression_output(&self) -> bool {
        false
    }
}

use std::io::Write;

use crate::error::{Result, ResultLiquidReplaceExt};
use crate::runtime::Renderable;
use crate::runtime::Runtime;

/// A raw template expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Text {
    text: String,
}

impl Text {
    /// Create a raw template expression.
    pub(crate) fn new<S: Into<String>>(text: S) -> Text {
        Text { text: text.into() }
    }
}

impl Renderable for Text {
    fn render_to(&self, writer: &mut dyn Write, _runtime: &dyn Runtime) -> Result<()> {
        // Use write_all for raw text output instead of write!/Display formatting.
        // Text nodes are the most common renderable element and this avoids the
        // overhead of the format machinery.
        writer
            .write_all(self.text.as_bytes())
            .replace("Failed to render")?;
        Ok(())
    }
}

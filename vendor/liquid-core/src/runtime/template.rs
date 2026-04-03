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

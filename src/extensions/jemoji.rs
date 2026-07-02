//! `jemoji` extension: the `jemoji` plugin as an [`HtmlTransform`].
//!
//! This is a thin adapter that reuses the existing byte-for-byte emoji logic in
//! [`crate::jemoji::process_jemoji`] (skip-tags for `<code>`/`<pre>`/`<tt>`/
//! `<script>`/`<style>`, HTML-attribute suppression, unknown-shortcode
//! passthrough, GitHub emoji CDN URLs). No emoji tables are reimplemented here.
//!
//! Activation is unchanged from the pre-extension-framework behavior: the
//! [`Registry`](super::Registry) auto-activates this transform when `jemoji` is
//! listed under `plugins:`/`gems:` (see [`crate::jemoji::has_jemoji_plugin`]).
//! No `extensions:` block is required.

use super::{HtmlTransform, SiteIndex, TransformContext, TransformResult};

/// The `jemoji` post-render transform.
#[derive(Debug, Clone, Default)]
pub struct Jemoji;

impl Jemoji {
    /// The transform's name (stable identifier used for ordering assertions).
    pub const NAME: &'static str = "jemoji";

    /// Build the transform.
    pub fn new() -> Self {
        Jemoji
    }
}

impl HtmlTransform for Jemoji {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn transform(
        &self,
        html: &str,
        _index: &SiteIndex,
        _ctx: &TransformContext,
    ) -> TransformResult {
        // Delegate verbatim to the existing processor; emoji apply to all
        // documents and ignore the site index / collection context.
        TransformResult {
            html: crate::jemoji::process_jemoji(html),
            warnings: Vec::new(),
        }
    }
}

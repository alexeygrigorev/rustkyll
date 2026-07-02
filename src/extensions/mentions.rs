//! `mentions` extension: the `jekyll-mentions` plugin as an [`HtmlTransform`].
//!
//! This is a thin adapter that reuses the existing byte-for-byte mention logic
//! in [`crate::mentions::process_mentions`] (skip-tags for `<code>`/`<pre>`/
//! `<a>`/`<script>`, email suppression, team-mention `@org/team` suppression).
//! No parsing rules are reimplemented here.
//!
//! Activation is unchanged from the pre-extension-framework behavior: the
//! [`Registry`](super::Registry) auto-activates this transform when
//! `jekyll-mentions` is listed under `plugins:`/`gems:` (see
//! [`crate::mentions::has_mentions_plugin`]). No `extensions:` block is required.

use super::{HtmlTransform, SiteIndex, TransformContext, TransformResult};
use crate::config::SiteConfig;

/// The `jekyll-mentions` post-render transform.
///
/// The GitHub-profile base URL is captured at construction time from
/// `jekyll-mentions.base_url` (via [`crate::mentions::mentions_base_url`]).
/// It deliberately does **not** use [`TransformContext::baseurl`], which is the
/// site `baseurl` used by `wikilinks` — a different value.
#[derive(Debug, Clone)]
pub struct Mentions {
    base_url: String,
}

impl Mentions {
    /// The transform's name (stable identifier used for ordering assertions).
    pub const NAME: &'static str = "mentions";

    /// Build the transform, capturing the mention base URL from site config.
    pub fn from_config(config: &SiteConfig) -> Self {
        Mentions {
            base_url: crate::mentions::mentions_base_url(config).to_string(),
        }
    }
}

impl HtmlTransform for Mentions {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn transform(
        &self,
        html: &str,
        _index: &SiteIndex,
        _ctx: &TransformContext,
    ) -> TransformResult {
        // Delegate verbatim to the existing processor; mentions apply to all
        // documents and ignore the site index / collection context.
        TransformResult {
            html: crate::mentions::process_mentions(html, &self.base_url),
            warnings: Vec::new(),
        }
    }
}

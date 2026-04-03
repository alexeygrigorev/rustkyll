//! Template engine module wrapping the `liquid` crate for Jekyll-compatible
//! Liquid template rendering.
//!
//! # Overview
//!
//! - `TemplateEngine` -- parse and render Liquid templates
//! - `LayoutEngine` -- load layouts and includes, render pages with wrapping
//! - `TemplateError` -- error types for template operations
//! - `yaml_to_liquid` / `yaml_mapping_to_object` -- convert YAML data to Liquid values
//! - `build_context` -- convenience helper for building template contexts

pub mod avatar_tag;
pub mod context;
pub mod details_tag;
pub mod engine;
pub mod error;
pub mod feed_meta_tag;
pub mod file_exists_tag;
pub mod filters;
pub mod gist_tag;
pub mod highlight_tag;
pub mod include_tag;
pub mod layout;
pub mod noop_tags;
pub mod octicon_tag;
pub mod seo_tag;
pub mod translate_tag;

pub use context::{
    build_context, normalize_frontmatter_date, yaml_mapping_to_object,
    yaml_mapping_to_object_with_tz, yaml_to_liquid,
};
pub use engine::{load_includes, load_includes_merged, TemplateEngine};
pub use error::TemplateError;
pub use layout::{build_render_context, load_layouts, Layout, LayoutEngine};

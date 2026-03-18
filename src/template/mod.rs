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
pub mod engine;
pub mod error;
pub mod filters;
pub mod highlight_tag;
pub mod include_tag;
pub mod layout;
pub mod seo_tag;

pub use context::{
    build_context, yaml_mapping_to_object, yaml_mapping_to_object_with_tz, yaml_to_liquid,
};
pub use engine::{load_includes, TemplateEngine};
pub use error::TemplateError;
pub use layout::{build_render_context, load_layouts, Layout, LayoutEngine};

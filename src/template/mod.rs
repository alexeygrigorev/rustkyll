//! Template engine module wrapping the `liquid` crate for Jekyll-compatible
//! Liquid template rendering.
//!
//! # Overview
//!
//! - `TemplateEngine` -- parse and render Liquid templates
//! - `TemplateError` -- error types for template operations
//! - `yaml_to_liquid` / `yaml_mapping_to_object` -- convert YAML data to Liquid values
//! - `build_context` -- convenience helper for building template contexts

pub mod context;
pub mod engine;
pub mod error;
pub mod filters;

pub use context::{build_context, yaml_mapping_to_object, yaml_to_liquid};
pub use engine::TemplateEngine;
pub use error::TemplateError;

pub mod archives;
pub mod collection;
pub mod compare;
pub mod config;
pub mod data;
pub mod docker_highlight;
pub mod feed;
pub mod frontmatter;
pub mod generator;
pub mod incremental;
pub mod kramdown;
pub mod kramdown_parser;
pub mod livereload;
pub mod pagination;
pub mod progress;
pub mod server;
pub mod sitemap;
pub mod static_files;
pub mod syntax;
pub mod template;
pub mod yaml;

/// Returns the name of this project.
pub fn project_name() -> &'static str {
    "rustkyll"
}

/// Returns the version string from Cargo.toml.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

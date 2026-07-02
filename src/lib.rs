pub mod alert_shorturl_generator;
pub mod archives;
pub mod collection;
pub mod compare;
pub mod config;
pub mod data;
pub mod docker_highlight;
pub mod extensions;
pub mod feed;
pub mod frontmatter;
pub mod generator;
pub mod incremental;
pub mod jemoji;
pub mod kramdown;
pub mod kramdown_parser;
pub mod livereload;
pub mod mentions;
pub mod pagination;
pub mod plugin_generators;
pub mod progress;
pub mod redirect_generator;
pub mod server;
pub mod sitemap;
pub mod static_files;
pub mod syntax;
pub mod template;
pub mod template_generators;
pub mod wallet_generator;
pub mod yaml;

/// Returns the name of this project.
pub fn project_name() -> &'static str {
    "rustkyll"
}

/// Returns the version string from Cargo.toml.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub mod archives;
pub mod collection;
pub mod compare;
pub mod config;
pub mod data;
pub mod feed;
pub mod frontmatter;
pub mod generator;
pub mod incremental;
pub mod jsonld;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_name() {
        assert_eq!(project_name(), "rustkyll");
    }

    #[test]
    fn test_version_is_not_empty() {
        assert!(!version().is_empty());
    }
}

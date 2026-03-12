pub mod collection;
pub mod config;
pub mod data;
pub mod frontmatter;
pub mod template;

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

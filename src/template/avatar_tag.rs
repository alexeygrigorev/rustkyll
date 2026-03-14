//! Custom `{% avatar %}` tag implementing the Jekyll Avatar plugin.
//!
//! Generates an `<img>` tag for a GitHub user's avatar. Supports these forms:
//! - `{% avatar USERNAME %}` -- literal username, default size (40)
//! - `{% avatar user=variable %}` -- username from a variable
//! - `{% avatar user=variable size=N %}` -- explicit pixel size
//!
//! Output matches the jekyll-avatar plugin: an `<img>` tag with `class`,
//! `src`, `srcset` (1x-4x), `alt`, `width`, and `height` attributes.

use std::io::Write;

use liquid_core::error::ResultLiquidReplaceExt;
use liquid_core::model::ValueView;
use liquid_core::parser::TryMatchToken;
use liquid_core::{Language, ParseTag, Renderable, Runtime, TagReflection, TagTokenIter};

/// The `{% avatar %}` tag parser/reflection.
#[derive(Copy, Clone, Debug, Default)]
pub struct AvatarTag;

impl TagReflection for AvatarTag {
    fn tag(&self) -> &'static str {
        "avatar"
    }

    fn description(&self) -> &'static str {
        "Generate a GitHub avatar img tag"
    }
}

/// Where the username comes from: a literal string or a variable name.
#[derive(Debug)]
enum UsernameSource {
    Literal(String),
    Variable(String),
}

impl ParseTag for AvatarTag {
    fn parse(
        &self,
        mut arguments: TagTokenIter<'_>,
        _options: &Language,
    ) -> liquid_core::Result<Box<dyn Renderable>> {
        let mut username_source: Option<UsernameSource> = None;
        let mut size: u32 = 40;

        // Parse arguments: either a literal username or key=value pairs
        while let Ok(next) = arguments.expect_next("") {
            match next.expect_identifier() {
                TryMatchToken::Matches(id) => {
                    let id_str = id.to_kstr().to_string();
                    if id_str == "user" || id_str == "size" {
                        // Expect "="
                        if let Ok(eq_token) = arguments.expect_next("") {
                            let _ = eq_token.expect_str("=").into_result();
                            if let Ok(val_token) = arguments.expect_next("") {
                                match val_token.expect_identifier() {
                                    TryMatchToken::Matches(val) => {
                                        let val_str = val.to_kstr().to_string();
                                        if id_str == "user" {
                                            username_source =
                                                Some(UsernameSource::Variable(val_str));
                                        } else if let Ok(n) = val_str.parse::<u32>() {
                                            size = n;
                                        }
                                    }
                                    TryMatchToken::Fails(token) => {
                                        // Try as value (for numeric size)
                                        match token.expect_value() {
                                            TryMatchToken::Matches(expr) => {
                                                let expr_str = format!("{}", expr);
                                                if id_str == "user" {
                                                    username_source =
                                                        Some(UsernameSource::Variable(expr_str));
                                                } else if let Ok(n) = expr_str.parse::<u32>() {
                                                    size = n;
                                                }
                                            }
                                            TryMatchToken::Fails(_) => {}
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Literal username
                        username_source = Some(UsernameSource::Literal(id_str));
                    }
                }
                TryMatchToken::Fails(token) => {
                    // Try as a value (e.g., a string literal)
                    match token.expect_value() {
                        TryMatchToken::Matches(expr) => {
                            let expr_str = format!("{}", expr);
                            if username_source.is_none() {
                                username_source = Some(UsernameSource::Literal(expr_str));
                            }
                        }
                        TryMatchToken::Fails(_) => {}
                    }
                }
            }
        }

        let source = username_source.unwrap_or(UsernameSource::Literal(String::new()));

        Ok(Box::new(Avatar {
            username_source: source,
            size,
        }))
    }

    fn reflection(&self) -> &dyn TagReflection {
        self
    }
}

#[derive(Debug)]
struct Avatar {
    username_source: UsernameSource,
    size: u32,
}

impl std::fmt::Display for Avatar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "avatar")
    }
}

/// Render the avatar `<img>` tag.
fn render_avatar(writer: &mut dyn Write, username: &str, size: u32) -> liquid_core::Result<()> {
    if username.is_empty() {
        return Ok(());
    }

    let size_class = if size <= 48 { " avatar-small" } else { "" };
    let base_url = format!(
        "https://avatars.githubusercontent.com/{}?v=4&amp;s={}",
        username, size
    );
    let srcset = format!(
        "https://avatars.githubusercontent.com/{user}?v=4&amp;s={s1} 1x, \
         https://avatars.githubusercontent.com/{user}?v=4&amp;s={s2} 2x, \
         https://avatars.githubusercontent.com/{user}?v=4&amp;s={s3} 3x, \
         https://avatars.githubusercontent.com/{user}?v=4&amp;s={s4} 4x",
        user = username,
        s1 = size,
        s2 = size * 2,
        s3 = size * 3,
        s4 = size * 4,
    );

    write!(
        writer,
        "<img class=\"avatar{}\" src=\"{}\" alt=\"{}\" srcset=\"{}\" width=\"{}\" height=\"{}\" />",
        size_class, base_url, username, srcset, size, size
    )
    .replace("Failed to render avatar tag")?;

    Ok(())
}

impl Renderable for Avatar {
    fn render_to(&self, writer: &mut dyn Write, runtime: &dyn Runtime) -> liquid_core::Result<()> {
        let username = match &self.username_source {
            UsernameSource::Literal(name) => name.clone(),
            UsernameSource::Variable(var_name) => {
                // Look up the variable in the runtime context
                // Try to resolve it as a variable path
                let var = runtime.try_get(&[liquid_core::model::ScalarCow::new(var_name.as_str())]);
                match var {
                    Some(val) => val.to_kstr().to_string(),
                    None => String::new(),
                }
            }
        };

        render_avatar(writer, &username, self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::super::engine::TemplateEngine;
    use liquid::Object;

    fn engine() -> TemplateEngine {
        TemplateEngine::new().unwrap()
    }

    #[test]
    fn test_avatar_literal_username() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng.parse_and_render("{% avatar parkr %}", &ctx).unwrap();
        assert!(output.contains("src=\"https://avatars.githubusercontent.com/parkr?v=4&amp;s=40\""));
        assert!(output.contains("alt=\"parkr\""));
        assert!(output.contains("width=\"40\""));
        assert!(output.contains("height=\"40\""));
        assert!(output.contains("class=\"avatar avatar-small\""));
    }

    #[test]
    fn test_avatar_literal_default_size() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng.parse_and_render("{% avatar octocat %}", &ctx).unwrap();
        // Default size is 40
        assert!(output.contains("s=40"));
    }

    #[test]
    fn test_avatar_user_variable() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "author".into(),
            liquid::model::Value::scalar("jekyllbot".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=author size=24 %}", &ctx)
            .unwrap();
        assert!(
            output.contains("src=\"https://avatars.githubusercontent.com/jekyllbot?v=4&amp;s=24\"")
        );
        assert!(output.contains("alt=\"jekyllbot\""));
        assert!(output.contains("width=\"24\""));
        assert!(output.contains("height=\"24\""));
    }

    #[test]
    fn test_avatar_user_variable_default_size() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "author".into(),
            liquid::model::Value::scalar("testuser".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=author %}", &ctx)
            .unwrap();
        assert!(output.contains("s=40"));
        assert!(output.contains("alt=\"testuser\""));
    }

    #[test]
    fn test_avatar_small_class() {
        let eng = engine();
        let ctx = Object::new();
        // Size 48 should still get avatar-small
        let output = eng.parse_and_render("{% avatar user48 %}", &ctx).unwrap();
        assert!(output.contains("class=\"avatar avatar-small\""));
    }

    #[test]
    fn test_avatar_large_no_small_class() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "user".into(),
            liquid::model::Value::scalar("biguser".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=user size=80 %}", &ctx)
            .unwrap();
        assert!(output.contains("class=\"avatar\""));
        assert!(!output.contains("avatar-small"));
    }

    #[test]
    fn test_avatar_srcset() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng.parse_and_render("{% avatar testuser %}", &ctx).unwrap();
        // Check srcset has 1x, 2x, 3x, 4x with correct sizes
        assert!(output.contains("s=40 1x"));
        assert!(output.contains("s=80 2x"));
        assert!(output.contains("s=120 3x"));
        assert!(output.contains("s=160 4x"));
    }

    #[test]
    fn test_avatar_srcset_custom_size() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "author".into(),
            liquid::model::Value::scalar("dev".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=author size=24 %}", &ctx)
            .unwrap();
        assert!(output.contains("s=24 1x"));
        assert!(output.contains("s=48 2x"));
        assert!(output.contains("s=72 3x"));
        assert!(output.contains("s=96 4x"));
    }

    #[test]
    fn test_avatar_whitespace_trimming() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "author".into(),
            liquid::model::Value::scalar("trimuser".to_string()),
        );
        let output = eng
            .parse_and_render("before{% avatar user=author size=24 -%}\nafter", &ctx)
            .unwrap();
        // With -%}, trailing whitespace/newline should be trimmed
        assert!(output.contains("trimuser"));
        // The output should have the img tag followed directly by "after"
        assert!(output.contains("/>after"));
    }

    #[test]
    fn test_avatar_empty_variable() {
        let eng = engine();
        let ctx = Object::new();
        // Variable "author" not set -- should produce empty output
        let output = eng
            .parse_and_render("{% avatar user=author %}", &ctx)
            .unwrap();
        assert_eq!(output, "");
    }
}

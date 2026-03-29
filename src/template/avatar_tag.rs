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
        let mut lazy = false;

        // Parse arguments: either a literal username or key=value pairs
        while let Ok(next) = arguments.expect_next("") {
            match next.expect_identifier() {
                TryMatchToken::Matches(id) => {
                    let id_str = id.to_kstr().to_string();
                    if id_str == "user" || id_str == "size" || id_str == "lazy" {
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
                                        } else if id_str == "lazy" {
                                            lazy = val_str == "true";
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
                                                } else if id_str == "lazy" {
                                                    lazy = expr_str == "true";
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
                    } else if username_source.is_none() {
                        // Only treat as literal username if we haven't already
                        // parsed a user= parameter; otherwise it could be an
                        // unknown key that will be followed by =value tokens
                        // which we need to consume.
                        //
                        // Peek ahead: if the next token is "=", this is an
                        // unknown key=value pair -- consume "=" and the value,
                        // then continue.
                        if let Ok(maybe_eq) = arguments.expect_next("") {
                            match maybe_eq.expect_str("=").into_result() {
                                Ok(_) => {
                                    // Unknown key=value: consume the value token
                                    let _ = arguments.expect_next("");
                                }
                                Err(_) => {
                                    // Not "=", so treat the original id as a
                                    // literal username. The token we just
                                    // consumed is something else; we can't push
                                    // it back, but this edge case (literal
                                    // username followed by non-key token) is
                                    // extremely rare in practice.
                                    username_source = Some(UsernameSource::Literal(id_str));
                                }
                            }
                        } else {
                            // No more tokens after this identifier -- it's a
                            // literal username at end of tag.
                            username_source = Some(UsernameSource::Literal(id_str));
                        }
                    } else {
                        // username_source already set; this is an unknown key.
                        // Try to consume "=" and value if present.
                        if let Ok(maybe_eq) = arguments.expect_next("") {
                            if maybe_eq.expect_str("=").into_result().is_ok() {
                                let _ = arguments.expect_next("");
                            }
                        }
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
            lazy,
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
    lazy: bool,
}

impl std::fmt::Display for Avatar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "avatar")
    }
}

/// Render the avatar `<img>` tag.
fn render_avatar(
    writer: &mut dyn Write,
    username: &str,
    size: u32,
    lazy: bool,
) -> liquid_core::Result<()> {
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

    if lazy {
        write!(
            writer,
            "<img class=\"avatar{}\" src=\"\" alt=\"{}\" data-src=\"{}\" data-srcset=\"{}\" data-proofer-ignore=\"true\" width=\"{}\" height=\"{}\" />",
            size_class, username, base_url, srcset, size, size
        )
        .replace("Failed to render avatar tag")?;
    } else {
        write!(
            writer,
            "<img class=\"avatar{}\" src=\"{}\" alt=\"{}\" srcset=\"{}\" width=\"{}\" height=\"{}\" />",
            size_class, base_url, username, srcset, size, size
        )
        .replace("Failed to render avatar tag")?;
    }

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

        render_avatar(writer, &username, self.size, self.lazy)
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

    // === Issue 513: parser bug -- unknown params must not overwrite username ===

    #[test]
    fn test_avatar_unknown_param_does_not_overwrite_user() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("argob".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=org size=60 lazy=true %}", &ctx)
            .unwrap();
        assert!(
            output.contains("alt=\"argob\""),
            "Expected alt=\"argob\", got: {}",
            output
        );
        assert!(
            !output.contains("alt=\"lazy\""),
            "Username must not be overwritten by unknown param 'lazy'"
        );
    }

    #[test]
    fn test_avatar_unknown_param_order_independent() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("testorg".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=org lazy=true size=60 %}", &ctx)
            .unwrap();
        assert!(
            output.contains("alt=\"testorg\""),
            "Expected alt=\"testorg\", got: {}",
            output
        );
    }

    #[test]
    fn test_avatar_multiple_unknown_params_ignored() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("myorg".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=org lazy=true foo=bar %}", &ctx)
            .unwrap();
        assert!(
            output.contains("alt=\"myorg\""),
            "Expected alt=\"myorg\", got: {}",
            output
        );
    }

    #[test]
    fn test_avatar_literal_username_no_regression() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% avatar username size=40 %}", &ctx)
            .unwrap();
        assert!(
            output.contains("alt=\"username\""),
            "Literal username should still work"
        );
    }

    #[test]
    fn test_avatar_user_variable_defaults() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("deforg".to_string()),
        );
        let output = eng.parse_and_render("{% avatar user=org %}", &ctx).unwrap();
        assert!(output.contains("alt=\"deforg\""));
        assert!(output.contains("s=40"), "Default size should be 40");
    }

    // === Issue 513: lazy loading ===

    #[test]
    fn test_avatar_lazy_variable_user() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("lazyorg".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=org size=60 lazy=true %}", &ctx)
            .unwrap();
        assert!(
            output.contains("src=\"\""),
            "Lazy avatar must have empty src, got: {}",
            output
        );
        assert!(
            output.contains("data-src=\"https://"),
            "Lazy avatar must have data-src, got: {}",
            output
        );
        assert!(
            output.contains("data-srcset=\""),
            "Lazy avatar must have data-srcset, got: {}",
            output
        );
        assert!(
            output.contains("data-proofer-ignore=\"true\""),
            "Lazy avatar must have data-proofer-ignore, got: {}",
            output
        );
    }

    #[test]
    fn test_avatar_lazy_literal_user() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% avatar lituser size=60 lazy=true %}", &ctx)
            .unwrap();
        assert!(
            output.contains("src=\"\""),
            "Lazy avatar must have empty src"
        );
        assert!(
            output.contains("data-src=\""),
            "Lazy avatar must have data-src"
        );
        assert!(
            output.contains("data-srcset=\""),
            "Lazy avatar must have data-srcset"
        );
        assert!(output.contains("alt=\"lituser\""));
    }

    #[test]
    fn test_avatar_eager_default() {
        let eng = engine();
        let ctx = Object::new();
        let output = eng
            .parse_and_render("{% avatar eageruser %}", &ctx)
            .unwrap();
        assert!(
            output.contains("src=\"https://"),
            "Eager avatar must have src with URL"
        );
        assert!(
            output.contains("srcset=\""),
            "Eager avatar must have srcset"
        );
        assert!(
            !output.contains("data-src"),
            "Eager avatar must NOT have data-src"
        );
        assert!(
            !output.contains("data-srcset"),
            "Eager avatar must NOT have data-srcset"
        );
    }

    #[test]
    fn test_avatar_eager_explicit_no_lazy() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "u".into(),
            liquid::model::Value::scalar("eageru".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=u size=40 %}", &ctx)
            .unwrap();
        assert!(
            !output.contains("data-src"),
            "No lazy attrs without lazy=true"
        );
        assert!(
            !output.contains("data-srcset"),
            "No lazy attrs without lazy=true"
        );
    }

    // === Issue 513: alt correctness ===

    #[test]
    fn test_avatar_alt_never_param_name() {
        let eng = engine();
        let mut ctx = Object::new();
        ctx.insert(
            "org".into(),
            liquid::model::Value::scalar("realorg".to_string()),
        );
        let output = eng
            .parse_and_render("{% avatar user=org size=60 lazy=true %}", &ctx)
            .unwrap();
        assert!(output.contains("alt=\"realorg\""));
        assert!(!output.contains("alt=\"lazy\""));
        assert!(!output.contains("alt=\"size\""));
        assert!(!output.contains("alt=\"true\""));
    }
}

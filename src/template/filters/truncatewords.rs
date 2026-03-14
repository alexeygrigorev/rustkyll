use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct TruncatewordsArgs {
    #[parameter(description = "Maximum number of words.", arg_type = "integer")]
    max_words: Expression,
    #[parameter(
        description = "Ellipsis string appended when truncated (default '...').",
        arg_type = "str"
    )]
    ellipsis: Option<Expression>,
}

/// Truncate a string to a specified number of words.
///
/// Appends `...` by default (or a custom suffix) when the string is truncated.
/// If the input has fewer words than the limit, it is returned unchanged.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "truncatewords",
    description = "Truncate a string to a specified number of words.",
    parameters(TruncatewordsArgs),
    parsed(TruncatewordsFilter)
)]
pub struct Truncatewords;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "truncatewords"]
struct TruncatewordsFilter {
    #[parameters]
    args: TruncatewordsArgs,
}

impl Filter for TruncatewordsFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let max_words = args.max_words as usize;

        let text = input.to_kstr();
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.len() <= max_words {
            return Ok(Value::scalar(text.to_string()));
        }

        let ellipsis = args
            .ellipsis
            .map(|e| e.to_kstr().to_string())
            .unwrap_or_else(|| "...".to_string());

        let truncated: String = words[..max_words].join(" ");
        Ok(Value::scalar(format!("{truncated}{ellipsis}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_basic() {
        let result =
            liquid_core::call_filter!(Truncatewords, "one two three four five", 3i64).unwrap();
        assert_eq!(result.to_kstr(), "one two three...");
    }

    #[test]
    fn test_fewer_words_than_limit() {
        let result = liquid_core::call_filter!(Truncatewords, "one two", 5i64).unwrap();
        assert_eq!(result.to_kstr(), "one two");
    }

    #[test]
    fn test_empty_input() {
        let result = liquid_core::call_filter!(Truncatewords, "", 3i64).unwrap();
        assert_eq!(result.to_kstr(), "");
    }

    #[test]
    fn test_exact_word_count() {
        let result = liquid_core::call_filter!(Truncatewords, "one", 1i64).unwrap();
        assert_eq!(result.to_kstr(), "one");
    }

    #[test]
    fn test_extra_whitespace() {
        let result = liquid_core::call_filter!(Truncatewords, "  one  two  ", 1i64).unwrap();
        assert_eq!(result.to_kstr(), "one...");
    }
}

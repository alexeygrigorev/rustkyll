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

        let text = input.to_kstr();
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return Ok(Value::scalar(text.to_string()));
        }

        // Match Jekyll behavior exactly:
        //   l = [words - 1, 0].max
        //   wordlist.length > l ? wordlist[0..l].join(" ") + ellipsis : input
        // This means:
        //   - words=0 still shows 1 word (l clamped to 0, range 0..=0)
        //   - Ellipsis is appended even when word count equals the limit
        //   - Only when input has strictly fewer words than the limit is it returned unchanged
        let max_words = args.max_words;
        let l = if max_words <= 1 {
            0usize
        } else {
            (max_words - 1) as usize
        };

        if words.len() > l {
            let ellipsis = args
                .ellipsis
                .map(|e| e.to_kstr().to_string())
                .unwrap_or_else(|| "...".to_string());

            let truncated: String = words[..=l].join(" ");
            Ok(Value::scalar(format!("{truncated}{ellipsis}")))
        } else {
            Ok(Value::scalar(text.to_string()))
        }
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
        // Jekyll appends "..." even when word count equals the limit
        let result = liquid_core::call_filter!(Truncatewords, "one", 1i64).unwrap();
        assert_eq!(result.to_kstr(), "one...");
    }

    #[test]
    fn test_extra_whitespace() {
        let result = liquid_core::call_filter!(Truncatewords, "  one  two  ", 1i64).unwrap();
        assert_eq!(result.to_kstr(), "one...");
    }

    // Issue 168, Category 1: truncatewords must preserve dollar signs and
    // special characters within words. The description "a $500,000 grand prize"
    // truncated at 3 words should produce "a $500,000 grand..." not "a$500,000 grand..."
    #[test]
    fn test_issue168_truncate_preserves_dollar_sign() {
        let input = "competition with a $500,000 grand prize";
        let result = liquid_core::call_filter!(Truncatewords, input, 4i64).unwrap();
        assert_eq!(result.to_kstr(), "competition with a $500,000...");
    }

    #[test]
    fn test_issue168_truncate_split_whitespace_consistent() {
        // Ensure split_whitespace treats multiple spaces correctly
        let input = "word1  word2  word3  word4";
        let result = liquid_core::call_filter!(Truncatewords, input, 2i64).unwrap();
        assert_eq!(result.to_kstr(), "word1 word2...");
    }

    // Issue 169: Jekyll appends "..." even when word count exactly equals the limit.
    // Ruby: wordlist.length > (words - 1) means len >= words triggers truncation.
    #[test]
    fn test_issue169_exact_word_count_appends_ellipsis() {
        // Jekyll: "one two three" | truncatewords: 3 => "one two three..."
        let result = liquid_core::call_filter!(Truncatewords, "one two three", 3i64).unwrap();
        assert_eq!(result.to_kstr(), "one two three...");
    }

    #[test]
    fn test_issue169_single_word_with_limit_one() {
        // Jekyll: "one" | truncatewords: 1 => "one..."
        let result = liquid_core::call_filter!(Truncatewords, "one", 1i64).unwrap();
        assert_eq!(result.to_kstr(), "one...");
    }

    #[test]
    fn test_issue169_zero_words_shows_first_word() {
        // Jekyll: "one two three" | truncatewords: 0 => "one..."
        // Ruby clamps l = max(0, words-1), so wordlist[0..0] = first word
        let result = liquid_core::call_filter!(Truncatewords, "one two three", 0i64).unwrap();
        assert_eq!(result.to_kstr(), "one...");
    }

    #[test]
    fn test_issue169_fewer_words_no_ellipsis() {
        // Jekyll: "one two" | truncatewords: 5 => "one two" (no ellipsis when fewer words)
        let result = liquid_core::call_filter!(Truncatewords, "one two", 5i64).unwrap();
        assert_eq!(result.to_kstr(), "one two");
    }
}

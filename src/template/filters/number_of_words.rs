use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct NumberOfWordsArgs {
    #[parameter(
        description = "Word counting mode: 'auto' or 'cjk' for CJK-aware counting.",
        arg_type = "str"
    )]
    mode: Option<Expression>,
}

/// Count the number of words in a string.
///
/// Supports an optional mode argument matching Jekyll's `number_of_words` filter:
/// - No argument (default): split on whitespace, count segments
/// - `'auto'` or `'cjk'`: CJK-aware counting where each CJK character counts as
///   one word, and non-CJK text is split on whitespace as usual
/// - Unrecognized mode: falls back to default whitespace counting
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "number_of_words",
    description = "Count the number of words in a string.",
    parameters(NumberOfWordsArgs),
    parsed(NumberOfWordsFilter)
)]
pub struct NumberOfWords;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "number_of_words"]
struct NumberOfWordsFilter {
    #[parameters]
    args: NumberOfWordsArgs,
}

/// Returns true if the character is in a CJK Unicode range.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Unified Ideographs Extension A
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{20000}'..='\u{2A6DF}' // CJK Unified Ideographs Extension B+
    )
}

/// Count words with CJK awareness. Each CJK character counts as one word.
/// Non-CJK text segments are split on whitespace.
fn count_words_cjk(text: &str) -> i64 {
    let mut count: i64 = 0;
    let mut in_non_cjk_word = false;

    for c in text.chars() {
        if is_cjk(c) {
            // Each CJK character is one word
            if in_non_cjk_word {
                // End the current non-CJK word
                count += 1;
                in_non_cjk_word = false;
            }
            count += 1;
        } else if c.is_whitespace() {
            if in_non_cjk_word {
                count += 1;
                in_non_cjk_word = false;
            }
        } else {
            // Non-CJK, non-whitespace character
            in_non_cjk_word = true;
        }
    }

    // Don't forget the last non-CJK word
    if in_non_cjk_word {
        count += 1;
    }

    count
}

impl Filter for NumberOfWordsFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let text = input.to_kstr();

        let use_cjk = args.mode.as_ref().map_or(false, |m| {
            let mode_str = m.to_kstr();
            mode_str.as_ref() == "auto" || mode_str.as_ref() == "cjk"
        });

        let count = if use_cjk {
            count_words_cjk(&text)
        } else {
            text.split_whitespace().count() as i64
        };

        Ok(Value::scalar(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Existing tests (default mode, no argument) ---

    #[test]
    fn test_two_words() {
        let result = liquid_core::call_filter!(NumberOfWords, "Hello world").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }

    #[test]
    fn test_empty_string() {
        let result = liquid_core::call_filter!(NumberOfWords, "").unwrap();
        assert_eq!(result.to_kstr(), "0");
    }

    #[test]
    fn test_extra_whitespace() {
        let result =
            liquid_core::call_filter!(NumberOfWords, "  spaces  between  words  ").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_single_word() {
        let result = liquid_core::call_filter!(NumberOfWords, "single").unwrap();
        assert_eq!(result.to_kstr(), "1");
    }

    #[test]
    fn test_tabs_and_newlines() {
        let result = liquid_core::call_filter!(NumberOfWords, "one\ntwo\tthree").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    // --- CJK mode tests ('auto' and 'cjk') ---

    #[test]
    fn test_auto_mode_pure_latin() {
        let result = liquid_core::call_filter!(NumberOfWords, "Hello world", "auto").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }

    #[test]
    fn test_cjk_mode_pure_latin() {
        let result = liquid_core::call_filter!(NumberOfWords, "Hello world", "cjk").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }

    #[test]
    fn test_auto_mode_pure_chinese() {
        // 5 Chinese characters => 5 words
        let result = liquid_core::call_filter!(
            NumberOfWords,
            "\u{4f60}\u{597d}\u{4e16}\u{754c}\u{5427}",
            "auto"
        )
        .unwrap();
        assert_eq!(result.to_kstr(), "5");
    }

    #[test]
    fn test_auto_mode_pure_hiragana() {
        // 3 hiragana characters => 3 words
        let result =
            liquid_core::call_filter!(NumberOfWords, "\u{3042}\u{3044}\u{3046}", "auto").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_auto_mode_pure_hangul() {
        // 4 hangul syllables => 4 words
        let result =
            liquid_core::call_filter!(NumberOfWords, "\u{ac00}\u{b098}\u{b2e4}\u{b77c}", "auto")
                .unwrap();
        assert_eq!(result.to_kstr(), "4");
    }

    #[test]
    fn test_auto_mode_mixed_latin_cjk() {
        // "Hello 世界" => 1 Latin word + 2 CJK characters = 3
        let result =
            liquid_core::call_filter!(NumberOfWords, "Hello \u{4e16}\u{754c}", "auto").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_cjk_mode_mixed_content() {
        // "Hello 世界" with 'cjk' mode => same as 'auto'
        let result =
            liquid_core::call_filter!(NumberOfWords, "Hello \u{4e16}\u{754c}", "cjk").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_auto_mode_cjk_with_spaces() {
        // CJK characters separated by spaces: each CJK char still counts individually
        let result =
            liquid_core::call_filter!(NumberOfWords, "\u{4f60} \u{597d} \u{4e16}", "auto").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_auto_mode_empty_string() {
        let result = liquid_core::call_filter!(NumberOfWords, "", "auto").unwrap();
        assert_eq!(result.to_kstr(), "0");
    }

    #[test]
    fn test_auto_mode_whitespace_only() {
        let result = liquid_core::call_filter!(NumberOfWords, "   \t\n  ", "auto").unwrap();
        assert_eq!(result.to_kstr(), "0");
    }

    #[test]
    fn test_unrecognized_mode_falls_back_to_default() {
        // 'foo' is unrecognized, falls back to whitespace splitting
        // CJK characters without spaces between them count as one "word" in default mode
        let result =
            liquid_core::call_filter!(NumberOfWords, "Hello \u{4e16}\u{754c}", "foo").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }

    #[test]
    fn test_auto_mode_non_ascii_latin_not_cjk() {
        // Accented French/German text should NOT be treated as CJK
        // "\u{00e9}t\u{00e9} \u{00fc}ber caf\u{00e9}" = 3 words, not counted as CJK
        let result = liquid_core::call_filter!(
            NumberOfWords,
            "\u{00e9}t\u{00e9} \u{00fc}ber caf\u{00e9}",
            "auto"
        )
        .unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_auto_mode_latin_sentence_with_embedded_cjk() {
        // "I love \u{6771}\u{4eac} very much" => 4 Latin words + 2 CJK chars = 6
        let result =
            liquid_core::call_filter!(NumberOfWords, "I love \u{6771}\u{4eac} very much", "auto")
                .unwrap();
        assert_eq!(result.to_kstr(), "6");
    }

    #[test]
    fn test_auto_mode_katakana() {
        // 3 katakana characters => 3
        let result =
            liquid_core::call_filter!(NumberOfWords, "\u{30a2}\u{30a4}\u{30a6}", "auto").unwrap();
        assert_eq!(result.to_kstr(), "3");
    }

    #[test]
    fn test_default_mode_with_cjk_counts_by_whitespace() {
        // Default mode (no arg): CJK characters without spaces = 1 word
        let result = liquid_core::call_filter!(NumberOfWords, "Hello \u{4e16}\u{754c}").unwrap();
        assert_eq!(result.to_kstr(), "2");
    }
}

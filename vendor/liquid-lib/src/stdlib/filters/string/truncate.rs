#![allow(clippy::string_add)]

use std::cmp;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct TruncateArgs {
    #[parameter(
        description = "The maximum length of the string, after which it will be truncated.",
        arg_type = "integer"
    )]
    length: Option<Expression>,

    #[parameter(
        description = "The text appended to the end of the string if it is truncated. This text counts to the maximum length of the string. Defaults to \"...\".",
        arg_type = "str"
    )]
    ellipsis: Option<Expression>,
}

/// `truncate` shortens a string down to the number of characters passed as a parameter.
///
/// Note that this function counts Unicode codepoints (matching Ruby's `String#length`),
/// not bytes or grapheme clusters.
///
/// If the number of characters specified is less than the length of the string, an ellipsis
/// (`...`) is appended to the string and is included in the character count.
///
/// ## Custom ellipsis
///
/// `truncate` takes an optional second parameter that specifies the sequence of characters to be
/// appended to the truncated string. By default this is an ellipsis (`...`), but you can specify a
/// different sequence.
///
/// The length of the second parameter counts against the number of characters specified by the
/// first parameter. For example, if you want to truncate a string to exactly 10 characters, and
/// use a 3-character ellipsis, use 13 for the first parameter of `truncate`, since the ellipsis
/// counts as 3 characters.
///
/// ## No ellipsis
///
/// You can truncate to the exact number of characters specified by the first parameter and show no
/// trailing characters by passing a blank string as the second parameter.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "truncate",
    description = "Shortens a string down to the number of characters passed as a parameter.",
    parameters(TruncateArgs),
    parsed(TruncateFilter)
)]
pub struct Truncate;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "truncate"]
struct TruncateFilter {
    #[parameters]
    args: TruncateArgs,
}

impl Filter for TruncateFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let length = args.length.unwrap_or(50) as usize;
        let truncate_string = args.ellipsis.unwrap_or_else(|| "...".into());
        // Issue 297: Count characters (codepoints) not bytes, matching Ruby.
        let ellipsis_char_len = truncate_string.chars().count();
        let l = length.saturating_sub(ellipsis_char_len);

        let input_string = input.to_kstr();
        // Issue 297: Count characters (codepoints) not bytes for input length,
        // matching Ruby's String#length. CJK chars are 1 codepoint each.
        let input_char_len = input_string.chars().count();
        let result = if length < input_char_len {
            let result: String =
                input_string.chars().take(l).collect::<String>() + truncate_string.as_str();
            Value::scalar(result)
        } else {
            input.to_value()
        };
        Ok(result)
    }
}

#[derive(Debug, FilterParameters)]
struct TruncateWordsArgs {
    #[parameter(
        description = "The maximum number of words, after which the string will be truncated.",
        arg_type = "integer"
    )]
    length: Option<Expression>,

    #[parameter(
        description = "The text appended to the end of the string if it is truncated. This text counts to the maximum word-count of the string. Defaults to \"...\".",
        arg_type = "str"
    )]
    ellipsis: Option<Expression>,
}

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "truncatewords",
    description = "Shortens a string down to the number of characters passed as a parameter.",
    parameters(TruncateWordsArgs),
    parsed(TruncateWordsFilter)
)]
pub struct TruncateWords;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "truncate"]
struct TruncateWordsFilter {
    #[parameters]
    args: TruncateWordsArgs,
}

impl Filter for TruncateWordsFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let words = args.length.unwrap_or(50) as usize;

        let truncate_string = args.ellipsis.unwrap_or_else(|| "...".into());

        let l = cmp::max(words, 0);

        let input_string = input.to_kstr();

        let word_list: Vec<&str> = input_string.split(' ').collect();
        let result = if words < word_list.len() {
            let result = itertools::join(word_list.iter().take(l), " ") + truncate_string.as_str();
            Value::scalar(result)
        } else {
            input.to_value()
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_truncate() {
        assert_eq!(
            liquid_core::call_filter!(
                Truncate,
                "I often quote myself.  It adds spice to my conversation.",
                17i64
            )
            .unwrap(),
            liquid_core::value!("I often quote ...")
        );
    }

    #[test]
    fn unit_truncate_negative_length() {
        assert_eq!(
            liquid_core::call_filter!(
                Truncate,
                "I often quote myself.  It adds spice to my conversation.",
                -17i64
            )
            .unwrap(),
            liquid_core::value!("I often quote myself.  It adds spice to my conversation.")
        );
    }

    #[test]
    fn unit_truncate_non_string() {
        assert_eq!(
            liquid_core::call_filter!(Truncate, 10000000f64, 5i64).unwrap(),
            liquid_core::value!("10...")
        );
    }

    #[test]
    fn unit_truncate_shopify_liquid() {
        // Tests from https://shopify.github.io/liquid/filters/truncate/
        let input = "Ground control to Major Tom.";

        assert_eq!(
            liquid_core::call_filter!(Truncate, input, 20i64).unwrap(),
            liquid_core::value!("Ground control to...")
        );

        assert_eq!(
            liquid_core::call_filter!(Truncate, input, 25i64, ", and so on").unwrap(),
            liquid_core::value!("Ground control, and so on")
        );

        assert_eq!(
            liquid_core::call_filter!(Truncate, input, 20i64, "").unwrap(),
            liquid_core::value!("Ground control to Ma")
        );
    }

    #[test]
    fn unit_truncate_three_arguments() {
        liquid_core::call_filter!(
            Truncate,
            "I often quote myself.  It adds spice to my conversation.",
            17i64,
            "...",
            0i64
        )
        .unwrap_err();
    }

    #[test]
    fn unit_truncate_unicode_codepoints_examples() {
        // Issue 297: truncate counts Unicode codepoints (matching Ruby's String#length).
        // Combining characters count as separate codepoints.
        //
        // "Here is an a\u{310}, e\u{301}, and o\u{308}\u{332}." = 27 codepoints
        // truncate: 20 -> keep 17 codepoints + "..."
        assert_eq!(
            liquid_core::call_filter!(
                Truncate,
                "Here is an a\u{310}, e\u{301}, and o\u{308}\u{332}.",
                20i64
            )
            .unwrap(),
            liquid_core::value!("Here is an a\u{310}, e\u{301}...")
        );

        // "Here is a RUST: \u{1F1F7}\u{1F1FA}\u{1F1F8}\u{1F1F9}." = 21 codepoints
        // truncate: 20 -> keep 17 codepoints + "..."
        assert_eq!(
            liquid_core::call_filter!(
                Truncate,
                "Here is a RUST: \u{1F1F7}\u{1F1FA}\u{1F1F8}\u{1F1F9}.",
                20i64
            )
            .unwrap(),
            liquid_core::value!("Here is a RUST: \u{1F1F7}...")
        );
    }

    #[test]
    fn unit_truncate_zero_arguments() {
        assert_eq!(
            liquid_core::call_filter!(
                Truncate,
                "I often quote myself.  It adds spice to my conversation."
            )
            .unwrap(),
            liquid_core::value!("I often quote myself.  It adds spice to my conv...")
        );
    }

    #[test]
    fn unit_truncatewords_negative_length() {
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "one two three", -1_i64).unwrap(),
            liquid_core::value!("one two three")
        );
    }

    #[test]
    fn unit_truncatewords_zero_length() {
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "one two three", 0_i64).unwrap(),
            liquid_core::value!("...")
        );
    }

    #[test]
    fn unit_truncatewords_no_truncation() {
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "one two three", 4_i64).unwrap(),
            liquid_core::value!("one two three")
        );
    }

    #[test]
    fn unit_truncatewords_truncate() {
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "one two three", 2_i64).unwrap(),
            liquid_core::value!("one two...")
        );
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "one two three", 2_i64, 1_i64).unwrap(),
            liquid_core::value!("one two1")
        );
    }

    #[test]
    fn unit_truncatewords_empty_string() {
        assert_eq!(
            liquid_core::call_filter!(TruncateWords, "", 1_i64).unwrap(),
            liquid_core::value!("")
        );
    }

    // Issue 297: CJK/Unicode character counting tests

    #[test]
    fn unit_truncate_cjk_counts_chars_not_bytes() {
        // 81 CJK characters = 243 bytes in UTF-8, but only 81 codepoints.
        // Ruby's truncate counts characters (codepoints), so truncate: 240 should NOT truncate.
        let cjk_81: String = std::iter::repeat('\u{4E16}').take(81).collect();
        assert_eq!(cjk_81.len(), 243); // 243 bytes
        assert_eq!(cjk_81.chars().count(), 81); // 81 chars

        let result = liquid_core::call_filter!(Truncate, cjk_81.as_str(), 240_i64).unwrap();
        // Should NOT be truncated (81 chars < 240 limit)
        assert_eq!(
            result,
            liquid_core::value!(cjk_81.as_str()),
            "CJK string of 81 chars (243 bytes) should NOT be truncated at limit 240"
        );
    }

    #[test]
    fn unit_truncate_241_ascii_chars() {
        // 241 ASCII characters should be truncated to 237 + "..."
        let input: String = "a".repeat(241);
        let result = liquid_core::call_filter!(Truncate, input.as_str(), 240_i64).unwrap();
        let expected = format!("{}{}", "a".repeat(237), "...");
        assert_eq!(result, liquid_core::value!(expected.as_str()));
    }

    #[test]
    fn unit_truncate_240_ascii_chars_no_truncation() {
        // Exactly 240 ASCII characters should NOT be truncated
        let input: String = "a".repeat(240);
        let result = liquid_core::call_filter!(Truncate, input.as_str(), 240_i64).unwrap();
        assert_eq!(
            result,
            liquid_core::value!(input.as_str()),
            "Exactly 240 chars should not be truncated"
        );
    }

    #[test]
    fn unit_truncate_mixed_ascii_cjk_counts_codepoints() {
        // 200 ASCII + 41 CJK = 241 codepoints. Should truncate.
        let input: String = "a".repeat(200) + &"\u{4E16}".repeat(41);
        assert_eq!(input.chars().count(), 241);
        let result = liquid_core::call_filter!(Truncate, input.as_str(), 240_i64).unwrap();
        // Should be truncated to 237 codepoints + "..."
        let expected_prefix: String = input.chars().take(237).collect::<String>();
        let expected = format!("{}...", expected_prefix);
        assert_eq!(result, liquid_core::value!(expected.as_str()));
    }
}

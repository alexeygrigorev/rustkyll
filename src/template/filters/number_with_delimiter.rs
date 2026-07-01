use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct NumberWithDelimiterArgs {
    #[parameter(
        description = "Thousands delimiter inserted between digit groups (default ',').",
        arg_type = "str"
    )]
    delimiter: Option<Expression>,
    #[parameter(
        description = "Decimal separator splitting integer and fractional parts (default '.').",
        arg_type = "str"
    )]
    separator: Option<Expression>,
}

/// Format a number with thousands delimiters.
///
/// Mirrors Jekyll's `number_with_delimiter` (inherited from ActiveSupport):
/// groups the integer part into triplets of digits separated by `delimiter`
/// (default `,`), leaving the fractional part untouched. The decimal separator
/// defaults to `.`.
///
/// Usage: `{{ 9700 | number_with_delimiter: "," }}` renders `9,700`.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "number_with_delimiter",
    description = "Format a number with thousands delimiters.",
    parameters(NumberWithDelimiterArgs),
    parsed(NumberWithDelimiterFilter)
)]
pub struct NumberWithDelimiter;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "number_with_delimiter"]
struct NumberWithDelimiterFilter {
    #[parameters]
    args: NumberWithDelimiterArgs,
}

impl Filter for NumberWithDelimiterFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let delimiter = args
            .delimiter
            .map(|d| d.to_kstr().to_string())
            .unwrap_or_else(|| ",".to_string());
        let separator = args
            .separator
            .map(|s| s.to_kstr().to_string())
            .unwrap_or_else(|| ".".to_string());

        // ActiveSupport calls `number.to_s`; `to_kstr` is the equivalent rendering
        // of the Liquid value (integers, floats, and numeric strings alike).
        let number_str = input.to_kstr().to_string();
        Ok(Value::scalar(format_with_delimiter(
            &number_str,
            &delimiter,
            &separator,
        )))
    }
}

/// Insert `delimiter` between every three digits of a number's integer part.
///
/// Mirrors ActiveSupport's classic algorithm: split on `separator`, group the
/// integer part (`parts[0]`) only, then rejoin. The fractional part, if any,
/// passes through unchanged.
fn format_with_delimiter(number_str: &str, delimiter: &str, separator: &str) -> String {
    // ActiveSupport splits on the separator argument (default ".").
    let (int_part, frac_part) = match number_str.split_once(separator) {
        Some((int, frac)) => (int, Some(frac)),
        None => (number_str, None),
    };

    let grouped = group_thousands(int_part, delimiter);
    match frac_part {
        Some(frac) => format!("{grouped}{separator}{frac}"),
        None => grouped,
    }
}

/// Group the digits of `s` into triplets separated by `delimiter`.
///
/// Replicates ActiveSupport's `gsub(/(\d)(?=(\d{3})+(?!\d))/)`: a delimiter is
/// inserted after a digit when the *contiguous* run of digits that immediately
/// follows it has a length that is a positive multiple of three. A leading sign
/// (`+`/`-`) and any non-digit characters pass through untouched.
fn group_thousands(s: &str, delimiter: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    if n == 0 {
        return String::new();
    }
    let mut out = String::with_capacity(s.len() + (n / 3) * delimiter.len() + delimiter.len());

    // A leading sign (+/-) is not a digit and is left in place by the regex.
    let start = match chars[0] {
        '+' | '-' => 1,
        _ => 0,
    };

    for i in 0..n {
        out.push(chars[i]);
        if i < start || !chars[i].is_ascii_digit() {
            continue;
        }
        // Count the contiguous ASCII digits immediately following position i.
        let mut j = i + 1;
        while j < n && chars[j].is_ascii_digit() {
            j += 1;
        }
        let following = j - (i + 1);
        if following > 0 && following % 3 == 0 {
            out.push_str(delimiter);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::engine::TemplateEngine;
    use liquid::Object;

    fn render(template: &str, ctx: &Object) -> String {
        let engine = TemplateEngine::new().unwrap();
        engine.parse_and_render(template, ctx).unwrap()
    }

    // --- end-to-end (matches the issue repro) ---

    #[test]
    fn test_issue_repro_with_delimiter_arg() {
        let ctx = Object::new();
        assert_eq!(
            render("{{ 9700 | number_with_delimiter: \",\" }}", &ctx),
            "9,700"
        );
    }

    #[test]
    fn test_default_delimiter_no_args() {
        // No args: delimiter defaults to ",".
        let ctx = Object::new();
        assert_eq!(
            render("{{ 1234567 | number_with_delimiter }}", &ctx),
            "1,234,567"
        );
    }

    #[test]
    fn test_from_variable() {
        let mut ctx = Object::new();
        ctx.insert("stars".into(), liquid::model::Value::scalar(9700i64));
        assert_eq!(render("{{ stars | number_with_delimiter }}", &ctx), "9,700");
    }

    #[test]
    fn test_string_numeric_input() {
        let mut ctx = Object::new();
        ctx.insert("n".into(), liquid::model::Value::scalar("9700"));
        assert_eq!(render("{{ n | number_with_delimiter }}", &ctx), "9,700");
    }

    #[test]
    fn test_custom_delimiter() {
        let ctx = Object::new();
        assert_eq!(
            render("{{ 1234567 | number_with_delimiter: \".\" }}", &ctx),
            "1.234.567"
        );
    }

    #[test]
    fn test_negative_number() {
        let ctx = Object::new();
        assert_eq!(
            render("{{ -1234567 | number_with_delimiter }}", &ctx),
            "-1,234,567"
        );
    }

    #[test]
    fn test_float_keeps_fraction() {
        let ctx = Object::new();
        // Fractional part is not grouped.
        assert_eq!(
            render("{{ 1234567.89 | number_with_delimiter }}", &ctx),
            "1,234,567.89"
        );
    }

    #[test]
    fn test_three_digits_unchanged() {
        let ctx = Object::new();
        assert_eq!(render("{{ 999 | number_with_delimiter }}", &ctx), "999");
    }

    #[test]
    fn test_zero_unchanged() {
        let ctx = Object::new();
        assert_eq!(render("{{ 0 | number_with_delimiter }}", &ctx), "0");
    }

    #[test]
    fn test_non_numeric_string_passthrough() {
        // ActiveSupport falls back to the input string when it can't be parsed
        // as a number; here it simply has no grouped digits.
        let mut ctx = Object::new();
        ctx.insert("n".into(), liquid::model::Value::scalar("n/a"));
        assert_eq!(render("{{ n | number_with_delimiter }}", &ctx), "n/a");
    }

    // --- call_filter unit tests ---

    #[test]
    fn test_filter_basic() {
        let result = liquid_core::call_filter!(NumberWithDelimiter, 9700i64).unwrap();
        assert_eq!(result.to_kstr(), "9,700");
    }

    #[test]
    fn test_filter_millions() {
        let result = liquid_core::call_filter!(NumberWithDelimiter, 1234567i64).unwrap();
        assert_eq!(result.to_kstr(), "1,234,567");
    }

    #[test]
    fn test_filter_custom_delimiter_and_separator() {
        // European style: input uses ',' as its decimal separator, so we group
        // with '.' as the thousands delimiter and ',' as the separator.
        // ActiveSupport splits on the separator argument, so the input must use
        // the same separator character.
        let result = liquid_core::call_filter!(NumberWithDelimiter, "1234,5", ".", ",").unwrap();
        assert_eq!(result.to_kstr(), "1.234,5");
    }
}

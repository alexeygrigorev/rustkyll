use std::cmp;

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

/// Custom `slice` filter that uses CHARACTER count consistently.
///
/// The liquid-lib stdlib `slice` filter mixes byte count (`str::len()`) for
/// bounds-checking with character-based iteration (`chars().skip().take()`).
/// For multi-byte UTF-8 content (e.g., em dash U+2014), this causes incorrect
/// slicing because the byte-based bounds are larger than the character count.
///
/// This filter uses `chars().count()` for all length calculations, matching
/// Ruby/Jekyll behavior where string indexing is character-based.
#[derive(Debug, FilterParameters)]
struct SliceArgs {
    #[parameter(description = "The offset of the slice.", arg_type = "integer")]
    offset: Expression,

    #[parameter(description = "The length of the slice.", arg_type = "integer")]
    length: Option<Expression>,
}

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "slice",
    description = "Takes a slice of a given string or array, using character-based indexing for strings.",
    parameters(SliceArgs),
    parsed(SliceFilter)
)]
pub struct Slice;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "slice"]
struct SliceFilter {
    #[parameters]
    args: SliceArgs,
}

fn canonicalize_slice(
    slice_offset: isize,
    slice_length: isize,
    vec_length: usize,
) -> (usize, usize) {
    let vec_length = vec_length as isize;

    // Cap slice_offset
    let slice_offset = cmp::min(slice_offset, vec_length);
    // Reverse indexing
    let slice_offset = if slice_offset < 0 {
        cmp::max(0, slice_offset + vec_length)
    } else {
        slice_offset
    };

    // Cap slice_length
    let slice_length = if slice_offset + slice_length > vec_length {
        vec_length - slice_offset
    } else {
        slice_length
    };

    (slice_offset as usize, slice_length as usize)
}

impl Filter for SliceFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;

        let offset = args.offset as isize;
        let length = args.length.unwrap_or(1) as isize;

        if length < 1 {
            return Ok(Value::scalar(""));
        }

        if let Some(input) = input.as_array() {
            let (offset, length) = canonicalize_slice(offset, length, input.size() as usize);
            Ok(Value::array(
                input
                    .values()
                    .skip(offset)
                    .take(length)
                    .map(|s| s.to_value()),
            ))
        } else {
            let input = input.to_kstr();
            // Use CHARACTER count, not byte count, for consistent behavior
            let char_count = input.chars().count();
            let (offset, length) = canonicalize_slice(offset, length, char_count);
            Ok(Value::scalar(
                input.chars().skip(offset).take(length).collect::<String>(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_517_slice_ascii() {
        let result = liquid_core::call_filter!(Slice, "I often quote myself.", 2, 3).unwrap();
        assert_eq!(result, liquid_core::value!("oft"));
    }

    #[test]
    fn test_517_slice_unicode_em_dash() {
        // "H2 — heading" has em dash at position 3 (0-indexed)
        // Slicing from position 3 for 1 char should give "—"
        let result = liquid_core::call_filter!(Slice, "H2 — heading", 3, 1).unwrap();
        assert_eq!(result, liquid_core::value!("—"));
    }

    #[test]
    fn test_517_slice_after_unicode() {
        // After em dash, "heading" starts at position 5
        let result = liquid_core::call_filter!(Slice, "H2 — heading", 5, 7).unwrap();
        assert_eq!(result, liquid_core::value!("heading"));
    }

    #[test]
    fn test_517_slice_chinese_chars() {
        let result = liquid_core::call_filter!(Slice, "你好世界test", 2, 3).unwrap();
        assert_eq!(result, liquid_core::value!("世界t"));
    }

    #[test]
    fn test_517_slice_negative_offset() {
        let result = liquid_core::call_filter!(Slice, "hello", -3, 2).unwrap();
        assert_eq!(result, liquid_core::value!("ll"));
    }
}

use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{Value, ValueView};

/// Custom `size` filter that returns CHARACTER count for strings, not byte count.
///
/// The liquid-lib stdlib `size` filter uses Rust's `str::len()` which returns bytes.
/// Ruby/Jekyll's `size` returns character count. For ASCII-only strings these are the
/// same, but for multi-byte UTF-8 content (e.g., em dash U+2014), byte count is higher.
///
/// This matters for Liquid templates that use `size` + `slice` together, because
/// `slice` operates on characters (via `chars().skip().take()`). If `size` returns
/// bytes but `slice` uses characters, the offset is wrong for non-ASCII content.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "size",
    description = "Returns the size of the input. For strings, returns character count (not byte count). For arrays/objects, returns element count.",
    parsed(SizeFilter)
)]
pub struct Size;

#[derive(Debug, Default, Display_filter)]
#[name = "size"]
struct SizeFilter;

impl Filter for SizeFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        if let Some(x) = input.as_scalar() {
            // Return CHARACTER count, not byte count
            Ok(Value::scalar(x.to_kstr().chars().count() as i64))
        } else if let Some(x) = input.as_array() {
            Ok(Value::scalar(x.size()))
        } else if let Some(x) = input.as_object() {
            Ok(Value::scalar(x.size()))
        } else {
            Ok(Value::scalar(0i64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_517_size_ascii_string() {
        let result = liquid_core::call_filter!(Size, "hello").unwrap();
        assert_eq!(result, Value::scalar(5i64));
    }

    #[test]
    fn test_517_size_unicode_em_dash() {
        // Em dash (U+2014) is 3 bytes in UTF-8 but 1 character
        let result = liquid_core::call_filter!(Size, "H2 — heading").unwrap();
        // "H2 — heading" has 12 characters (H,2, ,—, ,h,e,a,d,i,n,g)
        assert_eq!(result, Value::scalar(12i64));
    }

    #[test]
    fn test_517_size_unicode_chinese() {
        // Chinese characters are 3 bytes each
        let result = liquid_core::call_filter!(Size, "你好世界").unwrap();
        assert_eq!(result, Value::scalar(4i64));
    }

    #[test]
    fn test_517_size_array() {
        let result = liquid_core::call_filter!(Size, liquid_core::value!([1, 2, 3])).unwrap();
        assert_eq!(result, Value::scalar(3i64));
    }
}

use liquid_core::Expression;
use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{
    Display_filter, Filter, FilterParameters, FilterReflection, FromFilterParameters, ParseFilter,
};
use liquid_core::{Value, ValueView};

#[derive(Debug, FilterParameters)]
struct MapArgs {
    #[parameter(description = "The property name to map.", arg_type = "str")]
    property: Expression,
}

/// Map filter that flattens nested arrays, matching Jekyll/Ruby behavior.
///
/// In Jekyll (Ruby), `array.map(&:tags)` followed by `flatten` produces a flat
/// array when each item's property is itself an array. The liquid crate's
/// built-in `map` does not flatten, producing an array of arrays instead.
///
/// This custom filter maps each item's property and then flattens the result
/// if any of the mapped values are arrays. For scalar properties, it behaves
/// identically to the built-in map.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "map",
    description = "Creates an array of values by extracting the values of a named property from another object. Flattens nested arrays to match Jekyll behavior.",
    parameters(MapArgs),
    parsed(MapFilter)
)]
pub struct Map;

#[derive(Debug, FromFilterParameters, Display_filter)]
#[name = "map"]
struct MapFilter {
    #[parameters]
    args: MapArgs,
}

impl Filter for MapFilter {
    fn evaluate(&self, input: &dyn ValueView, runtime: &dyn Runtime) -> Result<Value> {
        let args = self.args.evaluate(runtime)?;
        let property = args.property.to_kstr();

        let array = match input.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Nil),
        };

        let mut result = Vec::new();
        let mut has_nested_arrays = false;

        // First pass: collect mapped values and detect nested arrays
        let mapped: Vec<Value> = array
            .values()
            .map(|item| {
                if let Some(obj) = item.as_object() {
                    if let Some(val) = obj.get(property.as_str()) {
                        if val.as_array().is_some() {
                            has_nested_arrays = true;
                        }
                        val.to_value()
                    } else {
                        Value::Nil
                    }
                } else {
                    Value::Nil
                }
            })
            .collect();

        if has_nested_arrays {
            // Flatten: if any mapped value is an array, expand it inline
            for val in mapped {
                if let Value::Array(inner) = val {
                    result.extend(inner);
                } else {
                    result.push(val);
                }
            }
        } else {
            result = mapped;
        }

        Ok(Value::Array(result))
    }
}

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

/// Map filter that extracts a named property from each element of an array.
///
/// Matches Jekyll/Ruby Liquid behavior: `map` does NOT auto-flatten nested
/// arrays. If each item's property is an array, the result is an array of
/// arrays. Use `| flatten` (or `| compact`) after `map` to flatten explicitly.
///
/// This is important for patterns like `group_by: "parent" | map: "items" | first`
/// used by the just-the-docs theme, where each group's `items` is an array
/// that must be preserved intact.
#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "map",
    description = "Creates an array of values by extracting the values of a named property from another object.",
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

        let result: Vec<Value> = array
            .values()
            .map(|item| {
                if let Some(obj) = item.as_object() {
                    if let Some(val) = obj.get(property.as_str()) {
                        val.to_value()
                    } else {
                        Value::Nil
                    }
                } else {
                    Value::Nil
                }
            })
            .collect();

        Ok(Value::Array(result))
    }
}

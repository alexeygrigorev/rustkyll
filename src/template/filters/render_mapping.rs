use liquid_core::Result;
use liquid_core::Runtime;
use liquid_core::{Display_filter, Filter, FilterReflection, ParseFilter};
use liquid_core::{ObjectView, Value, ValueView};

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "render_mapping",
    description = "Render YAML mapping values like Jekyll/Ruby hash strings.",
    parsed(RenderMappingFilter)
)]
pub struct RenderMapping;

#[derive(Debug, Default, Display_filter)]
#[name = "render_mapping"]
struct RenderMappingFilter;

fn render_value_like_ruby_hash(value: &dyn ValueView) -> String {
    if let Some(obj) = value.as_object() {
        return render_object_like_ruby_hash(obj);
    }
    if let Some(arr) = value.as_array() {
        let items: Vec<String> = arr.values().map(render_value_like_ruby_hash).collect();
        return format!("[{}]", items.join(", "));
    }
    if value.is_nil() {
        return "nil".to_string();
    }
    serde_json::to_string(&value.to_kstr().to_string())
        .unwrap_or_else(|_| format!("{:?}", value.to_kstr().to_string()))
}

fn render_object_like_ruby_hash(obj: &dyn ObjectView) -> String {
    use crate::template::filters::jsonify::KEY_ORDER_FIELD;

    let key_order: Option<Vec<String>> = obj.get(KEY_ORDER_FIELD).and_then(|v| {
        v.as_array().map(|arr| {
            arr.values()
                .map(|item| item.to_kstr().to_string())
                .collect::<Vec<_>>()
        })
    });

    let mut keys: Vec<String> = if let Some(ordered_keys) = key_order {
        let mut seen = std::collections::HashSet::new();
        let mut ordered = Vec::new();
        for key in ordered_keys {
            if key == KEY_ORDER_FIELD || !seen.insert(key.clone()) {
                continue;
            }
            ordered.push(key);
        }

        let mut remaining: Vec<String> = obj
            .keys()
            .map(|k| k.to_string())
            .filter(|k| k != KEY_ORDER_FIELD && !seen.contains(k))
            .collect();
        remaining.sort();
        ordered.extend(remaining);
        ordered
    } else {
        let mut keys: Vec<String> = obj
            .keys()
            .map(|k| k.to_string())
            .filter(|k| k != KEY_ORDER_FIELD)
            .collect();
        keys.sort();
        keys
    };

    let parts: Vec<String> = keys
        .drain(..)
        .filter_map(|key| {
            obj.get(key.as_str()).map(|val| {
                let key_str = serde_json::to_string(&key).unwrap_or_else(|_| format!("{key:?}"));
                let value_str = render_value_like_ruby_hash(val);
                format!("{key_str}=>{value_str}")
            })
        })
        .collect();

    format!("{{{}}}", parts.join(", "))
}

impl Filter for RenderMappingFilter {
    fn evaluate(&self, input: &dyn ValueView, _runtime: &dyn Runtime) -> Result<Value> {
        if input.as_object().is_some() {
            Ok(Value::scalar(render_value_like_ruby_hash(input)))
        } else {
            Ok(input.to_value())
        }
    }
}

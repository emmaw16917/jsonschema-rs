use serde_json::Value;
use std::collections::HashSet;

/// Traversal helpers for JSON instances.
///
/// These mirror utility patterns found in Python `jsonschema._utils`.

/// Collect the set of property keys that have been "covered" by
/// `properties`, `patternProperties`, or other schema-level declarations,
/// so that `additionalProperties` / `unevaluatedProperties` can detect
/// extras.
pub fn known_property_keys(
    instance: &Value,
    properties: Option<&Value>,
    pattern_properties: Option<&Value>,
) -> HashSet<String> {
    let mut known: HashSet<String> = HashSet::new();

    if let Some(Value::Object(props)) = properties {
        for key in props.keys() {
            known.insert(key.clone());
        }
    }

    if let Some(Value::Object(patterns)) = pattern_properties {
        // We can't know which *instance* keys match a pattern until runtime,
        // so we don't pre-fill here.  The caller must add keys that actually
        // matched one of the patterns after the fact.
        let _ = patterns; // consumed at runtime in additionalProperties
    }

    if let Value::Object(obj) = instance {
        // Start pessimistic — assume nothing is known unless declared above.
        // We'll let the keyword implementation subtract.
        let _ = obj;
    }

    known
}

/// Returns `true` if `a` and `b` are deeply equal, handling `f64` NaN
/// edge-cases (JSON forbids NaN, but defensive code handles it).
pub fn deep_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            // Both are numbers — compare via f64 for consistency.
            match (na.as_f64(), nb.as_f64()) {
                (Some(fa), Some(fb)) => {
                    if fa.is_nan() && fb.is_nan() {
                        true // JSON doesn't produce NaN, but be defensive.
                    } else {
                        (fa - fb).abs() < f64::EPSILON
                    }
                }
                _ => a == b,
            }
        }
        _ => a == b,
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_equal_numbers() {
        assert!(deep_equal(&serde_json::json!(1), &serde_json::json!(1)));
        assert!(deep_equal(&serde_json::json!(3.14), &serde_json::json!(3.14)));
        assert!(!deep_equal(&serde_json::json!(1), &serde_json::json!(2)));
    }

    #[test]
    fn test_deep_equal_mixed() {
        assert!(deep_equal(&Value::Null, &Value::Null));
        assert!(!deep_equal(&Value::Null, &serde_json::json!(0)));
        assert!(deep_equal(
            &Value::String("hi".into()),
            &Value::String("hi".into())
        ));
    }

    #[test]
    fn test_known_property_keys() {
        let props = serde_json::json!({"name": {}, "age": {}});
        let instance = serde_json::json!({"name": "Alice", "age": 30, "extra": true});
        let known = known_property_keys(&instance, Some(&props), None);
        assert!(known.contains("name"));
        assert!(known.contains("age"));
        assert!(!known.contains("extra"));
    }
}

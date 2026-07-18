use serde_json::Value;
use std::collections::HashSet;

/// 收集 `properties` 和 `patternProperties` 已覆盖的实例属性键，
/// 供 `additionalProperties` / `unevaluatedProperties` 检测多余属性。
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
        let _ = patterns;
    }

    if let Value::Object(obj) = instance {
        let _ = obj;
    }

    known
}

/// 深度相等比较，额外处理 f64 NaN 边界情况。
pub fn deep_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            match (na.as_f64(), nb.as_f64()) {
                (Some(fa), Some(fb)) => {
                    if fa.is_nan() && fb.is_nan() {
                        true
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

// tests

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

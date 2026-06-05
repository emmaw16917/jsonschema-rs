use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Pre-compiled schema, holding metadata extracted during compilation.
#[derive(Debug, Clone)]
pub struct CompiledSchema {
    /// The original JSON Schema value.
    pub raw: Value,
    /// Pre-compiled `pattern` / `patternProperties` regexes, keyed by their
    /// source string.
    pub precompiled_patterns: HashMap<String, Regex>,
}

impl CompiledSchema {
    pub fn new(raw: Value, patterns: HashMap<String, Regex>) -> Self {
        Self {
            raw,
            precompiled_patterns: patterns,
        }
    }
}

/// Alias — both schemas and instances are plain JSON values.
pub type Instance = Value;

// ---------------------------------------------------------------------------
// TypeChecker
// ---------------------------------------------------------------------------

/// Maps JSON Schema type names (e.g. `"string"`, `"integer"`) to predicate
/// functions that decide whether a given `Value` satisfies that type.
///
/// Mirrors Python `jsonschema._types.TypeChecker`.
pub struct TypeChecker {
    types: HashMap<String, Box<dyn Fn(&Value) -> bool + Send + Sync>>,
}

impl TypeChecker {
    /// Build the default type checker with the seven JSON Schema primitive
    /// types.
    pub fn default() -> Self {
        let mut types: HashMap<String, Box<dyn Fn(&Value) -> bool + Send + Sync>> = HashMap::new();

        types.insert("null".into(), Box::new(|v| v.is_null()));
        types.insert("boolean".into(), Box::new(|v| v.is_boolean()));
        types.insert("object".into(), Box::new(|v| v.is_object()));
        types.insert("array".into(), Box::new(|v| v.is_array()));
        types.insert("string".into(), Box::new(|v| v.is_string()));

        // "number" includes both integers and floats.
        types.insert(
            "number".into(),
            Box::new(|v: &Value| v.is_number()),
        );

        // "integer": must be an i64, or an f64 with zero fractional part.
        types.insert(
            "integer".into(),
            Box::new(|v: &Value| match v {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        let _ = i;
                        true
                    } else if let Some(f) = n.as_f64() {
                        f.fract() == 0.0
                    } else {
                        false
                    }
                }
                _ => false,
            }),
        );

        Self { types }
    }

    /// Register a custom type predicate.
    pub fn register(
        &mut self,
        name: &str,
        predicate: Box<dyn Fn(&Value) -> bool + Send + Sync>,
    ) {
        self.types.insert(name.to_string(), predicate);
    }

    /// Returns `true` if `instance` satisfies the JSON Schema type `type_name`.
    pub fn is_type(&self, instance: &Value, type_name: &str) -> bool {
        self.types
            .get(type_name)
            .map(|f| f(instance))
            .unwrap_or(false)
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_types() {
        let tc = TypeChecker::default();
        assert!(tc.is_type(&Value::Null, "null"));
        assert!(tc.is_type(&Value::Bool(true), "boolean"));
        assert!(tc.is_type(&serde_json::json!({}), "object"));
        assert!(tc.is_type(&serde_json::json!([]), "array"));
        assert!(tc.is_type(&Value::String("hi".into()), "string"));
        assert!(tc.is_type(&serde_json::json!(42), "integer"));
        assert!(tc.is_type(&serde_json::json!(3.14), "number"));
        assert!(tc.is_type(&serde_json::json!(3.0), "integer"));
        assert!(!tc.is_type(&serde_json::json!(3.14), "integer"));
        assert!(!tc.is_type(&Value::Null, "string"));
    }
}

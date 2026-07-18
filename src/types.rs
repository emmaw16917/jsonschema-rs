use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompiledSchema {
    pub raw: Value,
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

pub type Instance = Value;

// TypeChecker

//类型名到类型判断函数的映射。
pub struct TypeChecker {
    types: HashMap<String, Box<dyn Fn(&Value) -> bool + Send + Sync>>,
}

impl TypeChecker {
    /// 构建包含七种基本类型的默认类型检查器。
    pub fn default() -> Self {
        let mut types: HashMap<String, Box<dyn Fn(&Value) -> bool + Send + Sync>> = HashMap::new();

        types.insert("null".into(), Box::new(|v| v.is_null()));
        types.insert("boolean".into(), Box::new(|v| v.is_boolean()));
        types.insert("object".into(), Box::new(|v| v.is_object()));
        types.insert("array".into(), Box::new(|v| v.is_array()));
        types.insert("string".into(), Box::new(|v| v.is_string()));

        types.insert(
            "number".into(),
            Box::new(|v: &Value| v.is_number()),
        );

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

    //注册自定义类型判断函数。
    pub fn register(
        &mut self,
        name: &str,
        predicate: Box<dyn Fn(&Value) -> bool + Send + Sync>,
    ) {
        self.types.insert(name.to_string(), predicate);
    }

    //判断 `instance` 是否满足指定的 JSON Schema 类型。
    pub fn is_type(&self, instance: &Value, type_name: &str) -> bool {
        self.types
            .get(type_name)
            .map(|f| f(instance))
            .unwrap_or(false)
    }
}

// tests

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

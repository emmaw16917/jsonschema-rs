use jsonschema_rs::Validator;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Comprehensive integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_full_person_schema() {
    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "minLength": 1,
                "maxLength": 100
            },
            "age": {
                "type": "integer",
                "minimum": 0,
                "maximum": 150
            },
            "email": {
                "type": "string",
                "pattern": "^[^@]+@[^@]+[.][^@]+$"
            },
            "tags": {
                "type": "array",
                "items": {"type": "string"},
                "uniqueItems": true
            }
        },
        "required": ["name", "email"]
    });

    let validator = Validator::new(schema);

    // 有效
    assert!(validator.is_valid(&serde_json::json!({
        "name": "Alice",
        "age": 30,
        "email": "alice@example.com",
        "tags": ["rust", "json"]
    })));

    // 缺少必填字段
    assert!(!validator.is_valid(&serde_json::json!({
        "name": "Alice"
    })));

    // 类型错误
    assert!(!validator.is_valid(&serde_json::json!({
        "name": "Alice",
        "age": "thirty",
        "email": "alice@example.com"
    })));

    // 模式不匹配
    assert!(!validator.is_valid(&serde_json::json!({
        "name": "Alice",
        "email": "notanemail"
    })));

    // 重复元素
    assert!(!validator.is_valid(&serde_json::json!({
        "name": "Alice",
        "email": "a@b.c",
        "tags": ["rust", "rust"]
    })));
}

#[test]
fn test_nested_object() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "address": {
                "type": "object",
                "properties": {
                    "street": {"type": "string"},
                    "city": {"type": "string"},
                    "zip": {"type": "string", "pattern": "^[0-9]{5}$"}
                },
                "required": ["street", "city"]
            }
        }
    });

    let validator = Validator::new(schema);

    assert!(validator.is_valid(&serde_json::json!({
        "address": {
            "street": "123 Main St",
            "city": "Springfield",
            "zip": "12345"
        }
    })));

    assert!(!validator.is_valid(&serde_json::json!({
        "address": {
            "street": "123 Main St"
        }
    })));

    assert!(!validator.is_valid(&serde_json::json!({
        "address": {
            "street": "123 Main St",
            "city": "Springfield",
            "zip": "abc"
        }
    })));
}

#[test]
fn test_combinators() {
    let schema = serde_json::json!({
        "allOf": [
            {"type": "object", "properties": {"type": {"type": "string"}}},
            {
                "oneOf": [
                    {
                        "properties": {
                            "type": {"const": "circle"},
                            "radius": {"type": "number", "minimum": 0}
                        },
                        "required": ["radius"]
                    },
                    {
                        "properties": {
                            "type": {"const": "rectangle"},
                            "width": {"type": "number", "minimum": 0},
                            "height": {"type": "number", "minimum": 0}
                        },
                        "required": ["width", "height"]
                    }
                ]
            }
        ]
    });

    let validator = Validator::new(schema);

    // 有效的圆形
    assert!(validator.is_valid(&serde_json::json!({
        "type": "circle",
        "radius": 5.0
    })));

    // 有效的矩形
    assert!(validator.is_valid(&serde_json::json!({
        "type": "rectangle",
        "width": 10,
        "height": 20
    })));

    // 圆形缺少必填的 radius
    assert!(!validator.is_valid(&serde_json::json!({
        "type": "circle"
    })));

    // 此实例同时匹配两个 oneOf schema：
    // 带有 width/height 的圆形，但矩形 schema 要求 type=rectangle，因此仅匹配圆形。
    assert!(validator.is_valid(&serde_json::json!({
        "type": "circle",
        "radius": 5,
        "width": 10,
        "height": 20
    })));
}

#[test]
fn test_if_then_else() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "country": {"type": "string"}
        },
        "if": {"properties": {"country": {"const": "US"}}},
        "then": {"properties": {"zip": {"type": "string", "pattern": "^[0-9]{5}$"}}},
        "else": {"properties": {"zip": {"type": "string", "pattern": "^[A-Z0-9]{4,10}$"}}},
        "required": ["country"]
    });

    let validator = Validator::new(schema);

    // 美国邮编（5位数字）
    assert!(validator.is_valid(&serde_json::json!({
        "country": "US",
        "zip": "12345"
    })));

    // 美国邮编格式错误
    assert!(!validator.is_valid(&serde_json::json!({
        "country": "US",
        "zip": "AB12CD"
    })));

    // 非美国邮编（4-10位字母数字）
    assert!(validator.is_valid(&serde_json::json!({
        "country": "CA",
        "zip": "K1A0B1"
    })));
}

#[test]
fn test_additional_properties() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": false
    });

    let validator = Validator::new(schema);

    assert!(validator.is_valid(&serde_json::json!({"name": "Alice"})));
    assert!(!validator.is_valid(&serde_json::json!({
        "name": "Alice",
        "extra": true
    })));
}

#[test]
fn test_contains() {
    let schema = serde_json::json!({
        "type": "array",
        "contains": {"type": "string"}
    });

    let validator = Validator::new(schema);

    assert!(validator.is_valid(&serde_json::json!([1, "hello", true])));
    assert!(!validator.is_valid(&serde_json::json!([1, 2, 3])));
}

#[test]
fn test_boolean_schema() {
    // JSON Schema 允许 `true`（全部通过）和 `false`（全部不通过）作为 schema
    let v_true = Validator::new(Value::Bool(true));
    assert!(v_true.is_valid(&serde_json::json!(42)));
    assert!(v_true.is_valid(&Value::Null));

    let v_false = Validator::new(Value::Bool(false));
    assert!(!v_false.is_valid(&serde_json::json!(42)));
    assert!(!v_false.is_valid(&Value::Null));
}

#[test]
fn test_iter_errors_returns_multiple() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 3},
            "age": {"type": "integer", "minimum": 0}
        },
        "required": ["name"]
    });

    let validator = Validator::new(schema);

    let errors = validator.iter_errors(&serde_json::json!({
        "name": "",
        "age": -1
    }));

    // 应包含多个错误：minLength 违规、age minimum 违规
    assert!(errors.len() >= 2, "Expected at least 2 errors, got {}", errors.len());
    assert!(errors.iter().any(|e| e.keyword.as_deref() == Some("minLength")));
    assert!(errors.iter().any(|e| e.keyword.as_deref() == Some("minimum")));
}

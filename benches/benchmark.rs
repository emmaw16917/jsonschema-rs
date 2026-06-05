use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jsonschema_rs::Validator;
use serde_json::Value;

fn bench_type_check(c: &mut Criterion) {
    let schema = serde_json::json!({"type": "string"});
    let validator = Validator::new(schema);
    let instance = Value::String("hello world".into());

    c.bench_function("validate_type_string_valid", |b| {
        b.iter(|| validator.is_valid(black_box(&instance)))
    });
}

fn bench_numeric(c: &mut Criterion) {
    let schema = serde_json::json!({
        "type": "integer",
        "minimum": 0,
        "maximum": 100,
        "multipleOf": 5
    });
    let validator = Validator::new(schema);
    let instance = serde_json::json!(25);

    c.bench_function("validate_numeric_valid", |b| {
        b.iter(|| validator.is_valid(black_box(&instance)))
    });
}

fn bench_object_properties(c: &mut Criterion) {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1, "maxLength": 50},
            "age": {"type": "integer", "minimum": 0, "maximum": 150},
            "email": {"type": "string", "pattern": ".+@.+[.].+"}
        },
        "required": ["name"]
    });
    let validator = Validator::new(schema);
    let instance = serde_json::json!({
        "name": "Alice",
        "age": 30,
        "email": "alice@example.com"
    });

    c.bench_function("validate_object_properties_valid", |b| {
        b.iter(|| validator.is_valid(black_box(&instance)))
    });
}

fn bench_large_object(c: &mut Criterion) {
    // Generate a large object with 100 properties
    let schema = {
        let props: serde_json::map::Map<String, Value> = (0..100)
            .map(|i| {
                (
                    format!("field_{}", i),
                    serde_json::json!({"type": "integer", "minimum": 0}),
                )
            })
            .collect();
        serde_json::json!({"type": "object", "properties": props})
    };
    let validator = Validator::new(schema);
    let instance = {
        let fields: serde_json::map::Map<String, Value> = (0..100)
            .map(|i| (format!("field_{}", i), serde_json::json!(i)))
            .collect();
        serde_json::Value::Object(fields)
    };

    c.bench_function("validate_large_object_100_fields", |b| {
        b.iter(|| validator.is_valid(black_box(&instance)))
    });
}

fn bench_nested_all_of(c: &mut Criterion) {
    let schema = serde_json::json!({
        "allOf": [
            {"type": "object"},
            {"properties": {"name": {"type": "string"}}},
            {"required": ["name"]}
        ]
    });
    let validator = Validator::new(schema);
    let instance = serde_json::json!({"name": "Alice"});

    c.bench_function("validate_nested_all_of", |b| {
        b.iter(|| validator.is_valid(black_box(&instance)))
    });
}

criterion_group!(
    benches,
    bench_type_check,
    bench_numeric,
    bench_object_properties,
    bench_large_object,
    bench_nested_all_of
);
criterion_main!(benches);

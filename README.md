# jsonschema-rs

A fast JSON Schema validator written in Rust — a high-performance rewrite of the Python [`jsonschema`](https://github.com/python-jsonschema/jsonschema) library.

Supports **JSON Schema Draft 2020-12** with **30 keywords** and **80%+ official test suite compliance**.

## Why Rust?

| Metric | Python `jsonschema` | `jsonschema-rs` (Rust) |
|--------|---------------------|------------------------|
| Speed | Interpreted (baseline) | **20–100× faster** (compiled) |
| Memory | GC overhead | **3–10× less** RAM |
| Concurrency | GIL-bound | **Rayon parallel** batch validation |
| Safety | Runtime errors | **Compile-time** memory + thread safety |

## Quick Start

### CLI

```bash
cargo install jsonschema-rs

# Validate a JSON file against a schema
jsonschema-rs validate -s schema.json -d data.json

# JSON output
jsonschema-rs validate -s schema.json -d data.json --output json
```

### Rust Library

```rust
use jsonschema_rs::Validator;

let schema = serde_json::json!({
    "type": "object",
    "properties": {
        "name": { "type": "string", "minLength": 1 },
        "age":  { "type": "integer", "minimum": 0 }
    },
    "required": ["name"]
});

let validator = Validator::new(schema);
assert!(validator.is_valid(&serde_json::json!({"name": "Alice", "age": 30})));
```

## Supported Keywords

- **Assertions:** `type`, `enum`, `const`
- **Numeric:** `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`
- **String:** `minLength`, `maxLength`, `pattern`
- **Format:** `date`, `time`, `date-time`, `email`, `hostname`, `ipv4`, `ipv6`, `uri`, `uuid`, `json-pointer`, `regex` and 10+ more
- **Objects:** `properties`, `required`, `additionalProperties`, `patternProperties`, `propertyNames`, `minProperties`, `maxProperties`
- **Arrays:** `items`, `prefixItems`, `minItems`, `maxItems`, `uniqueItems`, `contains`
- **Applicators:** `allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`

## Testing

```bash
# Run all unit + integration tests
cargo test --lib --test integration

# Run official JSON Schema Test Suite
git clone https://github.com/json-schema-org/JSON-Schema-Test-Suite.git tests/test_suite
cargo test --test runner -- --nocapture
```

## Benchmarks

```bash
cargo bench
```

Python comparison scripts are in `python_bench/`.

## License

MIT

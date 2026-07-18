/// jsonschema-rs 性能基准测试
/// 运行: cargo run --release --bin perf_test
use std::time::Instant;

fn main() {
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║   jsonschema-rs  Performance Benchmark              ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();

    // 测试 1: 简单类型检查
    {
        let schema = serde_json::json!({"type": "string"});
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::Value::String("hello world".into());

        for _ in 0..50000 {
            v.is_valid(&data);
        }

        let n = 2_000_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  simple_type (valid)        {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 2: 带属性的对象
    {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name":  {"type": "string", "minLength": 1, "maxLength": 50},
                "age":   {"type": "integer", "minimum": 0, "maximum": 150},
                "email": {"type": "string", "pattern": ".+@.+[.].+"}
            },
            "required": ["name"]
        });
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::json!({
            "name": "Alice", "age": 30, "email": "alice@example.com"
        });

        for _ in 0..5000 {
            v.is_valid(&data);
        }

        let n = 500_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  object_properties (valid)  {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 3: 对象 — 无效（提前退出）
    {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name":  {"type": "string", "minLength": 1},
                "age":   {"type": "integer", "minimum": 0}
            },
            "required": ["name"]
        });
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::json!({"name": 42, "age": -1});

        for _ in 0..5000 {
            v.is_valid(&data);
        }

        let n = 500_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  object_properties (invalid){:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 4: 嵌套 allOf 组合器
    {
        let schema = serde_json::json!({
            "allOf": [
                {"type": "object"},
                {"properties": {"name": {"type": "string"}}},
                {"required": ["name"]}
            ]
        });
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::json!({"name": "Alice"});

        for _ in 0..5000 {
            v.is_valid(&data);
        }

        let n = 500_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  nested_allOf (valid)       {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 5: 100 字段的大对象
    {
        let schema = {
            let props: serde_json::map::Map<String, serde_json::Value> = (0..100)
                .map(|i| {
                    (
                        format!("field_{}", i),
                        serde_json::json!({"type": "integer", "minimum": 0}),
                    )
                })
                .collect();
            serde_json::json!({"type": "object", "properties": props})
        };
        let v = jsonschema_rs::Validator::new(schema);
        let data = {
            let fields: serde_json::map::Map<String, serde_json::Value> = (0..100)
                .map(|i| (format!("field_{}", i), serde_json::json!(i)))
                .collect();
            serde_json::Value::Object(fields)
        };

        for _ in 0..1000 {
            v.is_valid(&data);
        }

        let n = 100_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  large_object_100 (valid)   {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 6: 20 个元素的嵌套数组
    {
        let schema = serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id":    {"type": "integer"},
                    "value": {"type": "string"}
                },
                "required": ["id"]
            }
        });
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::Value::Array(
            (0..20)
                .map(|i| {
                    serde_json::json!({"id": i, "value": format!("item_{}", i)})
                })
                .collect(),
        );

        for _ in 0..1000 {
            v.is_valid(&data);
        }

        let n = 200_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  nested_array (valid)       {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 7: oneOf 组合器
    {
        let schema = serde_json::json!({
            "oneOf": [
                {"type": "string", "minLength": 3},
                {"type": "integer", "minimum": 0}
            ]
        });
        let v = jsonschema_rs::Validator::new(schema);

        let data_int = serde_json::json!(42);
        for _ in 0..5000 {
            v.is_valid(&data_int);
        }
        let n = 500_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data_int);
        }
        let elapsed = start.elapsed();
        println!(
            "  oneOf (valid)              {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    // 测试 8: 带 pattern (regex) 的 schema
    {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "email": {"type": "string", "pattern": "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"}
            }
        });
        let v = jsonschema_rs::Validator::new(schema);
        let data = serde_json::json!({"email": "alice@example.com"});

        for _ in 0..5000 {
            v.is_valid(&data);
        }

        let n = 500_000;
        let start = Instant::now();
        for _ in 0..n {
            v.is_valid(&data);
        }
        let elapsed = start.elapsed();
        println!(
            "  pattern_regex (valid)      {:>8} iter  {:>10.4}s  {:>12.0} ops/sec",
            format_n(n),
            elapsed.as_secs_f64(),
            n as f64 / elapsed.as_secs_f64(),
        );
    }

    println!();
    println!("All benchmarks complete.");
}

fn format_n(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
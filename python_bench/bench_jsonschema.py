#!/usr/bin/env python3
"""
Benchmark: Python `jsonschema` vs Rust `jsonschema-rs`.

Compares throughput (validations/sec) for several schema↔instance pairs.

Usage:
    python bench_jsonschema.py
    python bench_jsonschema.py --engine jsonschema
    python bench_jsonschema.py --engine fastjsonschema
    python bench_jsonschema.py --engine rust   # calls `jsonschema-rs` CLI
"""

import argparse
import json
import subprocess
import timeit
from pathlib import Path

# --- Test cases ---------------------------------------------------------

TEST_CASES = {
    "simple_type": {
        "schema": {"type": "string"},
        "data": "hello world",
    },
    "object_with_properties": {
        "schema": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1, "maxLength": 50},
                "age": {"type": "integer", "minimum": 0, "maximum": 150},
                "email": {"type": "string", "pattern": "^[^@]+@[^@]+\\.[^@]+$"},
            },
            "required": ["name", "email"],
        },
        "data": {"name": "Alice", "age": 30, "email": "alice@example.com"},
    },
    "nested_array": {
        "schema": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "integer"},
                    "value": {"type": "string"},
                },
                "required": ["id"],
            },
            "minItems": 5,
            "maxItems": 100,
        },
        "data": [{"id": i, "value": f"item_{i}"} for i in range(20)],
    },
    "all_of_combinator": {
        "schema": {
            "allOf": [
                {"type": "object"},
                {"properties": {"name": {"type": "string"}}},
                {"required": ["name"]},
            ]
        },
        "data": {"name": "Alice"},
    },
    "one_of_combinator": {
        "schema": {
            "oneOf": [
                {"type": "string"},
                {"type": "integer"},
            ]
        },
        "data": 42,
    },
    "large_object": {
        "schema": {
            "type": "object",
            "properties": {
                f"field_{i}": {"type": "integer", "minimum": 0}
                for i in range(100)
            },
        },
        "data": {f"field_{i}": i for i in range(100)},
    },
}

# --- Engine-specific runners --------------------------------------------

def bench_python_jsonschema(schema, data, iterations):
    from jsonschema import validate

    # Warm-up
    for _ in range(min(1000, iterations // 10)):
        validate(data, schema)

    timer = timeit.Timer(lambda: validate(data, schema))
    total = timer.timeit(number=iterations)
    return total, iterations / total


def bench_python_fastjsonschema(schema, data, iterations):
    from fastjsonschema import compile

    # fastjsonschema uses a compiled validator
    validator = compile(schema)

    # Warm-up
    for _ in range(min(1000, iterations // 10)):
        validator(data)

    timer = timeit.Timer(lambda: validator(data))
    total = timer.timeit(number=iterations)
    return total, iterations / total


def bench_rust_cli(schema, data, iterations):
    """Call the Rust jsonschema-rs CLI for each iteration (slow — use large
    batch inside the CLI instead)."""
    # Write schema and data to temp files
    schema_file = Path("__bench_schema.json")
    data_file = Path("__bench_data.json")
    schema_file.write_text(json.dumps(schema))
    data_file.write_text(json.dumps(data))

    try:
        # Warm-up
        for _ in range(min(50, iterations // 10)):
            subprocess.run(
                [
                    "cargo", "run", "--release", "--",
                    "validate", "-s", str(schema_file), "-d", str(data_file),
                ],
                capture_output=True,
                cwd="..",
            )

        # Timed
        def run():
            subprocess.run(
                [
                    "cargo", "run", "--release", "--",
                    "validate", "-s", str(schema_file), "-d", str(data_file),
                ],
                capture_output=True,
                cwd="..",
            )

        timer = timeit.Timer(run)
        total = timer.timeit(number=iterations)
        return total, iterations / total
    finally:
        schema_file.unlink(missing_ok=True)
        data_file.unlink(missing_ok=True)


# --- Main ---------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Benchmark JSON Schema validators")
    parser.add_argument(
        "--engine",
        choices=["jsonschema", "fastjsonschema", "rust", "all"],
        default="all",
    )
    parser.add_argument("--iterations", type=int, default=10000)
    args = parser.parse_args()

    engines = (
        ["jsonschema", "fastjsonschema"]
        if args.engine == "all"
        else [args.engine]
    )

    print(f"{'Test':<25} {'Engine':<20} {'Iterations':>10} {'Time(s)':>10} {'Ops/sec':>12}")
    print("-" * 80)

    for name, case in TEST_CASES.items():
        for engine in engines:
            fn_map = {
                "jsonschema": bench_python_jsonschema,
                "fastjsonschema": bench_python_fastjsonschema,
                "rust": bench_rust_cli,
            }

            # Determine iterations: fewer for expensive engines
            iters = args.iterations
            if engine == "rust":
                iters = max(100, iters // 50)  # CLI overhead is huge

            try:
                elapsed, ops = fn_map[engine](case["schema"], case["data"], iters)
                print(
                    f"{name:<25} {engine:<20} {iters:>10} {elapsed:>10.4f} {ops:>12.0f}"
                )
            except ImportError:
                print(f"{name:<25} {engine:<20} {'SKIP (not installed)':>35}")
            except Exception as e:
                print(f"{name:<25} {engine:<20} {'ERROR:':>10} {e}")


if __name__ == "__main__":
    main()

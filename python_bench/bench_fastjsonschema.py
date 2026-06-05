#!/usr/bin/env python3
"""
Quick benchmark: Python `fastjsonschema` (fastest Python JSON Schema validator).

Usage:
    pip install fastjsonschema
    python bench_fastjsonschema.py
"""

import timeit
from fastjsonschema import compile

# --- Test cases ---

def bench_simple():
    validator = compile({"type": "string"})
    data = "hello world"
    # Warm-up
    for _ in range(1000):
        validator(data)
    t = timeit.Timer(lambda: validator(data))
    total = t.timeit(number=100000)
    print(f"simple_type:         {100000/total:>12.0f} ops/sec  ({total:.4f}s)")


def bench_object():
    schema = {
        "type": "object",
        "properties": {
            "name": {"type": "string", "minLength": 1, "maxLength": 50},
            "age": {"type": "integer", "minimum": 0, "maximum": 150},
            "email": {"type": "string", "pattern": "^[^@]+@[^@]+\\.[^@]+$"},
        },
        "required": ["name", "email"],
    }
    validator = compile(schema)
    data = {"name": "Alice", "age": 30, "email": "alice@example.com"}
    for _ in range(1000):
        validator(data)
    t = timeit.Timer(lambda: validator(data))
    total = t.timeit(number=50000)
    print(f"object_properties:   {50000/total:>12.0f} ops/sec  ({total:.4f}s)")


def bench_large_object():
    schema = {
        "type": "object",
        "properties": {f"field_{i}": {"type": "integer", "minimum": 0} for i in range(100)},
    }
    validator = compile(schema)
    data = {f"field_{i}": i for i in range(100)}
    for _ in range(100):
        validator(data)
    t = timeit.Timer(lambda: validator(data))
    total = t.timeit(number=5000)
    print(f"large_object_100:    {5000/total:>12.0f} ops/sec  ({total:.4f}s)")


def bench_nested_array():
    schema = {
        "type": "array",
        "items": {
            "type": "object",
            "properties": {
                "id": {"type": "integer"},
                "value": {"type": "string"},
            },
            "required": ["id"],
        },
    }
    validator = compile(schema)
    data = [{"id": i, "value": f"item_{i}"} for i in range(20)]
    for _ in range(100):
        validator(data)
    t = timeit.Timer(lambda: validator(data))
    total = t.timeit(number=10000)
    print(f"nested_array:        {10000/total:>12.0f} ops/sec  ({total:.4f}s)")


if __name__ == "__main__":
    print("fastjsonschema benchmarks")
    print("-" * 50)
    bench_simple()
    bench_object()
    bench_large_object()
    bench_nested_array()

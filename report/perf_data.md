# Performance Benchmark Raw Data

## Test Environment

- **OS**: Windows 11 Home 10.0.26200
- **CPU**: Intel Core i7-13700H
- **Rust**: 1.88.0, release profile (LTO=true, codegen-units=1)
- **Python**: 3.13
- **Date**: 2026-07-15

## Rust jsonschema-rs

```
╔══════════════════════════════════════════════════════╗
║   jsonschema-rs  Performance Benchmark              ║
╚══════════════════════════════════════════════════════╝

  simple_type (valid)            2.0M iter      0.1479s      13519460 ops/sec
  object_properties (valid)      500K iter      0.3676s       1360352 ops/sec
  object_properties (invalid)    500K iter      0.5750s        869585 ops/sec
  nested_allOf (valid)           500K iter      0.1447s       3456337 ops/sec
  large_object_100 (valid)       100K iter      1.9439s         51444 ops/sec
  nested_array (valid)           200K iter      2.2965s         87089 ops/sec
  oneOf (valid)                  500K iter      0.2040s       2451443 ops/sec
  pattern_regex (valid)          500K iter      0.1504s       3324621 ops/sec
```

## Python fastjsonschema 2.21.2

```
fastjsonschema benchmarks
--------------------------------------------------
simple_type:             18288891 ops/sec  (0.0055s)
object_properties:        1135089 ops/sec  (0.0440s)
large_object_100:           77929 ops/sec  (0.0642s)
nested_array:              108408 ops/sec  (0.0922s)
```

## Python jsonschema 4.26.0

```
simple_type                50000 iter    8.0308s          6226 ops/sec
object_properties           5000 iter    3.1667s          1579 ops/sec
large_object_100            1000 iter   13.7350s            73 ops/sec
nested_array                2000 iter    1.4664s          1364 ops/sec
```

## Comparison Summary

| Benchmark | Rust jsonschema-rs | Python fastjsonschema | Python jsonschema | Rust vs jsonschema |
|-----------|-------------------|----------------------|-------------------|-------------------|
| simple_type | 13,519,460 | 18,288,891 | 6,226 | 2,172x |
| object_properties | 1,360,352 | 1,135,089 | 1,579 | 862x |
| large_object_100 | 51,444 | 77,929 | 73 | 705x |
| nested_array | 87,089 | 108,408 | 1,364 | 64x |

## Official Test Suite Results

```
Total: 2086 tests
Passed: 1741
Failed: 345
Rate: 83.5%

Failures by test file (top 10):
    46  unevaluatedProperties
    42  idn-hostname
    27  unevaluatedItems
    23  hostname
    20  date
    20  dynamicRef
    19  time
    17  ecmascript-regex
    17  uri
    16  refRemote
```

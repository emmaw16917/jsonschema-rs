//! JSON Schema 官方测试套件执行器
//! 测试文件位于 `tests/test_suite/`，可从
//! <https://github.com/json-schema-org/JSON-Schema-Test-Suite> 获取

use jsonschema_rs::Validator;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct TestCase {
    description: String,
    schema: serde_json::Value,
    tests: Vec<SubTest>,
}

#[derive(Debug, Deserialize)]
struct SubTest {
    description: String,
    data: serde_json::Value,
    valid: bool,
}

fn run_test_file(path: &Path) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            failures.push(format!("{}: cannot read: {}", path.display(), e));
            return failures;
        }
    };

    let test_cases: Vec<TestCase> = match serde_json::from_str(&content) {
        Ok(tc) => tc,
        Err(e) => {
            failures.push(format!("{}: cannot parse: {}", path.display(), e));
            return failures;
        }
    };

    for tc in &test_cases {
        let validator = Validator::new(tc.schema.clone());

        for st in &tc.tests {
            let is_valid = validator.is_valid(&st.data);

            if is_valid != st.valid {
                failures.push(format!(
                    "{} :: {} :: {}: expected valid={}, got valid={}",
                    path.file_stem().unwrap().to_string_lossy(),
                    tc.description,
                    st.description,
                    st.valid,
                    is_valid
                ));
            }
        }
    }

    failures
}

#[test]
fn run_official_test_suite() {
    let suite_dir = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/test_suite/tests/draft2020-12"
    ));

    if !suite_dir.exists() {
        eprintln!(
            "Skipping official test suite — directory not found: {}\n\
             Clone it with:\n  \
             git submodule add https://github.com/json-schema-org/JSON-Schema-Test-Suite.git tests/test_suite",
            suite_dir.display()
        );
        return;
    }

    // Collect all .json test files (recursively)
    let mut test_files = Vec::new();
    collect_json_files(&suite_dir, &mut test_files);

    if test_files.is_empty() {
        eprintln!("No test files found in {}", suite_dir.display());
        return;
    }

    let mut all_failures: Vec<String> = Vec::new();
    let mut total_tests = 0;
    let mut total_failed = 0;

    for file in &test_files {
        let failures = run_test_file(file);
        total_failed += failures.len();
        // We can't count total tests easily without re-parsing; approximate
        for f in &failures {
            all_failures.push(f.clone());
        }
    }

    // Count total tests
    for file in &test_files {
        if let Ok(content) = fs::read_to_string(file) {
            if let Ok(test_cases) = serde_json::from_str::<Vec<TestCase>>(&content) {
                total_tests += test_cases.iter().map(|tc| tc.tests.len()).sum::<usize>();
            }
        }
    }

    if total_failed > 0 {
        eprintln!(
            "\n=== TEST SUITE RESULTS ===\n\
             Total: {} tests, {} failed\n\
             Failures (first 20):",
            total_tests, total_failed
        );
        for f in all_failures.iter().take(20) {
            eprintln!("  ✗ {}", f);
        }
        if all_failures.len() > 20 {
            eprintln!("  ... and {} more", all_failures.len() - 20);
        }

        // Aggregate failures by test file
        let mut by_file: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for f in &all_failures {
            let file = f.split(" :: ").next().unwrap_or("unknown");
            *by_file.entry(file.to_string()).or_default() += 1;
        }
        eprintln!("\nFailures by test file:");
        let mut entries: Vec<_> = by_file.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (file, count) in entries {
            eprintln!("  {:>4}  {}", count, file);
        }
        panic!(
            "Official test suite: {}/{} tests failed",
            total_failed, total_tests
        );
    } else {
        println!(
            "✓ Official test suite: {}/{} passed",
            total_tests, total_tests
        );
    }
}

fn collect_json_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_json_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                out.push(path);
            }
        }
    }
}

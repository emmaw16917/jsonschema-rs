use crate::error::ValidationError;
use crate::keyword::KeywordRegistry;
use crate::refs::SchemaRegistry;
use crate::types::{CompiledSchema, TypeChecker};
use crate::validator::ValidationContext;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/// A ready-to-use JSON Schema validator.
///
/// Construct via `Validator::new(schema)`.  The schema is compiled once and can
/// then be used to validate many instances efficiently.
///
/// Corresponds to a concrete validator class instance in Python
/// (e.g. `Draft202012Validator(schema)`).
#[derive(Clone)]
pub struct Validator {
    compiled: Arc<CompiledSchema>,
    registry: Arc<KeywordRegistry>,
    type_checker: Arc<TypeChecker>,
    schema_registry: Option<Arc<SchemaRegistry>>,
}

impl Validator {
    /// Compile a JSON Schema `Value` into a reusable `Validator`.
    pub fn new(schema: Value) -> Self {
        let precompiled_patterns = compile_patterns(&schema);
        let compiled = Arc::new(CompiledSchema::new(schema, precompiled_patterns));
        let registry = Arc::new(KeywordRegistry::draft_2020_12());
        let type_checker = Arc::new(TypeChecker::default());

        Self {
            compiled,
            registry,
            type_checker,
            schema_registry: None,
        }
    }

    /// Compile with a custom `SchemaRegistry` for external `$ref` support.
    pub fn with_registry(mut self, registry: SchemaRegistry) -> Self {
        self.schema_registry = Some(Arc::new(registry));
        self
    }

    /// Validate `instance` against the compiled schema.
    ///
    /// Returns `Ok(())` on success, or the first `ValidationError` on failure.
    pub fn validate(&self, instance: &Value) -> Result<(), ValidationError> {
        let ctx = self.make_ctx();
        let errors = ctx.iter_errors(instance, &self.compiled.raw);
        errors.into_iter().next().map(Err).unwrap_or(Ok(()))
    }

    /// Returns `true` if `instance` is valid against the schema.
    pub fn is_valid(&self, instance: &Value) -> bool {
        self.validate(instance).is_ok()
    }

    /// Return *all* validation errors (empty Vec → valid).
    pub fn iter_errors(&self, instance: &Value) -> Vec<ValidationError> {
        let ctx = self.make_ctx();
        ctx.iter_errors(instance, &self.compiled.raw)
    }

    /// Build a `ValidationContext` borrowing from our Arc'd data.
    fn make_ctx(&self) -> ValidationContext<'_> {
        let schema_registry_ref = self.schema_registry.as_deref();
        ValidationContext::new(
            &self.compiled,
            &self.registry,
            &self.type_checker,
            schema_registry_ref,
        )
    }

    // -- accessors --------------------------------------------------------

    /// The raw (original) schema JSON.
    pub fn schema(&self) -> &Value {
        &self.compiled.raw
    }
}

// ---------------------------------------------------------------------------
// Compilation helpers
// ---------------------------------------------------------------------------

/// Walk the schema tree and pre-compile every `"pattern"` value into a
/// `Regex`, returning a map from pattern source string → compiled regex.
fn compile_patterns(schema: &Value) -> HashMap<String, Regex> {
    let mut patterns = HashMap::new();
    walk_and_collect_patterns(schema, &mut patterns);
    patterns
}

fn walk_and_collect_patterns(node: &Value, patterns: &mut HashMap<String, Regex>) {
    match node {
        Value::Object(obj) => {
            // If this object has a "pattern" key, compile it.
            if let Some(Value::String(p)) = obj.get("pattern") {
                if !patterns.contains_key(p) {
                    if let Ok(re) = Regex::new(p) {
                        patterns.insert(p.clone(), re);
                    }
                }
            }

            // Recurse into all values.
            for val in obj.values() {
                walk_and_collect_patterns(val, patterns);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                walk_and_collect_patterns(item, patterns);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple() {
        let schema = serde_json::json!({"type": "string"});
        let v = Validator::new(schema);
        assert!(v.is_valid(&Value::String("hi".into())));
        assert!(!v.is_valid(&serde_json::json!(42)));
    }

    #[test]
    fn test_validator_clone() {
        let v1 = Validator::new(serde_json::json!({"type": "integer"}));
        let v2 = v1.clone();
        assert!(v2.is_valid(&serde_json::json!(1)));
    }

    #[test]
    fn test_pattern_precompilation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "pattern": "^[A-Z][a-z]+$" },
                "email": { "type": "string", "pattern": ".+@.+[.].+" }
            }
        });
        let v = Validator::new(schema);
        // Patterns should have been pre-compiled.
        assert!(v.compiled.precompiled_patterns.len() >= 2);

        let valid = serde_json::json!({"name": "Alice", "email": "a@b.c"});
        assert!(v.is_valid(&valid));

        let invalid = serde_json::json!({"name": "alice", "email": "a@b.c"});
        assert!(!v.is_valid(&invalid));
    }
}

use crate::error::ValidationError;
use crate::keyword::KeywordRegistry;
use crate::refs::SchemaRegistry;
use crate::types::{CompiledSchema, TypeChecker};
use regex::Regex;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ValidationContext
// ---------------------------------------------------------------------------

/// The heart of the validation engine — carries all the state needed during a
/// recursive validation walk.
///
/// Corresponds to Python's `Validator` class (the concrete instance created by
/// `create()` factories in `validators.py`).
pub struct ValidationContext<'a> {
    /// Reference to the compiled schema being validated against.
    pub compiled: &'a CompiledSchema,

    /// All registered keyword validators.
    pub registry: &'a KeywordRegistry,

    /// Type checker used by the `type` keyword and other type-guards.
    pub type_checker: &'a TypeChecker,

    /// Optional external schema registry for resolving `$ref` URIs.
    pub schema_registry: Option<&'a SchemaRegistry>,

    /// Current position within the *instance* being validated.
    /// E.g. `["properties", "items", "0"]`.
    pub instance_path: Vec<String>,

    /// Current position within the *schema*.
    /// E.g. `["properties", "name", "minLength"]`.
    pub schema_path: Vec<String>,

    /// Pre-compiled regex patterns (extracted from `compiled` for convenience).
    pub precompiled: &'a HashMap<String, Regex>,

    /// Guard against infinite recursion in `$ref` resolution.  Tracks the
    /// `$ref` URIs that are currently being resolved on the call stack.
    visited_refs: RefCell<Vec<String>>,

    /// Maximum allowed recursive depth for `$ref` chains.
    max_ref_depth: usize,
}

impl<'a> ValidationContext<'a> {
    /// Create a new root-level validation context.
    pub fn new(
        compiled: &'a CompiledSchema,
        registry: &'a KeywordRegistry,
        type_checker: &'a TypeChecker,
        schema_registry: Option<&'a SchemaRegistry>,
    ) -> Self {
        Self {
            compiled,
            registry,
            type_checker,
            schema_registry,
            instance_path: Vec::new(),
            schema_path: Vec::new(),
            precompiled: &compiled.precompiled_patterns,
            visited_refs: RefCell::new(Vec::new()),
            max_ref_depth: 50,
        }
    }

    // ------------------------------------------------------------------
    // Public validation API
    // ------------------------------------------------------------------

    /// Recursively validate `instance` against `schema`.
    ///
    /// Returns all validation errors found (empty Vec → valid).
    ///
    /// Analogous to Python `iter_errors()`.
    pub fn iter_errors(&self, instance: &Value, schema: &Value) -> Vec<ValidationError> {
        // Boolean schemas
        match schema {
            Value::Bool(true) => return vec![],
            Value::Bool(false) => {
                return vec![ValidationError::new("False schema does not allow anything")];
            }
            _ => {}
        }

        // $ref resolution (handled before keyword dispatch)
        let effective_schema = if let Some(ref_val) = schema.get("$ref") {
            if let Some(ref_str) = ref_val.as_str() {
                // Cycle detection: if we've already seen this $ref on the
                // current call stack, skip it to avoid infinite recursion.
                {
                    let visited = self.visited_refs.borrow();
                    if visited.contains(&ref_str.to_string()) {
                        // Cycle detected — skip this $ref (the first occurrence
                        // already validated against it).
                        return vec![];
                    }
                    if visited.len() >= self.max_ref_depth {
                        return vec![ValidationError::new(format!(
                            "$ref chain exceeds maximum depth of {}",
                            self.max_ref_depth
                        ))
                        .with_keyword("$ref")];
                    }
                }

                self.visited_refs
                    .borrow_mut()
                    .push(ref_str.to_string());

                let result = if let Some(resolved) = self.resolve_ref(ref_str) {
                    let mut errors = self.iter_errors(instance, &resolved);
                    // Continue processing remaining keywords in the same schema
                    // object (keywords alongside $ref are allowed by the spec).
                    let remaining = self.collect_keyword_errors(
                        instance,
                        schema,
                        Some("$ref"),
                    );
                    errors.extend(remaining);
                    errors
                } else {
                    vec![ValidationError::new(format!(
                        "Could not resolve $ref: {}",
                        ref_str
                    ))
                    .with_keyword("$ref")]
                };

                self.visited_refs.borrow_mut().pop();
                return result;
            }
            schema
        } else {
            schema
        };

        self.collect_keyword_errors(instance, effective_schema, None)
    }

    /// Validates `instance` against a child `schema`, pushing `path_component`
    /// onto the instance path.
    ///
    /// Analogous to Python `descend()`.
    pub fn descend(
        &self,
        instance: &Value,
        schema: &Value,
        path_component: &str,
    ) -> Vec<ValidationError> {
        let child = ValidationContext {
            compiled: self.compiled,
            registry: self.registry,
            type_checker: self.type_checker,
            schema_registry: self.schema_registry,
            instance_path: {
                let mut p = self.instance_path.clone();
                p.push(path_component.to_string());
                p
            },
            schema_path: self.schema_path.clone(),
            precompiled: self.precompiled,
            visited_refs: RefCell::new(Vec::new()),
            max_ref_depth: self.max_ref_depth,
        };
        child.iter_errors(instance, schema)
    }

    /// Shortcut: `is_type()` delegates to the type checker.
    pub fn is_type(&self, instance: &Value, type_name: &str) -> bool {
        self.type_checker.is_type(instance, type_name)
    }

    /// Look up a pre-compiled regex pattern.
    pub fn get_compiled_pattern(&self, pattern: &str) -> Option<&Regex> {
        self.precompiled.get(pattern)
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Iterate every key-value pair in `schema` that is a registered keyword,
    /// call its `validate()` method, and collect errors.
    fn collect_keyword_errors(
        &self,
        instance: &Value,
        schema: &Value,
        skip_keyword: Option<&str>,
    ) -> Vec<ValidationError> {
        let mut errors: Vec<ValidationError> = Vec::new();

        if let Value::Object(obj) = schema {
            for (keyword_name, keyword_value) in obj {
                // Skip the keyword we already handled (e.g. $ref).
                if let Some(skip) = skip_keyword {
                    if keyword_name == skip {
                        continue;
                    }
                }

                if let Some(keyword) = self.registry.get(keyword_name) {
                    let keyword_errors =
                        keyword.validate(self, keyword_value, instance, schema);
                    for mut err in keyword_errors {
                        err.instance_path = self.instance_path.clone();
                        err.schema_path = {
                            let mut sp = self.schema_path.clone();
                            sp.push(keyword_name.clone());
                            sp
                        };
                        if err.keyword.is_none() {
                            err.keyword = Some(keyword_name.clone());
                        }
                        errors.push(err);
                    }
                }
            }
        }

        errors
    }

    /// Resolve a `$ref` string, first trying the schema registry (external),
    /// then JSON Pointer navigation within the root schema (internal).
    fn resolve_ref(&self, ref_val: &str) -> Option<Value> {
        // 1. Try the SchemaRegistry for external refs.
        if let Some(reg) = self.schema_registry {
            if let Some(result) = reg.resolve(&self.compiled.raw, ref_val) {
                return Some(result);
            }
        }

        // 2. Fall back to internal JSON Pointer against the root schema.
        if ref_val.starts_with('#') {
            let pointer = ref_val.trim_start_matches('#');
            if pointer.is_empty() {
                return Some(self.compiled.raw.clone());
            }
            // Use the refs module's JSON pointer resolution.
            return crate::refs::resolve_pointer(&self.compiled.raw, pointer);
        }

        None
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CompiledSchema;

    fn make_ctx(schema: Value) -> ValidationContext<'static> {
        // Leak to get 'static lifetime (acceptable in tests)
        let compiled: &'static CompiledSchema =
            Box::leak(Box::new(CompiledSchema::new(schema, HashMap::new())));
        let registry: &'static KeywordRegistry =
            Box::leak(Box::new(KeywordRegistry::draft_2020_12()));
        let type_checker: &'static TypeChecker =
            Box::leak(Box::new(TypeChecker::default()));
        ValidationContext::new(compiled, registry, type_checker, None)
    }

    #[test]
    fn test_boolean_true_schema() {
        let ctx = make_ctx(Value::Bool(true));
        let errs = ctx.iter_errors(&serde_json::json!(42), &Value::Bool(true));
        assert!(errs.is_empty());
    }

    #[test]
    fn test_boolean_false_schema() {
        let ctx = make_ctx(Value::Bool(false));
        let errs = ctx.iter_errors(&serde_json::json!(42), &Value::Bool(false));
        assert!(!errs.is_empty());
    }

    #[test]
    fn test_type_keyword_valid() {
        let schema = serde_json::json!({"type": "string"});
        let ctx = make_ctx(schema.clone());
        let errs = ctx.iter_errors(&Value::String("hello".into()), &schema);
        assert!(errs.is_empty(), "string should be valid for type=string");
    }

    #[test]
    fn test_type_keyword_invalid() {
        let schema = serde_json::json!({"type": "string"});
        let ctx = make_ctx(schema.clone());
        let errs = ctx.iter_errors(&serde_json::json!(42), &schema);
        assert!(!errs.is_empty(), "integer should be invalid for type=string");
    }

    #[test]
    fn test_minimum_valid() {
        let schema = serde_json::json!({"minimum": 10});
        let ctx = make_ctx(schema.clone());
        let errs = ctx.iter_errors(&serde_json::json!(15), &schema);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_minimum_invalid() {
        let schema = serde_json::json!({"minimum": 10});
        let ctx = make_ctx(schema.clone());
        let errs = ctx.iter_errors(&serde_json::json!(5), &schema);
        assert!(!errs.is_empty());
    }
}

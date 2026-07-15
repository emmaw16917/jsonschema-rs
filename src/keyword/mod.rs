pub mod applicator;
pub mod arrays;
pub mod assertions;
pub mod format;
pub mod numeric;
pub mod objects;
pub mod string;

use crate::error::ValidationError;
use crate::validator::ValidationContext;
use serde_json::Value;
use std::collections::HashMap;

/// Every JSON Schema keyword validator implements this trait.
///
/// Mirrors the Python keyword validator function signature:
/// `fn(validator, keyword_value, instance, schema) -> generator of ValidationError`
pub trait Keyword: Send + Sync {
    /// The keyword name, e.g. `"type"`, `"properties"`, `"minimum"`.
    fn name(&self) -> &'static str;

    /// Validate `instance` against this keyword's constraint.
    ///
    /// * `ctx` — the validation context (provides `is_type`, `descend`, etc.)
    /// * `keyword_value` — the value of this keyword inside the schema
    /// * `instance` — the JSON instance (fragment) being validated
    /// * `schema` — the full schema object (needed by some combinators)
    ///
    /// Returns an empty `Vec` on success, or one or more `ValidationError`s.
    fn validate(
        &self,
        ctx: &ValidationContext,
        keyword_value: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError>;
}

// ---------------------------------------------------------------------------
// KeywordRegistry
// ---------------------------------------------------------------------------

/// A collection of keyword implementations keyed by their JSON Schema name.
pub struct KeywordRegistry {
    keywords: HashMap<String, Box<dyn Keyword>>,
}

impl KeywordRegistry {
    /// Build a registry pre-populated with all Draft 2020-12 keywords.
    pub fn draft_2020_12() -> Self {
        let mut registry = Self {
            keywords: HashMap::new(),
        };

        // --- Assertions ---
        registry.insert(assertions::TypeKeyword);
        registry.insert(assertions::EnumKeyword);
        registry.insert(assertions::ConstKeyword);

        // --- Numeric ---
        registry.insert(numeric::MinimumKeyword);
        registry.insert(numeric::MaximumKeyword);
        registry.insert(numeric::ExclusiveMinimumKeyword);
        registry.insert(numeric::ExclusiveMaximumKeyword);
        registry.insert(numeric::MultipleOfKeyword);

        // --- String ---
        registry.insert(string::MinLengthKeyword);
        registry.insert(string::MaxLengthKeyword);
        registry.insert(string::PatternKeyword);

        // --- Format ---
        registry.insert(format::FormatKeyword);

        // --- Objects ---
        registry.insert(objects::PropertiesKeyword);
        registry.insert(objects::RequiredKeyword);
        registry.insert(objects::AdditionalPropertiesKeyword);
        registry.insert(objects::PatternPropertiesKeyword);
        registry.insert(objects::PropertyNamesKeyword);
        registry.insert(objects::MinPropertiesKeyword);
        registry.insert(objects::MaxPropertiesKeyword);
        registry.insert(objects::DependentRequiredKeyword);
        registry.insert(objects::DependentSchemasKeyword);
        registry.insert(objects::DependenciesKeyword);

        // --- Arrays ---
        registry.insert(arrays::ItemsKeyword);
        registry.insert(arrays::PrefixItemsKeyword);
        registry.insert(arrays::MinItemsKeyword);
        registry.insert(arrays::MaxItemsKeyword);
        registry.insert(arrays::UniqueItemsKeyword);
        registry.insert(arrays::ContainsKeyword);
        registry.insert(arrays::MinContainsKeyword);
        registry.insert(arrays::MaxContainsKeyword);

        // --- Applicators ---
        registry.insert(applicator::AllOfKeyword);
        registry.insert(applicator::AnyOfKeyword);
        registry.insert(applicator::OneOfKeyword);
        registry.insert(applicator::NotKeyword);
        registry.insert(applicator::IfKeyword);
        registry.insert(applicator::ThenKeyword);
        registry.insert(applicator::ElseKeyword);

        registry
    }

    fn insert(&mut self, kw: impl Keyword + 'static) {
        let name = kw.name().to_string();
        self.keywords.insert(name, Box::new(kw));
    }

    /// Look up a keyword by name.
    pub fn get(&self, name: &str) -> Option<&dyn Keyword> {
        self.keywords.get(name).map(|b| b.as_ref())
    }

    /// Number of registered keywords.
    pub fn len(&self) -> usize {
        self.keywords.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Helper shared across keyword modules
// ---------------------------------------------------------------------------

/// Normalise a keyword value that may be a single string or an array of
/// strings into a flat list.
pub(crate) fn ensure_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}

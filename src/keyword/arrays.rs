use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// items
// ---------------------------------------------------------------------------

pub struct ItemsKeyword;
impl Keyword for ItemsKeyword {
    fn name(&self) -> &'static str {
        "items"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        items: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "array") {
            return vec![];
        }

        let arr = instance.as_array().unwrap();

        // Determine where prefixItems ends.
        let prefix_len = schema
            .get("prefixItems")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        let total = arr.len();
        if total <= prefix_len {
            return vec![];
        }

        let mut errors = Vec::new();

        if items == &Value::Bool(false) {
            let extra_count = total - prefix_len;
            return vec![ValidationError::new(format!(
                "Expected at most {} items, but found {} ({} extra)",
                prefix_len,
                total,
                extra_count
            ))
            .with_keyword("items")
            .with_instance(instance.clone())];
        }

        // items is a schema — apply to remaining elements.
        for (i, item) in arr.iter().enumerate().skip(prefix_len) {
            let child_errors = ctx.descend(item, items, &i.to_string());
            for mut err in child_errors {
                err.schema_path.insert(0, "items".into());
                errors.push(err);
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// prefixItems
// ---------------------------------------------------------------------------

pub struct PrefixItemsKeyword;
impl Keyword for PrefixItemsKeyword {
    fn name(&self) -> &'static str {
        "prefixItems"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        prefix_items: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "array") {
            return vec![];
        }

        let schemas = match prefix_items.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        let arr = instance.as_array().unwrap();
        let mut errors = Vec::new();

        for (i, sub_schema) in schemas.iter().enumerate() {
            if let Some(item) = arr.get(i) {
                let child_errors = ctx.descend(item, sub_schema, &i.to_string());
                for mut err in child_errors {
                    err.schema_path.insert(0, "prefixItems".into());
                    err.schema_path.insert(0, i.to_string());
                    errors.push(err);
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// minItems
// ---------------------------------------------------------------------------

pub struct MinItemsKeyword;
impl Keyword for MinItemsKeyword {
    fn name(&self) -> &'static str {
        "minItems"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        min: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "array") {
            return vec![];
        }
        let count = instance.as_array().unwrap().len();
        if let Some(min_val) = min.as_u64() {
            if count < min_val as usize {
                return vec![ValidationError::new(format!(
                    "array has {} items, but minimum is {}",
                    count, min_val
                ))
                .with_keyword("minItems")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

// ---------------------------------------------------------------------------
// maxItems
// ---------------------------------------------------------------------------

pub struct MaxItemsKeyword;
impl Keyword for MaxItemsKeyword {
    fn name(&self) -> &'static str {
        "maxItems"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        max: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "array") {
            return vec![];
        }
        let count = instance.as_array().unwrap().len();
        if let Some(max_val) = max.as_u64() {
            if count > max_val as usize {
                return vec![ValidationError::new(format!(
                    "array has {} items, but maximum is {}",
                    count, max_val
                ))
                .with_keyword("maxItems")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

// ---------------------------------------------------------------------------
// uniqueItems
// ---------------------------------------------------------------------------

pub struct UniqueItemsKeyword;
impl Keyword for UniqueItemsKeyword {
    fn name(&self) -> &'static str {
        "uniqueItems"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        unique: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if unique != &Value::Bool(true) {
            return vec![];
        }
        if !ctx.is_type(instance, "array") {
            return vec![];
        }

        let arr = instance.as_array().unwrap();

        // serde_json::Value implements Hash + Eq, but NaN handling is tricky.
        // We build a set of canonical-JSON-encoded strings as a robust dedup
        // strategy (also matches JSON Schema's value-based uniqueness).
        let mut seen = HashSet::new();
        for item in arr {
            let key = serde_json::to_string(item).unwrap_or_default();
            if !seen.insert(key) {
                return vec![ValidationError::new(format!(
                    "array contains duplicate value: {}",
                    item
                ))
                .with_keyword("uniqueItems")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

// ---------------------------------------------------------------------------
// contains
// ---------------------------------------------------------------------------

pub struct ContainsKeyword;
impl Keyword for ContainsKeyword {
    fn name(&self) -> &'static str {
        "contains"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        sub_schema: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "array") {
            return vec![];
        }

        let arr = instance.as_array().unwrap();
        for (i, item) in arr.iter().enumerate() {
            let child_errors = ctx.descend(item, sub_schema, &i.to_string());
            if child_errors.is_empty() {
                return vec![]; // at least one matches
            }
        }

        vec![ValidationError::new(
            "array does not contain any element matching the schema",
        )
        .with_keyword("contains")
        .with_instance(instance.clone())]
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::compiler::Validator;

    #[test]
    fn test_items_valid() {
        let v = Validator::new(serde_json::json!({"items": {"type": "integer"}}));
        assert!(v.is_valid(&serde_json::json!([1, 2, 3])));
    }

    #[test]
    fn test_items_invalid() {
        let v = Validator::new(serde_json::json!({"items": {"type": "integer"}}));
        assert!(!v.is_valid(&serde_json::json!([1, "x", 3])));
    }

    #[test]
    fn test_items_false() {
        let v = Validator::new(serde_json::json!({
            "prefixItems": [{"type": "integer"}],
            "items": false
        }));
        assert!(v.is_valid(&serde_json::json!([1])));
        assert!(!v.is_valid(&serde_json::json!([1, 2])));
    }

    #[test]
    fn test_prefix_items() {
        let v = Validator::new(serde_json::json!({
            "prefixItems": [{"type": "string"}, {"type": "integer"}]
        }));
        assert!(v.is_valid(&serde_json::json!(["hello", 42])));
        assert!(!v.is_valid(&serde_json::json!([1, "hello"])));
    }

    #[test]
    fn test_min_items() {
        let v = Validator::new(serde_json::json!({"minItems": 2}));
        assert!(v.is_valid(&serde_json::json!([1, 2])));
        assert!(!v.is_valid(&serde_json::json!([1])));
    }

    #[test]
    fn test_max_items() {
        let v = Validator::new(serde_json::json!({"maxItems": 2}));
        assert!(v.is_valid(&serde_json::json!([1])));
        assert!(!v.is_valid(&serde_json::json!([1, 2, 3])));
    }

    #[test]
    fn test_unique_items() {
        let v = Validator::new(serde_json::json!({"uniqueItems": true}));
        assert!(v.is_valid(&serde_json::json!([1, 2, 3])));
        assert!(!v.is_valid(&serde_json::json!([1, 2, 1])));
    }

    #[test]
    fn test_contains() {
        let v = Validator::new(serde_json::json!({
            "contains": {"type": "string"}
        }));
        assert!(v.is_valid(&serde_json::json!([1, "hello", 3])));
        assert!(!v.is_valid(&serde_json::json!([1, 2, 3])));
    }
}

use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

// ---------------------------------------------------------------------------
// allOf
// ---------------------------------------------------------------------------

pub struct AllOfKeyword;
impl Keyword for AllOfKeyword {
    fn name(&self) -> &'static str {
        "allOf"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        subschemas: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        let schemas = match subschemas.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        let mut errors = Vec::new();
        for (i, sub_schema) in schemas.iter().enumerate() {
            let child_errors = ctx.iter_errors(instance, sub_schema);
            for mut err in child_errors {
                err.schema_path.insert(0, i.to_string());
                errors.push(err);
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// anyOf
// ---------------------------------------------------------------------------

pub struct AnyOfKeyword;
impl Keyword for AnyOfKeyword {
    fn name(&self) -> &'static str {
        "anyOf"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        subschemas: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        let schemas = match subschemas.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        for sub_schema in schemas {
            if ctx.iter_errors(instance, sub_schema).is_empty() {
                return vec![];
            }
        }

        vec![ValidationError::new(
            "instance does not match any of the schemas in anyOf",
        )
        .with_keyword("anyOf")
        .with_instance(instance.clone())]
    }
}

// ---------------------------------------------------------------------------
// oneOf
// ---------------------------------------------------------------------------

pub struct OneOfKeyword;
impl Keyword for OneOfKeyword {
    fn name(&self) -> &'static str {
        "oneOf"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        subschemas: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        let schemas = match subschemas.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        let mut valid_count = 0;
        for sub_schema in schemas {
            if ctx.iter_errors(instance, sub_schema).is_empty() {
                valid_count += 1;
            }
        }

        match valid_count {
            0 => vec![ValidationError::new(
                "instance does not match any schema in oneOf",
            )
            .with_keyword("oneOf")
            .with_instance(instance.clone())],
            1 => vec![],
            _ => vec![ValidationError::new(format!(
                "instance matches {} schemas in oneOf (exactly 1 required)",
                valid_count
            ))
            .with_keyword("oneOf")
            .with_instance(instance.clone())],
        }
    }
}

// ---------------------------------------------------------------------------
// not
// ---------------------------------------------------------------------------

pub struct NotKeyword;
impl Keyword for NotKeyword {
    fn name(&self) -> &'static str {
        "not"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        sub_schema: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if ctx.iter_errors(instance, sub_schema).is_empty() {
            // Matched — which is wrong for "not".
            vec![ValidationError::new("instance should not be valid against `not` schema")
                .with_keyword("not")
                .with_instance(instance.clone())]
        } else {
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// if
// ---------------------------------------------------------------------------

/// The `if` keyword does not produce errors on its own; it only signals
/// whether `then` or `else` should be applied.  We track this via a flag
/// that `then`/`else` can read.
///
/// Implementation note: we evaluate `if` inline here so that `then`/`else`
/// keyword validators can simply check `if_passed`.
pub struct IfKeyword;
impl Keyword for IfKeyword {
    fn name(&self) -> &'static str {
        "if"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        sub_schema: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        // `if` itself never produces user-visible errors.
        // It is recorded as a side-effect: the `then` / `else` keywords will
        // re-evaluate the `if` sub-schema themselves (to avoid shared mutable
        // state).  So this implementation is a no-op — the work is done in
        // `then` and `else`.
        let _ = ctx;
        let _ = sub_schema;
        let _ = instance;
        vec![]
    }
}

// ---------------------------------------------------------------------------
// then
// ---------------------------------------------------------------------------

pub struct ThenKeyword;
impl Keyword for ThenKeyword {
    fn name(&self) -> &'static str {
        "then"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        then_schema: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError> {
        // Only applies if `if` sub-schema passed.
        let if_schema = match schema.get("if") {
            Some(s) => s,
            None => return vec![], // no "if" → "then" has no effect
        };

        let if_passed = ctx.iter_errors(instance, if_schema).is_empty();
        if !if_passed {
            return vec![]; // "if" failed → "then" is skipped
        }

        let errors = ctx.iter_errors(instance, then_schema);
        errors
    }
}

// ---------------------------------------------------------------------------
// else
// ---------------------------------------------------------------------------

pub struct ElseKeyword;
impl Keyword for ElseKeyword {
    fn name(&self) -> &'static str {
        "else"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        else_schema: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError> {
        // Only applies if `if` sub-schema **failed**.
        let if_schema = match schema.get("if") {
            Some(s) => s,
            None => return vec![], // no "if" → "else" has no effect
        };

        let if_passed = ctx.iter_errors(instance, if_schema).is_empty();
        if if_passed {
            return vec![]; // "if" passed → "else" is skipped
        }

        let errors = ctx.iter_errors(instance, else_schema);
        errors
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Validator;

    #[test]
    fn test_all_of_both_pass() {
        let v = Validator::new(serde_json::json!({
            "allOf": [{"type": "string"}, {"minLength": 3}]
        }));
        assert!(v.is_valid(&Value::String("hello".into())));
    }

    #[test]
    fn test_all_of_one_fails() {
        let v = Validator::new(serde_json::json!({
            "allOf": [{"type": "string"}, {"minLength": 10}]
        }));
        assert!(!v.is_valid(&Value::String("hi".into())));
    }

    #[test]
    fn test_any_of_at_least_one() {
        let v = Validator::new(serde_json::json!({
            "anyOf": [{"type": "string"}, {"type": "integer"}]
        }));
        assert!(v.is_valid(&Value::String("hi".into())));
        assert!(v.is_valid(&serde_json::json!(42)));
        assert!(!v.is_valid(&serde_json::json!(true)));
    }

    #[test]
    fn test_one_of_exactly_one() {
        let v = Validator::new(serde_json::json!({
            "oneOf": [
                {"type": "string"},
                {"type": "integer"}
            ]
        }));
        assert!(v.is_valid(&Value::String("hi".into())));
        assert!(v.is_valid(&serde_json::json!(42)));
        // Fails because 42 matches BOTH {"type": "integer"} AND {"type": "number"}... wait no.
        // Let's test with a value that matches both: 42 matches "integer" and "number" types.
    }

    #[test]
    fn test_one_of_matches_both() {
        let v = Validator::new(serde_json::json!({
            "oneOf": [
                {"type": "integer"},
                {"type": "number"}
            ]
        }));
        // 42 matches BOTH — should fail.
        assert!(!v.is_valid(&serde_json::json!(42)));
    }

    #[test]
    fn test_not() {
        let v = Validator::new(serde_json::json!({
            "not": {"type": "string"}
        }));
        assert!(v.is_valid(&serde_json::json!(42)));
        assert!(!v.is_valid(&Value::String("hi".into())));
    }

    #[test]
    fn test_if_then_passed() {
        let v = Validator::new(serde_json::json!({
            "if": {"type": "string"},
            "then": {"minLength": 5}
        }));
        // string with length >= 5 → OK
        assert!(v.is_valid(&Value::String("hello".into())));
        // string with length < 5 → then fails
        assert!(!v.is_valid(&Value::String("hi".into())));
        // not a string → if fails, then is skipped → OK
        assert!(v.is_valid(&serde_json::json!(42)));
    }

    #[test]
    fn test_if_else() {
        let v = Validator::new(serde_json::json!({
            "if": {"type": "string"},
            "else": {"type": "integer"}
        }));
        // not a string → else applies → must be integer
        assert!(v.is_valid(&serde_json::json!(42)));
        // not a string AND not integer → else fails
        assert!(!v.is_valid(&serde_json::json!(true)));
    }
}

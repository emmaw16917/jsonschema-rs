use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

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
            vec![ValidationError::new("instance should not be valid against `not` schema")
                .with_keyword("not")
                .with_instance(instance.clone())]
        } else {
            vec![]
        }
    }
}

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
        let _ = ctx;
        let _ = sub_schema;
        let _ = instance;
        vec![]
    }
}

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
        let if_schema = match schema.get("if") {
            Some(s) => s,
            None => return vec![],
        };

        let if_passed = ctx.iter_errors(instance, if_schema).is_empty();
        if !if_passed {
            return vec![];
        }

        let errors = ctx.iter_errors(instance, then_schema);
        errors
    }
}

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
        let if_schema = match schema.get("if") {
            Some(s) => s,
            None => return vec![],
        };

        let if_passed = ctx.iter_errors(instance, if_schema).is_empty();
        if if_passed {
            return vec![];
        }

        let errors = ctx.iter_errors(instance, else_schema);
        errors
    }
}

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
    }

    #[test]
    fn test_one_of_matches_both() {
        let v = Validator::new(serde_json::json!({
            "oneOf": [
                {"type": "integer"},
                {"type": "number"}
            ]
        }));
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
        assert!(v.is_valid(&Value::String("hello".into())));
        assert!(!v.is_valid(&Value::String("hi".into())));
        assert!(v.is_valid(&serde_json::json!(42)));
    }

    #[test]
    fn test_if_else() {
        let v = Validator::new(serde_json::json!({
            "if": {"type": "string"},
            "else": {"type": "integer"}
        }));
        assert!(v.is_valid(&serde_json::json!(42)));
        assert!(!v.is_valid(&serde_json::json!(true)));
    }
}
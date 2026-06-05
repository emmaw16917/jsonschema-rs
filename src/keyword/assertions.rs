use crate::error::ValidationError;
use crate::instance::deep_equal;
use crate::keyword::ensure_string_list;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

// ---------------------------------------------------------------------------
// type
// ---------------------------------------------------------------------------

pub struct TypeKeyword;
impl Keyword for TypeKeyword {
    fn name(&self) -> &'static str {
        "type"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        types: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        let type_list = ensure_string_list(types);
        if type_list.is_empty() {
            return vec![];
        }

        let matched = type_list.iter().any(|t| ctx.is_type(instance, t));
        if !matched {
            vec![ValidationError::new(format!(
                "{} is not of type {}",
                instance,
                type_list.join(", ")
            ))
            .with_keyword("type")
            .with_instance(instance.clone())]
        } else {
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// enum
// ---------------------------------------------------------------------------

pub struct EnumKeyword;
impl Keyword for EnumKeyword {
    fn name(&self) -> &'static str {
        "enum"
    }

    fn validate(
        &self,
        _ctx: &ValidationContext,
        values: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        let arr = match values.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        if arr.iter().any(|v| deep_equal(v, instance)) {
            vec![]
        } else {
            vec![ValidationError::new(format!(
                "{} is not one of the allowed values",
                instance
            ))
            .with_keyword("enum")
            .with_instance(instance.clone())]
        }
    }
}

// ---------------------------------------------------------------------------
// const
// ---------------------------------------------------------------------------

pub struct ConstKeyword;
impl Keyword for ConstKeyword {
    fn name(&self) -> &'static str {
        "const"
    }

    fn validate(
        &self,
        _ctx: &ValidationContext,
        expected: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if deep_equal(expected, instance) {
            vec![]
        } else {
            vec![ValidationError::new(format!(
                "{} does not equal const value {}",
                instance, expected
            ))
            .with_keyword("const")
            .with_instance(instance.clone())]
        }
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
    fn test_type_string_valid() {
        let v = Validator::new(serde_json::json!({"type": "string"}));
        assert!(v.is_valid(&Value::String("hi".into())));
    }

    #[test]
    fn test_type_string_invalid() {
        let v = Validator::new(serde_json::json!({"type": "string"}));
        let errs = v.iter_errors(&serde_json::json!(42));
        assert!(!errs.is_empty());
        assert_eq!(errs[0].keyword.as_deref(), Some("type"));
    }

    #[test]
    fn test_type_multi() {
        let v = Validator::new(serde_json::json!({"type": ["string", "null"]}));
        assert!(v.is_valid(&Value::String("hi".into())));
        assert!(v.is_valid(&Value::Null));
        assert!(!v.is_valid(&serde_json::json!(42)));
    }

    #[test]
    fn test_enum_valid() {
        let v = Validator::new(serde_json::json!({"enum": [1, 2, 3]}));
        assert!(v.is_valid(&serde_json::json!(2)));
    }

    #[test]
    fn test_enum_invalid() {
        let v = Validator::new(serde_json::json!({"enum": [1, 2, 3]}));
        assert!(!v.is_valid(&serde_json::json!(99)));
    }

    #[test]
    fn test_const_valid() {
        let v = Validator::new(serde_json::json!({"const": "hello"}));
        assert!(v.is_valid(&Value::String("hello".into())));
    }

    #[test]
    fn test_const_invalid() {
        let v = Validator::new(serde_json::json!({"const": "hello"}));
        assert!(!v.is_valid(&Value::String("world".into())));
    }
}

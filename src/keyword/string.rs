use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

pub struct MinLengthKeyword;
impl Keyword for MinLengthKeyword {
    fn name(&self) -> &'static str {
        "minLength"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        min_len: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "string") {
            return vec![];
        }
        let s = instance.as_str().unwrap();
        let char_count = s.chars().count();
        if let Some(min) = min_len.as_u64() {
            if char_count < min as usize {
                return vec![ValidationError::new(format!(
                    "'{}' is shorter than minimum length of {}",
                    s, min
                ))
                .with_keyword("minLength")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

pub struct MaxLengthKeyword;
impl Keyword for MaxLengthKeyword {
    fn name(&self) -> &'static str {
        "maxLength"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        max_len: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "string") {
            return vec![];
        }
        let s = instance.as_str().unwrap();
        let char_count = s.chars().count();
        if let Some(max) = max_len.as_u64() {
            if char_count > max as usize {
                return vec![ValidationError::new(format!(
                    "'{}' is longer than maximum length of {}",
                    s, max
                ))
                .with_keyword("maxLength")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

pub struct PatternKeyword;
impl Keyword for PatternKeyword {
    fn name(&self) -> &'static str {
        "pattern"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        pattern: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "string") {
            return vec![];
        }
        let s = instance.as_str().unwrap();
        let pattern_str = pattern.as_str().unwrap();

        let matched = if let Some(re) = ctx.get_compiled_pattern(pattern_str) {
            re.is_match(s)
        } else if let Ok(re) = regex::Regex::new(pattern_str) {
            re.is_match(s)
        } else {
            return vec![];
        };

        if !matched {
            vec![ValidationError::new(format!(
                "'{}' does not match pattern '{}'",
                s, pattern_str
            ))
            .with_keyword("pattern")
            .with_instance(instance.clone())]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Validator;

    #[test]
    fn test_min_length_valid() {
        let v = Validator::new(serde_json::json!({"minLength": 3}));
        assert!(v.is_valid(&Value::String("abc".into())));
    }

    #[test]
    fn test_min_length_invalid() {
        let v = Validator::new(serde_json::json!({"minLength": 3}));
        assert!(!v.is_valid(&Value::String("ab".into())));
    }

    #[test]
    fn test_max_length() {
        let v = Validator::new(serde_json::json!({"maxLength": 5}));
        assert!(v.is_valid(&Value::String("hello".into())));
        assert!(!v.is_valid(&Value::String("too long".into())));
    }

    #[test]
    fn test_min_length_unicode() {
        let v = Validator::new(serde_json::json!({"minLength": 2}));
        assert!(v.is_valid(&Value::String("中文".into())));
        assert!(!v.is_valid(&Value::String("中".into())));
    }

    #[test]
    fn test_pattern_valid() {
        let v = Validator::new(serde_json::json!({"pattern": "^[A-Z][a-z]+$"}));
        assert!(v.is_valid(&Value::String("Alice".into())));
    }

    #[test]
    fn test_pattern_invalid() {
        let v = Validator::new(serde_json::json!({"pattern": "^[A-Z][a-z]+$"}));
        assert!(!v.is_valid(&Value::String("ALICE".into())));
    }

    #[test]
    fn test_string_keywords_skip_non_strings() {
        let v = Validator::new(serde_json::json!({"minLength": 3, "pattern": ".*"}));
        assert!(v.is_valid(&serde_json::json!(42)));
    }
}

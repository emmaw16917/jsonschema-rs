use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

fn as_f64(v: &Value) -> Option<f64> {
    v.as_f64()
}

pub struct MinimumKeyword;
impl Keyword for MinimumKeyword {
    fn name(&self) -> &'static str {
        "minimum"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        minimum: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "number") {
            return vec![];
        }
        let min = as_f64(minimum);
        let val = as_f64(instance);
        match (min, val) {
            (Some(min), Some(val)) if val < min => {
                vec![ValidationError::new(format!(
                    "{} is less than the minimum of {}",
                    val, min
                ))
                .with_keyword("minimum")
                .with_instance(instance.clone())]
            }
            _ => vec![],
        }
    }
}

pub struct MaximumKeyword;
impl Keyword for MaximumKeyword {
    fn name(&self) -> &'static str {
        "maximum"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        maximum: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "number") {
            return vec![];
        }
        let max = as_f64(maximum);
        let val = as_f64(instance);
        match (max, val) {
            (Some(max), Some(val)) if val > max => {
                vec![ValidationError::new(format!(
                    "{} is greater than the maximum of {}",
                    val, max
                ))
                .with_keyword("maximum")
                .with_instance(instance.clone())]
            }
            _ => vec![],
        }
    }
}

pub struct ExclusiveMinimumKeyword;
impl Keyword for ExclusiveMinimumKeyword {
    fn name(&self) -> &'static str {
        "exclusiveMinimum"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        minimum: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "number") {
            return vec![];
        }
        let min = as_f64(minimum);
        let val = as_f64(instance);
        match (min, val) {
            (Some(min), Some(val)) if val <= min => {
                vec![ValidationError::new(format!(
                    "{} is not greater than exclusive minimum of {}",
                    val, min
                ))
                .with_keyword("exclusiveMinimum")
                .with_instance(instance.clone())]
            }
            _ => vec![],
        }
    }
}

pub struct ExclusiveMaximumKeyword;
impl Keyword for ExclusiveMaximumKeyword {
    fn name(&self) -> &'static str {
        "exclusiveMaximum"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        maximum: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "number") {
            return vec![];
        }
        let max = as_f64(maximum);
        let val = as_f64(instance);
        match (max, val) {
            (Some(max), Some(val)) if val >= max => {
                vec![ValidationError::new(format!(
                    "{} is not less than exclusive maximum of {}",
                    val, max
                ))
                .with_keyword("exclusiveMaximum")
                .with_instance(instance.clone())]
            }
            _ => vec![],
        }
    }
}

pub struct MultipleOfKeyword;
impl Keyword for MultipleOfKeyword {
    fn name(&self) -> &'static str {
        "multipleOf"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        multiple: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "number") {
            return vec![];
        }
        let divisor = as_f64(multiple);
        let val = as_f64(instance);
        match (divisor, val) {
            (Some(d), Some(v)) if d != 0.0 && d.is_finite() => {
                let quotient = v / d;
                if !quotient.is_finite() {
                    return vec![ValidationError::new(format!(
                        "{} is not a multiple of {}",
                        v, d
                    ))
                    .with_keyword("multipleOf")
                    .with_instance(instance.clone())];
                }
                let nearest = quotient.round();
                let diff = (quotient - nearest).abs();
                let tolerance = f64::EPSILON * quotient.abs().max(1.0) * 100.0;

                if diff > tolerance {
                    vec![ValidationError::new(format!(
                        "{} is not a multiple of {}",
                        v, d
                    ))
                    .with_keyword("multipleOf")
                    .with_instance(instance.clone())]
                } else {
                    vec![]
                }
            }
            (Some(d), _) if d == 0.0 => {
                vec![]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Validator;

    #[test]
    fn test_minimum_valid() {
        let v = Validator::new(serde_json::json!({"minimum": 10}));
        assert!(v.is_valid(&serde_json::json!(10)));
        assert!(v.is_valid(&serde_json::json!(15)));
    }

    #[test]
    fn test_minimum_invalid() {
        let v = Validator::new(serde_json::json!({"minimum": 10}));
        assert!(!v.is_valid(&serde_json::json!(5)));
    }

    #[test]
    fn test_maximum() {
        let v = Validator::new(serde_json::json!({"maximum": 100}));
        assert!(v.is_valid(&serde_json::json!(100)));
        assert!(!v.is_valid(&serde_json::json!(101)));
    }

    #[test]
    fn test_exclusive_minimum() {
        let v = Validator::new(serde_json::json!({"exclusiveMinimum": 10}));
        assert!(!v.is_valid(&serde_json::json!(10)));
        assert!(v.is_valid(&serde_json::json!(10.1)));
    }

    #[test]
    fn test_exclusive_maximum() {
        let v = Validator::new(serde_json::json!({"exclusiveMaximum": 100}));
        assert!(!v.is_valid(&serde_json::json!(100)));
        assert!(v.is_valid(&serde_json::json!(99)));
    }

    #[test]
    fn test_multiple_of() {
        let v = Validator::new(serde_json::json!({"multipleOf": 5}));
        assert!(v.is_valid(&serde_json::json!(10)));
        assert!(v.is_valid(&serde_json::json!(15)));
        assert!(!v.is_valid(&serde_json::json!(12)));
    }

    #[test]
    fn test_numeric_wrong_type_skips() {
        let v = Validator::new(serde_json::json!({"minimum": 10}));
        assert!(v.is_valid(&Value::String("hello".into())));
    }
}

use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use serde_json::Value;

// ---------------------------------------------------------------------------
// properties
// ---------------------------------------------------------------------------

pub struct PropertiesKeyword;
impl Keyword for PropertiesKeyword {
    fn name(&self) -> &'static str {
        "properties"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        properties: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let mut errors = Vec::new();
        if let Value::Object(props) = properties {
            for (prop_name, sub_schema) in props {
                if let Some(prop_value) = instance.get(prop_name) {
                    // descend into the child instance
                    let child_errors = ctx.descend(prop_value, sub_schema, prop_name);
                    for mut err in child_errors {
                        err.schema_path.insert(0, prop_name.clone());
                        errors.push(err);
                    }
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// required
// ---------------------------------------------------------------------------

pub struct RequiredKeyword;
impl Keyword for RequiredKeyword {
    fn name(&self) -> &'static str {
        "required"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        required: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let required_arr = match required.as_array() {
            Some(a) => a,
            None => return vec![],
        };

        let mut errors = Vec::new();
        for req_val in required_arr {
            if let Some(key) = req_val.as_str() {
                if !instance.get(key).is_some() {
                    errors.push(
                        ValidationError::new(format!("'{}' is a required property", key))
                            .with_keyword("required")
                            .with_instance(instance.clone()),
                    );
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// additionalProperties
// ---------------------------------------------------------------------------

pub struct AdditionalPropertiesKeyword;
impl Keyword for AdditionalPropertiesKeyword {
    fn name(&self) -> &'static str {
        "additionalProperties"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        additional: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let obj = instance.as_object().unwrap();

        // Collect keys covered by "properties".
        let covered_keys = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .map(|props| props.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        // Also collect keys covered by "patternProperties" (runtime match).
        let pattern_covered: Vec<String> = {
            let mut matches = Vec::new();
            if let Some(Value::Object(patterns)) = schema.get("patternProperties") {
                for key in obj.keys() {
                    for pattern_str in patterns.keys() {
                        if let Ok(re) = regex::Regex::new(pattern_str) {
                            if re.is_match(key) {
                                matches.push(key.clone());
                                break;
                            }
                        }
                    }
                }
            }
            matches
        };

        let mut errors = Vec::new();
        for key in obj.keys() {
            if covered_keys.contains(key) || pattern_covered.contains(key) {
                continue;
            }

            // additionalProperties: false → reject any extra
            if additional == &Value::Bool(false) {
                errors.push(
                    ValidationError::new(format!(
                        "Additional properties are not allowed ('{}' was unexpected)",
                        key
                    ))
                    .with_keyword("additionalProperties")
                    .with_instance(instance.clone()),
                );
            } else if additional.is_object() {
                // additionalProperties with a schema → validate the value
                let child_errors = ctx.descend(&obj[key], additional, key);
                errors.extend(child_errors);
            }
            // additionalProperties: true → allow (silent)
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// patternProperties
// ---------------------------------------------------------------------------

pub struct PatternPropertiesKeyword;
impl Keyword for PatternPropertiesKeyword {
    fn name(&self) -> &'static str {
        "patternProperties"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        pattern_props: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let patterns = match pattern_props.as_object() {
            Some(p) => p,
            None => return vec![],
        };

        let obj = instance.as_object().unwrap();
        let mut errors = Vec::new();

        for (pattern_str, sub_schema) in patterns {
            let re = match regex::Regex::new(pattern_str) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for (key, val) in obj.iter() {
                if re.is_match(key) {
                    let child_errors = ctx.descend(val, sub_schema, key);
                    for mut err in child_errors {
                        err.schema_path.insert(0, pattern_str.clone());
                        errors.push(err);
                    }
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// propertyNames
// ---------------------------------------------------------------------------

pub struct PropertyNamesKeyword;
impl Keyword for PropertyNamesKeyword {
    fn name(&self) -> &'static str {
        "propertyNames"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        name_schema: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let mut errors = Vec::new();
        if let Value::Object(obj) = instance {
            for key in obj.keys() {
                let key_value = Value::String(key.clone());
                let child_errors = ctx.iter_errors(&key_value, name_schema);
                for mut err in child_errors {
                    err.instance_path.insert(0, key.clone());
                    errors.push(err);
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// minProperties
// ---------------------------------------------------------------------------

pub struct MinPropertiesKeyword;
impl Keyword for MinPropertiesKeyword {
    fn name(&self) -> &'static str {
        "minProperties"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        min: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }
        let count = instance.as_object().unwrap().len();
        if let Some(min_val) = min.as_u64() {
            if count < min_val as usize {
                return vec![ValidationError::new(format!(
                    "object has {} properties, but minimum is {}",
                    count, min_val
                ))
                .with_keyword("minProperties")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

// ---------------------------------------------------------------------------
// maxProperties
// ---------------------------------------------------------------------------

pub struct MaxPropertiesKeyword;
impl Keyword for MaxPropertiesKeyword {
    fn name(&self) -> &'static str {
        "maxProperties"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        max: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }
        let count = instance.as_object().unwrap().len();
        if let Some(max_val) = max.as_u64() {
            if count > max_val as usize {
                return vec![ValidationError::new(format!(
                    "object has {} properties, but maximum is {}",
                    count, max_val
                ))
                .with_keyword("maxProperties")
                .with_instance(instance.clone())];
            }
        }
        vec![]
    }
}

// ---------------------------------------------------------------------------
// dependentRequired
// ---------------------------------------------------------------------------

pub struct DependentRequiredKeyword;
impl Keyword for DependentRequiredKeyword {
    fn name(&self) -> &'static str {
        "dependentRequired"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        dependents: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let deps = match dependents.as_object() {
            Some(d) => d,
            None => return vec![],
        };

        let obj = instance.as_object().unwrap();
        let mut errors = Vec::new();

        for (prop_name, required_arr) in deps {
            // Only check if the property is present in the instance
            if !obj.contains_key(prop_name) {
                continue;
            }

            let required = match required_arr.as_array() {
                Some(a) => a,
                None => continue,
            };

            for req_val in required {
                if let Some(key) = req_val.as_str() {
                    if !obj.contains_key(key) {
                        errors.push(
                            ValidationError::new(format!(
                                "'{}' is present, so '{}' is also required",
                                prop_name, key
                            ))
                            .with_keyword("dependentRequired")
                            .with_instance(instance.clone()),
                        );
                    }
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// dependentSchemas
// ---------------------------------------------------------------------------

pub struct DependentSchemasKeyword;
impl Keyword for DependentSchemasKeyword {
    fn name(&self) -> &'static str {
        "dependentSchemas"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        dependents: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let deps = match dependents.as_object() {
            Some(d) => d,
            None => return vec![],
        };

        let obj = instance.as_object().unwrap();
        let mut errors = Vec::new();

        for (prop_name, sub_schema) in deps {
            if !obj.contains_key(prop_name) {
                continue;
            }

            let child_errors = ctx.iter_errors(instance, sub_schema);
            for mut err in child_errors {
                err.schema_path.insert(0, prop_name.clone());
                errors.push(err);
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// dependencies (legacy Draft 4/6/7 — compatibility)
// ---------------------------------------------------------------------------

pub struct DependenciesKeyword;
impl Keyword for DependenciesKeyword {
    fn name(&self) -> &'static str {
        "dependencies"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        dependents: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "object") {
            return vec![];
        }

        let deps = match dependents.as_object() {
            Some(d) => d,
            None => return vec![],
        };

        let obj = instance.as_object().unwrap();
        let mut errors = Vec::new();

        for (prop_name, dep_value) in deps {
            // Only check if the property is present in the instance
            if !obj.contains_key(prop_name) {
                continue;
            }

            if let Value::Array(required) = dep_value {
                // Array form — like dependentRequired
                for req_val in required {
                    if let Some(key) = req_val.as_str() {
                        if !obj.contains_key(key) {
                            errors.push(
                                ValidationError::new(format!(
                                    "'{}' is present, so '{}' is also required",
                                    prop_name, key
                                ))
                                .with_keyword("dependencies")
                                .with_instance(instance.clone()),
                            );
                        }
                    }
                }
            } else {
                // Schema form — like dependentSchemas
                let child_errors = ctx.iter_errors(instance, dep_value);
                for mut err in child_errors {
                    err.schema_path.insert(0, prop_name.clone());
                    errors.push(err);
                }
            }
        }
        errors
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::compiler::Validator;

    #[test]
    fn test_properties_valid() {
        let v = Validator::new(serde_json::json!({
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        }));
        let valid = serde_json::json!({"name": "Alice", "age": 30});
        assert!(v.is_valid(&valid));
    }

    #[test]
    fn test_properties_invalid() {
        let v = Validator::new(serde_json::json!({
            "properties": { "name": { "type": "string" } }
        }));
        let invalid = serde_json::json!({"name": 42});
        assert!(!v.is_valid(&invalid));
    }

    #[test]
    fn test_required_valid() {
        let v = Validator::new(serde_json::json!({"required": ["name"]}));
        assert!(v.is_valid(&serde_json::json!({"name": "Alice"})));
    }

    #[test]
    fn test_required_invalid() {
        let v = Validator::new(serde_json::json!({"required": ["name"]}));
        assert!(!v.is_valid(&serde_json::json!({})));
    }

    #[test]
    fn test_additional_properties_false() {
        let v = Validator::new(serde_json::json!({
            "properties": { "name": { "type": "string" } },
            "additionalProperties": false
        }));
        assert!(v.is_valid(&serde_json::json!({"name": "Alice"})));
        assert!(!v.is_valid(&serde_json::json!({"name": "Alice", "extra": 1})));
    }

    #[test]
    fn test_pattern_properties() {
        let v = Validator::new(serde_json::json!({
            "patternProperties": { "^S_": { "type": "string" } }
        }));
        assert!(v.is_valid(&serde_json::json!({"S_name": "Alice"})));
        assert!(!v.is_valid(&serde_json::json!({"S_age": 42})));
    }

    #[test]
    fn test_min_properties() {
        let v = Validator::new(serde_json::json!({"minProperties": 2}));
        assert!(v.is_valid(&serde_json::json!({"a": 1, "b": 2})));
        assert!(!v.is_valid(&serde_json::json!({"a": 1})));
    }

    #[test]
    fn test_max_properties() {
        let v = Validator::new(serde_json::json!({"maxProperties": 1}));
        assert!(v.is_valid(&serde_json::json!({"a": 1})));
        assert!(!v.is_valid(&serde_json::json!({"a": 1, "b": 2})));
    }
}

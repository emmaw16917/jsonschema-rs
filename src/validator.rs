use crate::error::ValidationError;
use crate::keyword::KeywordRegistry;
use crate::refs::SchemaRegistry;
use crate::types::{CompiledSchema, TypeChecker};
use regex::Regex;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

// ValidationContext

/// 校验引擎核心，携带递归校验过程中所需的所有状态。
pub struct ValidationContext<'a> {
    pub compiled: &'a CompiledSchema,
    pub registry: &'a KeywordRegistry,
    pub type_checker: &'a TypeChecker,
    pub schema_registry: Option<&'a SchemaRegistry>,

    /// 当前在实例中的位置，如 `["properties", "items", "0"]`。
    pub instance_path: Vec<String>,

    /// 当前在 schema 中的位置，如 `["properties", "name", "minLength"]`。
    pub schema_path: Vec<String>,

    /// 预编译的正则表达式。
    pub precompiled: &'a HashMap<String, Regex>,

    /// 防止 `$ref` 无限递归，记录当前调用栈中正在解析的 `$ref` URI。
    visited_refs: RefCell<Vec<String>>,

    /// `$ref` 链的最大递归深度。
    max_ref_depth: usize,
}

impl<'a> ValidationContext<'a> {
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

    // Public validation API

    /// 递归校验 `instance` 是否符合 `schema`，返回所有错误（空 Vec 表示通过）。
    pub fn iter_errors(&self, instance: &Value, schema: &Value) -> Vec<ValidationError> {
        match schema {
            Value::Bool(true) => return vec![],
            Value::Bool(false) => {
                return vec![ValidationError::new("False schema does not allow anything")];
            }
            _ => {}
        }

        let effective_schema = if let Some(ref_val) = schema.get("$ref") {
            if let Some(ref_str) = ref_val.as_str() {
                {
                    let visited = self.visited_refs.borrow();
                    if visited.contains(&ref_str.to_string()) {
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

    /// 对子 schema 进行校验，将`path_component`追加到实例路径。
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

    pub fn is_type(&self, instance: &Value, type_name: &str) -> bool {
        self.type_checker.is_type(instance, type_name)
    }

    pub fn get_compiled_pattern(&self, pattern: &str) -> Option<&Regex> {
        self.precompiled.get(pattern)
    }

    // Internal helpers

    /// 遍历 schema 中所有已注册关键字，调用其 `validate()` 并收集错误。
    fn collect_keyword_errors(
        &self,
        instance: &Value,
        schema: &Value,
        skip_keyword: Option<&str>,
    ) -> Vec<ValidationError> {
        let mut errors: Vec<ValidationError> = Vec::new();

        if let Value::Object(obj) = schema {
            for (keyword_name, keyword_value) in obj {
                if let Some(skip) = skip_keyword {
                    if keyword_name == skip {
                        continue;
                    }
                }

                if let Some(keyword) = self.registry.get(keyword_name) {
                    let keyword_errors =
                        keyword.validate(self, keyword_value, instance, schema);
                    for mut err in keyword_errors {
                        if err.instance_path.is_empty() {
                            err.instance_path = self.instance_path.clone();
                        }
                        if err.schema_path.is_empty() {
                            err.schema_path = {
                                let mut sp = self.schema_path.clone();
                                sp.push(keyword_name.clone());
                                sp
                            };
                        } else {
                            let mut sp = self.schema_path.clone();
                            sp.push(keyword_name.clone());
                            sp.append(&mut err.schema_path.clone());
                            err.schema_path = sp;
                        }
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

    /// 解析 `$ref` 字符串：先尝试外部 SchemaRegistry，再尝试内部 JSON Pointer / $anchor。
    fn resolve_ref(&self, ref_val: &str) -> Option<Value> {
        // 1. 尝试外部 SchemaRegistry。
        if let Some(reg) = self.schema_registry {
            if let Some(result) = reg.resolve(&self.compiled.raw, ref_val) {
                return Some(result);
            }
        }

        // 2. 内部解析（以 '#' 开头）。
        if ref_val.starts_with('#') {
            let fragment = ref_val.trim_start_matches('#');
            if fragment.is_empty() {
                return Some(self.compiled.raw.clone());
            }
            if fragment.starts_with('/') {
                return crate::refs::resolve_pointer(&self.compiled.raw, fragment);
            }
            return crate::refs::resolve_anchor(&self.compiled.raw, fragment);
        }

        // 3. 包含 fragment 的完整 URI（如 "http://example.com/schema.json#foo"）。
        if let Some(hash_pos) = ref_val.find('#') {
            let (uri, fragment) = ref_val.split_at(hash_pos);
            let fragment = fragment.trim_start_matches('#');

            if let Some(reg) = self.schema_registry {
                if let Some(doc) = reg.get(uri) {
                    if fragment.is_empty() {
                        return Some(doc.clone());
                    }
                    if fragment.starts_with('/') {
                        return crate::refs::resolve_pointer(doc, fragment);
                    }
                    return crate::refs::resolve_anchor(doc, fragment);
                }
            }

            if let Some(doc) = crate::refs::find_by_id(&self.compiled.raw, uri) {
                if fragment.is_empty() {
                    return Some(doc.clone());
                }
                if fragment.starts_with('/') {
                    return crate::refs::resolve_pointer(&doc, fragment);
                }
                return crate::refs::resolve_anchor(&doc, fragment);
            }
        }

        // 4. 在根 schema 中按 $id 查找子 schema。
        if let Some(doc) = crate::refs::find_by_id(&self.compiled.raw, ref_val) {
            return Some(doc.clone());
        }

        None
    }
}

// tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CompiledSchema;

    fn make_ctx(schema: Value) -> ValidationContext<'static> {
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

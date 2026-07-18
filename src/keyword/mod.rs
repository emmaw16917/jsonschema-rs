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

/// 所有 JSON Schema 关键字校验器需实现此 trait。
pub trait Keyword: Send + Sync {
    fn name(&self) -> &'static str;

    /// 校验实例是否符合该关键字约束。成功返回空 Vec，否则返回错误列表。
    fn validate(
        &self,
        ctx: &ValidationContext,
        keyword_value: &Value,
        instance: &Value,
        schema: &Value,
    ) -> Vec<ValidationError>;
}

pub struct KeywordRegistry {
    keywords: HashMap<String, Box<dyn Keyword>>,
}

impl KeywordRegistry {
    /// 构建包含所有 Draft 2020-12 关键字的注册表。
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

    pub fn get(&self, name: &str) -> Option<&dyn Keyword> {
        self.keywords.get(name).map(|b| b.as_ref())
    }

    pub fn len(&self) -> usize {
        self.keywords.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }
}

/// 将关键字值（单个字符串或字符串数组）归一化为字符串列表。
pub(crate) fn ensure_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
        _ => vec![],
    }
}

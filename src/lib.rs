//! # jsonschema-rs
//!
//! 一个高性能 JSON Schema 校验器，支持 Draft 2020-12。
//!
//! ## 快速示例
//!
//! ```rust
//! use jsonschema_rs::Validator;
//!
//! let schema = serde_json::json!({
//!     "type": "object",
//!     "properties": {
//!         "name": { "type": "string", "minLength": 1 },
//!         "age":  { "type": "integer", "minimum": 0 }
//!     },
//!     "required": ["name"]
//! });
//!
//! let validator = Validator::new(schema);
//!
//! let valid = serde_json::json!({"name": "Alice", "age": 30});
//! assert!(validator.is_valid(&valid));
//!
//! let invalid = serde_json::json!({"name": ""});
//! assert!(!validator.is_valid(&invalid));
//! ```
//!
//! ## 支持的关键字
//!
//! - **断言:** `type`, `enum`, `const`
//! - **数值:** `minimum`, `maximum`, `exclusiveMinimum`,
//!   `exclusiveMaximum`, `multipleOf`
//! - **字符串:** `minLength`, `maxLength`, `pattern`
//! - **对象:** `properties`, `required`, `additionalProperties`,
//!   `patternProperties`, `propertyNames`, `minProperties`, `maxProperties`
//! - **数组:** `items`, `prefixItems`, `minItems`, `maxItems`,
//!   `uniqueItems`, `contains`
//! - **组合:** `allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`
//! - **引用:** `$ref`（内部 + 外部，通过 `SchemaRegistry`）

pub mod compiler;
pub mod error;
pub mod instance;
pub mod keyword;
pub mod refs;
pub mod types;
pub mod validator;

// 重导出主要公开类型
pub use compiler::Validator;
pub use error::ValidationError;
pub use refs::SchemaRegistry;
pub use types::TypeChecker;

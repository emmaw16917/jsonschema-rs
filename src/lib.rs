//! # jsonschema-rs
//!
//! A fast JSON Schema validator written in Rust, designed as a drop-in
//! replacement for the Python [`jsonschema`](https://github.com/python-jsonschema/jsonschema)
//! library.
//!
//! Supports **JSON Schema Draft 2020-12**.
//!
//! ## Quick example
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
//! ## Supported keywords
//!
//! - **Assertions:** `type`, `enum`, `const`
//! - **Numeric:** `minimum`, `maximum`, `exclusiveMinimum`,
//!   `exclusiveMaximum`, `multipleOf`
//! - **String:** `minLength`, `maxLength`, `pattern`
//! - **Objects:** `properties`, `required`, `additionalProperties`,
//!   `patternProperties`, `propertyNames`, `minProperties`, `maxProperties`
//! - **Arrays:** `items`, `prefixItems`, `minItems`, `maxItems`,
//!   `uniqueItems`, `contains`
//! - **Applicators:** `allOf`, `anyOf`, `oneOf`, `not`, `if`/`then`/`else`
//! - **References:** `$ref` (internal + external via `SchemaRegistry`)

pub mod compiler;
pub mod error;
pub mod instance;
pub mod keyword;
pub mod refs;
pub mod types;
pub mod validator;

// Re-export the main user-facing types.
pub use compiler::Validator;
pub use error::ValidationError;
pub use refs::SchemaRegistry;
pub use types::TypeChecker;

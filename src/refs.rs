use serde_json::Value;
use std::collections::HashMap;

/// A registry that caches JSON Schema documents by their `$id`, and can
/// resolve `$ref` URIs (including JSON Pointer fragments) against them.
///
/// Corresponds to Python `jsonschema`'s `RefResolver`.
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    /// Mapping from resolved URI → schema document.
    store: HashMap<String, Value>,
}

impl SchemaRegistry {
    /// Register a schema document under the given URI.
    pub fn add(&mut self, uri: impl Into<String>, schema: Value) {
        self.store.insert(uri.into(), schema);
    }

    /// Resolve a `$ref` string (e.g. `"#/definitions/Foo"` or
    /// `"/schemas/geo.json#"`) relative to `base_doc`.
    ///
    /// Returns `None` if the reference cannot be resolved.
    pub fn resolve(&self, base_doc: &Value, ref_val: &str) -> Option<Value> {
        if ref_val.starts_with('#') {
            // Internal reference — JSON Pointer relative to base_doc.
            let pointer = ref_val.trim_start_matches('#');
            if pointer.is_empty() {
                return Some(base_doc.clone());
            }
            resolve_pointer(base_doc, pointer)
        } else if let Some(hash_pos) = ref_val.find('#') {
            // External reference with fragment.
            let (uri, pointer) = ref_val.split_at(hash_pos);
            let doc = self.store.get(uri)?;
            let pointer = pointer.trim_start_matches('#');
            if pointer.is_empty() {
                Some(doc.clone())
            } else {
                resolve_pointer(doc, pointer)
            }
        } else {
            // External reference without fragment — whole document.
            self.store.get(ref_val).cloned()
        }
    }
}

/// Walk a JSON Pointer (RFC 6901) string against a document, returning the
/// value at that path.
///
/// Examples:
/// * `"/definitions/Foo"` → `doc["definitions"]["Foo"]`
/// * `"/items/0"` → `doc["items"][0]`
/// * `"/a/b~1c"` → `doc["a"]["b/c"]`  (escaped `/` as `~1`)
pub fn resolve_pointer(doc: &Value, pointer: &str) -> Option<Value> {
    if pointer.is_empty() {
        return Some(doc.clone());
    }

    let segments = pointer
        .split('/')
        .skip(1) // first segment is empty because pointer starts with '/'
        .map(unescape_json_pointer);

    let mut current = doc;
    for segment in segments {
        current = if let Value::Object(obj) = current {
            obj.get(&segment)?
        } else if let Value::Array(arr) = current {
            let idx: usize = segment.parse().ok()?;
            arr.get(idx)?
        } else {
            return None;
        };
    }
    Some(current.clone())
}

/// Unescape a single JSON Pointer reference token.
///
/// `~1` → `/`
/// `~0` → `~`
/// `~` followed by anything else is invalid per RFC 6901; we treat it
/// literally.
fn unescape_json_pointer(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_pointer_object() {
        let doc = serde_json::json!({
            "definitions": {
                "Foo": { "type": "string" }
            }
        });
        let result = resolve_pointer(&doc, "/definitions/Foo");
        assert_eq!(result, Some(serde_json::json!({"type": "string"})));
    }

    #[test]
    fn test_internal_pointer_array() {
        let doc = serde_json::json!({
            "items": [{"type": "integer"}, {"type": "string"}]
        });
        let result = resolve_pointer(&doc, "/items/1");
        assert_eq!(result, Some(serde_json::json!({"type": "string"})));
    }

    #[test]
    fn test_escaped_slash() {
        let doc = serde_json::json!({
            "a": { "b/c": "hello" }
        });
        // "/a/b~1c" means "a" → "b/c"
        let result = resolve_pointer(&doc, "/a/b~1c");
        assert_eq!(result, Some(Value::String("hello".into())));
    }

    #[test]
    fn test_registry_external_ref() {
        let mut reg = SchemaRegistry::default();
        reg.add(
            "http://example.com/geo.json",
            serde_json::json!({
                "definitions": {
                    "Point": { "type": "object" }
                }
            }),
        );
        let base = serde_json::json!({});
        let result = reg.resolve(
            &base,
            "http://example.com/geo.json#/definitions/Point",
        );
        assert_eq!(result, Some(serde_json::json!({"type": "object"})));
    }

    #[test]
    fn test_empty_pointer() {
        let doc = serde_json::json!({"a": 1});
        let result = resolve_pointer(&doc, "");
        assert_eq!(result, Some(doc));
    }
}

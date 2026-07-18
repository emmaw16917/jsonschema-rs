use serde_json::Value;
use std::collections::HashMap;

//按 `$id` 缓存 JSON Schema 文档，支持解析 `$ref` URI（含 JSON Pointer 片段）。
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    pub(crate) store: HashMap<String, Value>,
}

impl SchemaRegistry {
    //注册一个 schema 文档（关联给定 URI）。
    pub fn add(&mut self, uri: impl Into<String>, schema: Value) {
        self.store.insert(uri.into(), schema);
    }

    //按 URI 查找文档。
    pub fn get(&self, uri: &str) -> Option<&Value> {
        self.store.get(uri)
    }

    //解析 `$ref` 字符串，返回 `None` 表示无法解析。
    pub fn resolve(&self, base_doc: &Value, ref_val: &str) -> Option<Value> {
        if ref_val.starts_with('#') {
            let fragment = ref_val.trim_start_matches('#');
            if fragment.is_empty() {
                return Some(base_doc.clone());
            }
            if fragment.starts_with('/') {
                return resolve_pointer(base_doc, fragment);
            }
            return resolve_anchor(base_doc, fragment);
        } else if let Some(hash_pos) = ref_val.find('#') {
            let (uri, pointer) = ref_val.split_at(hash_pos);
            let doc = self.store.get(uri)?;
            let pointer = pointer.trim_start_matches('#');
            if pointer.is_empty() {
                Some(doc.clone())
            } else if pointer.starts_with('/') {
                resolve_pointer(doc, pointer)
            } else {
                resolve_anchor(doc, pointer)
            }
        } else {
            self.store.get(ref_val).cloned()
        }
    }
}

//遍历 schema 树，查找 `$id` 匹配给定 URI 的节点。
pub fn find_by_id(doc: &Value, target_id: &str) -> Option<Value> {
    if let Value::Object(obj) = doc {
        if let Some(Value::String(id)) = obj.get("$id") {
            if id == target_id {
                return Some(doc.clone());
            }
        }
        for val in obj.values() {
            if let result @ Some(_) = find_by_id(val, target_id) {
                return result;
            }
        }
    } else if let Value::Array(arr) = doc {
        for item in arr {
            if let result @ Some(_) = find_by_id(item, target_id) {
                return result;
            }
        }
    }
    None
}

/// 按 JSON Pointer（RFC 6901）路径定位文档中的值。
pub fn resolve_pointer(doc: &Value, pointer: &str) -> Option<Value> {
    if pointer.is_empty() {
        return Some(doc.clone());
    }

    let segments = pointer
        .split('/')
        .skip(1)
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

/// 解析 `$anchor` 引用：遍历 schema 树，返回 `$anchor` 匹配的节点。
pub fn resolve_anchor(doc: &Value, anchor_name: &str) -> Option<Value> {
    let name = anchor_name.trim_start_matches('#');
    if name.is_empty() {
        return Some(doc.clone());
    }
    search_anchor(doc, name)
}

fn search_anchor(node: &Value, name: &str) -> Option<Value> {
    if let Value::Object(obj) = node {
        if let Some(Value::String(a)) = obj.get("$anchor") {
            if a == name {
                return Some(node.clone());
            }
        }
    }

    match node {
        Value::Object(obj) => {
            if obj.contains_key("$ref") {
                for val in obj.values() {
                    if let result @ Some(_) = search_anchor(val, name) {
                        return result;
                    }
                }
                return None;
            }

            for (key, val) in obj {
                if key == "$ref" {
                    continue;
                }
                if let result @ Some(_) = search_anchor(val, name) {
                    return result;
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let result @ Some(_) = search_anchor(item, name) {
                    return result;
                }
            }
            None
        }
        _ => None,
    }
}

/// JSON Pointer 转义字符反转：`~1` → `/`，`~0` → `~`。
fn unescape_json_pointer(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

// tests

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

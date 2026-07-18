use serde::Serialize;
use serde_json::Value;
use std::fmt;

//JSON 实例校验失败时的单条错误。
#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub message: String,

    //触发错误的关键字
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,

    //错误在实例中的位置
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub instance_path: Vec<String>,

    //错误在 schema 中的位置
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub schema_path: Vec<String>,

    //导致错误的实例片段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<Value>,
}

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            keyword: None,
            instance_path: Vec::new(),
            schema_path: Vec::new(),
            instance: None,
        }
    }

    pub fn with_keyword(mut self, kw: impl Into<String>) -> Self {
        self.keyword = Some(kw.into());
        self
    }

    pub fn with_instance(mut self, inst: Value) -> Self {
        self.instance = Some(inst);
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.instance_path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            let path = self
                .instance_path
                .iter()
                .fold(String::new(), |acc, seg| format!("{}/{}", acc, seg));
            write!(f, "{}: {}", path, self.message)
        }
    }
}

/// 快捷宏，用于关键字实现中构建错误。
#[macro_export]
macro_rules! validation_error {
    ($msg:expr) => {
        $crate::error::ValidationError::new($msg)
    };
    ($msg:expr, $keyword:expr) => {
        $crate::error::ValidationError::new($msg).with_keyword($keyword)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ValidationError::new("is the wrong type")
            .with_keyword("type")
            .with_instance(serde_json::json!(42));
        assert_eq!(err.message, "is the wrong type");
        assert_eq!(err.keyword.as_deref(), Some("type"));
        assert_eq!(err.instance, Some(serde_json::json!(42)));
    }

    #[test]
    fn test_error_display_with_path() {
        let mut err = ValidationError::new("is too short").with_keyword("minLength");
        err.instance_path = vec!["name".into()];
        let s = format!("{}", err);
        assert!(s.contains("/name"));
        assert!(s.contains("too short"));
    }
}

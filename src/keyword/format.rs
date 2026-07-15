use crate::error::ValidationError;
use crate::keyword::Keyword;
use crate::validator::ValidationContext;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Pre-compiled format validators (static to avoid re-compilation)
// ---------------------------------------------------------------------------

static DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap());

static TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$").unwrap()
});

static DATE_TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$",
    )
    .unwrap()
});

static EMAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap()
});

static HOSTNAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$").unwrap()
});

static IPV4_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$",
    )
    .unwrap()
});

static IPV6_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([0-9a-fA-F]{0,4}:){2,7}[0-9a-fA-F]{0,4}$",
    )
    .unwrap()
});

static URI_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z][a-zA-Z0-9+\-.]*://[^\s]*$").unwrap()
});

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$",
    )
    .unwrap()
});

static JSON_POINTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(/([^/~]|~[01])*)*$").unwrap());

// ---------------------------------------------------------------------------
// Format validation logic
// ---------------------------------------------------------------------------

fn validate_format_value(format: &str, value: &str) -> bool {
    match format {
        // Dates & times — regex validation
        "date" => DATE_RE.is_match(value),
        "time" => TIME_RE.is_match(value),
        "date-time" => DATE_TIME_RE.is_match(value),

        // Email — regex validation
        "email" => EMAIL_RE.is_match(value),
        "idn-email" => EMAIL_RE.is_match(value), // simplified

        // Hostnames
        "hostname" => HOSTNAME_RE.is_match(value) && value.len() <= 253,
        "idn-hostname" => true, // IDN hostnames tricky — accept for now

        // IP addresses
        "ipv4" => {
            if let Some(caps) = IPV4_RE.captures(value) {
                caps.iter()
                    .skip(1)
                    .flatten()
                    .all(|m| m.as_str().parse::<u8>().is_ok())
            } else {
                false
            }
        }
        "ipv6" => IPV6_RE.is_match(value),

        // URIs
        "uri" => URI_RE.is_match(value),
        "uri-reference" => true, // generous — accept anything
        "iri" => URI_RE.is_match(value),
        "iri-reference" => true,
        "uri-template" => value.contains('{'), // simplified

        // UUID
        "uuid" => UUID_RE.is_match(value),

        // JSON Pointer
        "json-pointer" => JSON_POINTER_RE.is_match(value) || value.is_empty(),
        "relative-json-pointer" => {
            value
                .split_once('#')
                .map_or(false, |(num, rest)| {
                    num.parse::<u32>().is_ok()
                        && (rest.is_empty()
                            || JSON_POINTER_RE.is_match(rest))
                })
        }

        // Duration — simplified ISO 8601 check
        "duration" => value.starts_with('P')
            && (value.contains('T') || value.contains('Y')
                || value.contains('M') || value.contains('D')),

        // Regex
        "regex" => regex::Regex::new(value).is_ok(),
        "ecmascript-regex" => true, // ECMAScript regex — accept (different engine)

        // Catch-all for unknown formats — accept
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// FormatKeyword
// ---------------------------------------------------------------------------

/// Implements the `format` keyword (JSON Schema Draft 2020-12 §7.2).
///
/// Note: format validation is *annotation-only* by default.  It only produces
/// errors when the schema explicitly opts in via `$vocabulary` or the
/// implementation chooses to validate by default (we do).
pub struct FormatKeyword;
impl Keyword for FormatKeyword {
    fn name(&self) -> &'static str {
        "format"
    }

    fn validate(
        &self,
        ctx: &ValidationContext,
        format: &Value,
        instance: &Value,
        _schema: &Value,
    ) -> Vec<ValidationError> {
        if !ctx.is_type(instance, "string") {
            return vec![];
        }

        let format_name = match format.as_str() {
            Some(f) => f,
            None => return vec![],
        };

        let value = instance.as_str().unwrap();

        if !validate_format_value(format_name, value) {
            vec![ValidationError::new(format!(
                "'{}' is not a valid '{}'",
                value, format_name
            ))
            .with_keyword("format")
            .with_instance(instance.clone())]
        } else {
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_valid() {
        assert!(validate_format_value("date", "2024-01-15"));
    }

    #[test]
    fn test_date_invalid() {
        assert!(!validate_format_value("date", "2024-1-5"));
    }

    #[test]
    fn test_date_time_valid() {
        assert!(validate_format_value("date-time", "2024-01-15T12:00:00Z"));
    }

    #[test]
    fn test_email_valid() {
        assert!(validate_format_value("email", "alice@example.com"));
    }

    #[test]
    fn test_email_invalid() {
        assert!(!validate_format_value("email", "notanemail"));
    }

    #[test]
    fn test_ipv4_valid() {
        assert!(validate_format_value("ipv4", "192.168.1.1"));
    }

    #[test]
    fn test_ipv4_invalid() {
        assert!(!validate_format_value("ipv4", "999.999.999.999"));
    }

    #[test]
    fn test_ipv6_valid() {
        assert!(validate_format_value("ipv6", "::1"));
    }

    #[test]
    fn test_uuid_valid() {
        assert!(validate_format_value(
            "uuid",
            "550e8400-e29b-41d4-a716-446655440000"
        ));
    }

    #[test]
    fn test_uuid_invalid() {
        assert!(!validate_format_value("uuid", "not-a-uuid"));
    }

    #[test]
    fn test_json_pointer_valid() {
        assert!(validate_format_value("json-pointer", "/foo/bar/0"));
        assert!(validate_format_value("json-pointer", ""));
    }

    #[test]
    fn test_json_pointer_invalid() {
        assert!(!validate_format_value("json-pointer", "no-slash"));
    }
}

//! Input validation for all CLI / IPC / API boundaries.
//!
//! AI agents are not trusted operators. They hallucinate paths, embed query
//! params in IDs, generate control characters, and pre-encode strings that
//! get double-encoded. Every input boundary must be validated here.

use std::path::{Component, Path, PathBuf};

use crate::DomainScanError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum allowed path length in bytes.
const MAX_PATH_LEN: usize = 4096;

/// Maximum allowed string input length.
const MAX_STRING_INPUT_LEN: usize = 10_000;

/// Maximum allowed regex pattern length.
const MAX_REGEX_LEN: usize = 1024;

/// Maximum allowed JSON input size in bytes.
const MAX_JSON_SIZE: usize = 1_048_576; // 1 MB

/// Maximum allowed JSON nesting depth.
const MAX_JSON_DEPTH: usize = 32;

// ---------------------------------------------------------------------------
// Structured error codes (for JSON error output)
// ---------------------------------------------------------------------------

/// Error code constants for structured CLI errors.
pub mod error_code {
    pub const INVALID_PATH: &str = "INVALID_PATH";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const INVALID_RESOURCE_ID: &str = "INVALID_RESOURCE_ID";
    pub const INVALID_JSON: &str = "INVALID_JSON";
    pub const INVALID_REGEX: &str = "INVALID_REGEX";
}

// ---------------------------------------------------------------------------
// Path validation
// ---------------------------------------------------------------------------

/// Validate a path input. Rejects:
/// - Paths containing `..` segments (traversal)
/// - Null bytes
/// - Paths exceeding MAX_PATH_LEN bytes
/// - Paths that escape the base directory after canonicalization
pub fn validate_path(input: &str, base_dir: &Path) -> Result<PathBuf, DomainScanError> {
    // Reject null bytes
    if input.contains('\0') {
        return Err(DomainScanError::InvalidPath(format!(
            "Path contains null byte: {:?}",
            input.replace('\0', "\\0")
        )));
    }

    // Reject overly long paths
    if input.len() > MAX_PATH_LEN {
        return Err(DomainScanError::InvalidPath(format!(
            "Path exceeds maximum length of {} bytes (got {})",
            MAX_PATH_LEN,
            input.len()
        )));
    }

    let path = Path::new(input);

    // Reject `..` components (defense-in-depth, before canonicalization)
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(DomainScanError::InvalidPath(format!(
                "Path contains traversal segment: {}",
                input
            )));
        }
    }

    // Resolve to absolute path
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };

    // Canonicalize if the path exists (resolves symlinks)
    let canonical = if resolved.exists() {
        resolved.canonicalize().map_err(|e| {
            DomainScanError::InvalidPath(format!("Cannot canonicalize path: {e}"))
        })?
    } else {
        resolved
    };

    // Ensure the resolved path doesn't escape the base directory
    let canonical_base = if base_dir.exists() {
        base_dir.canonicalize().map_err(|e| {
            DomainScanError::InvalidPath(format!("Cannot canonicalize base dir: {e}"))
        })?
    } else {
        base_dir.to_path_buf()
    };

    if !canonical.starts_with(&canonical_base) {
        return Err(DomainScanError::InvalidPath(format!(
            "Path escapes base directory: {} (resolved to {})",
            input,
            canonical.display()
        )));
    }

    Ok(canonical)
}

/// Validate an output path. Same rules as `validate_path` but the file
/// doesn't need to exist yet. Checks the parent directory instead.
pub fn validate_output_path(input: &str, base_dir: &Path) -> Result<PathBuf, DomainScanError> {
    // Reject null bytes
    if input.contains('\0') {
        return Err(DomainScanError::InvalidPath(format!(
            "Output path contains null byte: {:?}",
            input.replace('\0', "\\0")
        )));
    }

    if input.len() > MAX_PATH_LEN {
        return Err(DomainScanError::InvalidPath(format!(
            "Output path exceeds maximum length of {} bytes",
            MAX_PATH_LEN
        )));
    }

    let path = Path::new(input);

    // Reject `..` components
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return Err(DomainScanError::InvalidPath(format!(
                "Output path contains traversal segment: {}",
                input
            )));
        }
    }

    // Resolve and check that parent doesn't escape base
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };

    if let Some(parent) = resolved.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize().map_err(|e| {
                DomainScanError::InvalidPath(format!("Cannot canonicalize parent: {e}"))
            })?;
            let canonical_base = if base_dir.exists() {
                base_dir.canonicalize().map_err(|e| {
                    DomainScanError::InvalidPath(format!("Cannot canonicalize base dir: {e}"))
                })?
            } else {
                base_dir.to_path_buf()
            };

            if !canonical_parent.starts_with(&canonical_base) {
                return Err(DomainScanError::InvalidPath(format!(
                    "Output path escapes base directory: {}",
                    input
                )));
            }
        }
    }

    Ok(resolved)
}

// ---------------------------------------------------------------------------
// String input validation
// ---------------------------------------------------------------------------

/// Validate a string input (names, patterns, filter values). Rejects:
/// - Control characters below ASCII 0x20 (except \n and \t)
/// - Null bytes
/// - Strings exceeding MAX_STRING_INPUT_LEN
pub fn validate_string_input(input: &str) -> Result<&str, DomainScanError> {
    if input.len() > MAX_STRING_INPUT_LEN {
        return Err(DomainScanError::InvalidInput(format!(
            "Input exceeds maximum length of {} bytes (got {})",
            MAX_STRING_INPUT_LEN,
            input.len()
        )));
    }

    for (i, ch) in input.char_indices() {
        if ch == '\0' {
            return Err(DomainScanError::InvalidInput(format!(
                "Input contains null byte at position {i}"
            )));
        }
        // Allow \n (0x0A) and \t (0x09) but reject other control chars
        if ch < '\x20' && ch != '\n' && ch != '\t' {
            return Err(DomainScanError::InvalidInput(format!(
                "Input contains control character 0x{:02x} at position {i}",
                ch as u32
            )));
        }
    }

    Ok(input)
}

// ---------------------------------------------------------------------------
// Resource ID validation
// ---------------------------------------------------------------------------

/// Validate a resource identifier. Rejects:
/// - Embedded query params (`?`)
/// - Fragment identifiers (`#`)
/// - Pre-URL-encoded strings (`%`)
pub fn validate_resource_id(input: &str) -> Result<&str, DomainScanError> {
    if input.contains('?') {
        return Err(DomainScanError::InvalidResourceId(format!(
            "Resource ID contains query parameter: {}",
            input
        )));
    }
    if input.contains('#') {
        return Err(DomainScanError::InvalidResourceId(format!(
            "Resource ID contains fragment: {}",
            input
        )));
    }
    if input.contains('%') {
        return Err(DomainScanError::InvalidResourceId(format!(
            "Resource ID contains percent-encoding: {}",
            input
        )));
    }
    Ok(input)
}

// ---------------------------------------------------------------------------
// Regex validation
// ---------------------------------------------------------------------------

/// Validate a regex pattern. Rejects:
/// - Patterns that fail to compile
/// - Patterns exceeding MAX_REGEX_LEN
pub fn validate_regex(pattern: &str) -> Result<regex::Regex, DomainScanError> {
    if pattern.len() > MAX_REGEX_LEN {
        return Err(DomainScanError::InvalidRegex(format!(
            "Regex pattern exceeds maximum length of {} chars (got {})",
            MAX_REGEX_LEN,
            pattern.len()
        )));
    }

    regex::Regex::new(pattern).map_err(|e| {
        DomainScanError::InvalidRegex(format!("Invalid regex: {e}"))
    })
}

// ---------------------------------------------------------------------------
// JSON input validation
// ---------------------------------------------------------------------------

/// Validate raw JSON input. Rejects:
/// - Inputs exceeding MAX_JSON_SIZE bytes
/// - JSON that fails to parse
/// - JSON nested deeper than MAX_JSON_DEPTH levels
pub fn validate_json_input(input: &str) -> Result<serde_json::Value, DomainScanError> {
    if input.len() > MAX_JSON_SIZE {
        return Err(DomainScanError::InvalidJson(format!(
            "JSON input exceeds maximum size of {} bytes (got {})",
            MAX_JSON_SIZE,
            input.len()
        )));
    }

    let value: serde_json::Value = serde_json::from_str(input).map_err(|e| {
        DomainScanError::InvalidJson(format!("JSON parse error: {e}"))
    })?;

    let depth = json_depth(&value);
    if depth > MAX_JSON_DEPTH {
        return Err(DomainScanError::InvalidJson(format!(
            "JSON nesting depth exceeds maximum of {} (got {})",
            MAX_JSON_DEPTH, depth
        )));
    }

    Ok(value)
}

/// Alias for `validate_json_input` — matches the spec name `parse_json_input`.
pub fn parse_json_input(input: &str) -> Result<serde_json::Value, DomainScanError> {
    validate_json_input(input)
}

/// Recursively measure the nesting depth of a JSON value.
fn json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(arr) => {
            1 + arr.iter().map(json_depth).max().unwrap_or(0)
        }
        serde_json::Value::Object(map) => {
            1 + map.values().map(json_depth).max().unwrap_or(0)
        }
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // -- Path validation tests --

    #[test]
    fn path_rejects_dot_dot_traversal() {
        let dir = TempDir::new().ok();
        let base = dir.as_ref().map_or(Path::new("/tmp"), |d| d.path());
        let err = validate_path("../../etc/passwd", base).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidPath(_)));
        let msg = err.to_string();
        assert!(msg.contains("traversal"), "Expected 'traversal' in: {msg}");
    }

    #[test]
    fn path_rejects_null_byte() {
        let base = Path::new("/tmp");
        let err = validate_path("src/main\0.rs", base).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidPath(_)));
        assert!(err.to_string().contains("null byte"));
    }

    #[test]
    fn path_rejects_overly_long() {
        let base = Path::new("/tmp");
        let long_path = "a/".repeat(3000);
        let err = validate_path(&long_path, base).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidPath(_)));
        assert!(err.to_string().contains("maximum length"));
    }

    #[test]
    fn path_accepts_valid_relative() {
        let dir = TempDir::new().ok();
        if let Some(d) = &dir {
            std::fs::create_dir_all(d.path().join("src")).ok();
            std::fs::write(d.path().join("src/main.rs"), "fn main() {}").ok();
            let result = validate_path("src/main.rs", d.path());
            assert!(result.is_ok(), "Expected Ok, got: {result:?}");
        }
    }

    #[test]
    fn path_rejects_symlink_escape() {
        let dir = TempDir::new().ok();
        if let Some(d) = &dir {
            #[cfg(unix)]
            {
                let link = d.path().join("escape");
                std::os::unix::fs::symlink("/etc", &link).ok();
                let err = validate_path("escape/passwd", d.path()).unwrap_err();
                assert!(matches!(err, DomainScanError::InvalidPath(_)));
                assert!(err.to_string().contains("escapes base"));
            }
        }
    }

    // -- Output path validation tests --

    #[test]
    fn output_path_rejects_traversal() {
        let base = Path::new("/tmp");
        let err = validate_output_path("../../../etc/evil.json", base).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidPath(_)));
    }

    #[test]
    fn output_path_accepts_valid() {
        let dir = TempDir::new().ok();
        if let Some(d) = &dir {
            let result = validate_output_path("output.json", d.path());
            assert!(result.is_ok());
        }
    }

    // -- String validation tests --

    #[test]
    fn string_rejects_null_byte() {
        let err = validate_string_input("User\0Repository").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidInput(_)));
        assert!(err.to_string().contains("null byte"));
    }

    #[test]
    fn string_rejects_bell_char() {
        let err = validate_string_input("User\x07Repository").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidInput(_)));
        assert!(err.to_string().contains("control character"));
    }

    #[test]
    fn string_rejects_backspace() {
        let err = validate_string_input("User\x08Repo").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidInput(_)));
    }

    #[test]
    fn string_allows_newline_and_tab() {
        assert!(validate_string_input("line1\nline2").is_ok());
        assert!(validate_string_input("col1\tcol2").is_ok());
    }

    #[test]
    fn string_allows_valid_unicode() {
        assert!(validate_string_input("ユーザーRepository").is_ok());
        assert!(validate_string_input("café_service").is_ok());
        assert!(validate_string_input("Überklasse").is_ok());
    }

    #[test]
    fn string_rejects_overly_long() {
        let long = "a".repeat(MAX_STRING_INPUT_LEN + 1);
        let err = validate_string_input(&long).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidInput(_)));
    }

    // -- Resource ID validation tests --

    #[test]
    fn resource_id_rejects_query_params() {
        let err = validate_resource_id("abc123?fields=name").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    }

    #[test]
    fn resource_id_rejects_fragment() {
        let err = validate_resource_id("abc123#section").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    }

    #[test]
    fn resource_id_rejects_pre_encoded() {
        let err = validate_resource_id("%2e%2e%2f%2e%2e%2fetc%2fpasswd").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    }

    #[test]
    fn resource_id_accepts_valid() {
        assert!(validate_resource_id("abc123").is_ok());
        assert!(validate_resource_id("my-interface-name").is_ok());
        assert!(validate_resource_id("UserRepository").is_ok());
    }

    // -- Regex validation tests --

    #[test]
    fn regex_rejects_invalid_pattern() {
        let err = validate_regex("[unclosed").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidRegex(_)));
    }

    #[test]
    fn regex_rejects_oversized_pattern() {
        let pattern = "a".repeat(MAX_REGEX_LEN + 1);
        let err = validate_regex(&pattern).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidRegex(_)));
        assert!(err.to_string().contains("maximum length"));
    }

    #[test]
    fn regex_accepts_valid_pattern() {
        let re = validate_regex(".*Repository").ok();
        assert!(re.is_some());
        assert!(re.as_ref().is_some_and(|r| r.is_match("UserRepository")));
    }

    // -- JSON validation tests --

    #[test]
    fn json_rejects_oversized() {
        let json = format!(r#"{{"data": "{}"}}"#, "x".repeat(2_000_000));
        let err = validate_json_input(&json).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidJson(_)));
        assert!(err.to_string().contains("maximum size"));
    }

    #[test]
    fn json_rejects_deeply_nested() {
        let json = (0..100).fold(String::from("null"), |acc, _| format!("[{acc}]"));
        let err = validate_json_input(&json).unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidJson(_)));
        assert!(err.to_string().contains("nesting depth"));
    }

    #[test]
    fn json_rejects_malformed() {
        let err = validate_json_input("{not valid json}").unwrap_err();
        assert!(matches!(err, DomainScanError::InvalidJson(_)));
    }

    #[test]
    fn json_accepts_valid() {
        let result = validate_json_input(r#"{"name": "Repo", "languages": ["typescript"]}"#);
        assert!(result.is_ok());
    }

    #[test]
    fn json_accepts_at_max_depth() {
        // Exactly MAX_JSON_DEPTH levels should be fine
        let json = (0..MAX_JSON_DEPTH).fold(String::from("null"), |acc, _| format!("[{acc}]"));
        assert!(validate_json_input(&json).is_ok());
    }
}

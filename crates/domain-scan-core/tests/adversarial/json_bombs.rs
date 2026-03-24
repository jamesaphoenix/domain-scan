//! JSON bomb adversarial tests.
//!
//! AI agents may generate deeply nested JSON, massive payloads,
//! or malformed JSON that could cause stack overflow or OOM.

use domain_scan_core::input_validation::validate_json_input;
use domain_scan_core::DomainScanError;

#[test]
fn rejects_deeply_nested_json() {
    let json = (0..100).fold(String::from("null"), |acc, _| format!("[{acc}]"));
    let err = validate_json_input(&json).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
    assert!(err.to_string().contains("nesting depth"));
}

#[test]
fn rejects_oversized_json() {
    let json = format!(r#"{{"data": "{}"}}"#, "x".repeat(2_000_000));
    let err = validate_json_input(&json).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
    assert!(err.to_string().contains("maximum size"));
}

#[test]
fn rejects_malformed_json() {
    let err = validate_json_input("{not valid json}").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
    assert!(err.to_string().contains("parse error"));
}

#[test]
fn rejects_truncated_json() {
    let err = validate_json_input(r#"{"name": "incomplete"#).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
}

#[test]
fn rejects_deeply_nested_objects() {
    let json = (0..100).fold(String::from("null"), |acc, _| {
        format!(r#"{{"nested": {acc}}}"#)
    });
    let err = validate_json_input(&json).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
}

#[test]
fn accepts_valid_json() {
    let result = validate_json_input(r#"{"name": "Repo", "languages": ["typescript"]}"#);
    assert!(result.is_ok());
}

#[test]
fn accepts_at_max_depth() {
    // Exactly 32 levels should be fine
    let json = (0..32).fold(String::from("null"), |acc, _| format!("[{acc}]"));
    assert!(validate_json_input(&json).is_ok());
}

#[test]
fn accepts_empty_object() {
    assert!(validate_json_input("{}").is_ok());
}

#[test]
fn accepts_empty_array() {
    assert!(validate_json_input("[]").is_ok());
}

#[test]
fn rejects_empty_string() {
    let err = validate_json_input("").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidJson(_)));
}

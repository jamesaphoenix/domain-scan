//! Embedded query parameter adversarial tests.
//!
//! AI agents hallucinate resource IDs with embedded query params (`?`),
//! fragment identifiers (`#`), or pre-URL-encoded strings (`%`).

use domain_scan_core::input_validation::validate_resource_id;
use domain_scan_core::DomainScanError;

#[test]
fn rejects_query_params_in_resource_id() {
    let err = validate_resource_id("abc123?fields=name").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    assert!(err.to_string().contains("query parameter"));
}

#[test]
fn rejects_fragment_in_resource_id() {
    let err = validate_resource_id("abc123#section").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    assert!(err.to_string().contains("fragment"));
}

#[test]
fn rejects_pre_encoded_traversal() {
    let err = validate_resource_id("%2e%2e%2f%2e%2e%2fetc%2fpasswd").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
    assert!(err.to_string().contains("percent-encoding"));
}

#[test]
fn rejects_percent_in_middle() {
    let err = validate_resource_id("file%20name").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
}

#[test]
fn rejects_multiple_query_params() {
    let err = validate_resource_id("item?a=1&b=2").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidResourceId(_)));
}

#[test]
fn accepts_valid_resource_ids() {
    assert!(validate_resource_id("abc123").is_ok());
    assert!(validate_resource_id("my-interface-name").is_ok());
    assert!(validate_resource_id("UserRepository").is_ok());
    assert!(validate_resource_id("com.example.Service").is_ok());
    assert!(validate_resource_id("module/submodule").is_ok());
}

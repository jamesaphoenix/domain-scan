//! Control character adversarial tests.
//!
//! AI agents may inject null bytes, bell, backspace, or escape sequences
//! into name filters and other string inputs.

use domain_scan_core::input_validation::validate_string_input;
use domain_scan_core::DomainScanError;

#[test]
fn rejects_null_byte_in_name() {
    let err = validate_string_input("User\0Repository").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
    assert!(err.to_string().contains("null byte"));
}

#[test]
fn rejects_bell_character() {
    let err = validate_string_input("User\x07Repository").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
    assert!(err.to_string().contains("control character"));
}

#[test]
fn rejects_backspace_character() {
    let err = validate_string_input("User\x08Repo").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn rejects_form_feed() {
    let err = validate_string_input("User\x0cRepo").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn rejects_vertical_tab() {
    let err = validate_string_input("User\x0bRepo").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn rejects_escape_character() {
    let err = validate_string_input("User\x1bRepo").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn rejects_carriage_return_only() {
    // \r (0x0D) is below 0x20 and not \n or \t
    let err = validate_string_input("User\rRepo").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn allows_newline() {
    assert!(validate_string_input("line1\nline2").is_ok());
}

#[test]
fn allows_tab() {
    assert!(validate_string_input("col1\tcol2").is_ok());
}

#[test]
fn allows_printable_ascii() {
    assert!(validate_string_input("UserRepository_v2.0-beta").is_ok());
}

#[test]
fn rejects_overly_long_input() {
    let long = "a".repeat(10_001);
    let err = validate_string_input(&long).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
    assert!(err.to_string().contains("maximum length"));
}

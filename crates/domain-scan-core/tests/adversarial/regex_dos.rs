//! Regex DoS adversarial tests.
//!
//! AI agents may generate regex patterns that cause catastrophic backtracking
//! or are excessively long. The validator enforces a max pattern length.

use domain_scan_core::input_validation::validate_regex;
use domain_scan_core::DomainScanError;

#[test]
fn rejects_oversized_regex() {
    let pattern = "a".repeat(2000);
    let err = validate_regex(&pattern).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidRegex(_)));
    assert!(err.to_string().contains("maximum length"));
}

#[test]
fn rejects_invalid_regex_syntax() {
    let err = validate_regex("[unclosed").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidRegex(_)));
    assert!(err.to_string().contains("Invalid regex"));
}

#[test]
fn rejects_unbalanced_parens() {
    let err = validate_regex("(((abc").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidRegex(_)));
}

#[test]
fn rejects_invalid_quantifier() {
    let err = validate_regex("abc{-1}").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidRegex(_)));
}

#[test]
fn accepts_valid_simple_pattern() {
    let re = validate_regex(".*Repository");
    assert!(re.is_ok());
    assert!(re.expect("valid").is_match("UserRepository"));
}

#[test]
fn accepts_valid_complex_pattern() {
    let re = validate_regex(r"^[A-Z][a-zA-Z]*Service$");
    assert!(re.is_ok());
}

#[test]
fn accepts_pattern_at_max_length() {
    // Exactly 1024 chars should be fine
    let pattern = "a".repeat(1024);
    assert!(validate_regex(&pattern).is_ok());
}

#[test]
fn rejects_pattern_just_over_max() {
    let pattern = "a".repeat(1025);
    let err = validate_regex(&pattern).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidRegex(_)));
}

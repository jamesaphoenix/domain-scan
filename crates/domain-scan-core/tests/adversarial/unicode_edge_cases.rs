//! Unicode edge case adversarial tests.
//!
//! Some languages allow non-ASCII identifiers (CJK, accented, emoji).
//! These must be preserved. But null bytes and control chars in
//! non-ASCII strings must still be rejected.

use domain_scan_core::input_validation::validate_string_input;
use domain_scan_core::DomainScanError;

#[test]
fn allows_cjk_identifiers() {
    assert!(validate_string_input("ユーザーRepository").is_ok());
}

#[test]
fn allows_accented_identifiers() {
    assert!(validate_string_input("café_service").is_ok());
    assert!(validate_string_input("Überklasse").is_ok());
}

#[test]
fn allows_cyrillic_identifiers() {
    // Cyrillic е looks like Latin e — homoglyphs are valid identifiers
    assert!(validate_string_input("Usеr").is_ok());
}

#[test]
fn allows_emoji_in_identifiers() {
    assert!(validate_string_input("🚀Service").is_ok());
}

#[test]
fn allows_mixed_scripts() {
    assert!(validate_string_input("UserService_ユーザー_café").is_ok());
}

#[test]
fn rejects_null_byte_in_unicode_string() {
    let err = validate_string_input("ユーザー\0Service").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn rejects_control_char_in_unicode_string() {
    let err = validate_string_input("café\x07service").unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidInput(_)));
}

#[test]
fn allows_zero_width_joiner() {
    // U+200D zero-width joiner — above 0x20, so it's allowed
    assert!(validate_string_input("a\u{200D}b").is_ok());
}

#[test]
fn allows_right_to_left_mark() {
    // U+200F right-to-left mark — above 0x20, so it's allowed
    assert!(validate_string_input("abc\u{200F}def").is_ok());
}

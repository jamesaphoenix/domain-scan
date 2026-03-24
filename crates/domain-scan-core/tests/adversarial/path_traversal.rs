//! Path traversal adversarial tests.
//!
//! AI agents hallucinate paths like `../../.ssh/id_rsa` or `/etc/passwd`.
//! All must be rejected with `INVALID_PATH` structured errors.

use domain_scan_core::input_validation::{validate_path, validate_output_path};
use domain_scan_core::DomainScanError;
use tempfile::TempDir;

#[test]
fn rejects_dot_dot_traversal() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_path("../../.ssh/id_rsa", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
    assert!(err.to_string().contains("traversal"));
}

#[test]
fn rejects_dot_dot_at_start() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_path("../secret.txt", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
fn rejects_dot_dot_in_middle() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_path("src/../../../etc/passwd", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
fn rejects_null_byte_in_path() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_path("src/main\0.rs", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
    assert!(err.to_string().contains("null byte"));
}

#[test]
fn rejects_path_exceeding_max_length() {
    let dir = TempDir::new().expect("tempdir");
    let long_path = "a/".repeat(3000);
    let err = validate_path(&long_path, dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
    assert!(err.to_string().contains("maximum length"));
}

#[test]
fn accepts_valid_relative_path() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir");
    std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").expect("write");
    let result = validate_path("src/main.rs", dir.path());
    assert!(result.is_ok(), "Expected Ok, got: {result:?}");
}

#[test]
fn output_path_rejects_dot_dot() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_output_path("../../../etc/evil.json", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
fn output_path_rejects_null_byte() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_output_path("output\0.json", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
fn rejects_absolute_path_outside_base() {
    let dir = TempDir::new().expect("tempdir");
    let err = validate_path("/etc/passwd", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

//! Symlink escape adversarial tests.
//!
//! AI agents may follow symlinks that point outside the project root.
//! The path validator must detect and reject such paths after
//! canonicalization.

use domain_scan_core::input_validation::validate_path;
use domain_scan_core::DomainScanError;

#[cfg(unix)]
use tempfile::TempDir;

#[test]
#[cfg(unix)]
fn rejects_symlink_to_etc() {
    let dir = TempDir::new().expect("tempdir");
    let link = dir.path().join("escape");
    std::os::unix::fs::symlink("/etc", &link).expect("symlink");
    let err = validate_path("escape/passwd", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
    assert!(err.to_string().contains("escapes base"));
}

#[test]
#[cfg(unix)]
fn rejects_symlink_to_root() {
    let dir = TempDir::new().expect("tempdir");
    let link = dir.path().join("rootlink");
    std::os::unix::fs::symlink("/", &link).expect("symlink");
    let err = validate_path("rootlink/etc/passwd", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
#[cfg(unix)]
fn rejects_symlink_to_home() {
    let dir = TempDir::new().expect("tempdir");
    let link = dir.path().join("homelink");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::os::unix::fs::symlink(home, &link).expect("symlink");
    let err = validate_path("homelink/.ssh/id_rsa", dir.path()).unwrap_err();
    assert!(matches!(err, DomainScanError::InvalidPath(_)));
}

#[test]
#[cfg(unix)]
fn allows_symlink_within_base() {
    let dir = TempDir::new().expect("tempdir");
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("mkdir");
    std::fs::write(src_dir.join("main.rs"), "fn main() {}").expect("write");
    let link = dir.path().join("link_to_src");
    std::os::unix::fs::symlink(&src_dir, &link).expect("symlink");
    // Symlink within base should be fine
    let result = validate_path("link_to_src/main.rs", dir.path());
    assert!(result.is_ok(), "Expected Ok, got: {result:?}");
}

//! Integration tests for PHP query extraction.
//! Each test parses a real PHP fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/php/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::PHP)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::PHP,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_php_interfaces_count() {
    let ir = extract_fixture("interfaces.php");
    // UserRepositoryInterface, NotificationServiceInterface, CacheInterface
    // (traits from traits.php would be separate)
    let ifaces: Vec<_> = ir.interfaces.iter()
        .filter(|i| i.language_kind == InterfaceKind::Interface)
        .collect();
    assert_eq!(ifaces.len(), 3, "Expected 3 interfaces, got {}", ifaces.len());
}

#[test]
fn test_php_interface_names() {
    let ir = extract_fixture("interfaces.php");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"UserRepositoryInterface"), "Missing UserRepositoryInterface");
    assert!(names.contains(&"NotificationServiceInterface"), "Missing NotificationServiceInterface");
    assert!(names.contains(&"CacheInterface"), "Missing CacheInterface");
}

#[test]
fn test_php_interface_methods() {
    let ir = extract_fixture("interfaces.php");
    let user_repo = ir.interfaces.iter()
        .find(|i| i.name == "UserRepositoryInterface")
        .unwrap();
    assert_eq!(user_repo.methods.len(), 3, "UserRepositoryInterface should have 3 methods");
}

#[test]
fn test_php_interface_extends() {
    let ir = extract_fixture("interfaces.php");
    let notif = ir.interfaces.iter()
        .find(|i| i.name == "NotificationServiceInterface")
        .unwrap();
    assert!(!notif.extends.is_empty(), "NotificationServiceInterface should extend ServiceInterface");
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_php_classes_count() {
    let ir = extract_fixture("classes.php");
    // BaseEntity, UserService, OrderController
    assert_eq!(ir.classes.len(), 3, "Expected 3 classes, got {}", ir.classes.len());
}

#[test]
fn test_php_class_names() {
    let ir = extract_fixture("classes.php");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"BaseEntity"), "Missing BaseEntity");
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"OrderController"), "Missing OrderController");
}

#[test]
fn test_php_class_is_abstract() {
    let ir = extract_fixture("classes.php");
    let base = ir.classes.iter().find(|c| c.name == "BaseEntity").unwrap();
    assert!(base.is_abstract, "BaseEntity should be abstract");
}

#[test]
fn test_php_class_extends() {
    let ir = extract_fixture("classes.php");
    let order = ir.classes.iter().find(|c| c.name == "OrderController").unwrap();
    assert_eq!(order.extends.as_deref(), Some("BaseController"), "OrderController should extend BaseController");
}

#[test]
fn test_php_class_implements() {
    let ir = extract_fixture("classes.php");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    assert!(user_service.implements.contains(&"UserRepositoryInterface".to_string()),
        "UserService should implement UserRepositoryInterface");
}

#[test]
fn test_php_class_methods() {
    let ir = extract_fixture("classes.php");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    // __construct, findById, save, delete, create
    assert!(user_service.methods.len() >= 5,
        "UserService should have at least 5 methods, got {}", user_service.methods.len());
}

#[test]
fn test_php_class_static_method() {
    let ir = extract_fixture("classes.php");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    let create = user_service.methods.iter().find(|m| m.name == "create");
    assert!(create.is_some(), "Missing static method 'create'");
    assert!(create.unwrap().is_static, "'create' should be static");
}

#[test]
fn test_php_class_properties() {
    let ir = extract_fixture("classes.php");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    assert!(user_service.properties.len() >= 2, "UserService should have at least 2 properties");
}

#[test]
fn test_php_method_ownership() {
    let ir = extract_fixture("classes.php");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    for method in &user_service.methods {
        assert_eq!(method.owner.as_deref(), Some("UserService"), "Methods should have owner set");
    }
}

// =========================================================================
// traits.scm tests
// =========================================================================

#[test]
fn test_php_traits_count() {
    let ir = extract_fixture("traits.php");
    let traits: Vec<_> = ir.interfaces.iter()
        .filter(|i| i.language_kind == InterfaceKind::Trait)
        .collect();
    assert_eq!(traits.len(), 2, "Expected 2 traits, got {}", traits.len());
}

#[test]
fn test_php_trait_names() {
    let ir = extract_fixture("traits.php");
    let trait_names: Vec<&str> = ir.interfaces.iter()
        .filter(|i| i.language_kind == InterfaceKind::Trait)
        .map(|i| i.name.as_str())
        .collect();
    assert!(trait_names.contains(&"Timestampable"), "Missing Timestampable");
    assert!(trait_names.contains(&"SoftDeletable"), "Missing SoftDeletable");
}

#[test]
fn test_php_trait_methods() {
    let ir = extract_fixture("traits.php");
    let timestampable = ir.interfaces.iter()
        .find(|i| i.name == "Timestampable")
        .unwrap();
    assert_eq!(timestampable.methods.len(), 2, "Timestampable should have 2 methods");
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_php_imports_count() {
    let ir = extract_fixture("imports.php");
    assert!(ir.imports.len() >= 4, "Expected at least 4 imports, got {}", ir.imports.len());
}

#[test]
fn test_php_import_sources() {
    let ir = extract_fixture("imports.php");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.iter().any(|s| s.contains("User")), "Missing User import");
    assert!(sources.iter().any(|s| s.contains("Request")), "Missing Request import");
}

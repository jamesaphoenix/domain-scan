//! Integration tests for Kotlin query extraction.
//! Each test parses a real Kotlin fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/kotlin/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Kotlin)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Kotlin,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_kotlin_interfaces_count() {
    let ir = extract_fixture("interfaces.kt");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 interfaces");
}

#[test]
fn test_kotlin_interface_names() {
    let ir = extract_fixture("interfaces.kt");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"UserRepository"), "Missing UserRepository");
    assert!(names.contains(&"Closeable"), "Missing Closeable");
    assert!(names.contains(&"EventHandler"), "Missing EventHandler");
    assert!(names.contains(&"InternalCache"), "Missing InternalCache");
}

#[test]
fn test_kotlin_interface_kind() {
    let ir = extract_fixture("interfaces.kt");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Interface);
    }
}

#[test]
fn test_kotlin_interface_methods() {
    let ir = extract_fixture("interfaces.kt");
    let user_repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "UserRepository")
        .expect("UserRepository not found");
    assert_eq!(user_repo.methods.len(), 4);
    let method_names: Vec<&str> = user_repo.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"findById"));
    assert!(method_names.contains(&"findAll"));
    assert!(method_names.contains(&"save"));
    assert!(method_names.contains(&"delete"));
}

#[test]
fn test_kotlin_interface_generics() {
    let ir = extract_fixture("interfaces.kt");
    let handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert!(!handler.generics.is_empty(), "EventHandler should have generics");
}

#[test]
fn test_kotlin_interface_extends() {
    let ir = extract_fixture("interfaces.kt");
    let handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert!(
        !handler.extends.is_empty(),
        "EventHandler should extend Closeable"
    );
}

#[test]
fn test_kotlin_interface_visibility() {
    let ir = extract_fixture("interfaces.kt");
    let cache = ir
        .interfaces
        .iter()
        .find(|i| i.name == "InternalCache")
        .expect("InternalCache not found");
    assert_eq!(cache.visibility, Visibility::Private);
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_kotlin_classes_count() {
    let ir = extract_fixture("classes.kt");
    assert!(
        ir.classes.len() >= 4,
        "Expected at least 4 classes, got {}",
        ir.classes.len()
    );
}

#[test]
fn test_kotlin_class_names() {
    let ir = extract_fixture("classes.kt");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"BaseEntity"), "Missing BaseEntity");
    assert!(names.contains(&"Config"), "Missing Config");
}

#[test]
fn test_kotlin_class_abstract() {
    let ir = extract_fixture("classes.kt");
    let base = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseEntity")
        .expect("BaseEntity not found");
    assert!(base.is_abstract, "BaseEntity should be abstract");
}

#[test]
fn test_kotlin_class_generics() {
    let ir = extract_fixture("classes.kt");
    let config = ir
        .classes
        .iter()
        .find(|c| c.name == "Config")
        .expect("Config not found");
    assert!(
        !config.generics.is_empty(),
        "Config should have generics"
    );
}

#[test]
fn test_kotlin_class_methods() {
    let ir = extract_fixture("classes.kt");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    assert!(
        svc.methods.len() >= 2,
        "UserService should have at least 2 methods, got {}",
        svc.methods.len()
    );
}

#[test]
fn test_kotlin_class_visibility() {
    let ir = extract_fixture("classes.kt");
    let internal = ir
        .classes
        .iter()
        .find(|c| c.name == "InternalHelper");
    // InternalHelper is declared as `internal class`
    if let Some(helper) = internal {
        assert_ne!(
            helper.visibility,
            Visibility::Public,
            "InternalHelper should not be public"
        );
    }
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_kotlin_imports_count() {
    let ir = extract_fixture("imports.kt");
    assert!(
        ir.imports.len() >= 4,
        "Expected at least 4 imports, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_kotlin_import_sources() {
    let ir = extract_fixture("imports.kt");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(
        sources.iter().any(|s| s.contains("java.util")),
        "Should have java.util imports"
    );
}

#[test]
fn test_kotlin_wildcard_import() {
    let ir = extract_fixture("imports.kt");
    let wildcard = ir.imports.iter().find(|i| i.is_wildcard);
    assert!(
        wildcard.is_some(),
        "Should have at least one wildcard import (com.example.models.*)"
    );
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_kotlin_services_count() {
    let ir = extract_fixture("services.kt");
    assert!(
        ir.services.len() >= 2,
        "Expected at least 2 services, got {}",
        ir.services.len()
    );
}

#[test]
fn test_kotlin_service_names() {
    let ir = extract_fixture("services.kt");
    let names: Vec<&str> = ir.services.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"UserController"),
        "Missing UserController service"
    );
    assert!(
        names.contains(&"OrderService"),
        "Missing OrderService service"
    );
}

#[test]
fn test_kotlin_service_kind() {
    let ir = extract_fixture("services.kt");
    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "UserController")
        .expect("UserController not found");
    assert_eq!(
        controller.kind,
        ServiceKind::HttpController,
        "UserController should be HttpController"
    );
}

#[test]
fn test_kotlin_service_decorators() {
    let ir = extract_fixture("services.kt");
    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "UserController")
        .expect("UserController not found");
    assert!(
        controller
            .decorators
            .iter()
            .any(|d| d.contains("RestController")),
        "UserController should have @RestController"
    );
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_kotlin_schemas_count() {
    let ir = extract_fixture("schemas.kt");
    assert!(
        ir.schemas.len() >= 3,
        "Expected at least 3 data class schemas, got {}",
        ir.schemas.len()
    );
}

#[test]
fn test_kotlin_schema_names() {
    let ir = extract_fixture("schemas.kt");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"User"), "Missing User schema");
    assert!(
        names.contains(&"CreateUserRequest"),
        "Missing CreateUserRequest schema"
    );
    assert!(names.contains(&"OrderItem"), "Missing OrderItem schema");
}

#[test]
fn test_kotlin_schema_framework() {
    let ir = extract_fixture("schemas.kt");
    for schema in &ir.schemas {
        assert_eq!(
            schema.source_framework, "kotlin-data-class",
            "Schema {} should have kotlin-data-class framework",
            schema.name
        );
    }
}

#[test]
fn test_kotlin_schema_fields() {
    let ir = extract_fixture("schemas.kt");
    let user = ir
        .schemas
        .iter()
        .find(|s| s.name == "User")
        .expect("User schema not found");
    assert_eq!(
        user.fields.len(),
        4,
        "User should have 4 fields"
    );
    let field_names: Vec<&str> = user.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"));
    assert!(field_names.contains(&"name"));
    assert!(field_names.contains(&"email"));
    assert!(field_names.contains(&"age"));
}

#[test]
fn test_kotlin_non_data_class_excluded() {
    let ir = extract_fixture("schemas.kt");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&"NotADataClass"),
        "NotADataClass should not be detected as schema"
    );
}

// =========================================================================
// Cross-cutting: build_status and confidence
// =========================================================================

#[test]
fn test_kotlin_build_status() {
    let ir = extract_fixture("interfaces.kt");
    assert_eq!(ir.build_status, BuildStatus::Built);
    assert_eq!(ir.confidence, Confidence::High);
}

#[test]
fn test_kotlin_language() {
    let ir = extract_fixture("interfaces.kt");
    assert_eq!(ir.language, Language::Kotlin);
}

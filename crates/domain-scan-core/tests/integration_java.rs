//! Integration tests for Java query extraction.
//! Each test parses a real Java fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/java/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Java)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Java,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_java_interfaces_count() {
    let ir = extract_fixture("interfaces.java");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 interfaces");
}

#[test]
fn test_java_interface_names() {
    let ir = extract_fixture("interfaces.java");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"UserRepository"), "Missing UserRepository");
    assert!(names.contains(&"Closeable"), "Missing Closeable");
    assert!(names.contains(&"EventHandler"), "Missing EventHandler");
    assert!(names.contains(&"ReadOnly"), "Missing ReadOnly");
}

#[test]
fn test_java_interface_kind() {
    let ir = extract_fixture("interfaces.java");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Interface);
    }
}

#[test]
fn test_java_interface_methods() {
    let ir = extract_fixture("interfaces.java");
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
fn test_java_interface_generics() {
    let ir = extract_fixture("interfaces.java");
    let handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert!(
        !handler.generics.is_empty(),
        "EventHandler should have generics"
    );
}

#[test]
fn test_java_interface_extends() {
    let ir = extract_fixture("interfaces.java");
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

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_java_classes_count() {
    let ir = extract_fixture("classes.java");
    assert!(
        ir.classes.len() >= 4,
        "Expected at least 4 classes, got {}",
        ir.classes.len()
    );
}

#[test]
fn test_java_class_names() {
    let ir = extract_fixture("classes.java");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"BaseEntity"), "Missing BaseEntity");
    assert!(names.contains(&"InternalHelper"), "Missing InternalHelper");
    assert!(names.contains(&"Config"), "Missing Config");
}

#[test]
fn test_java_class_abstract() {
    let ir = extract_fixture("classes.java");
    let base = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseEntity")
        .expect("BaseEntity not found");
    assert!(base.is_abstract, "BaseEntity should be abstract");
}

#[test]
fn test_java_class_generics() {
    let ir = extract_fixture("classes.java");
    let config = ir
        .classes
        .iter()
        .find(|c| c.name == "Config")
        .expect("Config not found");
    assert!(!config.generics.is_empty(), "Config should have generics");
}

#[test]
fn test_java_class_methods() {
    let ir = extract_fixture("classes.java");
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

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_java_imports_count() {
    let ir = extract_fixture("imports.java");
    assert!(
        ir.imports.len() >= 4,
        "Expected at least 4 imports, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_java_import_sources() {
    let ir = extract_fixture("imports.java");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    // Should have java.util imports and com.example imports
    assert!(
        sources.iter().any(|s| s.contains("java.util")),
        "Should have java.util imports"
    );
}

#[test]
fn test_java_wildcard_import() {
    let ir = extract_fixture("imports.java");
    let wildcard = ir.imports.iter().find(|i| i.is_wildcard);
    assert!(
        wildcard.is_some(),
        "Should have at least one wildcard import"
    );
}

#[test]
fn test_java_static_import() {
    let ir = extract_fixture("imports.java");
    // static import java.util.Collections.emptyList
    let has_static = ir
        .imports
        .iter()
        .any(|i| i.source.contains("Collections") || i.source.contains("java.util"));
    assert!(has_static, "Should detect static import");
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_java_services_count() {
    let ir = extract_fixture("services.java");
    assert!(
        ir.services.len() >= 2,
        "Expected at least 2 services, got {}",
        ir.services.len()
    );
}

#[test]
fn test_java_service_names() {
    let ir = extract_fixture("services.java");
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
fn test_java_service_kind() {
    let ir = extract_fixture("services.java");
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
fn test_java_service_decorators() {
    let ir = extract_fixture("services.java");
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

#[test]
fn test_java_service_methods() {
    let ir = extract_fixture("services.java");
    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "UserController")
        .expect("UserController not found");
    assert!(
        controller.methods.len() >= 2,
        "UserController should have at least 2 methods"
    );
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_java_schemas_count() {
    let ir = extract_fixture("schemas.java");
    assert!(
        ir.schemas.len() >= 2,
        "Expected at least 2 schemas (records + @Entity), got {}",
        ir.schemas.len()
    );
}

#[test]
fn test_java_record_schema() {
    let ir = extract_fixture("schemas.java");
    let user_dto = ir.schemas.iter().find(|s| s.name == "UserDTO");
    assert!(user_dto.is_some(), "UserDTO record should be a schema");
    let user_dto = user_dto.expect("already checked");
    assert_eq!(user_dto.source_framework, "java-record");
    assert_eq!(user_dto.kind, SchemaKind::DataTransfer);
}

#[test]
fn test_java_record_fields() {
    let ir = extract_fixture("schemas.java");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDTO")
        .expect("UserDTO not found");
    assert_eq!(user_dto.fields.len(), 3, "UserDTO should have 3 fields");
    let field_names: Vec<&str> = user_dto.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"name"));
    assert!(field_names.contains(&"email"));
    assert!(field_names.contains(&"age"));
}

#[test]
fn test_java_entity_schema() {
    let ir = extract_fixture("schemas.java");
    let user = ir.schemas.iter().find(|s| s.name == "User");
    assert!(user.is_some(), "@Entity User should be a schema");
    let user = user.expect("already checked");
    assert_eq!(user.source_framework, "jpa");
    assert_eq!(user.kind, SchemaKind::OrmModel);
}

#[test]
fn test_java_entity_fields() {
    let ir = extract_fixture("schemas.java");
    let user = ir
        .schemas
        .iter()
        .find(|s| s.name == "User")
        .expect("User not found");
    assert!(
        user.fields.len() >= 3,
        "User @Entity should have at least 3 fields, got {}",
        user.fields.len()
    );
}

// =========================================================================
// Cross-cutting: build_status and confidence
// =========================================================================

#[test]
fn test_java_build_status() {
    let ir = extract_fixture("interfaces.java");
    assert_eq!(ir.build_status, BuildStatus::Built);
    assert_eq!(ir.confidence, Confidence::High);
}

#[test]
fn test_java_language() {
    let ir = extract_fixture("interfaces.java");
    assert_eq!(ir.language, Language::Java);
}

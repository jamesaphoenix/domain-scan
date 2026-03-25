//! Integration tests for Swift query extraction.
//! Each test parses a real Swift fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/swift/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Swift)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Swift,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests (protocols)
// =========================================================================

#[test]
fn test_swift_interfaces_count() {
    let ir = extract_fixture("interfaces.swift");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 protocols");
}

#[test]
fn test_swift_interfaces_names() {
    let ir = extract_fixture("interfaces.swift");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Drawable"), "Missing Drawable protocol");
    assert!(
        names.contains(&"Serializable"),
        "Missing Serializable protocol"
    );
    assert!(names.contains(&"Repository"), "Missing Repository protocol");
    assert!(names.contains(&"Comparable"), "Missing Comparable protocol");
}

#[test]
fn test_swift_interfaces_kind() {
    let ir = extract_fixture("interfaces.swift");
    for iface in &ir.interfaces {
        assert_eq!(
            iface.language_kind,
            InterfaceKind::Protocol,
            "{} should be Protocol",
            iface.name
        );
    }
}

#[test]
fn test_swift_interfaces_methods() {
    let ir = extract_fixture("interfaces.swift");
    let drawable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Drawable")
        .expect("Drawable");
    assert_eq!(drawable.methods.len(), 1, "Drawable has 1 method (draw)");
    assert_eq!(drawable.methods[0].name, "draw");
}

#[test]
fn test_swift_interfaces_properties() {
    let ir = extract_fixture("interfaces.swift");
    let drawable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Drawable")
        .expect("Drawable");
    assert_eq!(
        drawable.properties.len(),
        1,
        "Drawable has 1 property (color)"
    );
    assert_eq!(drawable.properties[0].name, "color");
    assert!(
        !drawable.properties[0].is_readonly,
        "color has both get and set"
    );
}

#[test]
fn test_swift_interfaces_inheritance() {
    let ir = extract_fixture("interfaces.swift");
    let serializable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Serializable")
        .expect("Serializable");
    assert!(
        serializable.extends.contains(&"Codable".to_string()),
        "Serializable extends Codable"
    );
}

#[test]
fn test_swift_interfaces_visibility() {
    let ir = extract_fixture("interfaces.swift");
    let serializable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Serializable")
        .expect("Serializable");
    assert_eq!(
        serializable.visibility,
        Visibility::Public,
        "Serializable is public"
    );

    let drawable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Drawable")
        .expect("Drawable");
    assert_eq!(
        drawable.visibility,
        Visibility::Internal,
        "Drawable defaults to internal"
    );
}

#[test]
fn test_swift_protocol_multiple_methods() {
    let ir = extract_fixture("interfaces.swift");
    let serializable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Serializable")
        .expect("Serializable");
    assert_eq!(serializable.methods.len(), 2, "Serializable has 2 methods");

    let repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Repository")
        .expect("Repository");
    assert_eq!(repo.methods.len(), 3, "Repository has 3 methods");
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_swift_classes_count() {
    let ir = extract_fixture("classes.swift");
    // UserManager, Point, Direction, AdminManager
    assert_eq!(ir.classes.len(), 4, "Expected 4 classes/structs/enums");
}

#[test]
fn test_swift_classes_names() {
    let ir = extract_fixture("classes.swift");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserManager"), "Missing UserManager");
    assert!(names.contains(&"Point"), "Missing Point struct");
    assert!(names.contains(&"Direction"), "Missing Direction enum");
    assert!(names.contains(&"AdminManager"), "Missing AdminManager");
}

#[test]
fn test_swift_class_methods() {
    let ir = extract_fixture("classes.swift");
    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    // init, addUser, createDefault, validate
    assert!(
        user_mgr.methods.len() >= 4,
        "UserManager should have at least 4 methods, got {}",
        user_mgr.methods.len()
    );

    let method_names: Vec<&str> = user_mgr.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"addUser"), "Missing addUser");
    assert!(
        method_names.contains(&"createDefault"),
        "Missing createDefault"
    );
    assert!(method_names.contains(&"validate"), "Missing validate");
    assert!(method_names.contains(&"init"), "Missing init");
}

#[test]
fn test_swift_class_properties() {
    let ir = extract_fixture("classes.swift");
    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    assert_eq!(user_mgr.properties.len(), 2, "UserManager has 2 properties");

    let users = user_mgr
        .properties
        .iter()
        .find(|p| p.name == "users")
        .expect("users property");
    assert!(!users.is_readonly, "users is var (mutable)");

    let max_users = user_mgr
        .properties
        .iter()
        .find(|p| p.name == "maxUsers")
        .expect("maxUsers property");
    assert!(max_users.is_readonly, "maxUsers is let (readonly)");
}

#[test]
fn test_swift_static_method() {
    let ir = extract_fixture("classes.swift");
    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    let create = user_mgr
        .methods
        .iter()
        .find(|m| m.name == "createDefault")
        .expect("createDefault");
    assert!(create.is_static, "createDefault should be static");
}

#[test]
fn test_swift_private_method() {
    let ir = extract_fixture("classes.swift");
    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    let validate = user_mgr
        .methods
        .iter()
        .find(|m| m.name == "validate")
        .expect("validate");
    assert_eq!(
        validate.visibility,
        Visibility::Private,
        "validate should be private"
    );
}

#[test]
fn test_swift_method_owner() {
    let ir = extract_fixture("classes.swift");
    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    for method in &user_mgr.methods {
        assert_eq!(
            method.owner.as_deref(),
            Some("UserManager"),
            "method {} owner should be UserManager",
            method.name
        );
    }
}

#[test]
fn test_swift_struct_properties() {
    let ir = extract_fixture("classes.swift");
    let point = ir
        .classes
        .iter()
        .find(|c| c.name == "Point")
        .expect("Point");
    assert_eq!(point.properties.len(), 2, "Point has 2 properties");

    for prop in &point.properties {
        assert!(prop.is_readonly, "Point properties are let (readonly)");
    }
}

#[test]
fn test_swift_class_inheritance() {
    let ir = extract_fixture("classes.swift");
    let admin = ir
        .classes
        .iter()
        .find(|c| c.name == "AdminManager")
        .expect("AdminManager");
    assert_eq!(
        admin.extends.as_deref(),
        Some("UserManager"),
        "AdminManager extends UserManager"
    );
}

#[test]
fn test_swift_class_visibility() {
    let ir = extract_fixture("classes.swift");
    let admin = ir
        .classes
        .iter()
        .find(|c| c.name == "AdminManager")
        .expect("AdminManager");
    assert_eq!(
        admin.visibility,
        Visibility::Public,
        "AdminManager is public"
    );

    let user_mgr = ir
        .classes
        .iter()
        .find(|c| c.name == "UserManager")
        .expect("UserManager");
    assert_eq!(
        user_mgr.visibility,
        Visibility::Internal,
        "UserManager defaults to internal"
    );
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_swift_imports_count() {
    let ir = extract_fixture("imports.swift");
    assert_eq!(ir.imports.len(), 6, "Expected 6 imports");
}

#[test]
fn test_swift_imports_sources() {
    let ir = extract_fixture("imports.swift");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"Foundation"), "Missing Foundation");
    assert!(sources.contains(&"UIKit"), "Missing UIKit");
    assert!(sources.contains(&"SwiftUI"), "Missing SwiftUI");
    assert!(sources.contains(&"Combine"), "Missing Combine");
    assert!(sources.contains(&"CoreData"), "Missing CoreData");
}

#[test]
fn test_swift_imports_are_wildcard() {
    let ir = extract_fixture("imports.swift");
    for imp in &ir.imports {
        assert!(imp.is_wildcard, "Swift imports are module-level (wildcard)");
    }
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_swift_services_count() {
    let ir = extract_fixture("services.swift");
    assert_eq!(ir.services.len(), 3, "Expected 3 services");
}

#[test]
fn test_swift_services_names() {
    let ir = extract_fixture("services.swift");
    let names: Vec<&str> = ir.services.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"APIController"), "Missing APIController");
    assert!(names.contains(&"UserRepository"), "Missing UserRepository");
}

#[test]
fn test_swift_services_kinds() {
    let ir = extract_fixture("services.swift");
    let user_svc = ir
        .services
        .iter()
        .find(|s| s.name == "UserService")
        .expect("UserService");
    assert_eq!(
        user_svc.kind,
        ServiceKind::Microservice,
        "UserService should be Microservice"
    );

    let controller = ir
        .services
        .iter()
        .find(|s| s.name == "APIController")
        .expect("APIController");
    assert_eq!(
        controller.kind,
        ServiceKind::HttpController,
        "APIController should be HttpController"
    );

    let repo = ir
        .services
        .iter()
        .find(|s| s.name == "UserRepository")
        .expect("UserRepository");
    assert_eq!(
        repo.kind,
        ServiceKind::Repository,
        "UserRepository should be Repository"
    );
}

#[test]
fn test_swift_services_methods() {
    let ir = extract_fixture("services.swift");
    let user_svc = ir
        .services
        .iter()
        .find(|s| s.name == "UserService")
        .expect("UserService");
    assert_eq!(user_svc.methods.len(), 3, "UserService has 3 methods");
}

#[test]
fn test_swift_services_attributes() {
    let ir = extract_fixture("services.swift");
    let user_svc = ir
        .services
        .iter()
        .find(|s| s.name == "UserService")
        .expect("UserService");
    assert!(
        user_svc.decorators.contains(&"MainActor".to_string()),
        "UserService should have @MainActor"
    );
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_swift_schemas_count() {
    let ir = extract_fixture("schemas.swift");
    // UserDTO, ProductResponse, OrderModel — InternalConfig is NOT Codable
    assert_eq!(ir.schemas.len(), 3, "Expected 3 Codable schemas");
}

#[test]
fn test_swift_schemas_names() {
    let ir = extract_fixture("schemas.swift");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserDTO"), "Missing UserDTO");
    assert!(
        names.contains(&"ProductResponse"),
        "Missing ProductResponse"
    );
    assert!(names.contains(&"OrderModel"), "Missing OrderModel");
    assert!(
        !names.contains(&"InternalConfig"),
        "InternalConfig should NOT be detected (not Codable)"
    );
}

#[test]
fn test_swift_schemas_kinds() {
    let ir = extract_fixture("schemas.swift");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDTO")
        .expect("UserDTO");
    assert_eq!(
        user_dto.kind,
        SchemaKind::DataTransfer,
        "UserDTO (struct) should be DataTransfer"
    );

    let order = ir
        .schemas
        .iter()
        .find(|s| s.name == "OrderModel")
        .expect("OrderModel");
    assert_eq!(
        order.kind,
        SchemaKind::OrmModel,
        "OrderModel (class) should be OrmModel"
    );
}

#[test]
fn test_swift_schemas_fields() {
    let ir = extract_fixture("schemas.swift");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDTO")
        .expect("UserDTO");
    assert_eq!(user_dto.fields.len(), 4, "UserDTO has 4 fields");

    let field_names: Vec<&str> = user_dto.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"), "Missing id field");
    assert!(field_names.contains(&"name"), "Missing name field");
    assert!(field_names.contains(&"email"), "Missing email field");
    assert!(field_names.contains(&"age"), "Missing age field");
}

#[test]
fn test_swift_schemas_optional_fields() {
    let ir = extract_fixture("schemas.swift");
    let product = ir
        .schemas
        .iter()
        .find(|s| s.name == "ProductResponse")
        .expect("ProductResponse");
    let desc = product
        .fields
        .iter()
        .find(|f| f.name == "description")
        .expect("description field");
    assert!(desc.is_optional, "description should be optional (String?)");
}

#[test]
fn test_swift_schemas_framework() {
    let ir = extract_fixture("schemas.swift");
    for schema in &ir.schemas {
        assert_eq!(
            schema.source_framework, "swift-codable",
            "framework should be swift-codable"
        );
    }
}

// =========================================================================
// extensions.scm tests
// =========================================================================

#[test]
fn test_swift_extensions_count() {
    let ir = extract_fixture("extensions.swift");
    assert_eq!(ir.implementations.len(), 3, "Expected 3 extensions");
}

#[test]
fn test_swift_extensions_targets() {
    let ir = extract_fixture("extensions.swift");
    let targets: Vec<&str> = ir
        .implementations
        .iter()
        .map(|i| i.target.as_str())
        .collect();
    assert!(targets.contains(&"MyClass"), "Missing MyClass extension");
    assert!(targets.contains(&"String"), "Missing String extension");
}

#[test]
fn test_swift_extension_protocol_conformance() {
    let ir = extract_fixture("extensions.swift");
    let printable_ext = ir
        .implementations
        .iter()
        .find(|i| i.target == "MyClass" && i.trait_name.as_deref() == Some("Printable"))
        .expect("MyClass: Printable extension");
    assert_eq!(
        printable_ext.methods.len(),
        1,
        "Printable extension has 1 method"
    );
}

#[test]
fn test_swift_extension_methods() {
    let ir = extract_fixture("extensions.swift");
    let simple_ext = ir
        .implementations
        .iter()
        .find(|i| i.target == "MyClass" && i.trait_name.is_none())
        .expect("Simple MyClass extension");
    assert_eq!(
        simple_ext.methods.len(),
        2,
        "Simple extension has 2 methods"
    );
}

// =========================================================================
// Top-level functions test
// =========================================================================

#[test]
fn test_swift_top_level_functions() {
    let ir = extract_fixture("classes.swift");
    // No top-level functions in classes.swift
    assert!(
        ir.functions.is_empty(),
        "classes.swift has no top-level functions"
    );
}

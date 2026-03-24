//! Integration tests for C# query extraction.
//! Each test parses a real C# fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/csharp/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::CSharp)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::CSharp,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_csharp_interfaces_count() {
    let ir = extract_fixture("interfaces.cs");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 interfaces");
}

#[test]
fn test_csharp_interface_names() {
    let ir = extract_fixture("interfaces.cs");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"IUserRepository"), "Missing IUserRepository");
    assert!(
        names.contains(&"INotificationService"),
        "Missing INotificationService"
    );
    assert!(names.contains(&"IRepository"), "Missing IRepository");
    assert!(
        names.contains(&"IInternalService"),
        "Missing IInternalService"
    );
}

#[test]
fn test_csharp_interface_methods() {
    let ir = extract_fixture("interfaces.cs");
    let repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IUserRepository")
        .expect("IUserRepository not found");
    assert_eq!(repo.methods.len(), 4, "IUserRepository should have 4 methods");
    let method_names: Vec<&str> = repo.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"GetById"));
    assert!(method_names.contains(&"GetAll"));
    assert!(method_names.contains(&"Save"));
    assert!(method_names.contains(&"Delete"));
}

#[test]
fn test_csharp_interface_visibility() {
    let ir = extract_fixture("interfaces.cs");
    let public_if = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IUserRepository")
        .expect("IUserRepository not found");
    assert_eq!(public_if.visibility, Visibility::Public);

    let internal_if = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IInternalService")
        .expect("IInternalService not found");
    assert_eq!(internal_if.visibility, Visibility::Internal);
}

#[test]
fn test_csharp_interface_generics() {
    let ir = extract_fixture("interfaces.cs");
    let generic = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IRepository")
        .expect("IRepository not found");
    assert_eq!(generic.generics, vec!["T"]);
}

#[test]
fn test_csharp_interface_kind() {
    let ir = extract_fixture("interfaces.cs");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Interface);
    }
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_csharp_classes_count() {
    let ir = extract_fixture("classes.cs");
    assert_eq!(ir.classes.len(), 4, "Expected 4 classes");
}

#[test]
fn test_csharp_class_names() {
    let ir = extract_fixture("classes.cs");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"BaseEntity"), "Missing BaseEntity");
    assert!(names.contains(&"GenericHandler"), "Missing GenericHandler");
    assert!(names.contains(&"InternalHelper"), "Missing InternalHelper");
}

#[test]
fn test_csharp_class_methods() {
    let ir = extract_fixture("classes.cs");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    // constructor + GetUser + CreateUser + ValidateUser + Create = 5
    assert_eq!(svc.methods.len(), 5, "UserService should have 5 methods");
}

#[test]
fn test_csharp_class_method_async() {
    let ir = extract_fixture("classes.cs");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    let get_user = svc
        .methods
        .iter()
        .find(|m| m.name == "GetUser")
        .expect("GetUser not found");
    assert!(get_user.is_async, "GetUser should be async");
}

#[test]
fn test_csharp_class_method_static() {
    let ir = extract_fixture("classes.cs");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    let create = svc
        .methods
        .iter()
        .find(|m| m.name == "Create")
        .expect("Create not found");
    assert!(create.is_static, "Create should be static");
}

#[test]
fn test_csharp_class_abstract() {
    let ir = extract_fixture("classes.cs");
    let base = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseEntity")
        .expect("BaseEntity not found");
    assert!(base.is_abstract, "BaseEntity should be abstract");
}

#[test]
fn test_csharp_class_visibility() {
    let ir = extract_fixture("classes.cs");
    let public_cls = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    assert_eq!(public_cls.visibility, Visibility::Public);

    let internal_cls = ir
        .classes
        .iter()
        .find(|c| c.name == "InternalHelper")
        .expect("InternalHelper not found");
    assert_eq!(internal_cls.visibility, Visibility::Internal);
}

#[test]
fn test_csharp_class_generics() {
    let ir = extract_fixture("classes.cs");
    let handler = ir
        .classes
        .iter()
        .find(|c| c.name == "GenericHandler")
        .expect("GenericHandler not found");
    assert_eq!(handler.generics, vec!["T"]);
}

#[test]
fn test_csharp_class_properties() {
    let ir = extract_fixture("classes.cs");
    let base = ir
        .classes
        .iter()
        .find(|c| c.name == "BaseEntity")
        .expect("BaseEntity not found");
    assert!(base.properties.len() >= 3, "BaseEntity should have at least 3 properties");
    let prop_names: Vec<&str> = base.properties.iter().map(|p| p.name.as_str()).collect();
    assert!(prop_names.contains(&"Id"));
    assert!(prop_names.contains(&"CreatedAt"));
    assert!(prop_names.contains(&"UpdatedAt"));
}

#[test]
fn test_csharp_class_method_owner() {
    let ir = extract_fixture("classes.cs");
    let svc = ir
        .classes
        .iter()
        .find(|c| c.name == "UserService")
        .expect("UserService not found");
    for method in &svc.methods {
        assert_eq!(
            method.owner.as_deref(),
            Some("UserService"),
            "Method {} should have owner UserService",
            method.name
        );
    }
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_csharp_imports_count() {
    let ir = extract_fixture("imports.cs");
    assert!(ir.imports.len() >= 6, "Expected at least 6 imports, got {}", ir.imports.len());
}

#[test]
fn test_csharp_import_sources() {
    let ir = extract_fixture("imports.cs");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(
        sources.iter().any(|s| s.contains("System")),
        "Should have System import"
    );
    assert!(
        sources.iter().any(|s| s.contains("Microsoft.AspNetCore.Mvc")),
        "Should have AspNetCore.Mvc import"
    );
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_csharp_services_count() {
    let ir = extract_fixture("services.cs");
    assert_eq!(ir.services.len(), 3, "Expected 3 services");
}

#[test]
fn test_csharp_service_names() {
    let ir = extract_fixture("services.cs");
    let names: Vec<&str> = ir.services.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UsersController"), "Missing UsersController");
    assert!(names.contains(&"HealthController"), "Missing HealthController");
    assert!(names.contains(&"OrderService"), "Missing OrderService");
}

#[test]
fn test_csharp_service_kind() {
    let ir = extract_fixture("services.cs");
    let users = ir
        .services
        .iter()
        .find(|s| s.name == "UsersController")
        .expect("UsersController not found");
    assert_eq!(users.kind, ServiceKind::HttpController);

    let orders = ir
        .services
        .iter()
        .find(|s| s.name == "OrderService")
        .expect("OrderService not found");
    assert_eq!(orders.kind, ServiceKind::Microservice);
}

#[test]
fn test_csharp_service_routes() {
    let ir = extract_fixture("services.cs");
    let users = ir
        .services
        .iter()
        .find(|s| s.name == "UsersController")
        .expect("UsersController not found");
    assert!(
        !users.routes.is_empty(),
        "UsersController should have routes"
    );
    let methods: Vec<&HttpMethod> = users.routes.iter().map(|r| &r.method).collect();
    assert!(methods.contains(&&HttpMethod::Get));
    assert!(methods.contains(&&HttpMethod::Post));
    assert!(methods.contains(&&HttpMethod::Put));
    assert!(methods.contains(&&HttpMethod::Delete));
}

#[test]
fn test_csharp_service_methods() {
    let ir = extract_fixture("services.cs");
    let users = ir
        .services
        .iter()
        .find(|s| s.name == "UsersController")
        .expect("UsersController not found");
    // constructor + GetAll + GetById + Create + Update + Delete = 6
    assert_eq!(
        users.methods.len(),
        6,
        "UsersController should have 6 methods"
    );
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_csharp_schemas_count() {
    let ir = extract_fixture("schemas.cs");
    // 2 records + 2 EF entities = 4 (NotASchema filtered out)
    assert_eq!(ir.schemas.len(), 4, "Expected 4 schemas, got {}", ir.schemas.len());
}

#[test]
fn test_csharp_schema_names() {
    let ir = extract_fixture("schemas.cs");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserDto"), "Missing UserDto");
    assert!(names.contains(&"OrderDto"), "Missing OrderDto");
    assert!(names.contains(&"UserEntity"), "Missing UserEntity");
    assert!(names.contains(&"OrderEntity"), "Missing OrderEntity");
    assert!(!names.contains(&"NotASchema"), "NotASchema should be filtered out");
}

#[test]
fn test_csharp_schema_kinds() {
    let ir = extract_fixture("schemas.cs");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDto")
        .expect("UserDto not found");
    assert_eq!(user_dto.kind, SchemaKind::DataTransfer);
    assert_eq!(user_dto.source_framework, "csharp-record");

    let user_entity = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserEntity")
        .expect("UserEntity not found");
    assert_eq!(user_entity.kind, SchemaKind::OrmModel);
    assert_eq!(user_entity.source_framework, "ef-core");
}

#[test]
fn test_csharp_record_fields() {
    let ir = extract_fixture("schemas.cs");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDto")
        .expect("UserDto not found");
    assert_eq!(user_dto.fields.len(), 3, "UserDto should have 3 fields");
    let field_names: Vec<&str> = user_dto.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"Name"));
    assert!(field_names.contains(&"Email"));
    assert!(field_names.contains(&"Age"));
}

#[test]
fn test_csharp_entity_fields_primary_key() {
    let ir = extract_fixture("schemas.cs");
    let user_entity = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserEntity")
        .expect("UserEntity not found");
    assert!(
        user_entity.fields.len() >= 4,
        "UserEntity should have at least 4 fields"
    );
    let id_field = user_entity
        .fields
        .iter()
        .find(|f| f.name == "Id")
        .expect("Id field not found");
    assert!(id_field.is_primary_key, "Id field should be primary key");
}

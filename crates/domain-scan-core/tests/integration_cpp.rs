//! Integration tests for C++ query extraction.
//! Each test parses a real C++ fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/cpp/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Cpp)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Cpp,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_cpp_classes_count() {
    let ir = extract_fixture("classes.cpp");
    // UserService, BaseRepository, Point, Container
    assert!(
        ir.classes.len() >= 4,
        "Expected at least 4 classes, got {}",
        ir.classes.len()
    );
}

#[test]
fn test_cpp_class_names() {
    let ir = extract_fixture("classes.cpp");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"), "Missing UserService");
    assert!(names.contains(&"Point"), "Missing Point (struct)");
}

#[test]
fn test_cpp_class_methods() {
    let ir = extract_fixture("classes.cpp");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService");
    assert!(user_service.is_some(), "UserService class not found");
    let user_service = user_service.unwrap();
    // getName, setName, instanceCount (constructor and destructor excluded)
    assert!(
        user_service.methods.len() >= 3,
        "Expected at least 3 methods on UserService, got {}",
        user_service.methods.len()
    );
}

#[test]
fn test_cpp_class_properties() {
    let ir = extract_fixture("classes.cpp");
    let point = ir.classes.iter().find(|c| c.name == "Point");
    assert!(point.is_some(), "Point struct not found");
    let point = point.unwrap();
    assert_eq!(
        point.properties.len(),
        3,
        "Point should have 3 properties (x, y, z)"
    );
}

#[test]
fn test_cpp_class_is_abstract() {
    let ir = extract_fixture("classes.cpp");
    let base = ir.classes.iter().find(|c| c.name == "BaseRepository");
    assert!(base.is_some(), "BaseRepository not found");
    assert!(
        base.unwrap().is_abstract,
        "BaseRepository should be abstract"
    );
}

#[test]
fn test_cpp_template_class_generics() {
    let ir = extract_fixture("classes.cpp");
    let container = ir.classes.iter().find(|c| c.name == "Container");
    assert!(container.is_some(), "Container class not found");
    let container = container.unwrap();
    assert!(
        !container.generics.is_empty(),
        "Container should have template parameters"
    );
    assert!(
        container.generics.contains(&"T".to_string()),
        "Container should have generic T"
    );
}

#[test]
fn test_cpp_struct_default_public_properties() {
    let ir = extract_fixture("classes.cpp");
    let point = ir.classes.iter().find(|c| c.name == "Point");
    assert!(point.is_some(), "Point not found");
    let point = point.unwrap();
    for prop in &point.properties {
        assert_eq!(
            prop.visibility,
            Visibility::Public,
            "Struct properties should default to public"
        );
    }
}

#[test]
fn test_cpp_method_ownership() {
    let ir = extract_fixture("classes.cpp");
    let user_service = ir.classes.iter().find(|c| c.name == "UserService").unwrap();
    for method in &user_service.methods {
        assert_eq!(
            method.owner.as_deref(),
            Some("UserService"),
            "Methods should have owner set"
        );
    }
}

// =========================================================================
// functions.scm tests
// =========================================================================

#[test]
fn test_cpp_functions_count() {
    let ir = extract_fixture("functions.cpp");
    // add, multiply, greet, helper
    assert!(
        ir.functions.len() >= 4,
        "Expected at least 4 functions, got {}",
        ir.functions.len()
    );
}

#[test]
fn test_cpp_function_names() {
    let ir = extract_fixture("functions.cpp");
    let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add"), "Missing add");
    assert!(names.contains(&"multiply"), "Missing multiply");
    assert!(names.contains(&"greet"), "Missing greet");
}

#[test]
fn test_cpp_function_parameters() {
    let ir = extract_fixture("functions.cpp");
    let add = ir.functions.iter().find(|f| f.name == "add").unwrap();
    assert_eq!(add.parameters.len(), 2, "add should have 2 parameters");
    assert_eq!(add.parameters[0].name, "a");
    assert_eq!(add.parameters[1].name, "b");
}

#[test]
fn test_cpp_function_return_type() {
    let ir = extract_fixture("functions.cpp");
    let add = ir.functions.iter().find(|f| f.name == "add").unwrap();
    assert_eq!(add.return_type.as_deref(), Some("int"));
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_cpp_imports_count() {
    let ir = extract_fixture("imports.cpp");
    assert!(
        ir.imports.len() >= 6,
        "Expected at least 6 includes, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_cpp_import_sources() {
    let ir = extract_fixture("imports.cpp");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"iostream"), "Missing iostream");
    assert!(sources.contains(&"vector"), "Missing vector");
    assert!(sources.contains(&"myheader.h"), "Missing myheader.h");
}

// =========================================================================
// virtual.scm tests (interfaces / abstract base classes)
// =========================================================================

#[test]
fn test_cpp_interfaces_count() {
    let ir = extract_fixture("interfaces.cpp");
    // IRepository, INotificationService (both have pure virtual methods)
    // ConcreteClass and BaseClass should NOT be interfaces
    assert_eq!(
        ir.interfaces.len(),
        2,
        "Expected 2 interfaces (abstract base classes), got {}",
        ir.interfaces.len()
    );
}

#[test]
fn test_cpp_interface_names() {
    let ir = extract_fixture("interfaces.cpp");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"IRepository"), "Missing IRepository");
    assert!(
        names.contains(&"INotificationService"),
        "Missing INotificationService"
    );
}

#[test]
fn test_cpp_interface_kind() {
    let ir = extract_fixture("interfaces.cpp");
    for iface in &ir.interfaces {
        assert_eq!(
            iface.language_kind,
            InterfaceKind::PureVirtual,
            "C++ interfaces should be PureVirtual"
        );
    }
}

#[test]
fn test_cpp_interface_methods() {
    let ir = extract_fixture("interfaces.cpp");
    let repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "IRepository")
        .unwrap();
    // save, remove, count (destructor excluded from method signatures)
    assert!(
        repo.methods.len() >= 3,
        "IRepository should have at least 3 method signatures, got {}",
        repo.methods.len()
    );
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_cpp_schemas_count() {
    let ir = extract_fixture("schemas.cpp");
    // UserDto, OrderDto (ActiveRecord has methods, excluded)
    assert_eq!(
        ir.schemas.len(),
        2,
        "Expected 2 schemas, got {}",
        ir.schemas.len()
    );
}

#[test]
fn test_cpp_schema_names() {
    let ir = extract_fixture("schemas.cpp");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserDto"), "Missing UserDto");
    assert!(names.contains(&"OrderDto"), "Missing OrderDto");
}

#[test]
fn test_cpp_schema_fields() {
    let ir = extract_fixture("schemas.cpp");
    let user_dto = ir.schemas.iter().find(|s| s.name == "UserDto").unwrap();
    assert_eq!(user_dto.fields.len(), 3, "UserDto should have 3 fields");
    let field_names: Vec<&str> = user_dto.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"), "Missing id field");
    assert!(field_names.contains(&"name"), "Missing name field");
    assert!(field_names.contains(&"email"), "Missing email field");
}

#[test]
fn test_cpp_schema_framework() {
    let ir = extract_fixture("schemas.cpp");
    for schema in &ir.schemas {
        assert_eq!(schema.source_framework, "cpp-struct");
    }
}

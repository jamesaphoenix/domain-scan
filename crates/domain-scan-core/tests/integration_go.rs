//! Integration tests for Go query extraction.
//! Each test parses a real Go fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from source string
fn extract_go(source: &str) -> IrFile {
    let tree = parse_source(source.as_bytes(), Language::Go)
        .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new("test.go"),
        Language::Go,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract: {e}"))
}

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/go/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Go)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Go,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// interfaces.scm tests
// =========================================================================

#[test]
fn test_go_interfaces_count() {
    let ir = extract_fixture("interfaces.go");
    assert_eq!(ir.interfaces.len(), 5, "Expected 5 interfaces");
}

#[test]
fn test_go_interface_names() {
    let ir = extract_fixture("interfaces.go");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Reader"));
    assert!(names.contains(&"Writer"));
    assert!(names.contains(&"ReadWriter"));
    assert!(names.contains(&"UserService"));
    assert!(names.contains(&"privateInterface"));
}

#[test]
fn test_go_interface_kind() {
    let ir = extract_fixture("interfaces.go");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Interface);
    }
}

#[test]
fn test_go_interface_visibility() {
    let ir = extract_fixture("interfaces.go");
    let reader = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Reader")
        .expect("Reader not found");
    assert_eq!(reader.visibility, Visibility::Public);

    let private = ir
        .interfaces
        .iter()
        .find(|i| i.name == "privateInterface")
        .expect("privateInterface not found");
    assert_eq!(private.visibility, Visibility::Private);
}

#[test]
fn test_go_interface_methods() {
    let ir = extract_fixture("interfaces.go");
    let user_svc = ir
        .interfaces
        .iter()
        .find(|i| i.name == "UserService")
        .expect("UserService not found");
    assert_eq!(user_svc.methods.len(), 3);
    let method_names: Vec<&str> = user_svc.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"GetUser"));
    assert!(method_names.contains(&"CreateUser"));
    assert!(method_names.contains(&"DeleteUser"));
}

#[test]
fn test_go_interface_extends() {
    let ir = extract_fixture("interfaces.go");
    let rw = ir
        .interfaces
        .iter()
        .find(|i| i.name == "ReadWriter")
        .expect("ReadWriter not found");
    assert!(rw.extends.contains(&"Reader".to_string()));
    assert!(rw.extends.contains(&"Writer".to_string()));
}

#[test]
fn test_go_interface_method_params() {
    let ir = extract_fixture("interfaces.go");
    let reader = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Reader")
        .expect("Reader not found");
    let read = reader
        .methods
        .iter()
        .find(|m| m.name == "Read")
        .expect("Read method not found");
    assert_eq!(read.parameters.len(), 1);
    assert_eq!(read.parameters[0].name, "p");
}

// =========================================================================
// structs.scm tests
// =========================================================================

#[test]
fn test_go_structs_count() {
    let ir = extract_fixture("structs.go");
    assert_eq!(ir.classes.len(), 3, "Expected 3 structs");
}

#[test]
fn test_go_struct_names() {
    let ir = extract_fixture("structs.go");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"User"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"internalState"));
}

#[test]
fn test_go_struct_fields() {
    let ir = extract_fixture("structs.go");
    let user = ir
        .classes
        .iter()
        .find(|c| c.name == "User")
        .expect("User not found");
    assert_eq!(user.properties.len(), 4);
    let id = user
        .properties
        .iter()
        .find(|p| p.name == "ID")
        .expect("ID field not found");
    assert_eq!(id.type_annotation.as_deref(), Some("string"));
    assert_eq!(id.visibility, Visibility::Public);
}

#[test]
fn test_go_struct_visibility() {
    let ir = extract_fixture("structs.go");
    let internal = ir
        .classes
        .iter()
        .find(|c| c.name == "internalState")
        .expect("internalState not found");
    assert_eq!(internal.visibility, Visibility::Private);
}

// =========================================================================
// functions.scm tests
// =========================================================================

#[test]
fn test_go_functions_count() {
    let ir = extract_fixture("functions.go");
    assert_eq!(ir.functions.len(), 5, "Expected 5 functions");
}

#[test]
fn test_go_function_names() {
    let ir = extract_fixture("functions.go");
    let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"Add"));
    assert!(names.contains(&"FetchData"));
    assert!(names.contains(&"processItems"));
    assert!(names.contains(&"NewUserService"));
    assert!(names.contains(&"validateInput"));
}

#[test]
fn test_go_function_visibility() {
    let ir = extract_fixture("functions.go");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "Add")
        .expect("Add not found");
    assert_eq!(add.visibility, Visibility::Public);

    let process = ir
        .functions
        .iter()
        .find(|f| f.name == "processItems")
        .expect("processItems not found");
    assert_eq!(process.visibility, Visibility::Private);
}

#[test]
fn test_go_function_params() {
    let ir = extract_fixture("functions.go");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "Add")
        .expect("Add not found");
    assert_eq!(add.parameters.len(), 2);
    assert_eq!(add.parameters[0].name, "a");
    assert_eq!(add.parameters[0].type_annotation.as_deref(), Some("int"));
}

#[test]
fn test_go_function_return_type() {
    let ir = extract_fixture("functions.go");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "Add")
        .expect("Add not found");
    assert!(add.return_type.is_some());
}

// =========================================================================
// methods.scm tests
// =========================================================================

#[test]
fn test_go_methods_grouped_by_receiver() {
    let ir = extract_fixture("methods.go");
    // Methods should be grouped into ImplDef by receiver type
    assert!(
        ir.implementations.len() >= 2,
        "Expected at least 2 impl groups (UserRepo, Logger)"
    );
}

#[test]
fn test_go_method_receiver_type() {
    let ir = extract_fixture("methods.go");
    let user_repo = ir
        .implementations
        .iter()
        .find(|i| i.target == "UserRepo")
        .expect("UserRepo impl not found");
    assert_eq!(user_repo.methods.len(), 3);
    let method_names: Vec<&str> = user_repo.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"FindByID"));
    assert!(method_names.contains(&"Save"));
    assert!(method_names.contains(&"Delete"));
}

#[test]
fn test_go_method_owner() {
    let ir = extract_fixture("methods.go");
    let user_repo = ir
        .implementations
        .iter()
        .find(|i| i.target == "UserRepo")
        .expect("UserRepo impl not found");
    for method in &user_repo.methods {
        assert_eq!(method.owner.as_deref(), Some("UserRepo"));
    }
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_go_imports_count() {
    let ir = extract_fixture("imports.go");
    assert!(
        ir.imports.len() >= 5,
        "Expected at least 5 imports, got {}",
        ir.imports.len()
    );
}

#[test]
fn test_go_import_paths() {
    let ir = extract_fixture("imports.go");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"fmt"));
    assert!(sources.contains(&"context"));
    assert!(sources.contains(&"net/http"));
}

#[test]
fn test_go_import_alias() {
    let ir = extract_fixture("imports.go");
    let aliased = ir
        .imports
        .iter()
        .find(|i| i.source == "github.com/example/pkg")
        .expect("aliased import not found");
    assert_eq!(aliased.symbols[0].alias.as_deref(), Some("myalias"));
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_go_schemas_tagged_structs() {
    let ir = extract_fixture("schemas.go");
    assert_eq!(ir.schemas.len(), 2, "Expected 2 tagged struct schemas");
}

#[test]
fn test_go_schema_names() {
    let ir = extract_fixture("schemas.go");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserDTO"));
    assert!(names.contains(&"CreateUserRequest"));
}

#[test]
fn test_go_schema_fields() {
    let ir = extract_fixture("schemas.go");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDTO")
        .expect("UserDTO not found");
    assert_eq!(user_dto.fields.len(), 4);
    let email = user_dto
        .fields
        .iter()
        .find(|f| f.name == "Email")
        .expect("Email field not found");
    assert!(email.constraints.contains(&"omitempty".to_string()));
}

#[test]
fn test_go_schema_non_tagged_excluded() {
    let ir = extract_fixture("schemas.go");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&"InternalConfig"),
        "InternalConfig should not be a schema (no tags)"
    );
}

// =========================================================================
// Inline tests
// =========================================================================

#[test]
fn test_go_simple_interface() {
    let ir = extract_go("package main\n\ntype Stringer interface {\n\tString() string\n}");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].name, "Stringer");
    assert_eq!(ir.interfaces[0].methods.len(), 1);
}

#[test]
fn test_go_simple_struct() {
    let ir = extract_go("package main\n\ntype Point struct {\n\tX float64\n\tY float64\n}");
    assert_eq!(ir.classes.len(), 1);
    assert_eq!(ir.classes[0].name, "Point");
    assert_eq!(ir.classes[0].properties.len(), 2);
}

#[test]
fn test_go_cross_language_consistency() {
    let ir = extract_go("package main\n\ntype MyInterface interface {\n\tDoThing() error\n}");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].language_kind, InterfaceKind::Interface);
    assert_eq!(ir.language, Language::Go);
}

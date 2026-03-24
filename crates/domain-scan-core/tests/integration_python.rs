//! Integration tests for Python query extraction.
//! Each test parses a real Python fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from source string
fn extract_py(source: &str) -> IrFile {
    let tree = parse_source(source.as_bytes(), Language::Python)
        .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new("test.py"),
        Language::Python,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract: {e}"))
}

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/python/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Python)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Python,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// classes.scm tests
// =========================================================================

#[test]
fn test_python_classes_count() {
    let ir = extract_fixture("classes.py");
    assert_eq!(ir.classes.len(), 3, "Expected 3 classes");
}

#[test]
fn test_python_class_names() {
    let ir = extract_fixture("classes.py");
    let names: Vec<&str> = ir.classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"UserService"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"_InternalHelper"));
}

#[test]
fn test_python_class_methods() {
    let ir = extract_fixture("classes.py");
    let user_svc = ir.classes.iter().find(|c| c.name == "UserService")
        .expect("UserService not found");
    // __init__ is excluded, so 3 methods
    assert_eq!(user_svc.methods.len(), 3, "Expected 3 methods (excluding __init__)");
    let method_names: Vec<&str> = user_svc.methods.iter().map(|m| m.name.as_str()).collect();
    assert!(method_names.contains(&"get_user"));
    assert!(method_names.contains(&"create_user"));
    assert!(method_names.contains(&"delete_user"));
}

#[test]
fn test_python_class_method_async() {
    let ir = extract_fixture("classes.py");
    let user_svc = ir.classes.iter().find(|c| c.name == "UserService")
        .expect("UserService not found");
    let create = user_svc.methods.iter().find(|m| m.name == "create_user")
        .expect("create_user not found");
    assert!(create.is_async, "create_user should be async");
}

#[test]
fn test_python_class_visibility() {
    let ir = extract_fixture("classes.py");
    let internal = ir.classes.iter().find(|c| c.name == "_InternalHelper")
        .expect("_InternalHelper not found");
    assert_eq!(internal.visibility, Visibility::Protected);
}

// =========================================================================
// functions.scm tests
// =========================================================================

#[test]
fn test_python_functions_count() {
    let ir = extract_fixture("functions.py");
    assert_eq!(ir.functions.len(), 5, "Expected 5 top-level functions");
}

#[test]
fn test_python_function_names() {
    let ir = extract_fixture("functions.py");
    let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"fetch_data"));
    assert!(names.contains(&"process_items"));
    assert!(names.contains(&"_private_helper"));
    assert!(names.contains(&"create_handler"));
}

#[test]
fn test_python_function_async() {
    let ir = extract_fixture("functions.py");
    let fetch = ir.functions.iter().find(|f| f.name == "fetch_data")
        .expect("fetch_data not found");
    assert!(fetch.is_async, "fetch_data should be async");
}

#[test]
fn test_python_function_parameters() {
    let ir = extract_fixture("functions.py");
    let add = ir.functions.iter().find(|f| f.name == "add")
        .expect("add not found");
    assert_eq!(add.parameters.len(), 2);
    assert_eq!(add.parameters[0].name, "a");
    assert_eq!(add.parameters[0].type_annotation.as_deref(), Some("int"));
}

#[test]
fn test_python_function_return_type() {
    let ir = extract_fixture("functions.py");
    let add = ir.functions.iter().find(|f| f.name == "add")
        .expect("add not found");
    assert_eq!(add.return_type.as_deref(), Some("int"));
}

#[test]
fn test_python_function_visibility() {
    let ir = extract_fixture("functions.py");
    let private = ir.functions.iter().find(|f| f.name == "_private_helper")
        .expect("_private_helper not found");
    assert_eq!(private.visibility, Visibility::Protected);
}

// =========================================================================
// protocols.scm tests
// =========================================================================

#[test]
fn test_python_protocols() {
    let ir = extract_fixture("protocols.py");
    assert!(ir.interfaces.len() >= 3, "Expected at least 3 protocols, got {}", ir.interfaces.len());
}

#[test]
fn test_python_protocol_kind() {
    let ir = extract_fixture("protocols.py");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Protocol);
    }
}

#[test]
fn test_python_protocol_names() {
    let ir = extract_fixture("protocols.py");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"Readable"));
    assert!(names.contains(&"Writable"));
    assert!(names.contains(&"Repository"));
}

// =========================================================================
// abstract.scm tests
// =========================================================================

#[test]
fn test_python_abstract_classes() {
    let ir = extract_fixture("abstract.py");
    assert!(ir.interfaces.len() >= 2, "Expected at least 2 ABC classes, got {}", ir.interfaces.len());
}

#[test]
fn test_python_abstract_kind() {
    let ir = extract_fixture("abstract.py");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::AbstractClass);
    }
}

#[test]
fn test_python_abstract_names() {
    let ir = extract_fixture("abstract.py");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"BaseHandler"));
    assert!(names.contains(&"BaseRepository"));
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_python_imports_count() {
    let ir = extract_fixture("imports.py");
    assert!(ir.imports.len() >= 6, "Expected at least 6 imports, got {}", ir.imports.len());
}

#[test]
fn test_python_import_modules() {
    let ir = extract_fixture("imports.py");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"os"));
    assert!(sources.contains(&"typing"));
}

#[test]
fn test_python_import_from_symbols() {
    let ir = extract_fixture("imports.py");
    let typing_import = ir.imports.iter().find(|i| i.source == "typing")
        .expect("typing import not found");
    let symbol_names: Vec<&str> = typing_import.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(symbol_names.contains(&"Optional"));
    assert!(symbol_names.contains(&"List"));
}

#[test]
fn test_python_import_alias() {
    let ir = extract_fixture("imports.py");
    let json_import = ir.imports.iter().find(|i| i.source == "json")
        .expect("json import not found");
    assert_eq!(json_import.symbols[0].alias.as_deref(), Some("j"));
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_python_schemas_count() {
    let ir = extract_fixture("schemas.py");
    assert!(ir.schemas.len() >= 3, "Expected at least 3 schemas, got {}", ir.schemas.len());
}

#[test]
fn test_python_schema_pydantic() {
    let ir = extract_fixture("schemas.py");
    let user_schema = ir.schemas.iter().find(|s| s.name == "UserSchema")
        .expect("UserSchema not found");
    assert_eq!(user_schema.source_framework, "pydantic");
    assert_eq!(user_schema.kind, SchemaKind::ValidationSchema);
}

#[test]
fn test_python_schema_dataclass() {
    let ir = extract_fixture("schemas.py");
    let user_dto = ir.schemas.iter().find(|s| s.name == "UserDTO")
        .expect("UserDTO not found");
    assert_eq!(user_dto.source_framework, "dataclass");
    assert_eq!(user_dto.kind, SchemaKind::DataTransfer);
}

#[test]
fn test_python_schema_typeddict() {
    let ir = extract_fixture("schemas.py");
    let user_dict = ir.schemas.iter().find(|s| s.name == "UserDict")
        .expect("UserDict not found");
    assert_eq!(user_dict.source_framework, "typeddict");
    assert_eq!(user_dict.kind, SchemaKind::DataTransfer);
}

// =========================================================================
// services.scm tests
// =========================================================================

#[test]
fn test_python_services_fastapi() {
    let ir = extract_fixture("services.py");
    assert!(ir.services.len() >= 4, "Expected at least 4 FastAPI services, got {}", ir.services.len());
}

#[test]
fn test_python_service_names() {
    let ir = extract_fixture("services.py");
    let names: Vec<&str> = ir.services.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"get_user"));
    assert!(names.contains(&"create_user"));
    assert!(names.contains(&"delete_user"));
    assert!(names.contains(&"update_user"));
}

#[test]
fn test_python_service_kind() {
    let ir = extract_fixture("services.py");
    for svc in &ir.services {
        assert_eq!(svc.kind, ServiceKind::HttpController);
    }
}

#[test]
fn test_python_service_routes() {
    let ir = extract_fixture("services.py");
    let get_user = ir.services.iter().find(|s| s.name == "get_user")
        .expect("get_user not found");
    assert_eq!(get_user.routes.len(), 1);
    assert_eq!(get_user.routes[0].method, HttpMethod::Get);
    assert_eq!(get_user.routes[0].path, "/users/{user_id}");
}

// =========================================================================
// Inline tests
// =========================================================================

#[test]
fn test_python_simple_class() {
    let ir = extract_py("class Foo:\n    def bar(self) -> int:\n        return 42");
    assert_eq!(ir.classes.len(), 1);
    assert_eq!(ir.classes[0].name, "Foo");
    assert_eq!(ir.classes[0].methods.len(), 1);
}

#[test]
fn test_python_simple_function() {
    let ir = extract_py("def hello():\n    pass");
    assert_eq!(ir.functions.len(), 1);
    assert_eq!(ir.functions[0].name, "hello");
}

#[test]
fn test_python_protocol_inline() {
    let ir = extract_py("from typing import Protocol\n\nclass MyProto(Protocol):\n    def do_thing(self) -> None:\n        ...");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].language_kind, InterfaceKind::Protocol);
}

#[test]
fn test_python_cross_language_consistency() {
    let ir = extract_py("from typing import Protocol\n\nclass MyInterface(Protocol):\n    def do_thing(self) -> None:\n        ...");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.language, Language::Python);
}

//! Integration tests for Rust query extraction.
//! Each test parses a real Rust fixture through tree-sitter and asserts the IR output.

use std::path::Path;

use domain_scan_core::ir::*;
use domain_scan_core::parser::parse_source;
use domain_scan_core::query_engine::extract;

/// Helper: extract from source string
fn extract_rs(source: &str) -> IrFile {
    let tree = parse_source(source.as_bytes(), Language::Rust)
        .unwrap_or_else(|e| panic!("Failed to parse: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new("test.rs"),
        Language::Rust,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract: {e}"))
}

/// Helper: extract from fixture file
fn extract_fixture(filename: &str) -> IrFile {
    let fixture_path = format!(
        "{}/tests/fixtures/rust/{filename}",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture {filename}: {e}"));
    let tree = parse_source(source.as_bytes(), Language::Rust)
        .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
    extract(
        &tree,
        source.as_bytes(),
        Path::new(&fixture_path),
        Language::Rust,
        BuildStatus::Built,
    )
    .unwrap_or_else(|e| panic!("Failed to extract {filename}: {e}"))
}

// =========================================================================
// traits.scm tests
// =========================================================================

#[test]
fn test_rust_traits_count() {
    let ir = extract_fixture("traits.rs");
    assert_eq!(ir.interfaces.len(), 4, "Expected 4 traits");
}

#[test]
fn test_rust_trait_names() {
    let ir = extract_fixture("traits.rs");
    let names: Vec<&str> = ir.interfaces.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"EventHandler"));
    assert!(names.contains(&"Repository"));
    assert!(names.contains(&"Serializable"));
    assert!(names.contains(&"PrivateTrait"));
}

#[test]
fn test_rust_trait_kind() {
    let ir = extract_fixture("traits.rs");
    for iface in &ir.interfaces {
        assert_eq!(iface.language_kind, InterfaceKind::Trait);
    }
}

#[test]
fn test_rust_trait_visibility() {
    let ir = extract_fixture("traits.rs");
    let event_handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert_eq!(event_handler.visibility, Visibility::Public);

    let private = ir
        .interfaces
        .iter()
        .find(|i| i.name == "PrivateTrait")
        .expect("PrivateTrait not found");
    assert_eq!(private.visibility, Visibility::Private);
}

#[test]
fn test_rust_trait_methods() {
    let ir = extract_fixture("traits.rs");
    let event_handler = ir
        .interfaces
        .iter()
        .find(|i| i.name == "EventHandler")
        .expect("EventHandler not found");
    assert_eq!(event_handler.methods.len(), 2);
    let method_names: Vec<&str> = event_handler
        .methods
        .iter()
        .map(|m| m.name.as_str())
        .collect();
    assert!(method_names.contains(&"handle"));
    assert!(method_names.contains(&"name"));
}

#[test]
fn test_rust_trait_generics() {
    let ir = extract_fixture("traits.rs");
    let repo = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Repository")
        .expect("Repository not found");
    assert!(!repo.generics.is_empty(), "Repository should have generics");
}

#[test]
fn test_rust_trait_bounds() {
    let ir = extract_fixture("traits.rs");
    let serializable = ir
        .interfaces
        .iter()
        .find(|i| i.name == "Serializable")
        .expect("Serializable not found");
    assert!(
        !serializable.extends.is_empty(),
        "Serializable should have trait bounds"
    );
}

// =========================================================================
// impls.scm tests
// =========================================================================

#[test]
fn test_rust_impls_count() {
    let ir = extract_fixture("impls.rs");
    assert!(
        ir.implementations.len() >= 3,
        "Expected at least 3 impl blocks, got {}",
        ir.implementations.len()
    );
}

#[test]
fn test_rust_impl_trait_name() {
    let ir = extract_fixture("impls.rs");
    let event_impl = ir
        .implementations
        .iter()
        .find(|i| i.trait_name.as_deref() == Some("EventHandler"))
        .expect("EventHandler impl not found");
    assert_eq!(event_impl.target, "MyService");
}

#[test]
fn test_rust_inherent_impl() {
    let ir = extract_fixture("impls.rs");
    let inherent = ir
        .implementations
        .iter()
        .find(|i| i.target == "MyService" && i.trait_name.is_none())
        .expect("Inherent MyService impl not found");
    assert!(!inherent.methods.is_empty());
}

#[test]
fn test_rust_impl_method_async() {
    let ir = extract_fixture("impls.rs");
    let inherent = ir
        .implementations
        .iter()
        .find(|i| i.target == "MyService" && i.trait_name.is_none())
        .expect("Inherent impl not found");
    let process = inherent
        .methods
        .iter()
        .find(|m| m.name == "process")
        .expect("process method not found");
    assert!(process.is_async, "process should be async");
}

// =========================================================================
// functions.scm tests
// =========================================================================

#[test]
fn test_rust_functions_count() {
    let ir = extract_fixture("functions.rs");
    assert_eq!(ir.functions.len(), 5, "Expected 5 top-level functions");
}

#[test]
fn test_rust_function_names() {
    let ir = extract_fixture("functions.rs");
    let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"fetch_data"));
    assert!(names.contains(&"private_helper"));
    assert!(names.contains(&"process_items"));
    assert!(names.contains(&"crate_visible"));
}

#[test]
fn test_rust_function_async() {
    let ir = extract_fixture("functions.rs");
    let fetch = ir
        .functions
        .iter()
        .find(|f| f.name == "fetch_data")
        .expect("fetch_data not found");
    assert!(fetch.is_async, "fetch_data should be async");

    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "add")
        .expect("add not found");
    assert!(!add.is_async, "add should not be async");
}

#[test]
fn test_rust_function_visibility() {
    let ir = extract_fixture("functions.rs");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "add")
        .expect("add not found");
    assert_eq!(add.visibility, Visibility::Public);

    let private = ir
        .functions
        .iter()
        .find(|f| f.name == "private_helper")
        .expect("private_helper not found");
    assert_eq!(private.visibility, Visibility::Private);
}

#[test]
fn test_rust_function_parameters() {
    let ir = extract_fixture("functions.rs");
    let add = ir
        .functions
        .iter()
        .find(|f| f.name == "add")
        .expect("add not found");
    assert_eq!(add.parameters.len(), 2);
    assert_eq!(add.parameters[0].name, "a");
    assert_eq!(add.parameters[0].type_annotation.as_deref(), Some("i32"));
}

// =========================================================================
// types.scm tests
// =========================================================================

#[test]
fn test_rust_types_structs() {
    let ir = extract_fixture("types.rs");
    let struct_names: Vec<&str> = ir
        .classes
        .iter()
        .filter(|c| {
            !c.decorators.is_empty()
                || !c.properties.is_empty()
                || c.name
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
        })
        .map(|c| c.name.as_str())
        .collect();
    assert!(struct_names.contains(&"User"));
    assert!(struct_names.contains(&"Config"));
}

#[test]
fn test_rust_types_enum() {
    let ir = extract_fixture("types.rs");
    let status = ir
        .classes
        .iter()
        .find(|c| c.name == "Status")
        .expect("Status enum not found");
    assert_eq!(status.visibility, Visibility::Public);
}

#[test]
fn test_rust_struct_fields() {
    let ir = extract_fixture("types.rs");
    let user = ir
        .classes
        .iter()
        .find(|c| c.name == "User")
        .expect("User struct not found");
    assert_eq!(user.properties.len(), 4);
    let id = user
        .properties
        .iter()
        .find(|p| p.name == "id")
        .expect("id field not found");
    assert_eq!(id.type_annotation.as_deref(), Some("u64"));
    assert_eq!(id.visibility, Visibility::Public);
}

#[test]
fn test_rust_type_aliases() {
    let ir = extract_fixture("types.rs");
    assert!(
        ir.type_aliases.len() >= 2,
        "Expected at least 2 type aliases, got {}",
        ir.type_aliases.len()
    );
    let user_id = ir
        .type_aliases
        .iter()
        .find(|t| t.name == "UserId")
        .expect("UserId type alias not found");
    assert_eq!(user_id.target, "u64");
}

// =========================================================================
// imports.scm tests
// =========================================================================

#[test]
fn test_rust_imports_count() {
    let ir = extract_fixture("imports.rs");
    assert_eq!(ir.imports.len(), 5, "Expected 5 imports");
}

#[test]
fn test_rust_import_paths() {
    let ir = extract_fixture("imports.rs");
    let sources: Vec<&str> = ir.imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.iter().any(|s| s.contains("HashMap")));
    assert!(sources.iter().any(|s| s.contains("PathBuf")));
}

// =========================================================================
// schemas.scm tests
// =========================================================================

#[test]
fn test_rust_schemas_serde() {
    let ir = extract_fixture("schemas.rs");
    assert_eq!(
        ir.schemas.len(),
        2,
        "Expected 2 serde schemas (UserDto and CreateUserRequest)"
    );
}

#[test]
fn test_rust_schema_names() {
    let ir = extract_fixture("schemas.rs");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"UserDto"));
    assert!(names.contains(&"CreateUserRequest"));
}

#[test]
fn test_rust_schema_fields() {
    let ir = extract_fixture("schemas.rs");
    let user_dto = ir
        .schemas
        .iter()
        .find(|s| s.name == "UserDto")
        .expect("UserDto not found");
    assert_eq!(user_dto.fields.len(), 3);
    let email = user_dto
        .fields
        .iter()
        .find(|f| f.name == "email")
        .expect("email field not found");
    assert!(email.is_optional);
    assert_eq!(user_dto.source_framework, "serde");
}

#[test]
fn test_rust_schema_non_serde_excluded() {
    let ir = extract_fixture("schemas.rs");
    let names: Vec<&str> = ir.schemas.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&"InternalState"),
        "InternalState should not be a schema (no Serialize/Deserialize)"
    );
}

// =========================================================================
// Inline tests (minimal source)
// =========================================================================

#[test]
fn test_rust_simple_trait() {
    let ir = extract_rs("pub trait Foo { fn bar(&self) -> u32; }");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].name, "Foo");
    assert_eq!(ir.interfaces[0].methods.len(), 1);
}

#[test]
fn test_rust_simple_function() {
    let ir = extract_rs("fn hello() {}");
    assert_eq!(ir.functions.len(), 1);
    assert_eq!(ir.functions[0].name, "hello");
}

#[test]
fn test_rust_simple_struct() {
    let ir = extract_rs("pub struct Point { pub x: f64, pub y: f64 }");
    assert_eq!(ir.classes.len(), 1);
    assert_eq!(ir.classes[0].name, "Point");
    assert_eq!(ir.classes[0].properties.len(), 2);
}

#[test]
fn test_rust_cross_language_consistency() {
    // A Rust trait and a TS interface both produce InterfaceDef
    let ir = extract_rs("pub trait MyTrait { fn do_thing(&self); }");
    assert_eq!(ir.interfaces.len(), 1);
    assert_eq!(ir.interfaces[0].language_kind, InterfaceKind::Trait);
    assert_eq!(ir.language, Language::Rust);
}
